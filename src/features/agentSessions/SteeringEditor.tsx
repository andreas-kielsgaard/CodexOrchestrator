import { useId, useState } from 'react';
import { ModalDialog } from '../../components/ModalDialog';
import type { SessionInteractionDto } from '../../application/agentSessions';
import type { AgentSessionWorkspaceController } from './useAgentSession';

export function SteeringEditor({
  interaction,
  controller,
  onClose,
}: {
  interaction: SessionInteractionDto;
  controller: AgentSessionWorkspaceController;
  onClose(): void;
}) {
  const heading = useId();
  const [text, setText] = useState(interaction.content.text ?? '');
  const [busy, setBusy] = useState(false);
  const [error, setError] = useState<string | null>(null);
  const act = async (action: 'save' | 'resume' | 'cancel') => {
    setBusy(true);
    setError(null);
    try {
      await controller.editSteering!(interaction, action, action === 'save' ? text : undefined);
      onClose();
    } catch (cause) {
      setError(String(cause));
    } finally {
      setBusy(false);
    }
  };
  return (
    <ModalDialog
      labelledBy={heading}
      dismissible={!busy}
      onClose={() => void act('resume')}
      className="session-queue-editor"
    >
      <h2 id={heading}>Edit steering message</h2>
      <p>Delivery is paused while you edit. Saving queues it for the next activity boundary.</p>
      <textarea
        aria-label="Steering message"
        value={text}
        onChange={(e) => setText(e.target.value)}
        autoFocus
      />
      {error && <p role="alert">{error}</p>}
      <footer>
        <button disabled={busy} onClick={() => void act('cancel')}>
          Remove
        </button>
        <button disabled={busy} onClick={() => void act('resume')}>
          Cancel editing
        </button>
        <button
          disabled={busy || (!text.trim() && !interaction.content.attachments?.length)}
          onClick={() => void act('save')}
        >
          Save steering
        </button>
      </footer>
    </ModalDialog>
  );
}
