import type {
  SprintControlSurfaceProjection,
  SprintExecutionSnapshotV1,
  SprintPlannerOutputV1,
} from './sprintControlSurfaceCompatibility';
import { deriveConcernState, deriveWorkUnitPresentation } from './sprintDerivedState';
import type { SprintReadModel } from './sprintReadModels';
import { projectSprintRelationshipGraph } from './sprintRelationshipGraph';

export function assembleSprintControlSurface(
  planner: SprintPlannerOutputV1,
  snapshot: SprintExecutionSnapshotV1,
  selectedPlanRevisionId: string,
): SprintControlSurfaceProjection {
  const selected = planner.planRevisions.find(({ id }) => id === selectedPlanRevisionId);
  if (!selected) fail('selected plan revision is unknown');
  const revisionViews = planner.planRevisions.map((revision) =>
    projectRevisionView(planner, snapshot, revision.id),
  );
  const selectedView = revisionViews.find(({ planRevisionId }) => planRevisionId === selected.id);
  if (!selectedView) fail('selected plan revision has no projected view');
  const stateByUnit = new Map(
    selectedView.workUnits.map((unit) => [unit.id, unit.presentationState]),
  );
  const decisions = new Map(
    snapshot.concernDecisions.map((decision) => [decision.concernId, decision]),
  );
  const concerns = planner.concerns.map((concern) => ({
    ...concern,
    state: deriveConcernState(concern, decisions.get(concern.id), stateByUnit),
  }));
  const documents = [
    ...planner.documents.map((document) => ({ ...document, provenance: 'planner' as const })),
    ...snapshot.generatedDocuments.map((document) => ({
      ...document,
      provenance: 'execution' as const,
    })),
  ].sort((left, right) => right.recordedAt.localeCompare(left.recordedAt));
  const readModel = assembleSprintReadModel(planner, snapshot, selected.id, selectedView.workUnits);
  return {
    sourceAuthority: 'recorded_compatibility',
    readModel,
    sprint: planner.sprint,
    activePlanRevisionId: snapshot.activePlanRevisionId,
    selectedPlanRevisionId: selected.id,
    revisionGraph: planner.planRevisions.map((revision) => ({
      ...revision,
      isActive: revision.id === snapshot.activePlanRevisionId,
      isSelected: revision.id === selected.id,
    })),
    workUnits: selectedView.workUnits,
    concerns,
    documents,
    mapLayout: selectedView.mapLayout,
    sprintPlannerActivities: selectedView.sprintPlannerActivities,
    sprintPlannerActivityGroups: selectedView.sprintPlannerActivityGroups,
    gates: selectedView.gates,
    parallelGroups: selectedView.parallelGroups,
    planChanges: selectedView.planChanges,
    revisionViews,
    agentSessions: snapshot.agentSessions,
    continuation: snapshot.continuation,
  };
}

function projectRevisionView(
  planner: SprintPlannerOutputV1,
  snapshot: SprintExecutionSnapshotV1,
  planRevisionId: string,
): SprintControlSurfaceProjection['revisionViews'][number] {
  const revision = planner.planRevisions.find(({ id }) => id === planRevisionId);
  if (!revision) fail('selected plan revision is unknown');
  const executions = new Map(
    snapshot.workUnits.map((execution) => [execution.workUnitId, execution]),
  );
  const selectedIds = new Set(revision.workUnitIds);
  const workUnits: SprintControlSurfaceProjection['workUnits'] = planner.workUnits
    .filter(({ id }) => selectedIds.has(id))
    .map((unit) => {
      const spec = unit.specRevisions
        .filter((candidate) => candidate.planRevisionId === revision.id)
        .sort((a, b) => a.revision - b.revision)
        .at(-1);
      if (!spec) fail(`selected plan has no spec revision for ${unit.id}`);
      const execution = executions.get(unit.id);
      const executionState = execution?.state ?? 'projected';
      return {
        id: unit.id,
        shortTitle: unit.shortTitle,
        summary: unit.summary,
        details: unit.details,
        concernIds: unit.concernIds,
        specRevision: {
          id: spec.id,
          revision: spec.revision,
          summary: spec.summary,
          details: spec.details,
        },
        executionState,
        presentationState: deriveWorkUnitPresentation(unit, executionState, executions),
        journey: {
          specRevisions: unit.specRevisions.map(({ id, revision, planRevisionId, summary }) => ({
            id,
            revision,
            planRevisionId,
            summary,
          })),
          attemptDetails: execution?.attempts ?? [],
          events: snapshot.events.filter(({ workUnitId }) => workUnitId === unit.id),
          attempts: execution?.attempts.length ?? 0,
          hasWorkerFeedback:
            execution?.attempts.some(({ workerFeedback }) => Boolean(workerFeedback)) ?? false,
          accepted: executionState === 'accepted',
          launched: execution?.actualLaunch !== undefined,
        },
        dependencies: unit.dependencies,
        parallelGroupId: unit.parallelGroupId,
      };
    });
  const activities = planner.sprintPlannerActivities.filter(
    ({ planRevisionId }) => planRevisionId === revision.id,
  );
  const gates = selectGates(planner, revision.id, workUnits);
  return {
    planRevisionId: revision.id,
    workUnits,
    sprintPlannerActivities: activities,
    sprintPlannerActivityGroups: activities,
    gates: gates.map((gate) => ({
      id: gate.id,
      kind: gate.kind,
      summary: gate.specRevisions.at(-1)?.summary ?? gate.id,
    })),
    parallelGroups: planner.parallelGroups.filter(
      ({ planRevisionId }) => planRevisionId === revision.id,
    ),
    planChanges: planner.planChanges.filter(
      ({ resultingPlanRevisionId }) => resultingPlanRevisionId === revision.id,
    ),
    mapLayout: projectSprintRelationshipGraph(planner, revision.id, workUnits, activities, gates),
  };
}

