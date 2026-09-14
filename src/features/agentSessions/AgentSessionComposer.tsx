import { Send, Square } from 'lucide-react';
import { useId, type FormEvent, type KeyboardEvent } from 'react';
import type { ComposerQuickFeatures } from './composerQuickActions';
import type { ComposerTargetSource } from './composerTargetActions';
import { useComposerQuickMenu } from './useComposerQuickMenu';
import { ComposerQuickMenu } from './ComposerQuickMenu';

export interface AgentSessionComposerProps {
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
  onSend(): void;
  onCancel(): void;
}

export function AgentSessionComposer(props: AgentSessionComposerProps) {
  const menuId = useId();
  const selectedOptions = props.quickFeatures
    ? [props.quickFeatures.selection.model, props.quickFeatures.selection.reasoningMode]
        .filter(Boolean)
        .join(' · ')
    : '';
  const inputDisabled = (props.active && !props.steeringAvailable) || props.sending;
  const menu = useComposerQuickMenu(
    props.draft,
    props.onDraftChange,
    props.quickFeatures,
    props.active,
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
      props.draft.trim() &&
      !props.sending &&
      (!props.active || props.steeringAvailable) &&
      !props.sendUnavailableReason
    )
      props.onSend();
  };
  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.nativeEvent.isComposing || event.keyCode === 229) return;
    if (menu.keyDown(event)) return;
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  };

  return (
    <form className="agent-session-composer" onSubmit={submit} aria-label="Send a message">
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
          placeholder={props.messagePlaceholder ?? 'What would you like the agent to do?'}
          aria-label={props.messageLabel ?? 'Message'}
          disabled={inputDisabled}
          rows={4}
        />
        <div className="composer-actions">
          {props.active && (
            <button
              className="cancel-agent-button"
              type="button"
              onClick={props.onCancel}
              disabled={props.canceling}
            >
              <Square size={15} aria-hidden="true" />
              {props.canceling ? 'Canceling…' : 'Cancel'}
            </button>
          )}
          {(!props.active || props.steeringAvailable) && (
            <span className="composer-send-action">
              <button
                className="send-agent-button"
                type="submit"
                disabled={
                  !props.draft.trim() || props.sending || Boolean(props.sendUnavailableReason)
                }
                aria-describedby={
                  props.keyboardHint === 'tooltip' ? 'composer-keyboard-hint' : undefined
                }
              >
                <Send size={16} aria-hidden="true" />
                {props.sending ? 'Sending…' : props.active ? 'Steer' : 'Send'}
              </button>
              {props.keyboardHint === 'tooltip' && (
                <span
                  className="composer-keyboard-tooltip"
                  id="composer-keyboard-hint"
                  role="tooltip"
                >
                  {props.quickFeatures || props.targetSource ? '/ for quick features. ' : ''}Enter
                  to send. Shift+Enter adds a new line.
                </span>
              )}
            </span>
          )}
        </div>
      </div>
      <p className="composer-hint">Enter to send · Shift+Enter for a new line</p>
      {(props.quickFeatures || props.targetSource) && (
        <p className="composer-quick-notice" role="status">
          {menu.notice ||
            (selectedOptions ? `Next message: ${selectedOptions}` : '') ||
            (props.targetSource
              ? 'Type /worktree or /device to choose a target. Use / for all quick features.'
              : 'Type / for model, reasoning, and skills')}
        </p>
      )}
      {props.sendUnavailableReason ? <p role="status">{props.sendUnavailableReason}</p> : null}
    </form>
  );
}
