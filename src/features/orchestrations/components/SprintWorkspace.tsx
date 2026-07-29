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
}: SprintWorkspaceProps) {
  const [selectedTab, setSelectedTab] = useState<SprintWorkspaceTab>('flow');
  const [selectedConcernId, setSelectedConcernId] = useState<string | null>(null);
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
        onOpenAgentSession={onOpenAgentSession}
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
        onOpenAgentSession={onOpenAgentSession}
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
          <h1>{workspace.sprint.title}</h1>
          <p>{workspace.sprint.summary}</p>
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
          />
        ) : undefined
      }
    />
  );
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
  // Reviewer references are attempt/review-oriented, so the existing Work Unit detail is their
  // smallest coherent surface; no reviewer is inferred from workflow prose or geometry.
  const reviewerSessions = workspace.agentSessionReferences
    .filter(
      (reference) =>
        reference.targetKind === 'work_unit_execution' &&
        executionIds.has(reference.targetId) &&
        reference.semanticRole === 'reviewer',
    )
    .map((reference) => ({
      sessionId: reference.agentSessionId,
      title: reference.title,
      workUnitId: unit.workUnitId,
      role: 'reviewer' as const,
      transcript: adjunctById.get(reference.agentSessionId)?.transcript,
    }));
  return [...existing, ...reviewerSessions];
}