function assembleSprintReadModel(
  planner: SprintPlannerOutputV1,
  snapshot: SprintExecutionSnapshotV1,
  selectedId: string,
  units: SprintControlSurfaceProjection['workUnits'],
): SprintReadModel {
  return {
    epic: { epicId: planner.epicId },
    sprint: {
      sprintId: planner.sprint.id,
      epicId: planner.epicId,
      title: planner.sprint.title,
      summary: planner.sprint.summary,
      details: planner.sprint.details,
    },
    sprintPlan: { sprintPlanId: planner.sprintPlan.id, sprintId: planner.sprintPlan.sprintId },
    activeSprintPlanRevisionId: snapshot.activePlanRevisionId,
    selectedSprintPlanRevisionId: selectedId,
    sprintPlanRevisions: planner.planRevisions.map((revision) => ({
      sprintPlanRevisionId: revision.id,
      sprintPlanId: planner.sprintPlan.id,
      revision: revision.revision,
      summary: revision.summary,
      supersedesSprintPlanRevisionId: revision.supersedesPlanRevisionId,
      workUnitIds: revision.workUnitIds,
      isActive: revision.id === snapshot.activePlanRevisionId,
      isSelected: revision.id === selectedId,
    })),
    sprintPlannerActivities: planner.sprintPlannerActivities.map((activity) => ({
      sprintPlannerActivityId: activity.id,
      sprintPlanRevisionId: activity.planRevisionId,
      title: activity.title,
      purpose: activity.purpose,
      workUnitIds: activity.workUnitIds,
      userReviewGateIds: activity.userReviewGateIds,
    })),
    workUnits: units.map((unit) => {
      const execution = snapshot.workUnits.find(({ workUnitId }) => workUnitId === unit.id);
      return {
        workUnitId: unit.id,
        title: unit.shortTitle,
        summary: unit.summary,
        details: unit.details,
        concernIds: unit.concernIds,
        sprintPlanRevisionId: selectedId,
        selectedScopeDefinitionId: unit.specRevision.id,
        executionState: unit.executionState,
        presentationState: unit.presentationState,
        executionRequestObserved: false,
        launchObserved: execution?.actualLaunch !== undefined,
        responsibilityAccepted: unit.executionState === 'accepted',
        attempts: unit.journey.attempts,
        dependencies: unit.dependencies,
      };
    }),
    agentSessionReferences: snapshot.agentSessions.map(({ id, ...session }) => ({
      agentSessionRefId: id,
      ...session,
    })),
    continuation: snapshot.continuation,
  };
}

function selectGates(
  planner: SprintPlannerOutputV1,
  revisionId: string,
  workUnits: SprintControlSurfaceProjection['workUnits'],
): SprintPlannerOutputV1['gates'] {
  const ids = new Set(
    planner.gates
      .filter((gate) => gate.specRevisions.some((spec) => spec.planRevisionId === revisionId))
      .map(({ id }) => id),
  );
  workUnits.forEach((unit) =>
    unit.dependencies.forEach(({ gateId }) => {
      if (gateId) ids.add(gateId);
    }),
  );
  let changed = true;
  while (changed) {
    changed = false;
    planner.gates.forEach((gate) => {
      if (!ids.has(gate.id)) return;
      gate.specRevisions.forEach((spec) =>
        spec.requiresGateIds.forEach((id) => {
          if (!ids.has(id)) {
            ids.add(id);
            changed = true;
          }
        }),
      );
    });
  }
  return planner.gates
    .filter(({ id }) => ids.has(id))
    .map((gate) => {
      const exact = gate.specRevisions.filter(
        ({ planRevisionId }) => planRevisionId === revisionId,
      );
      return { ...gate, specRevisions: exact.length ? exact : gate.specRevisions.slice(0, 1) };
    });
}
function fail(message: string): never {
  throw new Error(`Invalid Sprint control surface data: ${message}`);
}
