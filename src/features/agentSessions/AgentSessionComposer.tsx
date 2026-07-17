import { Send, Square } from 'lucide-react';
import type { FormEvent, KeyboardEvent } from 'react';

export interface AgentSessionComposerProps {
  draft: string;
  workingDirectory: string;
  isNewSession: boolean;
  sending: boolean;
  active: boolean;
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
  const submit = (event?: FormEvent) => {
    event?.preventDefault();
    if (props.draft.trim() && !props.sending && !props.active) props.onSend();
  };
  const handleKeyDown = (event: KeyboardEvent<HTMLTextAreaElement>) => {
    if (event.key === 'Enter' && !event.shiftKey) {
      event.preventDefault();
      submit();
    }
  };

  return (
    <form className="agent-session-composer" onSubmit={submit} aria-label="Send a message">
      {props.isNewSession && props.showWorkingDirectory && (
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
        <textarea
          value={props.draft}
          onChange={(event) => props.onDraftChange(event.target.value)}
          onKeyDown={handleKeyDown}
          placeholder={props.messagePlaceholder ?? 'What would you like the agent to do?'}
          aria-label={props.messageLabel ?? 'Message'}
          disabled={props.active || props.sending}
          rows={4}
        />
        {props.active ? (
          <button
            className="cancel-agent-button"
            type="button"
            onClick={props.onCancel}
            disabled={props.canceling}
          >
            <Square size={15} aria-hidden="true" />
            {props.canceling ? 'Canceling…' : 'Cancel'}
          </button>
        ) : (
          <span className="composer-send-action">
            <button
              className="send-agent-button"
              type="submit"
              disabled={!props.draft.trim() || props.sending}
              aria-describedby={
                props.keyboardHint === 'tooltip' ? 'composer-keyboard-hint' : undefined
              }
            >
              <Send size={16} aria-hidden="true" />
              {props.sending ? 'Sending…' : 'Send'}
            </button>
            {props.keyboardHint === 'tooltip' && (
              <span
                className="composer-keyboard-tooltip"
                id="composer-keyboard-hint"
                role="tooltip"
              >
                Enter to send. Shift+Enter adds a new line.
              </span>
            )}
          </span>
        )}
      </div>
      <p className="composer-hint">Enter to send · Shift+Enter for a new line</p>
    </form>
  );
}
