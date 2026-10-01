import { Image, Plus, Send, Square, X } from 'lucide-react';
import {
  useEffect,
  useId,
  useRef,
  useState,
  type ChangeEvent,
  type DragEvent,
  type FormEvent,
  type KeyboardEvent,
  type ClipboardEvent,
  type ReactNode,
} from 'react';
import type { NewAgentImageAttachmentDto } from '../../application/agentSessions';
import type { ComposerQuickFeatures } from './composerQuickActions';
import type { ComposerTargetSource } from './composerTargetActions';
import { useComposerQuickMenu } from './useComposerQuickMenu';
import { ComposerQuickMenu } from './ComposerQuickMenu';
import { ComposerHoverGroup } from './ComposerHoverGroup';
import './compactSessionChrome.css';

export interface AgentSessionComposerProps {
  toolbar?: ReactNode;
  quickFeatures?: ComposerQuickFeatures;
  targetSource?: ComposerTargetSource;
  draft: string;
  workingDirectory: string;
  isNewSession: boolean;
  sending: boolean;
  sendUnavailableReason?: string;
  active: boolean;
  steeringAvailable?: boolean;
  needsWorkingDirectory?: boolean;
  canceling: boolean;
  messageLabel?: string;
  messagePlaceholder?: string;
  showWorkingDirectory: boolean;
  keyboardHint: 'tooltip' | 'hidden';
  onDraftChange(value: string): void;
  onWorkingDirectoryChange(value: string): void;
  onSend(attachments: readonly NewAgentImageAttachmentDto[]): void;
  onSteer?(attachments: readonly NewAgentImageAttachmentDto[]): void;
  onCancel(): void;
}

