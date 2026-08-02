import { useMemo, useState } from 'react';
import type { SprintWorkspacePresentationV1 } from '../../../application/orchestrations';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';
import type {
  SprintAgentSessionPresentation,
  WorkUnitAgentSessionPresentation,
} from '../orchestrationModel';
import { DetailWorkspace } from './DetailWorkspace';
import { SharedAgentSessionPanel } from './SharedAgentSessionPanel';
import '../styles/orchestrationSubdetail.css';

type RevisionWorkUnit = SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number];
type RoleReport = SprintWorkspacePresentationV1['roleReports'][number];
type PlannerReport = Extract<RoleReport, { readonly toolName: 'record_work_slice_plan' }>;
type HandlerActivity = Extract<RoleReport, { readonly toolName: 'report_handler_activity' }>;
type WorkerActivity = Extract<RoleReport, { readonly toolName: 'report_worker_activity' }>;

export interface PlanningPointWorkUnitRelationship {
  readonly workUnit: RevisionWorkUnit;
  readonly handlers: readonly WorkUnitAgentSessionPresentation[];
  readonly implementers: readonly WorkUnitAgentSessionPresentation[];
  readonly handlerActivity?: HandlerActivity;
  readonly workerActivity?: WorkerActivity;
}

export interface WorkSlicePlanningPointDetailWorkspaceProps {
  readonly workSlicePlanningPointGroup: SprintWorkspacePresentationV1['revisionViews'][number]['workSlicePlanningPointGroups'][number];
  readonly currentWorkState: string;
  readonly workUnitRelationships: readonly PlanningPointWorkUnitRelationship[];
  readonly plannerReport?: PlannerReport;
  readonly plannerSession?: SprintAgentSessionPresentation;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly onBack: () => void;
  readonly onOpenWorkUnit: (workUnitId: string, lifecycleEntryId?: string) => void;
  readonly onOpenAgentSession?: (sessionId: string) => void;
}

export function WorkSlicePlanningPointDetailWorkspace({
  workSlicePlanningPointGroup,
  currentWorkState,
  workUnitRelationships,
  plannerReport,
  plannerSession,
  agentSessionComposition,
  onBack,
  onOpenWorkUnit,
  onOpenAgentSession,
}: WorkSlicePlanningPointDetailWorkspaceProps) {
  const [linkedScopeIds, setLinkedScopeIds] = useState<readonly string[]>([]);
  const [hoveredScopeId, setHoveredScopeId] = useState<string | null>(null);
  const highlightedScopeIds = new Set(
    hoveredScopeId ? [...linkedScopeIds, hoveredScopeId] : linkedScopeIds,
  );

  return (
    <DetailWorkspace
      ariaLabel={`Work Slice planning point detail: ${workSlicePlanningPointGroup.title}`}
      controlsLabel="Work Slice planning point controls"
      contextLabel="Work Slice planning point context"
      backLabel="Back to Sprint"
      onBack={onBack}
      focusBackOnMount
      control={
        <span className="current-work-state">
          <small>Current work</small>
          <strong>{currentWorkState}</strong>
        </span>
      }
      context={
        <div className="subdetail-context planning-point-context">
          <p className="eyebrow">Work Slice planning point</p>
          <h1>{workSlicePlanningPointGroup.title}</h1>
          <p>{workSlicePlanningPointGroup.purpose}</p>
          <div className="planning-point-context__planner" aria-label="Work Slice Planner origin">
            <span aria-hidden="true">WSP</span>
            <strong>Work Slice Planner</strong>
            <small>{plannerSession?.identity?.agentName ?? plannerSession?.title}</small>
          </div>
          <section aria-label="Work Slice Planner structured analysis">
            <h2>Managed analysis and forecast</h2>
            {plannerReport?.analysisItems.length ? (
              <ul>
                {plannerReport.analysisItems.map((item) => {
                  const highlighted = item.linkedWorkUnitScopeIds.some((scopeId) =>
                    highlightedScopeIds.has(scopeId),
                  );
                  return (
                    <li key={item.analysisItemId}>
                      <button
                        type="button"
                        className={highlighted ? 'is-highlighted' : undefined}
                        onPointerEnter={() => setLinkedScopeIds(item.linkedWorkUnitScopeIds)}
                        onPointerLeave={() => setLinkedScopeIds([])}
                        onFocus={() => setLinkedScopeIds(item.linkedWorkUnitScopeIds)}
                        onBlur={() => setLinkedScopeIds([])}
                      >
                        {item.text}
                      </button>
                    </li>
                  );
                })}
              </ul>
            ) : (
              <p>Structured Work Slice Planner analysis is unavailable.</p>
            )}
          </section>
        </div>
      }
      primary={
        <div className="work-slice-planning-point-detail">
          <PlanningPointTimeline
            plannerSession={plannerSession}
            plannerReport={plannerReport}
            workUnitRelationships={workUnitRelationships}
            highlightedScopeIds={highlightedScopeIds}
            onHoveredScopeIdChange={setHoveredScopeId}
            onOpenWorkUnit={onOpenWorkUnit}
          />
          {plannerSession ? (
            <section className="plan-agent-sessions" aria-label="Work Slice Planner Agent Session">
              <SharedAgentSessionPanel
                ariaLabel="Work Slice Planner Agent Session conversation surface"
                conversationAriaLabel="Work Slice Planner conversation"
                session={plannerSession}
                composition={agentSessionComposition}
                onOpenStandalone={onOpenAgentSession}
                displayMode="always_open"
              />
            </section>
          ) : null}
        </div>
      }
    />
  );
}

