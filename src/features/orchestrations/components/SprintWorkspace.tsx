import type {
  SprintWorkspacePresentationAdjunct,
  SprintWorkspaceDetailLocation,
  SprintAgentSessionPresentation,
  WorkUnitAgentSessionPresentation,
} from '../orchestrationModel';
import type {
  ArtifactAccessController,
  SprintAutomaticContinuationPolicyController,
  SprintWorkspacePresentationV1,
} from '../../../application/orchestrations';
import { useEffect, useRef, useState } from 'react';
import { DetailWorkspace } from './DetailWorkspace';
import { SprintContinuationControl } from './SprintContinuationControl';
import { SprintConcernsPanel } from './SprintConcernsPanel';
import { SprintDocumentsPanel } from './SprintDocumentsPanel';
import { SprintFlowMap } from './SprintFlowMap';
import { SprintWorkspaceTabs, type SprintWorkspaceTab } from './SprintWorkspaceTabs';
import { SharedAgentSessionPanel } from './SharedAgentSessionPanel';
import {
  type PlanningPointWorkUnitRelationship,
  WorkSlicePlanningPointDetailWorkspace,
} from './WorkSlicePlanningPointDetailWorkspace';
import { WorkUnitDetailWorkspace } from './WorkUnitDetailWorkspace';
import '../styles/sprintWorkspace.css';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';
import type {
  ContextualFileReviewResult,
  ContextualFileReviewFailureReason,
} from '../../../application/contextualFileReview';

type SprintFileReviewControlState =
  | { readonly kind: 'idle' | 'pending' }
  | {
      readonly kind: 'failed';
      readonly reason: ContextualFileReviewFailureReason;
      readonly message: string;
    };

export interface SprintWorkspaceProps {
  readonly workspace: SprintWorkspacePresentationV1;
  readonly adjunct?: SprintWorkspacePresentationAdjunct;
  readonly artifactAccessController: ArtifactAccessController;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly automaticContinuationPolicyController?: SprintAutomaticContinuationPolicyController;
  readonly selectedRevisionId: string;
  readonly onSelectedRevisionChange: (revisionId: string) => void;
  readonly detailLocation: SprintWorkspaceDetailLocation;
  readonly onDetailLocationChange: (location: SprintWorkspaceDetailLocation) => void;
  readonly onBack: () => void;
  readonly onOpenAgentSession?: (sessionId: string) => void;
  readonly onRequestFileReview?: (sprintId: string) => Promise<ContextualFileReviewResult>;
}