export function AgentSessionComposer(props: AgentSessionComposerProps) {
  const picker = useRef<HTMLInputElement>(null);
  const [attachments, setAttachments] = useState<NewAgentImageAttachmentDto[]>([]);
  const [attachmentNotice, setAttachmentNotice] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const menuId = useId();
  const inputDisabled = false;
  const menu = useComposerQuickMenu(
    props.draft,
    props.onDraftChange,
    props.quickFeatures,
    inputDisabled,
    props.targetSource,
  );
  const submit = (event?: FormEvent) => {
    event?.preventDefault();
    if (menu.open) {
      menu.accept();
      return;
    }
    if (
      (props.draft.trim() || attachments.length) &&
      !props.sending &&
      !props.sendUnavailableReason
    ) {
      props.onSend(attachments);
      setAttachments([]);
    }
  };

  useEffect(() => {
    if (typeof window === 'undefined' || !('__TAURI_INTERNALS__' in window)) return;
    let disposed = false;
    let unlisten: (() => void) | undefined;
    void import('@tauri-apps/api/webview').then(async ({ getCurrentWebview }) => {
      unlisten = await getCurrentWebview().onDragDropEvent(async ({ payload }) => {
        if (disposed) return;
        if (payload.type === 'enter' || payload.type === 'over') setDragging(true);
        if (payload.type === 'leave') setDragging(false);
        if (payload.type !== 'drop') return;
        setDragging(false);
        const { invoke } = await import('@tauri-apps/api/core');
        try {
          const images = await Promise.all(
            payload.paths.map((path) =>
              invoke<NewAgentImageAttachmentDto>('read_agent_session_image', { path }),
            ),
          );
          appendAttachments(images);
        } catch (cause) {
          setAttachmentNotice(cause instanceof Error ? cause.message : String(cause));
        }
      });
    });
    return () => {
      disposed = true;
      unlisten?.();
    };
  }, []);

  const appendAttachments = (next: readonly NewAgentImageAttachmentDto[]) => {
    setAttachments((current) => {
      const combined = [...current];
      for (const image of next) {
        if (combined.length === 4) break;
        if (!combined.some((existing) => existing.dataBase64 === image.dataBase64))
          combined.push(image);
      }
      if (current.length + next.length > 4) setAttachmentNotice('Up to 4 images per message.');
      else setAttachmentNotice(null);
      return combined;
    });
  };

  const chooseImages = async (event: ChangeEvent<HTMLInputElement>) => {
    const files = [...(event.target.files ?? [])];
    event.target.value = '';
    try {
      appendAttachments(await Promise.all(files.map(readBrowserImage)));
    } catch (cause) {
      setAttachmentNotice(cause instanceof Error ? cause.message : String(cause));
    }
  };

  const handleDrop = async (event: DragEvent<HTMLFormElement>) => {
    event.preventDefault();
    setDragging(false);
    if (!event.dataTransfer.files.length) return;
    try {
      appendAttachments(await Promise.all([...event.dataTransfer.files].map(readBrowserImage)));
    } catch (cause) {
      setAttachmentNotice(cause instanceof Error ? cause.message : String(cause));
    }
  };
  const handlePaste = async (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const itemFiles = [...event.clipboardData.items]
      .filter((item) => item.kind === 'file' && item.type.startsWith('image/'))
      .map((item) => item.getAsFile())
      .filter((file): file is File => file !== null);
    const images = itemFiles.length
      ? itemFiles
      : [...event.clipboardData.files].filter((file) => file.type.startsWith('image/'));
    if (!images.length) return;
    event.preventDefault();
    try {
      const pasted = await Promise.all(
        images.map(async (file, index) => {
          const attachment = await readBrowserImage(file);
          return {
            ...attachment,
            name: file.name || `Pasted image ${index + 1}.${imageExtension(file.type)}`,
          };
        }),
      );
      appendAttachments(pasted);
    } catch (cause) {
      setAttachmentNotice(cause instanceof Error ? cause.message : String(cause));
    }
  };
  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.nativeEvent.isComposing || event.keyCode === 229) return;
    if (menu.keyDown(event)) return;
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      if (
        event.ctrlKey &&
        props.active &&
        props.steeringAvailable &&
        props.onSteer &&
        (props.draft.trim() || attachments.length) &&
        !props.sending
      ) {
        props.onSteer(attachments);
        setAttachments([]);
        return;
      }
      if (event.ctrlKey && props.active) {
        if (!props.sending)
          setAttachmentNotice(
            'Steering is not available while this turn is preparing. Your message has not been queued.',
          );
        return;
      }
      submit();
    }
  };

  const sendAction = (
    <span className="composer-send-action">
      <button
        className="send-agent-button"
        type="submit"
        disabled={
          (!props.draft.trim() && !attachments.length) ||
          props.sending ||
          Boolean(props.sendUnavailableReason)
        }
        aria-describedby={props.keyboardHint === 'tooltip' ? 'composer-keyboard-hint' : undefined}
      >
        <Send size={16} aria-hidden="true" />
        {props.sending ? 'Sending…' : props.active ? 'Queue' : 'Send'}
      </button>
      {props.keyboardHint === 'tooltip' && (
        <span className="composer-keyboard-tooltip" id="composer-keyboard-hint" role="tooltip">
          {props.quickFeatures || props.targetSource ? '/ for quick features. ' : ''}
          Enter to {props.active ? 'queue' : 'send'}.{' '}
          {props.active && props.steeringAvailable && props.onSteer
            ? 'Ctrl+Enter steers the current turn. '
            : ''}
          Shift+Enter adds a new line.
        </span>
      )}
    </span>
  );

  return (
    <form
      className={`agent-session-composer${dragging ? ' agent-session-composer--dragging' : ''}`}
      onSubmit={submit}
      onDragEnter={(event) => {
        event.preventDefault();
        setDragging(true);
      }}
      onDragOver={(event) => event.preventDefault()}
      onDragLeave={(event) => {
        if (!event.currentTarget.contains(event.relatedTarget as Node | null)) setDragging(false);
      }}
      onDrop={handleDrop}
      aria-label="Send a message"
    >
      {(props.isNewSession || props.needsWorkingDirectory) && props.showWorkingDirectory && (
        <label className="working-directory-field">
          <span>
            Working directory <small>optional</small>
          </span>
          <input
            value={props.workingDirectory}
            onChange={(event) => props.onWorkingDirectoryChange(event.target.value)}
            placeholder="C:\\path\\to\\workspace"
            aria-label="Working directory"
          />
        </label>
      )}
      <div className="composer-input-row">
        <ComposerQuickMenu menu={menu} id={menuId} />
        <textarea
          ref={menu.textarea}
          aria-controls={menu.open ? menuId : undefined}
          aria-haspopup={props.quickFeatures || props.targetSource ? 'listbox' : undefined}
          aria-autocomplete={props.quickFeatures || props.targetSource ? 'list' : undefined}
          aria-activedescendant={
            menu.open && menu.items.length ? `${menuId}-${menu.selectedIndex}` : undefined
          }
          value={props.draft}
          onChange={(event) => props.onDraftChange(event.target.value)}
          onKeyDown={handleKeyDown}
          onPaste={(event) => void handlePaste(event)}
          placeholder={props.messagePlaceholder ?? 'What would you like the agent to do?'}
          aria-label={props.messageLabel ?? 'Message'}
          disabled={inputDisabled}
          rows={3}
        />
        {attachments.length > 0 && (
          <ComposerHoverGroup
            className="composer-attachment-group"
            label="Attached images"
            summary={
              <>
                <img
                  src={`data:${attachments[0].mediaType};base64,${attachments[0].dataBase64}`}
                  alt={attachments[0].name}
                />
                {attachments.length > 1 && <span>+{attachments.length - 1}</span>}
              </>
            }
          >
            <ul className="composer-attachments" aria-label="Attached images">
              {attachments.map((attachment, index) => (
                <li key={`${attachment.name}-${index}`}>
                  <img
                    src={`data:${attachment.mediaType};base64,${attachment.dataBase64}`}
                    alt={attachment.name}
                  />
                  <span>{attachment.name}</span>
                  <button
                    type="button"
                    aria-label={`Remove ${attachment.name}`}
                    onClick={() =>
                      setAttachments((current) => current.filter((_, i) => i !== index))
                    }
                  >
                    <X size={14} aria-hidden="true" />
                  </button>
                </li>
              ))}
            </ul>
          </ComposerHoverGroup>
        )}
      </div>
      <div className={props.toolbar ? 'session-composer-toolbar' : 'composer-actions-row'}>
        <div className="composer-leading-actions">
          <input
            ref={picker}
            type="file"
            aria-label="Image files"
            accept="image/png,image/jpeg,image/gif,image/webp"
            multiple
            hidden
            onChange={(event) => void chooseImages(event)}
          />
          <button
            className="attach-image-button"
            type="button"
            aria-label="Attach images"
            title="Attach images"
            onClick={() => picker.current?.click()}
          >
            <Plus size={18} aria-hidden="true" />
          </button>
          {props.toolbar}
        </div>
        <div className="composer-actions">
          {props.active ? (
            <ComposerHoverGroup
              label="Turn actions"
              summary="Turn actions"
              className="composer-turn-group"
              trigger={sendAction}
            >
              <button
                className="cancel-agent-button"
                type="button"
                onClick={props.onCancel}
                disabled={props.canceling}
              >
                <Square size={15} aria-hidden="true" />
                {props.canceling ? 'Canceling…' : 'Cancel'}
              </button>
              {props.steeringAvailable && props.onSteer && (
                <button
                  className="steer-agent-button"
                  type="button"
                  disabled={(!props.draft.trim() && !attachments.length) || props.sending}
                  onClick={() => {
                    props.onSteer?.(attachments);
                    setAttachments([]);
                  }}
                >
                  Steer current turn
                </button>
              )}
            </ComposerHoverGroup>
          ) : (
            sendAction
          )}
        </div>
      </div>
      {menu.notice && (
        <p className="composer-quick-notice" role="status">
          {menu.notice}
        </p>
      )}
      {dragging && (
        <div className="composer-drop-overlay">
          <Image size={20} aria-hidden="true" /> Drop images to attach
        </div>
      )}
      {attachmentNotice && (
        <p className="composer-attachment-notice" role="status">
          {attachmentNotice}
        </p>
      )}
      {props.sendUnavailableReason ? <p role="status">{props.sendUnavailableReason}</p> : null}
    </form>
  );
}

const MAX_IMAGE_BYTES = 10 * 1024 * 1024;
const SUPPORTED_IMAGE_TYPES = new Set(['image/png', 'image/jpeg', 'image/gif', 'image/webp']);

async function readBrowserImage(file: File): Promise<NewAgentImageAttachmentDto> {
  if (!SUPPORTED_IMAGE_TYPES.has(file.type))
    throw new Error(`${file.name} is not a supported image.`);
  if (file.size > MAX_IMAGE_BYTES) throw new Error(`${file.name} is larger than 10 MB.`);
  const bytes = new Uint8Array(await file.arrayBuffer());
  let binary = '';
  for (let offset = 0; offset < bytes.length; offset += 0x8000)
    binary += String.fromCharCode(...bytes.subarray(offset, offset + 0x8000));
  const digest = new Uint8Array(await crypto.subtle.digest('SHA-256', bytes));
  const id = [...digest].map((value) => value.toString(16).padStart(2, '0')).join('');
  return { id, name: file.name, mediaType: file.type, dataBase64: btoa(binary) };
}

function imageExtension(mediaType: string): string {
  if (mediaType === 'image/jpeg') return 'jpg';
  return mediaType.slice('image/'.length) || 'png';
}
