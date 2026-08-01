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
  readonly onOpenWorkSlicePlanningPointGroup?: (
    workSlicePlanningPointId: string,
    opener: HTMLButtonElement,
  ) => void;
  readonly onOpenWorkUnit?: (workUnitId: string, opener: HTMLButtonElement) => void;
  readonly highlightedSprintRunnerConcernId?: string | null;
  readonly hoveredGraphElement?: {
    readonly kind: 'work_slice_planning_point' | 'work_unit' | 'gate';
    readonly id: string;
  } | null;
  readonly onHoveredGraphElementChange?: (
    element: {
      readonly kind: 'work_slice_planning_point' | 'work_unit' | 'gate';
      readonly id: string;
    } | null,
  ) => void;
}

export function SprintFlowMap({
  workspace,
  selectedRevisionId,
  onSelectedRevisionChange,
  onOpenWorkSlicePlanningPointGroup,
  onOpenWorkUnit,
  highlightedSprintRunnerConcernId,
  hoveredGraphElement,
  onHoveredGraphElementChange,
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
  const workSlicePlanningPointGroupPositions = new Map(
    layout.workSlicePlanningPointGroupPositions.map((position) => [position.id, position]),
  );
  const userReviews = selectedView.gates.filter(
    ({ presentationRole }) => presentationRole.kind === 'accepted_review_marker',
  );
  const highlightedRefs = new Set(
    workspace.sprintRunnerConcerns
      .find(
        ({ sprintRunnerConcernId }) => sprintRunnerConcernId === highlightedSprintRunnerConcernId,
      )
      ?.graphElementRefs.map(({ kind, id }) => `${kind}:${id}`) ?? [],
  );
  const priorRevision = workspace.revisionViews
    .filter(({ revision }) => revision < selectedView.revision)
    .sort((left, right) => right.revision - left.revision)[0];
  const priorWorkUnitIds = new Set(priorRevision?.workUnits.map(({ workUnitId }) => workUnitId));

  return (
    <section className="sprint-flow" aria-label="Sprint plan overview">
      <div className="sprint-flow__overlay">
        <div className="sprint-flow__legend" aria-label="Flow states">
          <span data-state="planned">Planned</span>
          <span data-state="processing">Processing</span>
          <span data-state="completed">Completed</span>
          <span data-state="divergent">Later divergence</span>
        </div>
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
              .filter(({ scopeWorkSlicePlanningPointId }) => !scopeWorkSlicePlanningPointId)
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

          {selectedView.workSlicePlanningPointGroups.map((group) => {
            const box = workSlicePlanningPointGroupPositions.get(group.workSlicePlanningPointId);
            if (!box) return null;
            return (
              <section
                key={group.workSlicePlanningPointId}
                className={`sprint-plan-region${
                  highlightedRefs.has(`work_slice_planning_point:${group.workSlicePlanningPointId}`)
                    ? ' is-runner-concern-highlighted'
                    : ''
                }${
                  hoveredGraphElement?.kind === 'work_slice_planning_point' &&
                  hoveredGraphElement.id === group.workSlicePlanningPointId
                    ? ' is-hovered'
                    : ''
                }`}
                aria-label={`Work Slice planning point: ${group.title}`}
                style={{ left: box.x, top: box.y, width: box.width, height: box.height }}
                onPointerEnter={() =>
                  onHoveredGraphElementChange?.({
                    kind: 'work_slice_planning_point',
                    id: group.workSlicePlanningPointId,
                  })
                }
                onPointerLeave={() => onHoveredGraphElementChange?.(null)}
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
                      ({ scopeWorkSlicePlanningPointId }) =>
                        scopeWorkSlicePlanningPointId === group.workSlicePlanningPointId,
                    )
                    .map((connector) => (
                      <path
                        key={`${connector.from}-${connector.to}`}
                        className={`sprint-flow__connector sprint-flow__connector--${connector.kind}`}
                        data-connector={`${connector.from}->${connector.to}`}
                        data-connector-scope={group.workSlicePlanningPointId}
                        d={connector.path}
                      />
                    ))}
                </svg>
                <button
                  type="button"
                  className="sprint-plan-region__open"
                  data-work-slice-planning-point-id={group.workSlicePlanningPointId}
                  data-flow-element-kind="work_slice_planning_point"
                  data-flow-element-id={group.workSlicePlanningPointId}
                  aria-label={`Open Work Slice planning point: ${group.title}`}
                  onClick={(event) =>
                    onOpenWorkSlicePlanningPointGroup?.(
                      group.workSlicePlanningPointId,
                      event.currentTarget,
                    )
                  }
                />
                <header>
                  <div className="sprint-plan-region__title">
                    <span>Work Slice planning point</span>
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
                        className={`sprint-work-unit sprint-work-unit--${displayState(unit.presentationState)}${
                          priorRevision && !priorWorkUnitIds.has(unit.workUnitId)
                            ? ' sprint-work-unit--divergent'
                            : ''
                        }${
                          highlightedRefs.has(`work_unit:${unit.workUnitId}`)
                            ? ' is-runner-concern-highlighted'
                            : ''
                        }${
                          hoveredGraphElement?.kind === 'work_unit' &&
                          hoveredGraphElement.id === unit.workUnitId
                            ? ' is-hovered'
                            : ''
                        }`}
                        style={{ left: position.x - box.x, top: position.y - box.y }}
                        onPointerEnter={(event) => {
                          event.stopPropagation();
                          onHoveredGraphElementChange?.({
                            kind: 'work_unit',
                            id: unit.workUnitId,
                          });
                        }}
                        onPointerLeave={(event) => {
                          event.stopPropagation();
                          onHoveredGraphElementChange?.(null);
                        }}
                      >
                        <button
                          type="button"
                          className="sprint-work-unit__open"
                          data-work-unit-id={unit.workUnitId}
                          data-flow-element-kind="work_unit"
                          data-flow-element-id={unit.workUnitId}
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
                          {priorRevision && !priorWorkUnitIds.has(unit.workUnitId) ? (
                            <small>Added in later plan</small>
                          ) : null}
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
