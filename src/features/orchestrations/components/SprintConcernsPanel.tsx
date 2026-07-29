import { ArrowLeft } from 'lucide-react';
import { useEffect, useRef } from 'react';
import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import '../styles/sprintInformationSurfaces.css';

export function SprintConcernsPanel({
  workspace,
  selectedConcernId,
  onSelectConcern,
  onOpenWorkUnit,
}: {
  readonly workspace: SprintWorkspacePresentationV1;
  readonly selectedConcernId: string | null;
  readonly onSelectConcern: (id: string | null) => void;
  readonly onOpenWorkUnit: (id: string, opener: HTMLButtonElement) => void;
}) {
  const originatingConcernRef = useRef<string | null>(null);
  const concernRefs = useRef(new Map<string, HTMLButtonElement>());
  const selected = workspace.concerns.find(({ concernId }) => concernId === selectedConcernId);

  useEffect(() => {
    if (selectedConcernId || !originatingConcernRef.current) return;
    concernRefs.current.get(originatingConcernRef.current)?.focus();
    originatingConcernRef.current = null;
  }, [selectedConcernId]);

  if (selected) {
    const selectedView = workspace.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === workspace.activeSprintPlanRevisionId,
    );
    const linked = selected.requiredWorkUnitIds
      .map((id) => selectedView?.workUnits.find(({ workUnitId }) => workUnitId === id))
      .filter((unit) => unit !== undefined);
    return (
      <div className="sprint-concern-detail" aria-label={`Concern detail: ${selected.title}`}>
        <button
          type="button"
          className="sprint-concern-detail__back"
          onClick={() => {
            originatingConcernRef.current = selected.concernId;
            onSelectConcern(null);
          }}
        >
          <ArrowLeft size={15} aria-hidden="true" />
          Back to concerns
        </button>
        <header>
          <span className={`sprint-semantic-state sprint-semantic-state--${selected.state}`}>
            {concernStateLabel(selected.state)}
          </span>
          <h2>{selected.title}</h2>
          <p>{selected.summary}</p>
        </header>
        <p className="sprint-concern-detail__body">{selected.details}</p>
        <section aria-label="Linked Work Units">
          <h3>Linked Work Units</h3>
          <div className="sprint-concern-detail__units">
            {linked.map((unit) => (
              <button
                key={unit.workUnitId}
                type="button"
                className="sprint-concern-work-unit"
                data-concern-work-unit-id={unit.workUnitId}
                onClick={(event) => onOpenWorkUnit(unit.workUnitId, event.currentTarget)}
              >
                <span>
                  <code>{unit.workUnitId}</code>
                  <strong>{unit.title}</strong>
                </span>
                <span>{workUnitStateLabel(unit.presentationState)}</span>
              </button>
            ))}
          </div>
        </section>
      </div>
    );
  }

  return (
    <div className="sprint-concerns-overview" aria-label="Sprint concerns overview">
      <div className="sprint-concerns-grid">
        {workspace.concerns.map((concern) => (
          <button
            key={concern.concernId}
            ref={(node) => {
              if (node) concernRefs.current.set(concern.concernId, node);
              else concernRefs.current.delete(concern.concernId);
            }}
            type="button"
            className="sprint-concern-card"
            data-concern-id={concern.concernId}
            onClick={() => {
              originatingConcernRef.current = concern.concernId;
              onSelectConcern(concern.concernId);
            }}
          >
            <span className={`sprint-semantic-state sprint-semantic-state--${concern.state}`}>
              {concernStateLabel(concern.state)}
            </span>
            <strong>{concern.title}</strong>
            <span>{concern.summary}</span>
            <small>
              {concern.requiredWorkUnitIds.length} linked Work Unit
              {concern.requiredWorkUnitIds.length === 1 ? '' : 's'}
            </small>
          </button>
        ))}
      </div>
    </div>
  );
}

function concernStateLabel(
  state:
    | SprintWorkspacePresentationV1['concerns'][number]['state']
    | SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['presentationState'],
) {
  return (
    {
      not_started: 'Planned',
      waiting_for_dependencies: 'Planned',
      requested: 'Processing',
      launched: 'Processing',
      returned: 'Processing',
      under_review: 'Processing',
      integrated: 'Completed',
      responsibility_accepted: 'Completed',
      deferred: 'Deferred',
    } as const
  )[state];
}

function workUnitStateLabel(
  state: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number]['presentationState'],
) {
  return (
    {
      not_started: 'Planned',
      waiting_for_dependencies: 'Planned · waiting for dependencies',
      requested: 'Processing · requested',
      launched: 'Processing · working',
      returned: 'Processing · returned',
      under_review: 'Processing · under review',
      integrated: 'Completed · integrated',
      responsibility_accepted: 'Completed',
      deferred: 'Planned · deferred',
    } as const
  )[state];
}
