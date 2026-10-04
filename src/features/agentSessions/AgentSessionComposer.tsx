import { FileText, Image, Plus, Send, Square, X } from 'lucide-react';
import {
  useEffect,
  useLayoutEffect,
  useId,
  useRef,
  useState,
  useSyncExternalStore,
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
import { useComposerAutosize } from './useComposerAutosize';
import type { ComposerDraftStore } from './composerDraftStore';
import { SkillReferenceText, skillReferenceParts, useSkillReferences } from './SkillReferences';
import {
  isImageAttachment,
  isLongPaste,
  readBrowserAttachment,
  readComposerClipboard,
  textAttachment,
} from './composerAttachments';

export interface AgentSessionComposerProps {
  toolbar?: ReactNode;
  quickFeatures?: ComposerQuickFeatures;
  targetSource?: ComposerTargetSource;
  draft: string;
  draftSource?: ComposerDraftStore;
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
  const draft = useSyncExternalStore(
    props.draftSource?.subscribe ?? (() => () => {}),
    props.draftSource?.getSnapshot ?? (() => props.draft),
    props.draftSource?.getSnapshot ?? (() => props.draft),
  );
  props = { ...props, draft };
  const skills = useSkillReferences().skills;
  const hasSkillReferences =
    props.draft.includes('$') &&
    skillReferenceParts(props.draft, skills).some((part) => part.skill);
  const skillMirror = useRef<HTMLDivElement>(null);
  const picker = useRef<HTMLInputElement>(null);
  const [attachments, setAttachments] = useState<NewAgentImageAttachmentDto[]>([]);
  const [attachmentNotice, setAttachmentNotice] = useState<string | null>(null);
  const [dragging, setDragging] = useState(false);
  const [clipboardText, setClipboardText] = useState<string | null>(null);
  const [pasteMenu, setPasteMenu] = useState<{ x: number; y: number } | null>(null);
  const refreshClipboard = () =>
    void readComposerClipboard()
      .then((value) => setClipboardText(value.text))
      .catch(() => setClipboardText(null));
  const pasteAsFile = async () => {
    try {
      const { text } = await readComposerClipboard();
      if (text) appendAttachments([await textAttachment(text)]);
    } catch (cause) {
      setAttachmentNotice(String(cause));
    }
    setPasteMenu(null);
  };
  const menuId = useId();
  const inputDisabled = false;
  const menu = useComposerQuickMenu(
    props.draft,
    props.onDraftChange,
    props.quickFeatures,
    inputDisabled,
    props.targetSource,
  );
  useComposerAutosize(menu.textarea, props.draft);
  useLayoutEffect(() => {
    const input = menu.textarea.current;
    const mirror = skillMirror.current;
    if (!input || !mirror) return;
    const align = () => {
      // Exclude the native scrollbar from the mirrored text's wrapping width.
      mirror.style.width = `${input.clientWidth}px`;
      mirror.scrollTop = input.scrollTop;
    };
    align();
    const observer = typeof ResizeObserver === 'undefined' ? null : new ResizeObserver(align);
    observer?.observe(input);
    return () => observer?.disconnect();
  }, [hasSkillReferences, menu.textarea]);
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
      if (current.length + next.length > 4) setAttachmentNotice('Up to 4 attachments per message.');
      else setAttachmentNotice(null);
      return combined;
    });
  };

  const chooseImages = async (event: ChangeEvent<HTMLInputElement>) => {
    const files = [...(event.target.files ?? [])];
    event.target.value = '';
    try {
      appendAttachments(await Promise.all(files.map(readBrowserAttachment)));
    } catch (cause) {
      setAttachmentNotice(cause instanceof Error ? cause.message : String(cause));
    }
  };

  const handleDrop = async (event: DragEvent<HTMLFormElement>) => {
    event.preventDefault();
    setDragging(false);
    if (!event.dataTransfer.files.length) return;
    try {
      appendAttachments(
        await Promise.all([...event.dataTransfer.files].map(readBrowserAttachment)),
      );
    } catch (cause) {
      setAttachmentNotice(cause instanceof Error ? cause.message : String(cause));
    }
  };
  const handlePaste = async (event: ClipboardEvent<HTMLTextAreaElement>) => {
    const itemFiles = [...event.clipboardData.items]
      .filter((item) => item.kind === 'file')
      .map((item) => item.getAsFile())
      .filter((file): file is File => file !== null);
    const images = itemFiles.length ? itemFiles : [...event.clipboardData.files];
    const text = event.clipboardData.getData?.('text/plain') ?? '';
    if (!images.length && isLongPaste(text)) {
      event.preventDefault();
      try {
        appendAttachments([await textAttachment(text)]);
      } catch (cause) {
        setAttachmentNotice(String(cause));
      }
      return;
    }
    if (!images.length) {
      if (!text && '__TAURI_INTERNALS__' in window) {
        event.preventDefault();
        try {
          const clipboard = await readComposerClipboard();
          const { invoke } = await import('@tauri-apps/api/core');
          appendAttachments(
            await Promise.all(
              clipboard.paths.map((path) =>
                invoke<NewAgentImageAttachmentDto>('read_agent_session_image', { path }),
              ),
            ),
          );
        } catch (cause) {
          setAttachmentNotice(String(cause));
        }
      }
      return;
    }
    event.preventDefault();
    try {
      const pasted = await Promise.all(
        images.map(async (file, index) => {
          const attachment = await readBrowserAttachment(file);
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
        <div className="composer-text-editor">
          {hasSkillReferences && (
            <div className="composer-skill-mirror" ref={skillMirror}>
              <SkillReferenceText text={props.draft} />
              {'\n'}
            </div>
          )}
          <textarea
            className={hasSkillReferences ? 'composer-skill-input' : undefined}
            onScroll={(event) => {
              if (skillMirror.current)
                skillMirror.current.scrollTop = event.currentTarget.scrollTop;
            }}
            ref={menu.textarea}
            aria-controls={menu.open ? menuId : undefined}
            aria-haspopup={props.quickFeatures || props.targetSource ? 'listbox' : undefined}
            aria-autocomplete={props.quickFeatures || props.targetSource ? 'list' : undefined}
            aria-activedescendant={
              menu.open && menu.items.length ? `${menuId}-${menu.selectedIndex}` : undefined
            }
            value={props.draft}
            onChange={(event) => {
              menu.setCaret(event.target.selectionStart);
              props.onDraftChange(event.target.value);
            }}
            onSelect={(event) => menu.setCaret(event.currentTarget.selectionStart)}
            onKeyDown={handleKeyDown}
            onPaste={(event) => void handlePaste(event)}
            onContextMenu={(event) => {
              event.preventDefault();
              refreshClipboard();
              setPasteMenu({ x: event.clientX, y: event.clientY });
            }}
            placeholder={props.messagePlaceholder ?? 'What would you like the agent to do?'}
            aria-label={props.messageLabel ?? 'Message'}
            disabled={inputDisabled}
            rows={3}
          />
        </div>
        {pasteMenu && (
          <div className="composer-paste-backdrop" onPointerDown={() => setPasteMenu(null)}>
            <div
              className="session-context-menu composer-paste-menu"
              role="menu"
              style={{ left: pasteMenu.x, top: pasteMenu.y }}
              onPointerDown={(event) => event.stopPropagation()}
            >
              <button
                type="button"
                role="menuitem"
                disabled={!clipboardText}
                onClick={() => void pasteAsFile()}
              >
                Paste as text file
              </button>
            </div>
          </div>
        )}
        {attachments.length > 0 && (
          <ComposerHoverGroup
            className="composer-attachment-group"
            label="Attachments"
            summary={
              <>
                {isImageAttachment(attachments[attachments.length - 1]) ? (
                  <img
                    src={`data:${attachments[attachments.length - 1].mediaType};base64,${attachments[attachments.length - 1].dataBase64}`}
                    alt={attachments[attachments.length - 1].name}
                  />
                ) : (
                  <FileText size={24} aria-label={attachments[attachments.length - 1].name} />
                )}
                {attachments.length > 1 && <span>+{attachments.length - 1}</span>}
              </>
            }
          >
            <ul className="composer-attachments" aria-label="Attachments">
              {attachments.map((attachment, index) => (
                <li key={`${attachment.name}-${index}`}>
                  {isImageAttachment(attachment) ? (
                    <img
                      src={`data:${attachment.mediaType};base64,${attachment.dataBase64}`}
                      alt={attachment.name}
                    />
                  ) : (
                    <FileText size={24} aria-hidden="true" />
                  )}
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
            aria-label="Attachment files"
            multiple
            hidden
            onChange={(event) => void chooseImages(event)}
          />
          <span onMouseEnter={refreshClipboard} onFocus={refreshClipboard}>
            <ComposerHoverGroup
              label="Add attachment"
              summary={<Plus size={18} aria-hidden="true" />}
              className="composer-add-group"
            >
              <button type="button" onClick={() => picker.current?.click()}>
                Attach file
              </button>
              <button type="button" disabled={!clipboardText} onClick={() => void pasteAsFile()}>
                Paste as text file
              </button>
            </ComposerHoverGroup>
          </span>
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
          <Image size={20} aria-hidden="true" /> Drop files to attach
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

function imageExtension(mediaType: string): string {
  if (mediaType === 'image/jpeg') return 'jpg';
  return mediaType.slice('image/'.length) || 'png';
}
