import { useMemo } from 'react';
import {
  AlertOctagon,
  Check,
  Circle,
  Clock3,
  LoaderCircle,
  Pause,
  SearchCheck,
  UserRoundCheck,
} from 'lucide-react';
import type {
  SprintWorkspacePresentationV1,
  ProductWorkUnitPresentationState,
} from '../../../application/orchestrations';
import { projectSprintConnectorRoutes, projectSprintFlowLayout } from '../sprintFlowLayout';
import '../styles/sprintFlowMap.css';

export interface SprintFlowMapProps {
  readonly workspace: SprintWorkspacePresentationV1;
  readonly selectedRevisionId: string;
  readonly onSelectedRevisionChange: (revisionId: string) => void;
  readonly onOpenSprintPlannerActivityGroup?: (
    sprintPlannerActivityId: string,
    opener: HTMLButtonElement,
  ) => void;
  readonly onOpenWorkUnit?: (workUnitId: string, opener: HTMLButtonElement) => void;
}

export function SprintFlowMap({
  workspace,
  selectedRevisionId,
  onSelectedRevisionChange,
  onOpenSprintPlannerActivityGroup,
  onOpenWorkUnit,
}: SprintFlowMapProps) {
  const selectedView =
    workspace.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === selectedRevisionId,
    ) ?? workspace.revisionViews[0];
  const layout = useMemo(() => projectSprintFlowLayout(selectedView), [selectedView]);
  const connectorRoutes = useMemo(
    () => projectSprintConnectorRoutes(selectedView, layout),
    [selectedView, layout],
  );
  const positions = new Map(layout.positions.map((position) => [position.id, position]));
  const sprintPlannerActivityGroupPositions = new Map(
    layout.sprintPlannerActivityGroupPositions.map((position) => [position.id, position]),
  );
  const userReviews = selectedView.gates.filter(
    ({ presentationRole }) => presentationRole.kind === 'accepted_review_marker',
  );

  return (
    <section className="sprint-flow" aria-label="Sprint plan overview">
      <div className="sprint-flow__overlay">
        <label className="sprint-revision-select">
          <span className="visually-hidden">Plan revision</span>
          <select
            aria-label="Plan revision"
            value={selectedRevisionId}
            onChange={(event) => onSelectedRevisionChange(event.target.value)}
          >
            {workspace.revisions.map((revision) => (
              <option key={revision.sprintPlanRevisionId} value={revision.sprintPlanRevisionId}>
                {revision.sprintPlanRevisionId} - {revision.isCurrent ? 'Current' : 'Historical'}
              </option>
            ))}
          </select>
        </label>
      </div>
      <div className="sprint-flow__viewport" tabIndex={0} aria-label={`${selectedRevisionId} map`}>
        <div
          className="sprint-map-canvas"
          aria-label={`${selectedRevisionId} Sprint and Work Unit planning`}
          style={{ width: layout.width, height: layout.height }}
        >
          <svg aria-hidden="true" width={layout.width} height={layout.height}>
            {connectorRoutes
              .filter(({ scopeSprintPlannerActivityId }) => !scopeSprintPlannerActivityId)
              .map((connector) => {
                return (
                  <path
                    key={`${connector.from}-${connector.to}`}
                    className={`sprint-flow__connector sprint-flow__connector--${connector.kind}`}
                    data-connector={`${connector.from}->${connector.to}`}
                    data-connector-scope="map-gutter"
                    d={connector.path}
                  />
                );
              })}
          </svg>

          {selectedView.plannerActivityGroups.map((group) => {
            const box = sprintPlannerActivityGroupPositions.get(group.sprintPlannerActivityId);
            if (!box) return null;
            return (
              <section
                key={group.sprintPlannerActivityId}
                className="sprint-plan-region"
                aria-label={`Plan: ${group.title}`}
                style={{ left: box.x, top: box.y, width: box.width, height: box.height }}
              >
                <svg
                  className="sprint-plan-region__connectors"
                  aria-hidden="true"
                  width={box.width}
                  height={box.height}
                  viewBox={`0 0 ${box.width} ${box.height}`}
                >
                  {connectorRoutes
                    .filter(
                      ({ scopeSprintPlannerActivityId }) =>
                        scopeSprintPlannerActivityId === group.sprintPlannerActivityId,
                    )
                    .map((connector) => (
                      <path
                        key={`${connector.from}-${connector.to}`}
                        className={`sprint-flow__connector sprint-flow__connector--${connector.kind}`}
                        data-connector={`${connector.from}->${connector.to}`}
                        data-connector-scope={group.sprintPlannerActivityId}
                        d={connector.path}
                      />
                    ))}
                </svg>
                <button
                  type="button"
                  className="sprint-plan-region__open"
                  data-sprint-planner-activity-id={group.sprintPlannerActivityId}
                  aria-label={`Open Plan: ${group.title}`}
                  onClick={(event) =>
                    onOpenSprintPlannerActivityGroup?.(
                      group.sprintPlannerActivityId,
                      event.currentTarget,
                    )
                  }
                />
                <header>
                  <div className="sprint-plan-region__title">
                    <span>Plan</span>
                    <strong>{group.title}</strong>
                  </div>
                  {selectedView.gates.some(
                    ({ gateId, presentationRole }) =>
                      presentationRole.kind === 'accepted_review_marker' &&
                      selectedView.workUnits.some(
                        (unit) =>
                          group.workUnitScopeIds.includes(unit.workUnitScopeId) &&
                          unit.gateIds.includes(gateId),
                      ),
                  ) && (
                    <span className="sprint-review-marker sprint-review-marker--plan">
                      <UserRoundCheck size={13} aria-hidden="true" />
                      Plan review
                    </span>
                  )}
                </header>
                {selectedView.workUnits
                  .filter((unit) => group.workUnitScopeIds.includes(unit.workUnitScopeId))
                  .map((unit) => {
                    const position = positions.get(unit.workUnitId);
                    if (!position) return null;
                    const attachedReviews = userReviews.filter((gate) =>
                      unit.gateIds.includes(gate.gateId),
                    );
                    return (
                      <article
                        key={unit.workUnitId}
                        className={`sprint-work-unit sprint-work-unit--${displayState(unit.presentationState)}`}
                        style={{ left: position.x - box.x, top: position.y - box.y }}
                      >
                        <button
                          type="button"
                          className="sprint-work-unit__open"
                          data-work-unit-id={unit.workUnitId}
                          aria-label={`Open Work Unit ${unit.workUnitId}: ${unit.title}, ${statusLabel(unit.presentationState)}`}
                          title={unit.summary}
                          onClick={(event) => {
                            event.stopPropagation();
                            onOpenWorkUnit?.(unit.workUnitId, event.currentTarget);
                          }}
                        >
                          <span className="sprint-work-unit__state">
                            {statusIcon(unit.presentationState)}
                            {displayState(unit.presentationState) === 'completed'
                              ? null
                              : statusLabel(unit.presentationState)}
                          </span>
                          <strong>{unit.title}</strong>
                          <code>{unit.workUnitId}</code>
                        </button>
                        {attachedReviews.map((gate) => (
                          <span
                            key={gate.gateId}
                            className="sprint-review-marker sprint-review-marker--unit"
                            aria-label={`${gate.gateId} user review for ${unit.workUnitId}`}
                          >
                            <UserRoundCheck size={12} aria-hidden="true" />
                          </span>
                        ))}
                      </article>
                    );
                  })}
              </section>
            );
          })}
        </div>
      </div>
    </section>
  );
}