function PlanningPointTimeline({
  plannerSession,
  plannerReport,
  workUnitRelationships,
  highlightedScopeIds,
  onHoveredScopeIdChange,
  onOpenWorkUnit,
}: {
  readonly plannerSession?: SprintAgentSessionPresentation;
  readonly plannerReport?: PlannerReport;
  readonly workUnitRelationships: readonly PlanningPointWorkUnitRelationship[];
  readonly highlightedScopeIds: ReadonlySet<string>;
  readonly onHoveredScopeIdChange: (scopeId: string | null) => void;
  readonly onOpenWorkUnit: (workUnitId: string, lifecycleEntryId?: string) => void;
}) {
  const layout = useMemo(
    () => planningPointTimelineLayout(workUnitRelationships, plannerReport),
    [workUnitRelationships, plannerReport],
  );
  const positionByScopeId = new Map(layout.nodes.map((node) => [node.scopeId, node]));

  return (
    <section className="planning-point-timeline" aria-label="Work Slice causal timeline">
      <header>
        <div>
          <p className="eyebrow">Causal timeline</p>
          <h2>Parallel Work Unit flow</h2>
        </div>
        <ul aria-label="Dependency meanings">
          <li data-dependency-kind="functional_output">Functional output</li>
          <li data-dependency-kind="shared_resource_exclusion">Shared resource exclusion</li>
          <li data-dependency-kind="merge_join" data-join-semantics="independent_prerequisites">
            Independent completion gate
          </li>
          <li data-dependency-kind="merge_join" data-join-semantics="merged_result">
            Merged result
          </li>
        </ul>
      </header>
      <div className="planning-point-timeline__viewport" tabIndex={0}>
        <div
          className="planning-point-timeline__canvas"
          style={{ width: layout.width, height: layout.height }}
        >
          <div className="planning-point-timeline__planner" style={{ left: 18, top: 24 }}>
            <span>WSP</span>
            <strong>Work Slice Planner</strong>
            <small>{plannerSession?.identity?.agentName ?? plannerSession?.title}</small>
          </div>
          <svg
            aria-label="Typed Work Unit dependencies"
            width={layout.width}
            height={layout.height}
          >
            <defs>
              <marker
                id="dependency-arrow"
                viewBox="0 0 10 10"
                refX="9"
                refY="5"
                markerWidth="6"
                markerHeight="6"
                orient="auto-start-reverse"
              >
                <path d="M 0 0 L 10 5 L 0 10 z" />
              </marker>
              <marker
                id="merged-output-arrow"
                viewBox="0 0 12 12"
                refX="11"
                refY="6"
                markerWidth="8"
                markerHeight="8"
                orient="auto-start-reverse"
              >
                <path d="M 0 6 L 6 1 L 12 6 L 6 11 z" />
              </marker>
              <marker
                id="prerequisite-completion-arrow"
                viewBox="0 0 10 10"
                refX="9"
                refY="5"
                markerWidth="7"
                markerHeight="7"
                orient="auto-start-reverse"
              >
                <path d="M 1 1 L 9 5 L 1 9" />
              </marker>
            </defs>
            {plannerReport?.dependencies.map((dependency) => {
              const to = positionByScopeId.get(dependency.toWorkUnitScopeId);
              if (!to) return null;
              if (
                dependency.kind === 'merge_join' &&
                dependency.joinSemantics === 'independent_prerequisites'
              ) {
                const inputs = dependency.inputWorkUnitScopeIds
                  .map((scopeId) => positionByScopeId.get(scopeId))
                  .filter((position): position is NonNullable<typeof position> => !!position);
                if (inputs.length !== dependency.inputWorkUnitScopeIds.length) return null;
                const highlighted = [to, ...inputs].some(({ scopeId }) =>
                  highlightedScopeIds.has(scopeId),
                );
                const gateX = to.x - 42;
                const gateY = to.y + layout.nodeHeight / 2;
                return (
                  <g
                    key={dependency.dependencyId}
                    className={highlighted ? 'is-highlighted' : undefined}
                    data-dependency-id={dependency.dependencyId}
                    data-dependency-kind={dependency.kind}
                    data-join-semantics={dependency.joinSemantics}
                    data-input-scope-ids={dependency.inputWorkUnitScopeIds.join(' ')}
                  >
                    {inputs.map((input) => {
                      const startX = input.x + layout.nodeWidth;
                      const startY = input.y + layout.nodeHeight / 2;
                      const bend = Math.max(28, (gateX - startX) / 2);
                      return (
                        <path
                          key={input.scopeId}
                          className="planning-point-dependency__prerequisite-leg"
                          data-prerequisite-input={input.scopeId}
                          d={`M ${startX} ${startY} C ${startX + bend} ${startY}, ${gateX - bend} ${gateY}, ${gateX - 9} ${gateY}`}
                        />
                      );
                    })}
                    <circle
                      className="planning-point-dependency__completion-gate"
                      cx={gateX}
                      cy={gateY}
                      r="9"
                    />
                    <path
                      className="planning-point-dependency__completion-path"
                      data-geometry="independent-completion-gate"
                      d={`M ${gateX + 9} ${gateY} H ${to.x}`}
                      markerEnd="url(#prerequisite-completion-arrow)"
                    />
                    <text x={gateX - 8} y={gateY - 14} textAnchor="end">
                      {dependency.label} · completion gate
                    </text>
                  </g>
                );
              }
              const from = positionByScopeId.get(dependency.fromWorkUnitScopeId);
              if (!from) return null;
              const highlighted =
                highlightedScopeIds.has(from.scopeId) || highlightedScopeIds.has(to.scopeId);
              const startX = from.x + layout.nodeWidth;
              const startY = from.y + layout.nodeHeight / 2;
              const endX = to.x;
              const endY = to.y + layout.nodeHeight / 2;
              const bend = Math.max(34, (endX - startX) / 2);
              return (
                <g
                  key={dependency.dependencyId}
                  className={highlighted ? 'is-highlighted' : undefined}
                  data-dependency-id={dependency.dependencyId}
                  data-dependency-kind={dependency.kind}
                  data-join-semantics={
                    dependency.kind === 'merge_join' ? dependency.joinSemantics : undefined
                  }
                >
                  <path
                    data-geometry={
                      dependency.kind === 'merge_join' ? 'merged-output' : 'direct-dependency'
                    }
                    d={`M ${startX} ${startY} C ${startX + bend} ${startY}, ${endX - bend} ${endY}, ${endX} ${endY}`}
                    markerEnd={
                      dependency.kind === 'merge_join'
                        ? 'url(#merged-output-arrow)'
                        : 'url(#dependency-arrow)'
                    }
                  />
                  <text x={(startX + endX) / 2} y={(startY + endY) / 2 - 6}>
                    {dependency.label}
                    {dependency.kind === 'merge_join' ? ' · merged result' : ''}
                  </text>
                </g>
              );
            })}
          </svg>
          {layout.nodes.map((node) => {
            const relationship = workUnitRelationships.find(
              ({ workUnit }) => workUnit.workUnitScopeId === node.scopeId,
            )!;
            return (
              <WorkUnitTimelineTile
                key={node.scopeId}
                relationship={relationship}
                highlighted={highlightedScopeIds.has(node.scopeId)}
                style={{ left: node.x, top: node.y }}
                onHoveredChange={(hovered) => onHoveredScopeIdChange(hovered ? node.scopeId : null)}
                onOpenWorkUnit={onOpenWorkUnit}
              />
            );
          })}
        </div>
      </div>
    </section>
  );
}