export function SprintWorkspace({
  workspace,
  adjunct,
  artifactAccessController,
  agentSessionComposition,
  automaticContinuationPolicyController,
  selectedRevisionId,
  onSelectedRevisionChange,
  detailLocation,
  onDetailLocationChange,
  onBack,
  onOpenAgentSession,
  onRequestFileReview,
}: SprintWorkspaceProps) {
  const [selectedTab, setSelectedTab] = useState<SprintWorkspaceTab>('flow');
  const [selectedConcernId, setSelectedConcernId] = useState<string | null>(null);
  const [highlightedSprintRunnerConcernId, setHighlightedSprintRunnerConcernId] = useState<
    string | null
  >(null);
  const [hoveredGraphElement, setHoveredGraphElement] = useState<{
    readonly kind: 'work_slice_planning_point' | 'work_unit' | 'gate';
    readonly id: string;
  } | null>(null);
  const sprintRunnerConcernFocusIndexRef = useRef(new Map<string, number>());
  const sprintRestoreRef = useRef<{
    kind: 'work_slice_planning_point' | 'work_unit';
    id: string;
  } | null>(null);
  const concernRestoreWorkUnitRef = useRef<string | null>(null);
  const fileReviewRequestSequence = useRef(0);
  const [fileReviewState, setFileReviewState] = useState<SprintFileReviewControlState>({
    kind: 'idle',
  });
  const planningValue =
    workspace.sprint.planningState.source.status === 'available'
      ? workspace.sprint.planningState.value
      : undefined;
  const hasStartedPlan = planningValue?.kind === 'started_plan';
  const hasPreStartForecast = planningValue?.kind === 'pre_start_forecast';
  const planningUnavailableReason =
    workspace.sprint.planningState.source.status === 'available'
      ? 'The planning state is not available.'
      : workspace.sprint.planningState.source.reason;

  useEffect(() => {
    if (detailLocation.kind !== 'sprint' || !sprintRestoreRef.current) return;
    const restore = sprintRestoreRef.current;
    sprintRestoreRef.current = null;
    document
      .querySelector<HTMLButtonElement>(
        restore.kind === 'work_slice_planning_point'
          ? `[data-work-slice-planning-point-id="${restore.id}"]`
          : `[data-work-unit-id="${restore.id}"]`,
      )
      ?.focus();
  }, [detailLocation]);

  useEffect(() => {
    fileReviewRequestSequence.current += 1;
    setFileReviewState({ kind: 'idle' });
  }, [workspace.sprint.sprintId]);

  const requestFileReview = async () => {
    if (!onRequestFileReview || fileReviewState.kind === 'pending') return;
    const sequence = ++fileReviewRequestSequence.current;
    setFileReviewState({ kind: 'pending' });
    const result = await onRequestFileReview(workspace.sprint.sprintId);
    if (fileReviewRequestSequence.current !== sequence) return;
    setFileReviewState(
      result.status === 'failed'
        ? { kind: 'failed', reason: result.reason, message: result.message }
        : { kind: 'idle' },
    );
  };
  const fileReviewControl =
    hasStartedPlan && onRequestFileReview ? (
      <SprintFileReviewControl state={fileReviewState} onRequest={requestFileReview} />
    ) : undefined;

  useEffect(() => {
    if (detailLocation.kind !== 'sprint' || !concernRestoreWorkUnitRef.current) return;
    const id = concernRestoreWorkUnitRef.current;
    concernRestoreWorkUnitRef.current = null;
    document.querySelector<HTMLButtonElement>(`[data-concern-work-unit-id="${id}"]`)?.focus();
  }, [detailLocation]);

  const selectedView = workspace.revisionViews.find(
    ({ sprintPlanRevisionId }) => sprintPlanRevisionId === selectedRevisionId,
  )!;
  const activeView = workspace.revisionViews.find(
    ({ sprintPlanRevisionId }) => sprintPlanRevisionId === workspace.activeSprintPlanRevisionId,
  )!;
  const ownerOf = (workUnitId: string, view: (typeof workspace.revisionViews)[number]) =>
    view.workSlicePlanningPointGroups.find(({ workUnitScopeIds }) =>
      workUnitScopeIds.includes(
        view.workUnits.find((unit) => unit.workUnitId === workUnitId)?.workUnitScopeId ?? '',
      ),
    );

  if (hasStartedPlan && detailLocation.kind === 'work_unit') {
    const view = workspace.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === detailLocation.revisionId,
    )!;
    const workSlicePlanningPointGroup = view.workSlicePlanningPointGroups.find(
      ({ workSlicePlanningPointId }) =>
        workSlicePlanningPointId === detailLocation.workSlicePlanningPointId,
    )!;
    const unit = view.workUnits.find(({ workUnitId }) => workUnitId === detailLocation.workUnitId)!;
    return (
      <WorkUnitDetailWorkspace
        unit={unit}
        lifecycleEntries={workspace.workUnitLifecycle.filter(
          ({ workUnitId }) => workUnitId === unit.workUnitId,
        )}
        workSlicePlanningPointGroupTitle={workSlicePlanningPointGroup.title}
        sessions={workUnitSessions(workspace, unit, adjunct)}
        agentSessionComposition={agentSessionComposition}
        backLabel={
          detailLocation.origin === 'concern'
            ? 'Back to Concern'
            : 'Back to Work Slice planning point'
        }
        onBack={() => {
          if (detailLocation.origin === 'concern') {
            concernRestoreWorkUnitRef.current = detailLocation.workUnitId;
            onDetailLocationChange({ kind: 'sprint' });
            return;
          }
          onDetailLocationChange({
            kind: 'work_slice_planning_point',
            revisionId: detailLocation.revisionId,
            workSlicePlanningPointId: detailLocation.workSlicePlanningPointId,
          });
        }}
        onOpenAgentSession={onOpenAgentSession}
        sprintControl={fileReviewControl}
      />
    );
  }

  if (hasStartedPlan && detailLocation.kind === 'work_slice_planning_point') {
    const view = workspace.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === detailLocation.revisionId,
    )!;
    const workSlicePlanningPointGroup = view.workSlicePlanningPointGroups.find(
      ({ workSlicePlanningPointId }) =>
        workSlicePlanningPointId === detailLocation.workSlicePlanningPointId,
    )!;
    return (
      <WorkSlicePlanningPointDetailWorkspace
        workSlicePlanningPointGroup={workSlicePlanningPointGroup}
        currentWorkState={workSlicePlanningPointState(workSlicePlanningPointGroup, view)}
        workUnitRelationships={planningPointWorkUnitRelationships(
          workspace,
          view,
          workSlicePlanningPointGroup,
          adjunct,
        )}
        plannerSession={workSlicePlanningPointSession(
          workspace,
          workSlicePlanningPointGroup.workSlicePlanningPointId,
          adjunct,
        )}
        agentSessionComposition={agentSessionComposition}
        workflow={adjunct?.workSlicePlanningPointWorkflows.find(
          ({ workSlicePlanningPointId }) =>
            workSlicePlanningPointId === workSlicePlanningPointGroup.workSlicePlanningPointId,
        )}
        onBack={() => onDetailLocationChange({ kind: 'sprint' })}
        onOpenWorkUnit={(workUnitId) => {
          onDetailLocationChange({
            kind: 'work_unit',
            revisionId: view.sprintPlanRevisionId,
            workSlicePlanningPointId: workSlicePlanningPointGroup.workSlicePlanningPointId,
            workUnitId,
            origin: 'work_slice_planning_point',
          });
        }}
        onOpenAgentSession={onOpenAgentSession}
        sprintControl={fileReviewControl}
      />
    );
  }

  return (
    <DetailWorkspace
      ariaLabel="Sprint detail"
      controlsLabel="Sprint controls"
      contextLabel="Sprint context"
      backLabel="Back to Epic"
      onBack={onBack}
      focusBackOnMount
      hotbarNavigation={
        hasStartedPlan ? (
          <SprintWorkspaceTabs selected={selectedTab} onSelect={setSelectedTab} />
        ) : undefined
      }
      control={
        <div className="sprint-header-controls">
          {fileReviewControl}
          <SprintContinuationControl
            automaticEnabled={workspace.continuation.policy?.automaticEnabled ?? false}
            controller={automaticContinuationPolicyController}
            policyUpdateIntent={
              workspace.continuation.policy
                ? {
                    level: 'sprint',
                    sprintId: workspace.sprint.sprintId,
                    policyId: workspace.continuation.policy.policyId,
                    automaticEnabled: workspace.continuation.policy.automaticEnabled,
                  }
                : undefined
            }
          />
        </div>
      }
      context={
        <div className="sprint-context">
          <p className="eyebrow">Sprint</p>
          <h1>{workspace.sprint.title}</h1>
          <span
            className={`sprint-context__state sprint-context__state--${
              workspace.sprint.lifecycle?.value ??
              workspace.sprint.lifecycle?.source.status ??
              'unavailable'
            }`}
          >
            {sprintLifecycleLabel(workspace.sprint.lifecycle)}
          </span>
          <p>{workspace.sprint.summary}</p>
          <section className="sprint-context__objectives" aria-label="Epic Runner objectives">
            <h2>Epic Runner objectives</h2>
            {workspace.epicRunnerObjectives.length > 0 ? (
              <ul>
                {workspace.epicRunnerObjectives.map((objective) => (
                  <li key={objective.objectiveId}>{objective.title}</li>
                ))}
              </ul>
            ) : (
              <p>No recorded Epic Runner Sprint objectives.</p>
            )}
          </section>
          {hasStartedPlan && workspace.sprintRunnerConcerns.length > 0 ? (
            <section
              className="sprint-context__runner-concerns"
              aria-label="Sprint Runner concerns"
            >
              <h2>Sprint Runner concerns</h2>
              <ul>
                {workspace.sprintRunnerConcerns.map((sprintRunnerConcern) => {
                  const relatedToHover = hoveredGraphElement
                    ? sprintRunnerConcern.graphElementRefs.some(
                        (reference) =>
                          reference.kind === hoveredGraphElement.kind &&
                          reference.id === hoveredGraphElement.id,
                      )
                    : false;
                  return (
                    <li key={sprintRunnerConcern.sprintRunnerConcernId}>
                      <button
                        type="button"
                        className={
                          highlightedSprintRunnerConcernId ===
                            sprintRunnerConcern.sprintRunnerConcernId || relatedToHover
                            ? 'is-highlighted'
                            : undefined
                        }
                        aria-pressed={
                          highlightedSprintRunnerConcernId ===
                          sprintRunnerConcern.sprintRunnerConcernId
                        }
                        onPointerEnter={() =>
                          setHighlightedSprintRunnerConcernId(
                            sprintRunnerConcern.sprintRunnerConcernId,
                          )
                        }
                        onPointerLeave={() => setHighlightedSprintRunnerConcernId(null)}
                        onFocus={() =>
                          setHighlightedSprintRunnerConcernId(
                            sprintRunnerConcern.sprintRunnerConcernId,
                          )
                        }
                        onBlur={() => setHighlightedSprintRunnerConcernId(null)}
                        onClick={() => {
                          setSelectedTab('flow');
                          focusNextSprintRunnerConcernGraphElement(
                            sprintRunnerConcern,
                            selectedView,
                            sprintRunnerConcernFocusIndexRef.current,
                          );
                        }}
                      >
                        {sprintRunnerConcern.title}
                      </button>
                    </li>
                  );
                })}
              </ul>
            </section>
          ) : null}
        </div>
      }
      primary={
        <>
          {hasPreStartForecast ? (
            <section className="sprint-forecast" aria-label="Sprint Runner pre-start forecast">
              <p className="eyebrow">Sprint Runner forecast</p>
              <h2>Concerns before Sprint start</h2>
              <p>
                This forecast stays intentionally low resolution until the Sprint starts and the
                current branch and repository state can be reevaluated.
              </p>
              {workspace.concerns.length > 0 ? (
                <ul>
                  {workspace.concerns.map((concern) => (
                    <li key={concern.concernId}>
                      <strong>{concern.title}</strong>
                      <span>{concern.summary}</span>
                    </li>
                  ))}
                </ul>
              ) : (
                <p>No sourced pre-start concerns are available.</p>
              )}
            </section>
          ) : !hasStartedPlan ? (
            <section className="sprint-forecast" aria-label="Sprint planning unavailable">
              <p className="eyebrow">Sprint Runner plan</p>
              <h2>Planning state unavailable</h2>
              <p>{planningUnavailableReason}</p>
            </section>
          ) : null}
          {hasStartedPlan && selectedTab === 'flow' && (
            <section
              className="sprint-tab-panel"
              id="sprint-flow-panel"
              role="tabpanel"
              aria-labelledby="sprint-flow-tab"
            >
              <section className="sprint-surface-host" aria-label="Sprint Runner plan">
                <header className="sprint-start-assessment">
                  <span>Started plan</span>
                  <strong>{planningValue.repositoryAssessmentSummary}</strong>
                  <time dateTime={planningValue.reevaluatedAt}>
                    Reevaluated {new Date(planningValue.reevaluatedAt).toLocaleString()}
                  </time>
                </header>
                <SprintFlowMap
                  workspace={workspace}
                  selectedRevisionId={selectedRevisionId}
                  onSelectedRevisionChange={onSelectedRevisionChange}
                  highlightedSprintRunnerConcernId={highlightedSprintRunnerConcernId}
                  hoveredGraphElement={hoveredGraphElement}
                  onHoveredGraphElementChange={setHoveredGraphElement}
                  onOpenWorkSlicePlanningPointGroup={(workSlicePlanningPointId) => {
                    sprintRestoreRef.current = {
                      kind: 'work_slice_planning_point',
                      id: workSlicePlanningPointId,
                    };
                    onDetailLocationChange({
                      kind: 'work_slice_planning_point',
                      revisionId: selectedRevisionId,
                      workSlicePlanningPointId,
                    });
                  }}
                  onOpenWorkUnit={(workUnitId) => {
                    const owner = ownerOf(workUnitId, selectedView);
                    if (!owner) return;
                    sprintRestoreRef.current = { kind: 'work_unit', id: workUnitId };
                    onDetailLocationChange({
                      kind: 'work_unit',
                      revisionId: selectedRevisionId,
                      workSlicePlanningPointId: owner.workSlicePlanningPointId,
                      workUnitId,
                      origin: 'work_slice_planning_point',
                    });
                  }}
                />
              </section>
            </section>
          )}
          {hasStartedPlan && selectedTab === 'concerns' && (
            <section
              className="sprint-tab-panel"
              id="sprint-concerns-panel"
              role="tabpanel"
              aria-labelledby="sprint-concerns-tab"
            >
              <SprintConcernsPanel
                workspace={workspace}
                selectedConcernId={selectedConcernId}
                onSelectConcern={setSelectedConcernId}
                onOpenWorkUnit={(workUnitId) => {
                  const owner = ownerOf(workUnitId, activeView);
                  if (!owner) return;
                  onDetailLocationChange({
                    kind: 'work_unit',
                    revisionId: activeView.sprintPlanRevisionId,
                    workSlicePlanningPointId: owner.workSlicePlanningPointId,
                    workUnitId,
                    origin: 'concern',
                  });
                }}
              />
            </section>
          )}
          {hasStartedPlan && selectedTab === 'documents' && (
            <section
              className="sprint-tab-panel"
              id="sprint-documents-panel"
              role="tabpanel"
              aria-labelledby="sprint-documents-tab"
            >
              <SprintDocumentsPanel
                documents={workspace.documents}
                artifactAccess={artifactAccessController}
              />
            </section>
          )}
        </>
      }
      agentSession={
        adjunct?.agentSession ? (
          <SharedAgentSessionPanel
            ariaLabel="Sprint Agent Session"
            conversationAriaLabel="Sprint Agent Session conversation"
            session={adjunct.agentSession}
            composition={agentSessionComposition}
            onOpenStandalone={onOpenAgentSession}
            displayMode="always_open"
          />
        ) : undefined
      }
    />
  );
}

