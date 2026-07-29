import type {
  SprintWorkspacePresentationAdjunct,
  SprintWorkspaceDetailLocation,
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
import { SprintPlannerActivityDetailWorkspace } from './SprintPlannerActivityDetailWorkspace';
import { WorkUnitDetailWorkspace } from './WorkUnitDetailWorkspace';
import '../styles/sprintWorkspace.css';
import type { EmbeddedAgentSessionComposition } from '../../agentSessions';

export interface SprintWorkspaceProps {
  readonly workspace: SprintWorkspacePresentationV1;
  readonly epicObjective: string;
  readonly adjunct?: SprintWorkspacePresentationAdjunct;
  readonly artifactAccessController: ArtifactAccessController;
  readonly agentSessionComposition?: EmbeddedAgentSessionComposition;
  readonly automaticContinuationPolicyController?: SprintAutomaticContinuationPolicyController;
  readonly selectedRevisionId: string;
  readonly onSelectedRevisionChange: (revisionId: string) => void;
  readonly detailLocation: SprintWorkspaceDetailLocation;
  readonly onDetailLocationChange: (location: SprintWorkspaceDetailLocation) => void;
  readonly onBack: () => void;
  readonly onOpenFileReviewSource?: (sourceId: string) => void;
}

export function SprintWorkspace({
  workspace,
  epicObjective,
  adjunct,
  artifactAccessController,
  agentSessionComposition,
  automaticContinuationPolicyController,
  selectedRevisionId,
  onSelectedRevisionChange,
  detailLocation,
  onDetailLocationChange,
  onBack,
  onOpenFileReviewSource,
}: SprintWorkspaceProps) {
  const [selectedTab, setSelectedTab] = useState<SprintWorkspaceTab>('flow');
  const [selectedConcernId, setSelectedConcernId] = useState<string | null>(null);
  const [highlightedProblemId, setHighlightedProblemId] = useState<string | null>(null);
  const [hoveredGraphElement, setHoveredGraphElement] = useState<{
    readonly kind: 'sprint_planner_activity' | 'work_unit' | 'gate';
    readonly id: string;
  } | null>(null);
  const problemFocusIndexRef = useRef(new Map<string, number>());
  const sprintRestoreRef = useRef<{
    kind: 'sprint_planner_activity_group' | 'work_unit';
    id: string;
  } | null>(null);
  const concernRestoreWorkUnitRef = useRef<string | null>(null);

  useEffect(() => {
    if (detailLocation.kind !== 'sprint' || !sprintRestoreRef.current) return;
    const restore = sprintRestoreRef.current;
    sprintRestoreRef.current = null;
    document
      .querySelector<HTMLButtonElement>(
        restore.kind === 'sprint_planner_activity_group'
          ? `[data-sprint-planner-activity-id="${restore.id}"]`
          : `[data-work-unit-id="${restore.id}"]`,
      )
      ?.focus();
  }, [detailLocation]);

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
    view.plannerActivityGroups.find(({ workUnitScopeIds }) =>
      workUnitScopeIds.includes(
        view.workUnits.find((unit) => unit.workUnitId === workUnitId)?.workUnitScopeId ?? '',
      ),
    );

  if (detailLocation.kind === 'work_unit') {
    const view = workspace.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === detailLocation.revisionId,
    )!;
    const plannerActivityGroup = view.plannerActivityGroups.find(
      ({ sprintPlannerActivityId }) =>
        sprintPlannerActivityId === detailLocation.sprintPlannerActivityId,
    )!;
    const unit = view.workUnits.find(({ workUnitId }) => workUnitId === detailLocation.workUnitId)!;
    return (
      <WorkUnitDetailWorkspace
        unit={unit}
        lifecycleEntries={workspace.workUnitLifecycle.filter(
          ({ workUnitId }) => workUnitId === unit.workUnitId,
        )}
        sprintPlannerActivityGroupTitle={plannerActivityGroup.title}
        sessions={workUnitSessions(workspace, unit, adjunct)}
        agentSessionComposition={agentSessionComposition}
        backLabel={detailLocation.origin === 'concern' ? 'Back to Concern' : undefined}
        onBack={() => {
          if (detailLocation.origin === 'concern') {
            concernRestoreWorkUnitRef.current = detailLocation.workUnitId;
            onDetailLocationChange({ kind: 'sprint' });
            return;
          }
          onDetailLocationChange({
            kind: 'sprint_planner_activity_group',
            revisionId: detailLocation.revisionId,
            sprintPlannerActivityId: detailLocation.sprintPlannerActivityId,
          });
        }}
      />
    );
  }

  if (detailLocation.kind === 'sprint_planner_activity_group') {
    const view = workspace.revisionViews.find(
      ({ sprintPlanRevisionId }) => sprintPlanRevisionId === detailLocation.revisionId,
    )!;
    const plannerActivityGroup = view.plannerActivityGroups.find(
      ({ sprintPlannerActivityId }) =>
        sprintPlannerActivityId === detailLocation.sprintPlannerActivityId,
    )!;
    return (
      <SprintPlannerActivityDetailWorkspace
        plannerActivityGroup={plannerActivityGroup}
        sessions={plannerActivitySessions(
          workspace,
          plannerActivityGroup.sprintPlannerActivityId,
          adjunct,
        )}
        agentSessionComposition={agentSessionComposition}
        workflow={adjunct?.plannerActivityWorkflows.find(
          ({ sprintPlannerActivityId }) =>
            sprintPlannerActivityId === plannerActivityGroup.sprintPlannerActivityId,
        )}
        onBack={() => onDetailLocationChange({ kind: 'sprint' })}
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
      hotbarNavigation={<SprintWorkspaceTabs selected={selectedTab} onSelect={setSelectedTab} />}
      control={
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
          <section className="sprint-context__objectives" aria-label="Epic Planner objectives">
            <h2>Epic Planner objectives</h2>
            <ul>
              <li>{epicObjective}</li>
            </ul>
          </section>
          {workspace.problems.length > 0 ? (
            <section className="sprint-context__problems" aria-label="Sprint Planner problems">
              <h2>Sprint Planner problems</h2>
              <ul>
                {workspace.problems.map((problem) => {
                  const relatedToHover = hoveredGraphElement
                    ? problem.graphElementRefs.some(
                        (reference) =>
                          reference.kind === hoveredGraphElement.kind &&
                          reference.id === hoveredGraphElement.id,
                      )
                    : false;
                  return (
                    <li key={problem.problemId}>
                      <button
                        type="button"
                        className={
                          highlightedProblemId === problem.problemId || relatedToHover
                            ? 'is-highlighted'
                            : undefined
                        }
                        aria-pressed={highlightedProblemId === problem.problemId}
                        onPointerEnter={() => setHighlightedProblemId(problem.problemId)}
                        onPointerLeave={() => setHighlightedProblemId(null)}
                        onFocus={() => setHighlightedProblemId(problem.problemId)}
                        onBlur={() => setHighlightedProblemId(null)}
                        onClick={() => {
                          setSelectedTab('flow');
                          focusNextProblemGraphElement(
                            problem,
                            selectedView,
                            problemFocusIndexRef.current,
                          );
                        }}
                      >
                        {problem.title}
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
          {selectedTab === 'flow' && (
            <section
              className="sprint-tab-panel"
              id="sprint-flow-panel"
              role="tabpanel"
              aria-labelledby="sprint-flow-tab"
            >
              <section className="sprint-surface-host" aria-label="Sprint planning workspace">
                <SprintFlowMap
                  workspace={workspace}
                  selectedRevisionId={selectedRevisionId}
                  onSelectedRevisionChange={onSelectedRevisionChange}
                  highlightedProblemId={highlightedProblemId}
                  hoveredGraphElement={hoveredGraphElement}
                  onHoveredGraphElementChange={setHoveredGraphElement}
                  onOpenSprintPlannerActivityGroup={(sprintPlannerActivityId) => {
                    sprintRestoreRef.current = {
                      kind: 'sprint_planner_activity_group',
                      id: sprintPlannerActivityId,
                    };
                    onDetailLocationChange({
                      kind: 'sprint_planner_activity_group',
                      revisionId: selectedRevisionId,
                      sprintPlannerActivityId,
                    });
                  }}
                  onOpenWorkUnit={(workUnitId) => {
                    const owner = ownerOf(workUnitId, selectedView);
                    if (!owner) return;
                    sprintRestoreRef.current = { kind: 'work_unit', id: workUnitId };
                    onDetailLocationChange({
                      kind: 'work_unit',
                      revisionId: selectedRevisionId,
                      sprintPlannerActivityId: owner.sprintPlannerActivityId,
                      workUnitId,
                      origin: 'sprint_planner_activity_group',
                    });
                  }}
                />
              </section>
            </section>
          )}
          {selectedTab === 'concerns' && (
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
                    sprintPlannerActivityId: owner.sprintPlannerActivityId,
                    workUnitId,
                    origin: 'concern',
                  });
                }}
              />
            </section>
          )}
          {selectedTab === 'documents' && (
            <section
              className="sprint-tab-panel"
              id="sprint-documents-panel"
              role="tabpanel"
              aria-labelledby="sprint-documents-tab"
            >
              <SprintDocumentsPanel
                documents={workspace.documents}
                artifactAccess={artifactAccessController}
                onOpenFileReviewSource={onOpenFileReviewSource}
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
            displayMode="always_open"
          />
        ) : undefined
      }
    />
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

function focusNextProblemGraphElement(
  problem: SprintWorkspacePresentationV1['problems'][number],
  view: SprintWorkspacePresentationV1['revisionViews'][number],
  focusIndexes: Map<string, number>,
) {
  const priority = (reference: (typeof problem.graphElementRefs)[number]) => {
    if (reference.kind === 'work_unit') {
      const state = view.workUnits.find(
        ({ workUnitId }) => workUnitId === reference.id,
      )?.presentationState;
      if (['requested', 'launched', 'returned', 'under_review'].includes(state ?? '')) return 0;
      if (['integrated', 'responsibility_accepted'].includes(state ?? '')) return 1;
      return 2;
    }
    if (reference.kind === 'sprint_planner_activity') {
      const group = view.plannerActivityGroups.find(
        ({ sprintPlannerActivityId }) => sprintPlannerActivityId === reference.id,
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
  const ordered = [...problem.graphElementRefs].sort(
    (left, right) =>
      priority(left) - priority(right) ||
      `${left.kind}:${left.id}`.localeCompare(`${right.kind}:${right.id}`),
  );
  if (!ordered.length) return;
  const index = focusIndexes.get(problem.problemId) ?? 0;
  const next = ordered[index % ordered.length];
  focusIndexes.set(problem.problemId, (index + 1) % ordered.length);
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

function plannerActivitySessions(
  workspace: SprintWorkspacePresentationV1,
  plannerActivityId: string,
  adjunct?: SprintWorkspacePresentationAdjunct,
) {
  const adjunctById = new Map(
    (adjunct?.plannerActivitySessions ?? []).map((session) => [session.sessionId, session]),
  );
  return workspace.agentSessionReferences
    .filter(
      (reference) =>
        reference.targetKind === 'sprint_planner_activity' &&
        reference.targetId === plannerActivityId &&
        ['sprint_planner', 'work_unit_planner'].includes(reference.semanticRole),
    )
    .map((reference) => ({
      sessionId: reference.agentSessionId,
      title: reference.title,
      transcript: adjunctById.get(reference.agentSessionId)?.transcript,
    }));
}

function workUnitSessions(
  workspace: SprintWorkspacePresentationV1,
  unit: SprintWorkspacePresentationV1['revisionViews'][number]['workUnits'][number],
  adjunct?: SprintWorkspacePresentationAdjunct,
) {
  const existing = (adjunct?.workUnitSessions ?? []).filter(
    (session) => session.workUnitId === unit.workUnitId,
  );
  const executionIds = new Set(unit.attempts.map((attempt) => attempt.workUnitExecutionId));
  const adjunctById = new Map(existing.map((session) => [session.sessionId, session]));
  const referenced = workspace.agentSessionReferences
    .filter(
      (reference) =>
        reference.targetKind === 'work_unit_execution' &&
        executionIds.has(reference.targetId) &&
        ['work_unit_handler', 'work_unit_worker', 'reviewer'].includes(reference.semanticRole),
    )
    .map((reference) => ({
      sessionId: reference.agentSessionId,
      title: reference.title,
      workUnitId: unit.workUnitId,
      role: (
        {
          work_unit_handler: 'handler',
          work_unit_worker: 'worker',
          reviewer: 'reviewer',
        } as const
      )[reference.semanticRole as 'work_unit_handler' | 'work_unit_worker' | 'reviewer'],
      transcript: adjunctById.get(reference.agentSessionId)?.transcript,
    }));
  return [
    ...new Map(
      [...existing, ...referenced].map((session) => [session.sessionId, session]),
    ).values(),
  ];
}