function WorkUnitTimelineTile({
  relationship,
  highlighted,
  style,
  onHoveredChange,
  onOpenWorkUnit,
}: {
  readonly relationship: PlanningPointWorkUnitRelationship;
  readonly highlighted: boolean;
  readonly style: Readonly<{ left: number; top: number }>;
  readonly onHoveredChange: (hovered: boolean) => void;
  readonly onOpenWorkUnit: (workUnitId: string, lifecycleEntryId?: string) => void;
}) {
  const { workUnit, handlers, implementers, handlerActivity, workerActivity } = relationship;
  return (
    <article
      className={`planning-point-work-unit${highlighted ? ' is-highlighted' : ''}`}
      style={style}
      tabIndex={0}
      aria-label={`Work Unit ${workUnit.workUnitId}: ${workUnit.title}`}
      onPointerEnter={() => onHoveredChange(true)}
      onPointerLeave={() => onHoveredChange(false)}
      onFocus={() => onHoveredChange(true)}
      onBlur={() => onHoveredChange(false)}
    >
      <header>
        <span>Work Unit</span>
        <small>{workUnitStatusLabel(workUnit.presentationState)}</small>
      </header>
      <code>{workUnit.workUnitId}</code>
      <h3>{workUnit.title}</h3>
      <button type="button" onClick={() => onOpenWorkUnit(workUnit.workUnitId)}>
        Open Work Unit
      </button>
      <AgentActivity
        role="Handler"
        workUnitId={workUnit.workUnitId}
        session={handlers[0]}
        activity={handlerActivity}
        onOpenWorkUnit={onOpenWorkUnit}
      />
      <AgentActivity
        role="Worker"
        workUnitId={workUnit.workUnitId}
        session={implementers[0]}
        activity={workerActivity}
        onOpenWorkUnit={onOpenWorkUnit}
      />
    </article>
  );
}