function SprintFileReviewControl({
  state,
  onRequest,
}: {
  readonly state: SprintFileReviewControlState;
  readonly onRequest: () => Promise<void>;
}) {
  return (
    <div className="sprint-file-review-control">
      <button type="button" disabled={state.kind === 'pending'} onClick={() => void onRequest()}>
        Review files
      </button>
      {state.kind === 'pending' ? (
        <small role="status">Preparing File Review…</small>
      ) : state.kind === 'failed' ? (
        <small role="alert" data-reason={state.reason}>
          {state.message}
        </small>
      ) : null}
    </div>
  );
}

function sprintLifecycleLabel(lifecycle: SprintWorkspacePresentationV1['sprint']['lifecycle']) {
  if (!lifecycle) return 'State unavailable';
  if (lifecycle.source.status !== 'available') return `State ${lifecycle.source.status}`;
  const value = lifecycle.value;
  if (!value) return 'State unavailable';
  return {
    not_started: 'Planned',
    in_progress: 'Processing',
    completed: 'Completed',
  }[value];
}

function focusNextSprintRunnerConcernGraphElement(
  sprintRunnerConcern: SprintWorkspacePresentationV1['sprintRunnerConcerns'][number],
  view: SprintWorkspacePresentationV1['revisionViews'][number],
  focusIndexes: Map<string, number>,
) {
  const priority = (reference: (typeof sprintRunnerConcern.graphElementRefs)[number]) => {
    if (reference.kind === 'work_unit') {
      const state = view.workUnits.find(
        ({ workUnitId }) => workUnitId === reference.id,
      )?.presentationState;
      if (['requested', 'launched', 'returned', 'under_review'].includes(state ?? '')) return 0;
      if (['integrated', 'responsibility_accepted'].includes(state ?? '')) return 1;
      return 2;
    }
    if (reference.kind === 'work_slice_planning_point') {
      const group = view.workSlicePlanningPointGroups.find(
        ({ workSlicePlanningPointId }) => workSlicePlanningPointId === reference.id,
      );
      const states = view.workUnits
        .filter(({ workUnitScopeId }) => group?.workUnitScopeIds.includes(workUnitScopeId))
        .map(({ presentationState }) => presentationState);
      if (
        states.some((state) =>
          ['requested', 'launched', 'returned', 'under_review'].includes(state),
        )
      )
        return 0;
      if (
        states.length &&
        states.every((state) => ['integrated', 'responsibility_accepted'].includes(state))
      )
        return 1;
    }
    return 2;
  };
  const ordered = [...sprintRunnerConcern.graphElementRefs].sort(
    (left, right) =>
      priority(left) - priority(right) ||
      `${left.kind}:${left.id}`.localeCompare(`${right.kind}:${right.id}`),
  );
  if (!ordered.length) return;
  const index = focusIndexes.get(sprintRunnerConcern.sprintRunnerConcernId) ?? 0;
  const next = ordered[index % ordered.length];
  focusIndexes.set(sprintRunnerConcern.sprintRunnerConcernId, (index + 1) % ordered.length);
  requestAnimationFrame(() => {
    const element = Array.from(
      document.querySelectorAll<HTMLElement>('[data-flow-element-kind][data-flow-element-id]'),
    ).find(
      (candidate) =>
        candidate.dataset.flowElementKind === next.kind &&
        candidate.dataset.flowElementId === next.id,
    );
    element?.focus();
    element?.scrollIntoView?.({ block: 'center', inline: 'center' });
  });
}