const DISPLAY_STATE = {
  not_started: 'not_started',
  waiting_for_dependencies: 'waiting_for_dependencies',
  requested: 'working',
  launched: 'working',
  returned: 'under_review',
  under_review: 'under_review',
  integrated: 'completed',
  responsibility_accepted: 'completed',
  deferred: 'deferred',
} as const satisfies Record<ProductWorkUnitPresentationState, string>;

function displayState(state: ProductWorkUnitPresentationState) {
  return DISPLAY_STATE[state];
}

function statusLabel(state: ProductWorkUnitPresentationState) {
  const display = displayState(state);
  return (
    {
      not_started: 'Not started',
      waiting_for_dependencies: 'Waiting',
      working: 'Working',
      under_review: 'Under review',
      completed: 'Completed',
      blocked: 'Blocked',
      deferred: 'Deferred',
    } as const
  )[display];
}

function statusIcon(state: ProductWorkUnitPresentationState) {
  const display = displayState(state);
  const Icon = (
    {
      not_started: Circle,
      waiting_for_dependencies: Clock3,
      working: LoaderCircle,
      under_review: SearchCheck,
      completed: Check,
      blocked: AlertOctagon,
      deferred: Pause,
    } as const
  )[display];
  return <Icon size={15} aria-hidden="true" />;
}
