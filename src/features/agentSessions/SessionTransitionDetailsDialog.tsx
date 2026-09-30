import { ModalDialog } from '../../components/ModalDialog';
import { agentProviderLabel } from '../../application/agentProviders';
import { targetActionLabel } from './sessionQueueLabels';
import { transitionLabel, type SessionTransition } from './sessionHistoryProjector';
export function SessionTransitionDetailsDialog({
  transition,
  onClose,
}: {
  transition: SessionTransition;
  onClose(): void;
}) {
  const receipt = transition.kind === 'workspace' ? transition.receipt : null;
  const operation = transition.kind === 'blocked' ? transition.operation : null;
  const destination = receipt?.destination ?? operation?.stagedTarget;
  return (
    <ModalDialog
      labelledBy="session-transition-title"
      onClose={onClose}
      className="session-transition-details"
    >
      <header>
        <h2 id="session-transition-title">{transitionLabel(transition)}</h2>
        <button type="button" onClick={onClose} aria-label="Close transition details">
          ×
        </button>
      </header>
      {transition.kind === 'provider' ? (
        <>
          <p>{agentProviderLabel(transition.provider)} took over this conversation.</p>
          <dl>
            <dt>Previous provider</dt>
            <dd>{agentProviderLabel(transition.previous)}</dd>
            <dt>Delivered turn</dt>
            <dd>{transition.invocationId}</dd>
          </dl>
        </>
      ) : (
        <>
          <p>{receipt ? 'Completed successfully.' : operation?.error}</p>
          {destination && (
            <dl>
              <dt>Device</dt>
              <dd>{destination.execution.deviceName}</dd>
              <dt>Branch</dt>
              <dd>{destination.branchRef || 'Session folder'}</dd>
              <dt>Worktree</dt>
              <dd>{destination.path}</dd>
            </dl>
          )}
          {receipt?.source && (
            <p>
              From {receipt.source.execution.deviceName} · {receipt.source.path}
            </p>
          )}
          {receipt && <p>{new Date(receipt.completedAt).toLocaleString()}</p>}
          <details>
            <summary>Actual steps and diagnostics</summary>
            <ol>
              {(receipt?.actions ?? operation?.actions ?? []).map((action) => (
                <li key={action.kind}>
                  {targetActionLabel(action.kind)} · {action.status}
                  {action.detail && <p>{action.detail}</p>}
                  {action.error && <p>{action.error}</p>}
                </li>
              ))}
            </ol>
          </details>
        </>
      )}
    </ModalDialog>
  );
}