function AgentActivity({
  role,
  workUnitId,
  session,
  activity,
  onOpenWorkUnit,
}: {
  readonly role: 'Handler' | 'Worker';
  readonly workUnitId: string;
  readonly session?: WorkUnitAgentSessionPresentation;
  readonly activity?: HandlerActivity | WorkerActivity;
  readonly onOpenWorkUnit: (workUnitId: string, lifecycleEntryId?: string) => void;
}) {
  if (!session)
    return (
      <div
        className="planning-point-work-unit__agent-activity"
        aria-label={`${workUnitId} ${role} unavailable`}
      >
        <strong>{role}: Unavailable</strong>
        <span>No typed Agent Session relationship is recorded.</span>
      </div>
    );
  return (
    <button
      type="button"
      className="planning-point-work-unit__agent-activity"
      onClick={() => onOpenWorkUnit(workUnitId, activity?.lifecycleEntryId)}
      aria-label={`Open ${workUnitId} ${role} lifecycle${activity ? ` at ${activity.activity}` : ''}`}
    >
      <span
        className="planning-point-work-unit__identity"
        style={{ borderColor: session.identity?.visualIdentity.accentColor }}
        aria-hidden="true"
      >
        {session.identity?.visualIdentity.token ?? role.slice(0, 1)}
      </span>
      <span>
        <strong>
          {role}: {session.identity?.agentName ?? session.title}
        </strong>
        <small>{activity?.summary ?? 'Current activity unavailable'}</small>
      </span>
    </button>
  );
}

function planningPointTimelineLayout(
  relationships: readonly PlanningPointWorkUnitRelationship[],
  plannerReport?: PlannerReport,
) {
  const nodeWidth = 248;
  const nodeHeight = 194;
  const left = 210;
  const columnGap = 120;
  const laneGap = 28;
  const dependencyByTarget = new Map<string, string[]>();
  plannerReport?.dependencies.forEach((dependency) => {
    const sourceScopeIds =
      dependency.kind === 'merge_join' && dependency.joinSemantics === 'independent_prerequisites'
        ? dependency.inputWorkUnitScopeIds
        : [dependency.fromWorkUnitScopeId];
    const { toWorkUnitScopeId } = dependency;
    dependencyByTarget.set(toWorkUnitScopeId, [
      ...(dependencyByTarget.get(toWorkUnitScopeId) ?? []),
      ...sourceScopeIds,
    ]);
  });
  const depths = new Map<string, number>();
  const depth = (scopeId: string, seen = new Set<string>()): number => {
    if (depths.has(scopeId)) return depths.get(scopeId)!;
    if (seen.has(scopeId)) return 0;
    const parents = dependencyByTarget.get(scopeId) ?? [];
    const value = parents.length
      ? Math.max(...parents.map((parent) => depth(parent, new Set([...seen, scopeId])) + 1))
      : 0;
    depths.set(scopeId, value);
    return value;
  };
  const nodes = relationships.map(({ workUnit }, lane) => ({
    scopeId: workUnit.workUnitScopeId,
    x: left + depth(workUnit.workUnitScopeId) * (nodeWidth + columnGap),
    y: 24 + lane * (nodeHeight + laneGap),
  }));
  const maxDepth = Math.max(0, ...nodes.map(({ scopeId }) => depth(scopeId)));
  return {
    nodes,
    nodeWidth,
    nodeHeight,
    width: left + (maxDepth + 1) * (nodeWidth + columnGap) + 40,
    height: Math.max(260, 48 + relationships.length * (nodeHeight + laneGap)),
  };
}

function workUnitStatusLabel(state: RevisionWorkUnit['presentationState']) {
  return {
    not_started: 'Planned',
    waiting_for_dependencies: 'Waiting for dependencies',
    requested: 'Requested',
    launched: 'In progress',
    returned: 'Returned',
    under_review: 'Under review',
    integrated: 'Completed',
    responsibility_accepted: 'Completed',
    deferred: 'Deferred',
  }[state];
}
