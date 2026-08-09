import type { AgentSessionProductLocation } from '../application/agentSessionNavigation';
import type { EpicPlanningDraftSummary } from '../application/orchestrations';
import type { OrchestrationLoadState } from './useOrchestrationLoad';

export interface OrchestrationRecoveryProps {
  readonly load: Exclude<OrchestrationLoadState, { readonly kind: 'ready' }>;
  readonly currentLocation?: AgentSessionProductLocation | null;
  readonly planningDrafts?: readonly EpicPlanningDraftSummary[];
  readonly onPlanEpic?: () => void;
  onOpenDraft?(draft: EpicPlanningDraftSummary): void;
  /** Re-queries the unavailable read while retaining the typed work-unit destination. */
  onReturnToCurrentWorkUnit?(): void;
}

/** A read failure has recovery actions, but never invents an Epic or a work location. */
export function OrchestrationRecovery({
  load,
  currentLocation = null,
  planningDrafts = [],
  onPlanEpic,
  onOpenDraft,
  onReturnToCurrentWorkUnit,
}: OrchestrationRecoveryProps) {
  const loading = load.kind === 'loading';
  const unavailable = load.kind === 'unavailable' || load.kind === 'failed';
  const diagnostic =
    load.kind === 'failed' ? load.message : 'reason' in load ? load.reason : undefined;
  const currentWorkUnit = currentLocation?.kind === 'work_unit';
  const canPlanWithoutKnownRoute =
    currentLocation === null && planningDrafts.length === 0 && Boolean(onPlanEpic);

  return (
    <main
      className="orchestration-section orchestration-recovery"
      aria-label="Orchestration"
      aria-busy={loading}
    >
      <header className="orchestration-page-header">
        <p className="eyebrow">Orchestration</p>
        <h1>
          {loading
            ? 'Loading orchestration data'
            : unavailable
              ? 'Orchestration data unavailable'
              : 'No orchestration data'}
        </h1>
        <p role={loading || !unavailable ? 'status' : 'alert'}>
          {loading
            ? 'Loading orchestration data…'
            : unavailable
              ? 'The current orchestration view cannot be shown right now. Its status is unknown until Retry succeeds.'
              : 'No orchestration records are available.'}
        </p>
        <div
          className="orchestration-recovery__actions"
          aria-label="Orchestration recovery actions"
        >
          {unavailable && (
            <button
              className="orchestration-page-header__plan"
              type="button"
              onClick={() => void load.refresh()}
            >
              Retry
            </button>
          )}
          {currentWorkUnit && onReturnToCurrentWorkUnit && (
            <button type="button" onClick={onReturnToCurrentWorkUnit}>
              Return to current Work Unit
            </button>
          )}
          {canPlanWithoutKnownRoute && (
            <button type="button" onClick={onPlanEpic}>
              Plan an Epic
            </button>
          )}
        </div>
        {diagnostic && !loading && (
          <details className="orchestration-recovery__technical-details">
            <summary>Technical details</summary>
            <p>{diagnostic}</p>
          </details>
        )}
      </header>
      {planningDrafts.length > 0 && (
        <section className="orchestration-list" aria-label="Active Epic planning drafts">
          <table>
            <tbody>
              {planningDrafts.map((draft) => (
                <tr key={draft.epicPlanningDraftId}>
                  <td>
                    <button
                      className="orchestration-list__open"
                      type="button"
                      onClick={() => onOpenDraft?.(draft)}
                    >
                      <strong>{draft.title ?? 'Untitled Epic draft'}</strong>
                      <small>Pre-initiation planning draft</small>
                    </button>
                  </td>
                  <td>Planning</td>
                  <td>Draft</td>
                </tr>
              ))}
            </tbody>
          </table>
        </section>
      )}
    </main>
  );
}