function workSlicePlanningPointSession(
  workspace: SprintWorkspacePresentationV1,
  workSlicePlanningPointId: string,
  adjunct?: SprintWorkspacePresentationAdjunct,
): SprintAgentSessionPresentation | undefined {
  const adjunctById = new Map(
    (adjunct?.workSlicePlanningPointSessions ?? []).map((session) => [session.sessionId, session]),
  );
  const sessions = workspace.agentSessionReferences
    .filter(
      (reference) =>
        reference.targetKind === 'work_slice_planning_point' &&
        reference.targetId === workSlicePlanningPointId &&
        reference.semanticRole === 'work_slice_planner',
    )
    .map((reference) => ({
      sessionId: reference.agentSessionId,
      title: reference.title,
      transcript: adjunctById.get(reference.agentSessionId)?.transcript,
    }));
  return sessions.length === 1 ? sessions[0] : undefined;
}

function workSlicePlanningPointState(
  group: SprintWorkspacePresentationV1['revisionViews'][number]['workSlicePlanningPointGroups'][number],
  view: SprintWorkspacePresentationV1['revisionViews'][number],
) {
  const states = view.workUnits
    .filter(({ workUnitScopeId }) => group.workUnitScopeIds.includes(workUnitScopeId))
    .map(({ presentationState }) => presentationState);
  if (states.length === 0) return 'No scoped Work Units';
  if (states.some((state) => ['requested', 'launched', 'returned', 'under_review'].includes(state)))
    return 'Processing';
  if (states.every((state) => ['integrated', 'responsibility_accepted'].includes(state)))
    return 'Completed';
  if (states.every((state) => state === 'deferred')) return 'Deferred';
  if (states.every((state) => ['not_started', 'waiting_for_dependencies'].includes(state)))
    return 'Planned';
  return 'Mixed';
}

