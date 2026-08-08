import { useEffect, useRef, useState } from 'react';

export function EpicInitiationConfirmationModal({
  confirmation,
}: {
  readonly confirmation: ReturnType<
    typeof import('./useEpicInitiationConfirmation').useEpicInitiationConfirmation
  >;
}) {
  const confirmRef = useRef<HTMLButtonElement>(null);
  const rootBranchRef = useRef<HTMLInputElement>(null);
  const priorFocus = useRef<HTMLElement | null>(null);
  const current = confirmation.current;
  const requestId = current?.request.requestId;
  const [rootBranch, setRootBranch] = useState('');
  useEffect(() => {
    if (!requestId) return;
    setRootBranch('');
    priorFocus.current = document.activeElement as HTMLElement | null;
    rootBranchRef.current?.focus();
    return () => priorFocus.current?.focus();
  }, [requestId]);
  if (!current) return null;
  const reject = () => void confirmation.resolve('rejected');
  return (
    <div
      className="epic-initiation-confirmation"
      role="presentation"
      onMouseDown={(event) => {
        if (event.target === event.currentTarget && !confirmation.resolving) reject();
      }}
    >
      <section
        className="epic-initiation-confirmation__dialog"
        role="dialog"
        aria-modal="true"
        aria-labelledby="epic-initiation-confirmation-title"
        aria-describedby="epic-initiation-confirmation-description"
        onKeyDown={(event) => {
          if (event.key === 'Escape' && !confirmation.resolving) reject();
          if (event.key !== 'Tab') return;
          const buttons = Array.from(
            event.currentTarget.querySelectorAll<HTMLElement>(
              'input:not(:disabled), button:not(:disabled)',
            ),
          );
          if (!buttons.length) return;
          const first = buttons[0];
          const last = buttons.at(-1)!;
          if (event.shiftKey && document.activeElement === first) {
            event.preventDefault();
            last.focus();
          } else if (!event.shiftKey && document.activeElement === last) {
            event.preventDefault();
            first.focus();
          }
        }}
      >
        <p className="eyebrow">Epic initiation confirmation</p>
        <h2 id="epic-initiation-confirmation-title">Initiate this Epic?</h2>
        <p id="epic-initiation-confirmation-description">
          This confirms the current durable proposal. It does not start a product Sprint.
        </p>
        <dl>
          <div>
            <dt>Epic</dt>
            <dd>{current.details?.title ?? current.request.epicPlanningDraftId}</dd>
          </div>
          <div>
            <dt>Requested by</dt>
            <dd>
              {current.request.source.kind === 'agent'
                ? 'Epic Plan Builder agent'
                : 'Initiate button'}
            </dd>
          </div>
          <div>
            <dt>Proposed scope</dt>
            <dd>
              {current.details
                ? `${current.details.sprintTitles.length} Sprint${current.details.sprintTitles.length === 1 ? '' : 's'}: ${current.details.sprintTitles.join(', ')}`
                : current.detailsUnavailable
                  ? 'Current proposal details are unavailable; the durable draft identity is shown above.'
                  : 'Loading current proposal details…'}
            </dd>
          </div>
        </dl>
        <label htmlFor="epic-confirmation-root-branch">Epic root branch</label>
        <input
          ref={rootBranchRef}
          id="epic-confirmation-root-branch"
          value={rootBranch}
          onChange={(event) => setRootBranch(event.target.value)}
          placeholder="codex/epic-workflow-ux-test"
        />
        {confirmation.queuedCount > 0 && (
          <p role="status">
            {confirmation.queuedCount} other confirmation request
            {confirmation.queuedCount === 1 ? '' : 's'} waiting.
          </p>
        )}
        {confirmation.error && <p role="alert">{confirmation.error}</p>}
        <div className="epic-initiation-confirmation__actions">
          <button type="button" disabled={confirmation.resolving} onClick={reject}>
            Cancel
          </button>
          <button
            ref={confirmRef}
            type="button"
            disabled={confirmation.resolving || !rootBranch.trim()}
            onClick={() => void confirmation.resolve('confirmed', rootBranch)}
          >
            {confirmation.resolving ? 'Resolving…' : 'Confirm initiation'}
          </button>
        </div>
      </section>
    </div>
  );
}