function planningPointWorkUnitRelationships(
  workspace: SprintWorkspacePresentationV1,
  view: SprintWorkspacePresentationV1['revisionViews'][number],
  group: SprintWorkspacePresentationV1['revisionViews'][number]['workSlicePlanningPointGroups'][number],
  adjunct?: SprintWorkspacePresentationAdjunct,
): readonly PlanningPointWorkUnitRelationship[] {
  return group.workUnitScopeIds.map((workUnitScopeId) => {
    const workUnit = view.workUnits.find(
      (candidate) => candidate.workUnitScopeId === workUnitScopeId,
    );
    if (!workUnit)
      throw new Error(
        `Work Slice planning point ${group.workSlicePlanningPointId} references missing scope ${workUnitScopeId}`,
      );
    const sessions = workUnitSessions(workspace, workUnit, adjunct);
    return {
      workUnit,
      handlers: sessions.filter(({ role }) => role === 'handler'),
      implementers: sessions.filter(({ role }) => role === 'implementer'),
    };
  });
}

function workUnitSessions(
  workspace: SprintWorkspacePresentationV1,
  unit: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number],
  adjunct?: SprintWorkspacePresentationAdjunct,
): readonly WorkUnitAgentSessionPresentation[] {
  const adjunctSessions = (adjunct?.workUnitSessions ?? []).filter(
    (session) => session.workUnitId === unit.workUnitId,
  );
  const executionIds = new Set(unit.attempts.map((attempt) => attempt.workUnitExecutionId));
  const adjunctById = new Map(
    [...adjunctSessions, ...(adjunct?.workSlicePlanningPointSessions ?? [])].map((session) => [
      session.sessionId,
      session,
    ]),
  );
  const referenced: WorkUnitAgentSessionPresentation[] = workspace.agentSessionReferences
    .filter(
      (reference) =>
        reference.targetKind === 'work_unit_execution' &&
        executionIds.has(reference.targetId) &&
        ['work_unit_handler', 'work_unit_implementer'].includes(reference.semanticRole),
    )
    .map((reference) => ({
      sessionId: reference.agentSessionId,
      title: reference.title,
      workUnitId: unit.workUnitId,
      role: (
        {
          work_unit_handler: 'handler',
          work_unit_implementer: 'implementer',
        } as const
      )[reference.semanticRole as 'work_unit_handler' | 'work_unit_implementer'],
      transcript: adjunctById.get(reference.agentSessionId)?.transcript,
    }));
  const view = workspace.revisionViews.find(
    ({ sprintPlanRevisionId }) => sprintPlanRevisionId === unit.sprintPlanRevisionId,
  );
  const owner = view?.workSlicePlanningPointGroups.find(({ workUnitScopeIds }) =>
    workUnitScopeIds.includes(unit.workUnitScopeId),
  );
  const planners: WorkUnitAgentSessionPresentation[] = owner
    ? workspace.agentSessionReferences
        .filter(
          (reference) =>
            reference.targetKind === 'work_slice_planning_point' &&
            reference.targetId === owner.workSlicePlanningPointId &&
            reference.semanticRole === 'work_slice_planner',
        )
        .map((reference) => ({
          sessionId: reference.agentSessionId,
          title: reference.title,
          workUnitId: unit.workUnitId,
          role: 'work_slice_planner',
          transcript: adjunctById.get(reference.agentSessionId)?.transcript,
        }))
    : [];
  return [
    ...new Map(
      [...planners, ...referenced].map((session) => [session.sessionId, session]),
    ).values(),
  ];
}
