/** Canonical product composition path. It intentionally has no compatibility or fixture imports. */
import { decodeAgentControlContractsV1 } from './agentControlDecoder';
import { decodeArtifactAccessContractsV1 } from './artifactAccessDecoder';
import { decodeOrchestrationEventsV1 } from './orchestrationEventsDecoder';
import type {
  ProductContinuationReadModelV1,
  ProductAgentSessionReferenceReadModelV1,
  ProductSprintReadModelV1,
  ProductSprintRevisionViewV1,
  ProductSprintWorkspaceNarrativesV1,
  ProductReadModelsV1,
  ProductReadCompositionInputV1,
  ProductReadReferenceIndexV1,
  ProductWorkUnitPresentationState,
} from './productReadModels';
import { projectBootstrapTransitionStatus } from './epicBootstrapTransition';

/**
 * Composition envelope rule: one decoded event root plus one decoded control/artifact root and a
 * reference index. Every indexed identity exists in that event root; available sources reference
 * known facts/provenance. A selected revision is a validated presentation selector, defaulting to
 * the terminal revision of the already-decoded linear chain.
 */
export function composeProductOrchestrationReadModels(
  input: ProductReadCompositionInputV1,
): ProductReadModelsV1 {
  const events = decodeOrchestrationEventsV1(input.events);
  const agentControl = decodeAgentControlContractsV1(input.agentControl);
  const artifactAccess = decodeArtifactAccessContractsV1(input.artifactAccess);
  const facts = eventFacts(events);
  validateReferenceIndex(input.referenceIndex, events, facts);
  validateCrossContractReferences(events, agentControl, artifactAccess, facts);
  validateBootstrapTransitions(
    input,
    events.epics.map((epic) => epic.epicId),
  );

  const index = indexReferenceData(input.referenceIndex);
  const sessions = events.agentSessionReferences.map((reference) => ({
    ...reference,
    ...required(index.sessions, reference.agentSessionId, 'Agent Session reference index'),
  }));
  const epics = events.epics.map((epic) => {
    const epicInfo = required(index.epics, epic.epicId, 'Epic reference index');
    const overview = required(index.overviews, epic.epicId, 'Epic overview reference index');
    const sprints = events.sprints
      .filter((sprint) => sprint.epicId === epic.epicId)
      .map((sprint) =>
        composeSprint(
          events,
          agentControl,
          artifactAccess,
          index,
          sessions,
          sprint.sprintId,
          input.selection,
        ),
      );
    return {
      epicId: epic.epicId,
      title: epicInfo.title,
      goal: epicInfo.goal,
      source: epicInfo.source,
      overview: {
        currentMovement: overview.currentMovement,
        state: overview.state,
      },
      sprints,
      agentSessionReferences: sessions.filter((reference) =>
        belongsToEpic(events, reference, epic.epicId),
      ),
      continuation: composeContinuation(events, agentControl, 'epic', epic.epicId),
      ...(input.bootstrapTransition
        ? {
            bootstrapTransition: projectBootstrapTransitionStatus(
              input.bootstrapTransition.query.transitions.find(
                (transition) => transition.epicId === epic.epicId,
              )!,
            ),
          }
        : {}),
    };
  });
  return {
    epics,
    unassociatedAgentSessionReferences: sessions.filter(
      (reference) => reference.targetKind === 'other',
    ),
  };
}

function validateBootstrapTransitions(
  input: ProductReadCompositionInputV1,
  epicIds: readonly string[],
) {
  if (!input.bootstrapTransition) return;
  const transitions = input.bootstrapTransition.query.transitions;
  if (transitions.length !== epicIds.length)
    fail('bootstrap transition query must contain one transition per initiated Epic');
  for (const transition of transitions) {
    if (!epicIds.includes(transition.epicId)) fail('bootstrap transition references unknown Epic');
    if (
      input.bootstrapTransition.initiationIdsByEpic[transition.epicId] !== transition.initiationId
    )
      fail('bootstrap transition does not match the native initiation identity');
  }
}

function composeSprint(
  events: ReturnType<typeof decodeOrchestrationEventsV1>,
  agentControl: ReturnType<typeof decodeAgentControlContractsV1>,
  artifacts: ReturnType<typeof decodeArtifactAccessContractsV1>,
  index: ReturnType<typeof indexReferenceData>,
  sessions: readonly ProductAgentSessionReferenceReadModelV1[],
  sprintId: string,
  selection: ProductReadCompositionInputV1['selection'],
): ProductSprintReadModelV1 {
  const sprint = required(index.sprints, sprintId, 'Sprint reference index');
  const plan = events.sprintPlans.find((candidate) => candidate.sprintId === sprintId);
  if (!plan) fail(`missing Sprint Plan for ${sprintId}`);
  const revisions = events.sprintPlanRevisions.filter(
    (revision) => revision.sprintPlanId === plan.sprintPlanId,
  );
  const orderedRevisions = orderRevisions(revisions);
  const current = orderedRevisions.at(-1);
  if (!current) fail(`Sprint ${sprintId} has no plan revision`);
  const selectedId =
    selection?.selectedSprintPlanRevisionIds?.[sprintId] ?? current.sprintPlanRevisionId;
  if (!orderedRevisions.some((revision) => revision.sprintPlanRevisionId === selectedId))
    fail(`selected revision ${selectedId} does not belong to Sprint ${sprintId}`);
  const artifactIds = new Set(
    Array.from(index.artifactOwnership.values())
      .filter((ownership) => ownership.sprintId === sprintId)
      .map((ownership) => ownership.artifactId),
  );
  const documentIds = new Set(
    Array.from(index.documentOwnership.values())
      .filter((ownership) => ownership.sprintId === sprintId)
      .map((ownership) => ownership.documentRefId),
  );
  const plannerActivities = events.sprintPlannerActivities
    .filter((activity) => activity.sprintPlanId === plan.sprintPlanId)
    .map((activity) => ({
      ...required(
        index.activities,
        activity.sprintPlannerActivityId,
        'planner activity reference index',
      ),
      assessedSprintPlanRevisionIds: activity.assessedSprintPlanRevisionIds,
    }));
  const workspacePresentation = composeWorkspacePresentation(
    index.workspacePresentation,
    events,
    sprintId,
    documentIds,
  );
  const revisionViews = orderedRevisions.map((revision) =>
    composeRevisionView(
      events,
      index,
      revision,
      current.sprintPlanRevisionId,
      selectedId,
      plannerActivities,
      workspacePresentation,
    ),
  );
  const currentWorkUnits = required(
    new Map(revisionViews.map((view) => [view.sprintPlanRevisionId, view.workUnits])),
    current.sprintPlanRevisionId,
    'current revision view',
  );
  return {
    sprintId,
    epicId:
      events.sprints.find((candidate) => candidate.sprintId === sprintId)?.epicId ??
      fail('missing Sprint'),
    title: sprint.title,
    summary: sprint.summary,
    details: sprint.details,
    source: sprint.source,
    ...(sprint.lifecycle ? { lifecycle: sprint.lifecycle } : {}),
    sprintPlan: {
      sprintPlanId: plan.sprintPlanId,
      currentSprintPlanRevisionId: current.sprintPlanRevisionId,
      selectedSprintPlanRevisionId: selectedId,
      revisions: orderedRevisions.map((revision) => ({
        sprintPlanRevisionId: revision.sprintPlanRevisionId,
        revision: revision.revision,
        summary: required(
          index.revisions,
          revision.sprintPlanRevisionId,
          'revision reference index',
        ).summary,
        source: required(index.revisions, revision.sprintPlanRevisionId, 'revision reference index')
          .source,
        ...(revision.supersedesSprintPlanRevisionId
          ? { supersedesSprintPlanRevisionId: revision.supersedesSprintPlanRevisionId }
          : {}),
        isCurrent: revision.sprintPlanRevisionId === current.sprintPlanRevisionId,
        isSelected: revision.sprintPlanRevisionId === selectedId,
        workUnitScopes: events.workUnitScopes
          .filter((scope) => scope.sprintPlanRevisionId === revision.sprintPlanRevisionId)
          .map((scope) => ({
            workUnitScopeId: scope.workUnitScopeId,
            workUnitId: scope.workUnitId,
            dependsOnWorkUnitScopeIds: scope.dependsOnWorkUnitScopeIds,
            gateIds: scope.gateIds,
          })),
      })),
    },
    plannerActivities,
    revisionViews,
    concerns: index.concerns
      .filter((concern) => concern.sprintId === sprintId)
      .map((concern) => ({
        ...concern,
        state: deriveConcernState(concern, currentWorkUnits),
      })),
    reviews: events.reviews
      .filter(
        (review) =>
          (review.subjectKind === 'sprint_plan_revision' &&
            revisions.some((revision) => revision.sprintPlanRevisionId === review.subjectId)) ||
          (review.subjectKind === 'document_reference' && documentIds.has(review.subjectId)),
      )
      .map(({ reviewId, subjectKind, subjectId, outcome, rationaleArtifactId }) => ({
        reviewId,
        subjectKind: subjectKind as 'sprint_plan_revision' | 'document_reference',
        subjectId,
        ...(outcome ? { outcome } : {}),
        ...(rationaleArtifactId ? { rationaleArtifactId } : {}),
      })),
    documents: artifacts.documents
      .filter((document) => documentIds.has(document.documentRefId))
      .map((document) => ({
        ...document,
        ownershipSource: required(
          index.documentOwnership,
          document.documentRefId,
          'Document ownership reference index',
        ).source,
      })),
    internalArtifacts: artifacts.artifacts
      .filter((artifact) => artifactIds.has(artifact.artifactId))
      .map(({ artifactId, kind, provenanceReference }) => ({
        artifactId,
        kind,
        provenanceReference,
        ownershipSource: required(
          index.artifactOwnership,
          artifactId,
          'artifact ownership reference index',
        ).source,
      })),
    workspacePresentation,
    agentSessionReferences: sessions.filter((reference) =>
      belongsToSprint(events, reference, sprintId),
    ),
    continuation: composeContinuation(events, agentControl, 'sprint', sprintId),
  };
}

function composeWorkspacePresentation(
  metadata: ProductReadReferenceIndexV1['sprintWorkspacePresentation'],
  events: ReturnType<typeof decodeOrchestrationEventsV1>,
  sprintId: string,
  documentIds: ReadonlySet<string>,
) {
  const empty = { plannerActivityMembership: [], gates: [], documents: [] } as const;
  if (!metadata) return empty;
  const revisionIds = new Set(
    events.sprintPlanRevisions
      .filter((revision) =>
        events.sprintPlans.some(
          (plan) => plan.sprintPlanId === revision.sprintPlanId && plan.sprintId === sprintId,
        ),
      )
      .map((revision) => revision.sprintPlanRevisionId),
  );
  return {
    plannerActivityMembership: metadata.plannerActivityMembership.filter((membership) =>
      revisionIds.has(membership.sprintPlanRevisionId),
    ),
    gates: metadata.gates.filter((presentation) =>
      events.gates.some(
        (gate) => gate.gateId === presentation.gateId && revisionIds.has(gate.sprintPlanRevisionId),
      ),
    ),
    documents: metadata.documents.filter((presentation) =>
      documentIds.has(presentation.documentRefId),
    ),
    ...(metadata.epicPlannerObjectives
      ? {
          epicPlannerObjectives: metadata.epicPlannerObjectives.filter(
            ({ sprintId: candidate }) => candidate === sprintId,
          ),
        }
      : {}),
    ...(metadata.problems
      ? {
          problems: metadata.problems.filter(({ sprintId: candidate }) => candidate === sprintId),
        }
      : {}),
    ...(metadata.workUnitLifecycle
      ? {
          workUnitLifecycle: metadata.workUnitLifecycle.filter(
            ({ sprintId: candidate }) => candidate === sprintId,
          ),
        }
      : {}),
    ...(metadata.narratives?.find((narratives) => narratives.sprintId === sprintId)
      ? {
          narratives: projectWorkspaceNarratives(
            metadata.narratives.find((narratives) => narratives.sprintId === sprintId)!,
          ),
        }
      : {}),
  };
}

function composeRevisionView(
  events: ReturnType<typeof decodeOrchestrationEventsV1>,
  index: ReturnType<typeof indexReferenceData>,
  revision: ReturnType<typeof decodeOrchestrationEventsV1>['sprintPlanRevisions'][number],
  currentSprintPlanRevisionId: string,
  selectedSprintPlanRevisionId: string,
  plannerActivities: readonly ProductSprintReadModelV1['plannerActivities'][number][],
  workspacePresentation: ReturnType<typeof composeWorkspacePresentation>,
): import('./productReadModels').ProductSprintRevisionViewV1 {
  const scopes = events.workUnitScopes
    .filter((scope) => scope.sprintPlanRevisionId === revision.sprintPlanRevisionId)
    .sort((left, right) => left.workUnitScopeId.localeCompare(right.workUnitScopeId));
  const unitViews = scopes.map((scope) => composeWorkUnit(events, index, scope));
  const unitByScope = new Map(unitViews.map((unit) => [unit.workUnitScopeId, unit]));
  const workUnits = unitViews.map((unit) => ({
    ...unit,
    presentationState: deriveWorkUnitState(unit, unitByScope),
  }));
  const gatePresentationById = new Map(
    workspacePresentation.gates.map((presentation) => [presentation.gateId, presentation]),
  );
  return {
    sprintPlanRevisionId: revision.sprintPlanRevisionId,
    revision: revision.revision,
    summary: required(index.revisions, revision.sprintPlanRevisionId, 'revision reference index')
      .summary,
    source: required(index.revisions, revision.sprintPlanRevisionId, 'revision reference index')
      .source,
    ...(revision.supersedesSprintPlanRevisionId
      ? { supersedesSprintPlanRevisionId: revision.supersedesSprintPlanRevisionId }
      : {}),
    isCurrent: revision.sprintPlanRevisionId === currentSprintPlanRevisionId,
    isSelected: revision.sprintPlanRevisionId === selectedSprintPlanRevisionId,
    workUnitScopes: scopes.map((scope) => ({
      workUnitScopeId: scope.workUnitScopeId,
      workUnitId: scope.workUnitId,
      dependsOnWorkUnitScopeIds: scope.dependsOnWorkUnitScopeIds,
      gateIds: scope.gateIds,
    })),
    plannerActivityGroups: workspacePresentation.plannerActivityMembership
      .filter((membership) => membership.sprintPlanRevisionId === revision.sprintPlanRevisionId)
      .map((membership) => {
        const activity = required(
          new Map(
            plannerActivities.map((candidate) => [candidate.sprintPlannerActivityId, candidate]),
          ),
          membership.sprintPlannerActivityId,
          'Planner Activity group',
        );
        return {
          sprintPlannerActivityId: activity.sprintPlannerActivityId,
          title: activity.title,
          purpose: activity.purpose,
          source: activity.source,
          membershipSource: membership.source,
          workUnitScopeIds: [...membership.workUnitScopeIds].sort(),
        };
      })
      .sort((left, right) =>
        left.sprintPlannerActivityId.localeCompare(right.sprintPlannerActivityId),
      ),
    workUnits,
    gates: events.gates
      .filter((gate) => gate.sprintPlanRevisionId === revision.sprintPlanRevisionId)
      .sort((left, right) => left.gateId.localeCompare(right.gateId))
      .map((gate) => {
        const presentation = required(
          gatePresentationById,
          gate.gateId,
          'gate workspace presentation',
        );
        return {
          ...required(index.gates, gate.gateId, 'gate reference index'),
          gateId: gate.gateId,
          criteriaRevisionIds: events.gateCriteriaRevisions
            .filter((criteria) => criteria.gateId === gate.gateId)
            .map((criteria) => criteria.gateCriteriaRevisionId),
          feedback: events.feedbackRecords
            .filter((feedback) => feedback.gateId === gate.gateId)
            .map(({ feedbackRecordId, boundary }) => ({ feedbackRecordId, boundary })),
          presentationRole: presentation.role,
          presentationSource: presentation.source,
        };
      }),
    reviews: events.reviews
      .filter(
        (review) =>
          review.subjectKind === 'sprint_plan_revision' &&
          review.subjectId === revision.sprintPlanRevisionId,
      )
      .map(({ reviewId, outcome, rationaleArtifactId }) => ({
        reviewId,
        ...(outcome ? { outcome } : {}),
        ...(rationaleArtifactId ? { rationaleArtifactId } : {}),
      })),
  };
}

function projectWorkspaceNarratives(
  narratives: Readonly<{ readonly sprintId: string }> & ProductSprintWorkspaceNarrativesV1,
): ProductSprintWorkspaceNarrativesV1 {
  return {
    ...(narratives.direction ? { direction: narratives.direction } : {}),
    ...(narratives.progress ? { progress: narratives.progress } : {}),
    ...(narratives.attention ? { attention: narratives.attention } : {}),
  };
}

function composeWorkUnit(
  events: ReturnType<typeof decodeOrchestrationEventsV1>,
  index: ReturnType<typeof indexReferenceData>,
  scope: ReturnType<typeof decodeOrchestrationEventsV1>['workUnitScopes'][number],
) {
  const info = required(index.workUnits, scope.workUnitId, 'Work Unit reference index');
  const executions = events.workUnitExecutions.filter(
    (execution) => execution.fixedWorkUnitScopeId === scope.workUnitScopeId,
  );
  const executionIds = new Set(executions.map((execution) => execution.workUnitExecutionId));
  const attempts = events.attempts.filter((attempt) =>
    executionIds.has(attempt.workUnitExecutionId),
  );
  const attemptIds = new Set(attempts.map((attempt) => attempt.attemptId));
  const launches = events.observedLaunches.filter((launch) =>
    executionIds.has(launch.workUnitExecutionId),
  );
  const launchIds = new Set(launches.map((launch) => launch.observedLaunchId));
  return {
    ...info,
    workUnitId: scope.workUnitId,
    workUnitScopeId: scope.workUnitScopeId,
    sprintPlanRevisionId: scope.sprintPlanRevisionId,
    fixedExecutionScopeIds: executions.map((execution) => execution.fixedWorkUnitScopeId),
    dependencies: scope.dependsOnWorkUnitScopeIds.map((workUnitScopeId) => ({
      workUnitScopeId,
      workUnitId:
        events.workUnitScopes.find((candidate) => candidate.workUnitScopeId === workUnitScopeId)
          ?.workUnitId ?? fail('missing dependency scope'),
    })),
    gateIds: scope.gateIds,
    attempts: attempts.map(({ attemptId, workUnitExecutionId }) => ({
      attemptId,
      workUnitExecutionId,
      returned: events.observedReturns.some((returned) => returned.attemptId === attemptId),
    })),
    reviews: events.reviews
      .filter(
        (review) =>
          (review.subjectKind === 'work_unit_execution' && executionIds.has(review.subjectId)) ||
          (review.subjectKind === 'attempt' && attemptIds.has(review.subjectId)),
      )
      .map(({ reviewId, outcome, subjectKind, subjectId }) => ({
        reviewId,
        ...(outcome ? { outcome } : {}),
        ...(subjectKind === 'attempt' ? { attemptId: subjectId } : {}),
      })),
    observed: {
      executionRequested: events.executionRequests.some((request) =>
        executionIds.has(request.workUnitExecutionId),
      ),
      launched: launches.length > 0,
      returned: events.observedReturns.some((returned) => launchIds.has(returned.observedLaunchId)),
      integrated: events.observedIntegrations.some((integration) =>
        executionIds.has(integration.workUnitExecutionId),
      ),
      responsibilityAccepted: events.observedCompletions.some(
        (completion) =>
          completion.subjectKind === 'work_unit_execution' &&
          executionIds.has(completion.subjectId) &&
          completion.responsibilityAccepted,
      ),
    },
    presentationState: 'not_started' as ProductWorkUnitPresentationState,
  };
}

function composeContinuation(
  events: ReturnType<typeof decodeOrchestrationEventsV1>,
  controls: ReturnType<typeof decodeAgentControlContractsV1>,
  level: 'sprint' | 'epic',
  targetId: string,
): ProductContinuationReadModelV1 {
  const policy = controls.continuationPolicies.find((candidate) =>
    level === 'sprint'
      ? candidate.level === 'sprint' && candidate.sprintId === targetId
      : candidate.level === 'epic' && candidate.epicId === targetId,
  );
  const eligibility =
    policy &&
    controls.continuationEligibilityEvaluations.find(
      (candidate) => candidate.continuationPolicyId === policy.continuationPolicyId,
    );
  const commands = controls.commands.filter((command) =>
    level === 'sprint'
      ? command.commandKind === 'request_next_ready_work_unit_planner' &&
        command.target.kind === 'next_ready_work_unit_planner' &&
        command.target.sprintId === targetId
      : command.commandKind === 'request_next_sprint_planner' &&
        command.target.kind === 'next_sprint_planner' &&
        command.target.epicId === targetId,
  );
  const commandIds = new Set(commands.map((command) => command.agentControlCommandId));
  const eventFacts = events.policyEligibilityFacts.filter(
    (fact) => fact.level === level && fact.targetId === targetId,
  );
  const requests = events.continuationRequests.filter((request) =>
    eventFacts.some((fact) => fact.policyEligibilityFactId === request.policyEligibilityFactId),
  );
  const requestIds = new Set(requests.map((request) => request.continuationRequestId));
  const observedContinuationIds = events.observedContinuations
    .filter((continuation) => requestIds.has(continuation.continuationRequestId))
    .map((continuation) => continuation.observedContinuationId);
  return {
    level,
    policy: policy
      ? { policyId: policy.continuationPolicyId, automaticEnabled: policy.autoFlowEnabled }
      : null,
    eligibility: eligibility
      ? {
          evaluationId: eligibility.continuationEligibilityEvaluationId,
          status: eligibility.result.status,
          ...(eligibility.result.feedbackBoundary
            ? { feedbackBoundary: eligibility.result.feedbackBoundary }
            : {}),
        }
      : null,
    commandResults: controls.results
      .filter((result) => commandIds.has(result.agentControlCommandId))
      .map(({ agentControlCommandId, state }) => ({ commandId: agentControlCommandId, state })),
    eventEligibilityFacts: eventFacts.map(
      ({ policyEligibilityFactId, autoFlowEnabled, eligible }) => ({
        policyEligibilityFactId,
        automaticEnabled: autoFlowEnabled,
        eligible,
      }),
    ),
    continuationRequests: requests.map(({ continuationRequestId, targetKind }) => ({
      continuationRequestId,
      targetKind,
    })),
    observedContinuationIds,
    initiationObserved: observedContinuationIds.length > 0,
  };
}

function deriveWorkUnitState(
  unit: ReturnType<typeof composeWorkUnit>,
  byScope: ReadonlyMap<string, ReturnType<typeof composeWorkUnit>>,
): ProductWorkUnitPresentationState {
  if (unit.observed.responsibilityAccepted) return 'responsibility_accepted';
  if (unit.observed.integrated) return 'integrated';
  if (unit.reviews.length) return 'under_review';
  if (unit.observed.returned) return 'returned';
  if (unit.observed.launched) return 'launched';
  if (unit.observed.executionRequested) return 'requested';
  if (
    unit.dependencies.some(
      ({ workUnitScopeId }) =>
        byScope.get(workUnitScopeId)?.observed.responsibilityAccepted !== true,
    )
  )
    return 'waiting_for_dependencies';
  return 'not_started';
}

function deriveConcernState(
  concern: ProductReadReferenceIndexV1['concerns'][number],
  units: ProductSprintRevisionViewV1['workUnits'],
) {
  if (concern.stateAuthority.kind === 'explicit_decision')
    return concern.stateAuthority.decision === 'accepted'
      ? ('responsibility_accepted' as const)
      : ('deferred' as const);
  const states = concern.requiredWorkUnitIds.map(
    (id) => units.find((unit) => unit.workUnitId === id)?.presentationState ?? 'not_started',
  );
  if (states.includes('responsibility_accepted'))
    return states.every((state) => state === 'responsibility_accepted')
      ? 'responsibility_accepted'
      : 'waiting_for_dependencies';
  return states.includes('waiting_for_dependencies')
    ? 'waiting_for_dependencies'
    : (states[0] ?? 'not_started');
}

function validateCrossContractReferences(
  events: ReturnType<typeof decodeOrchestrationEventsV1>,
  controls: ReturnType<typeof decodeAgentControlContractsV1>,
  artifacts: ReturnType<typeof decodeArtifactAccessContractsV1>,
  facts: ReadonlyMap<string, string | undefined>,
) {
  const sessionRefs = new Set(
    events.agentSessionReferences.map((reference) => reference.agentSessionRefId),
  );
  const eventIdentity = new Set([
    ...events.epics.map((item) => item.epicId),
    ...events.sprints.map((item) => item.sprintId),
    ...events.agentSessionReferences.map((item) => item.agentSessionRefId),
  ]);
  controls.commands.forEach((command) => {
    if (!sessionRefs.has(command.recipientAgentSessionRefId))
      fail('Agent Control recipient must be an Orchestration Event Agent Session reference');
    const targetId =
      command.target.kind === 'next_ready_work_unit_planner'
        ? command.target.sprintId
        : command.target.kind === 'next_sprint_planner'
          ? command.target.epicId
          : command.target.agentSessionRefId;
    if (!eventIdentity.has(targetId))
      fail('Agent Control target is not present in the Orchestration Event root');
  });
  const eventResultByCommand = new Set<string>();
  controls.results.forEach((result) => {
    if (result.state !== 'orchestration_event_recorded') return;
    if (!result.orchestrationEventReference || !facts.has(result.orchestrationEventReference))
      fail('event-recorded Agent Control result requires an Orchestration Event fact');
    if (eventResultByCommand.has(result.agentControlCommandId))
      fail('Agent Control command cannot record contradictory Orchestration Event outcomes');
    eventResultByCommand.add(result.agentControlCommandId);
  });
  const artifactsById = new Map(
    events.internalArtifacts.map((artifact) => [artifact.artifactId, artifact.provenanceId]),
  );
  artifacts.artifacts.forEach((artifact) => {
    if (artifactsById.get(artifact.artifactId) !== artifact.provenanceReference)
      fail('artifact contract must match Orchestration Event artifact identity and provenance');
    artifact.relatedFactReferences?.forEach((reference) => {
      if (!facts.has(reference)) fail('artifact related fact is not an Orchestration Event fact');
    });
  });
  if (
    artifacts.artifacts.length !== events.internalArtifacts.length ||
    artifacts.artifacts.some((artifact) => !artifactsById.has(artifact.artifactId))
  )
    fail('artifact contracts must represent every Orchestration Event artifact');
  const documentsById = new Map(
    events.documentReferences.map((document) => [document.documentRefId, document.provenanceId]),
  );
  artifacts.documents.forEach((document) => {
    if (documentsById.get(document.documentRefId) !== document.provenanceReference)
      fail('Document contract must match Orchestration Event Document identity and provenance');
    const eventDocument = events.documentReferences.find(
      (candidate) => candidate.documentRefId === document.documentRefId,
    );
    if (!eventDocument || !sameMembers(document.artifactIds, eventDocument.artifactIds))
      fail('Document contract artifact membership must match its Orchestration Event Document');
  });
  if (
    artifacts.documents.length !== events.documentReferences.length ||
    artifacts.documents.some((document) => !documentsById.has(document.documentRefId))
  )
    fail('Document contracts must represent every Orchestration Event Document');
}

function validateReferenceIndex(
  index: ProductReadReferenceIndexV1,
  events: ReturnType<typeof decodeOrchestrationEventsV1>,
  facts: ReadonlyMap<string, string | undefined>,
) {
  const identities = new Set([
    ...events.epics.map((item) => item.epicId),
    ...events.sprints.map((item) => item.sprintId),
    ...events.sprintPlanRevisions.map((item) => item.sprintPlanRevisionId),
    ...events.sprintPlannerActivities.map((item) => item.sprintPlannerActivityId),
    ...events.workUnits.map((item) => item.workUnitId),
    ...events.gates.map((item) => item.gateId),
    ...events.agentSessions.map((item) => item.agentSessionId),
    ...events.internalArtifacts.map((item) => item.artifactId),
    ...events.documentReferences.map((item) => item.documentRefId),
  ]);
  const entries: readonly {
    readonly source: import('./productReadModels').ReadSourceAuthorityV1;
  }[] = [
    ...index.epics,
    ...index.epicOverviews.map((item) => ({ source: item.currentMovement.source })),
    ...index.epicOverviews.map((item) => ({ source: item.state.source })),
    ...index.sprints,
    ...index.sprintPlanRevisions,
    ...index.plannerActivities,
    ...index.workUnits,
    ...index.gates,
    ...index.concerns,
    ...index.agentSessions,
    ...index.artifactOwnership,
    ...index.documentOwnership,
  ];
  const indexed = [
    ...index.epics.map((entry) => entry.epicId),
    ...index.sprints.map((entry) => entry.sprintId),
    ...index.sprintPlanRevisions.map((entry) => entry.sprintPlanRevisionId),
    ...index.plannerActivities.map((entry) => entry.sprintPlannerActivityId),
    ...index.workUnits.map((entry) => entry.workUnitId),
    ...index.gates.map((entry) => entry.gateId),
    ...index.agentSessions.map((entry) => entry.agentSessionId),
    ...index.artifactOwnership.map((entry) => entry.artifactId),
    ...index.documentOwnership.map((entry) => entry.documentRefId),
  ];
  indexed.forEach((id) => {
    if (!identities.has(id)) fail(`reference index contains unknown identity ${id}`);
  });
  entries.forEach((entry) => {
    if (
      entry.source.status === 'available' &&
      !entry.source.sourceReferences.every((reference) => facts.has(reference))
    )
      fail('available reference index source must name known facts or provenance');
  });
  index.concerns.forEach((concern) => {
    if (!events.sprints.some((sprint) => sprint.sprintId === concern.sprintId))
      fail('concern references an unknown Sprint');
    concern.requiredWorkUnitIds.forEach((id) => {
      if (!events.workUnits.some((unit) => unit.workUnitId === id))
        fail('concern references an unknown Work Unit');
    });
    if (concern.stateAuthority.kind === 'explicit_decision') {
      const provenanceId = concern.stateAuthority.provenanceId;
      if (!events.provenance.some((item) => item.provenanceId === provenanceId))
        fail('explicit concern decision requires Event provenance');
    }
  });
  index.epicOverviews.forEach((overview) => {
    if (!events.epics.some((epic) => epic.epicId === overview.epicId))
      fail('Epic overview references an unknown Epic');
    validateSourcedReadValue(overview.currentMovement, facts, 'Epic current movement');
    validateSourcedReadValue(overview.state, facts, 'Epic state');
  });
  requireComplete(
    index.epics,
    events.epics.map((item) => item.epicId),
    'Epic',
    (item) => item.epicId,
  );
  requireComplete(
    index.epicOverviews,
    events.epics.map((item) => item.epicId),
    'Epic overview',
    (item) => item.epicId,
  );
  requireComplete(
    index.sprints,
    events.sprints.map((item) => item.sprintId),
    'Sprint',
    (item) => item.sprintId,
  );
  requireComplete(
    index.artifactOwnership,
    events.internalArtifacts.map((item) => item.artifactId),
    'artifact ownership',
    (item) => item.artifactId,
  );
  requireComplete(
    index.documentOwnership,
    events.documentReferences.map((item) => item.documentRefId),
    'Document ownership',
    (item) => item.documentRefId,
  );
  const artifactOwnerById = new Map(
    index.artifactOwnership.map((ownership) => [ownership.artifactId, ownership]),
  );
  index.artifactOwnership.forEach((ownership) => {
    if (!events.sprints.some((sprint) => sprint.sprintId === ownership.sprintId))
      fail('artifact ownership references an unknown Sprint');
    validateAvailableSource(ownership.source, facts, 'artifact ownership');
  });
  index.documentOwnership.forEach((ownership) => {
    if (!events.sprints.some((sprint) => sprint.sprintId === ownership.sprintId))
      fail('Document ownership references an unknown Sprint');
    validateAvailableSource(ownership.source, facts, 'Document ownership');
    const eventDocument = events.documentReferences.find(
      (document) => document.documentRefId === ownership.documentRefId,
    );
    if (!eventDocument) fail('Document ownership references an unknown Document');
    eventDocument.artifactIds.forEach((artifactId) => {
      if (artifactOwnerById.get(artifactId)?.sprintId !== ownership.sprintId)
        fail('Document ownership must match every linked artifact owner');
    });
  });
  requireComplete(
    index.sprintPlanRevisions,
    events.sprintPlanRevisions.map((item) => item.sprintPlanRevisionId),
    'revision',
    (item) => item.sprintPlanRevisionId,
  );
  requireComplete(
    index.plannerActivities,
    events.sprintPlannerActivities.map((item) => item.sprintPlannerActivityId),
    'planner activity',
    (item) => item.sprintPlannerActivityId,
  );
  requireComplete(
    index.workUnits,
    events.workUnits.map((item) => item.workUnitId),
    'Work Unit',
    (item) => item.workUnitId,
  );
  requireComplete(
    index.gates,
    events.gates.map((item) => item.gateId),
    'gate',
    (item) => item.gateId,
  );
  requireComplete(
    index.agentSessions,
    events.agentSessions.map((item) => item.agentSessionId),
    'Agent Session',
    (item) => item.agentSessionId,
  );
  validateWorkspacePresentation(index.sprintWorkspacePresentation, events, facts, index);
}

function validateWorkspacePresentation(
  metadata: ProductReadReferenceIndexV1['sprintWorkspacePresentation'],
  events: ReturnType<typeof decodeOrchestrationEventsV1>,
  facts: ReadonlyMap<string, string | undefined>,
  index: ProductReadReferenceIndexV1,
) {
  const scopeIds = events.workUnitScopes.map((scope) => scope.workUnitScopeId);
  const documentIds = events.documentReferences.map((document) => document.documentRefId);
  const gateIds = events.gates.map((gate) => gate.gateId);
  if (!metadata) {
    if (scopeIds.length || documentIds.length || gateIds.length)
      fail(
        'workspace presentation metadata is required for scoped Work Units, gates, and Documents',
      );
    return;
  }
  requireComplete(metadata.gates, gateIds, 'gate presentation', (item) => item.gateId);
  requireComplete(
    metadata.documents,
    documentIds,
    'Document presentation',
    (item) => item.documentRefId,
  );
  const scopeById = new Map(events.workUnitScopes.map((scope) => [scope.workUnitScopeId, scope]));
  const revisionById = new Map(
    events.sprintPlanRevisions.map((revision) => [revision.sprintPlanRevisionId, revision]),
  );
  const activityById = new Map(
    events.sprintPlannerActivities.map((activity) => [activity.sprintPlannerActivityId, activity]),
  );
  const planById = new Map(events.sprintPlans.map((plan) => [plan.sprintPlanId, plan]));
  const documentOwnerById = new Map(
    index.documentOwnership.map((owner) => [owner.documentRefId, owner.sprintId]),
  );
  const coveredScopeIds = new Set<string>();
  metadata.plannerActivityMembership.forEach((membership) => {
    validateAvailableSource(membership.source, facts, 'Planner Activity membership');
    const activity = required(activityById, membership.sprintPlannerActivityId, 'Planner Activity');
    const revision = required(revisionById, membership.sprintPlanRevisionId, 'revision');
    if (activity.sprintPlanId !== revision.sprintPlanId)
      fail('Planner Activity membership must use an Activity and revision from the same plan');
    if (!activity.assessedSprintPlanRevisionIds.includes(revision.sprintPlanRevisionId))
      fail('Planner Activity membership revision must be assessed by the Activity');
    if (!planById.has(revision.sprintPlanId))
      fail('Planner Activity membership revision has no plan');
    if (new Set(membership.workUnitScopeIds).size !== membership.workUnitScopeIds.length)
      fail('Planner Activity membership cannot repeat a Work Unit scope');
    membership.workUnitScopeIds.forEach((scopeId) => {
      const scope = required(scopeById, scopeId, 'Work Unit scope');
      if (scope.sprintPlanRevisionId !== revision.sprintPlanRevisionId)
        fail('Planner Activity membership Work Unit scope must belong to its revision');
      if (!events.workUnits.some((unit) => unit.workUnitId === scope.workUnitId))
        fail('Planner Activity membership Work Unit scope has an unknown Work Unit');
      if (coveredScopeIds.has(scopeId))
        fail(
          'workspace presentation must assign each Work Unit scope to exactly one Planner Activity',
        );
      coveredScopeIds.add(scopeId);
    });
  });
  if (scopeIds.some((scopeId) => !coveredScopeIds.has(scopeId)))
    fail(
      'workspace presentation must explicitly place every Work Unit scope in a Planner Activity',
    );
  metadata.gates.forEach((presentation) => {
    if (!events.gates.some((gate) => gate.gateId === presentation.gateId))
      fail('gate presentation references an unknown gate');
    validateAvailableSource(presentation.source, facts, 'gate presentation');
    if (presentation.role.kind === 'other' && !presentation.role.fallbackLabel)
      fail('other gate presentation roles require a fallback label');
  });
  const displayOrders = new Set<string>();
  metadata.documents.forEach((presentation) => {
    const ownerSprintId = required(
      documentOwnerById,
      presentation.documentRefId,
      'Document ownership',
    );
    if (!Number.isSafeInteger(presentation.displayOrder) || presentation.displayOrder < 0)
      fail('Document presentation display order must be a non-negative integer');
    const displayOrderKey = `${ownerSprintId}:${presentation.displayOrder}`;
    if (displayOrders.has(displayOrderKey))
      fail('Document presentation display order must be unique within its Sprint');
    displayOrders.add(displayOrderKey);
    validateSourcedReadValue(presentation.recordedAt, facts, 'Document presentation recorded time');
    validateSourcedReadValue(presentation.displayCategory, facts, 'Document presentation category');
    presentation.sprintPlanRevisionIds.forEach((revisionId) => {
      const revision = required(revisionById, revisionId, 'Document presentation revision');
      if (planById.get(revision.sprintPlanId)?.sprintId !== ownerSprintId)
        fail('Document presentation revision must belong to the Document Sprint');
    });
    presentation.sprintPlannerActivityIds.forEach((activityId) => {
      const activity = required(activityById, activityId, 'Document presentation Planner Activity');
      if (planById.get(activity.sprintPlanId)?.sprintId !== ownerSprintId)
        fail('Document presentation Planner Activity must belong to the Document Sprint');
    });
    presentation.workUnitScopeIds.forEach((scopeId) => {
      const scope = required(scopeById, scopeId, 'Document presentation Work Unit scope');
      const revision = required(
        revisionById,
        scope.sprintPlanRevisionId,
        'Document presentation revision',
      );
      if (planById.get(revision.sprintPlanId)?.sprintId !== ownerSprintId)
        fail('Document presentation Work Unit scope must belong to the Document Sprint');
    });
  });
  const objectiveIds = new Set<string>();
  (metadata.epicPlannerObjectives ?? []).forEach((objective) => {
    if (!objective.objectiveId.trim()) fail('Epic Planner Sprint objective requires an identity');
    if (objectiveIds.has(objective.objectiveId))
      fail('Epic Planner Sprint objectives cannot repeat an objective identity');
    objectiveIds.add(objective.objectiveId);
    if (!objective.title.trim()) fail('Epic Planner Sprint objective requires a title');
    validateAvailableSource(objective.source, facts, 'Epic Planner Sprint objective');
    if (!events.sprints.some(({ sprintId }) => sprintId === objective.sprintId))
      fail('Epic Planner Sprint objective references an unknown Sprint');
  });
  const problemIds = new Set<string>();
  (metadata.problems ?? []).forEach((problem) => {
    if (problemIds.has(problem.problemId))
      fail('workspace problems cannot repeat a problem identity');
    problemIds.add(problem.problemId);
    validateAvailableSource(problem.source, facts, 'workspace problem');
    if (!events.sprints.some(({ sprintId }) => sprintId === problem.sprintId))
      fail('workspace problem references an unknown Sprint');
    if (!problem.graphElementRefs.length)
      fail('workspace problem must link to at least one graph element');
    problem.graphElementRefs.forEach((reference) => {
      const sprintIds =
        reference.kind === 'work_unit'
          ? events.workUnitScopes
              .filter(({ workUnitId }) => workUnitId === reference.id)
              .map(
                ({ sprintPlanRevisionId }) =>
                  planById.get(
                    required(revisionById, sprintPlanRevisionId, 'revision').sprintPlanId,
                  )?.sprintId,
              )
          : reference.kind === 'gate'
            ? events.gates
                .filter(({ gateId }) => gateId === reference.id)
                .map(
                  ({ sprintPlanRevisionId }) =>
                    planById.get(
                      required(revisionById, sprintPlanRevisionId, 'revision').sprintPlanId,
                    )?.sprintId,
                )
            : events.sprintPlannerActivities
                .filter(({ sprintPlannerActivityId }) => sprintPlannerActivityId === reference.id)
                .map(({ sprintPlanId }) => planById.get(sprintPlanId)?.sprintId);
      if (!sprintIds.length) fail('workspace problem references an unknown graph element');
      if (!sprintIds.includes(problem.sprintId))
        fail('workspace problem graph element must belong to the same Sprint');
    });
  });
  const lifecycleIds = new Set<string>();
  const lifecycleSequences = new Set<string>();
  (metadata.workUnitLifecycle ?? []).forEach((entry) => {
    if (lifecycleIds.has(entry.entryId))
      fail('Work Unit lifecycle cannot repeat an entry identity');
    lifecycleIds.add(entry.entryId);
    if (!Number.isSafeInteger(entry.sequence) || entry.sequence < 0)
      fail('Work Unit lifecycle sequence must be a non-negative integer');
    const sequenceKey = `${entry.workUnitId}:${entry.sequence}`;
    if (lifecycleSequences.has(sequenceKey))
      fail('Work Unit lifecycle sequence must be unique within a Work Unit');
    lifecycleSequences.add(sequenceKey);
    if (!events.sprints.some(({ sprintId }) => sprintId === entry.sprintId))
      fail('Work Unit lifecycle references an unknown Sprint');
    if (!events.workUnits.some(({ workUnitId }) => workUnitId === entry.workUnitId))
      fail('Work Unit lifecycle references an unknown Work Unit');
    const workUnitBelongsToSprint = events.workUnitScopes.some((scope) => {
      if (scope.workUnitId !== entry.workUnitId) return false;
      const revision = revisionById.get(scope.sprintPlanRevisionId);
      return revision ? planById.get(revision.sprintPlanId)?.sprintId === entry.sprintId : false;
    });
    if (!workUnitBelongsToSprint) fail('Work Unit lifecycle Work Unit must belong to its Sprint');
    if (!events.agentSessions.some(({ agentSessionId }) => agentSessionId === entry.agentSessionId))
      fail('Work Unit lifecycle references an unknown Agent Session');
    const sessionAssociatedWithWorkUnit = events.agentSessionReferences.some((reference) => {
      if (
        reference.agentSessionId !== entry.agentSessionId ||
        reference.targetKind !== 'work_unit_execution'
      )
        return false;
      const execution = events.workUnitExecutions.find(
        ({ workUnitExecutionId }) => workUnitExecutionId === reference.targetId,
      );
      if (!execution || execution.workUnitId !== entry.workUnitId) return false;
      const scope = scopeById.get(execution.fixedWorkUnitScopeId);
      const revision = scope && revisionById.get(scope.sprintPlanRevisionId);
      return revision ? planById.get(revision.sprintPlanId)?.sprintId === entry.sprintId : false;
    });
    const sessionAssociatedWithOwningPlannerActivity = events.agentSessionReferences.some(
      (reference) => {
        if (
          reference.agentSessionId !== entry.agentSessionId ||
          reference.targetKind !== 'sprint_planner_activity'
        )
          return false;
        return metadata.plannerActivityMembership.some((membership) => {
          if (membership.sprintPlannerActivityId !== reference.targetId) return false;
          return membership.workUnitScopeIds.some((scopeId) => {
            const scope = scopeById.get(scopeId);
            const revision = scope && revisionById.get(scope.sprintPlanRevisionId);
            return (
              scope?.workUnitId === entry.workUnitId &&
              (revision ? planById.get(revision.sprintPlanId)?.sprintId : undefined) ===
                entry.sprintId
            );
          });
        });
      },
    );
    if (!sessionAssociatedWithWorkUnit && !sessionAssociatedWithOwningPlannerActivity)
      fail(
        'Work Unit lifecycle Agent Session must be associated with its Work Unit or owning planner activity and Sprint',
      );
    validateAvailableSource(entry.source, facts, 'Work Unit lifecycle');
  });
  const narratives = metadata.narratives ?? [];
  if (new Set(narratives.map((narrative) => narrative.sprintId)).size !== narratives.length)
    fail('workspace narratives must contain at most one entry per Sprint');
  narratives.forEach((narrative) => {
    if (!events.sprints.some((sprint) => sprint.sprintId === narrative.sprintId))
      fail('workspace narratives reference an unknown Sprint');
    if (narrative.direction)
      validateSourcedReadValue(narrative.direction, facts, 'direction narrative');
    if (narrative.progress)
      validateSourcedReadValue(narrative.progress, facts, 'progress narrative');
    if (narrative.attention)
      validateSourcedReadValue(narrative.attention, facts, 'attention narrative');
  });
}

function eventFacts(events: ReturnType<typeof decodeOrchestrationEventsV1>) {
  const rows: readonly (readonly [string, string | undefined])[] = [
    ...events.executionRequests.map(
      (item) => [item.executionRequestId, item.provenanceId] as const,
    ),
    ...events.observedLaunches.map((item) => [item.observedLaunchId, item.provenanceId] as const),
    ...events.observedReturns.map((item) => [item.observedReturnId, item.provenanceId] as const),
    ...events.reviews.map((item) => [item.reviewId, item.provenanceId] as const),
    ...events.observedIntegrations.map(
      (item) => [item.observedIntegrationId, item.provenanceId] as const,
    ),
    ...events.observedCompletions.map(
      (item) => [item.observedCompletionId, item.provenanceId] as const,
    ),
    ...events.continuationRequests.map(
      (item) => [item.continuationRequestId, item.provenanceId] as const,
    ),
    ...events.observedContinuations.map(
      (item) => [item.observedContinuationId, item.provenanceId] as const,
    ),
    ...events.observedHandoffs.map((item) => [item.observedHandoffId, item.provenanceId] as const),
    ...events.feedbackRecords.map((item) => [item.feedbackRecordId, item.provenanceId] as const),
    ...events.policyEligibilityFacts.map(
      (item) => [item.policyEligibilityFactId, item.provenanceId] as const,
    ),
    ...events.internalArtifacts.map((item) => [item.artifactId, item.provenanceId] as const),
    ...events.documentReferences.map((item) => [item.documentRefId, item.provenanceId] as const),
    ...events.provenance.map((item) => [item.provenanceId, undefined] as const),
  ];
  return new Map(rows);
}
function belongsToEpic(
  events: ReturnType<typeof decodeOrchestrationEventsV1>,
  reference: ProductAgentSessionReferenceReadModelV1,
  epicId: string,
) {
  if (reference.targetKind === 'epic') return reference.targetId === epicId;
  if (reference.targetKind === 'sprint')
    return events.sprints.some(
      (sprint) => sprint.sprintId === reference.targetId && sprint.epicId === epicId,
    );
  if (reference.targetKind === 'sprint_planner_activity') {
    const activity = events.sprintPlannerActivities.find(
      (item) => item.sprintPlannerActivityId === reference.targetId,
    );
    const plan =
      activity && events.sprintPlans.find((item) => item.sprintPlanId === activity.sprintPlanId);
    return plan
      ? events.sprints.some(
          (sprint) => sprint.sprintId === plan.sprintId && sprint.epicId === epicId,
        )
      : false;
  }
  if (reference.targetKind === 'work_unit_execution') {
    const execution = events.workUnitExecutions.find(
      (item) => item.workUnitExecutionId === reference.targetId,
    );
    const scope =
      execution &&
      events.workUnitScopes.find((item) => item.workUnitScopeId === execution.fixedWorkUnitScopeId);
    const revision =
      scope &&
      events.sprintPlanRevisions.find(
        (item) => item.sprintPlanRevisionId === scope.sprintPlanRevisionId,
      );
    const plan =
      revision && events.sprintPlans.find((item) => item.sprintPlanId === revision.sprintPlanId);
    return plan
      ? events.sprints.some(
          (sprint) => sprint.sprintId === plan.sprintId && sprint.epicId === epicId,
        )
      : false;
  }
  return false;
}
function belongsToSprint(
  events: ReturnType<typeof decodeOrchestrationEventsV1>,
  reference: ProductAgentSessionReferenceReadModelV1,
  sprintId: string,
) {
  if (reference.targetKind === 'sprint') return reference.targetId === sprintId;
  if (reference.targetKind === 'sprint_planner_activity') {
    const activity = events.sprintPlannerActivities.find(
      (item) => item.sprintPlannerActivityId === reference.targetId,
    );
    return activity
      ? events.sprintPlans.some(
          (plan) => plan.sprintPlanId === activity.sprintPlanId && plan.sprintId === sprintId,
        )
      : false;
  }
  if (reference.targetKind === 'work_unit_execution') {
    const execution = events.workUnitExecutions.find(
      (item) => item.workUnitExecutionId === reference.targetId,
    );
    const scope =
      execution &&
      events.workUnitScopes.find((item) => item.workUnitScopeId === execution.fixedWorkUnitScopeId);
    const revision =
      scope &&
      events.sprintPlanRevisions.find(
        (item) => item.sprintPlanRevisionId === scope.sprintPlanRevisionId,
      );
    return revision
      ? events.sprintPlans.some(
          (plan) => plan.sprintPlanId === revision.sprintPlanId && plan.sprintId === sprintId,
        )
      : false;
  }
  return false;
}
function indexReferenceData(index: ProductReadReferenceIndexV1) {
  return {
    epics: new Map(index.epics.map((item) => [item.epicId, item])),
    sprints: new Map(index.sprints.map((item) => [item.sprintId, item])),
    revisions: new Map(index.sprintPlanRevisions.map((item) => [item.sprintPlanRevisionId, item])),
    activities: new Map(
      index.plannerActivities.map((item) => [item.sprintPlannerActivityId, item]),
    ),
    workUnits: new Map(index.workUnits.map((item) => [item.workUnitId, item])),
    gates: new Map(index.gates.map((item) => [item.gateId, item])),
    concerns: index.concerns,
    sessions: new Map(index.agentSessions.map((item) => [item.agentSessionId, item])),
    overviews: new Map(index.epicOverviews.map((item) => [item.epicId, item])),
    artifactOwnership: new Map(index.artifactOwnership.map((item) => [item.artifactId, item])),
    documentOwnership: new Map(index.documentOwnership.map((item) => [item.documentRefId, item])),
    workspacePresentation: index.sprintWorkspacePresentation,
  };
}
function orderRevisions<
  T extends {
    readonly sprintPlanRevisionId: string;
    readonly supersedesSprintPlanRevisionId?: string;
  },
>(revisions: readonly T[]): readonly T[] {
  const byPrior = new Map(
    revisions
      .filter((revision) => revision.supersedesSprintPlanRevisionId)
      .map((revision) => [revision.supersedesSprintPlanRevisionId, revision]),
  );
  const root = revisions.find((revision) => !revision.supersedesSprintPlanRevisionId);
  if (!root) fail('revision root is missing');
  const result = [root];
  while (byPrior.has(result.at(-1)?.sprintPlanRevisionId))
    result.push(byPrior.get(result.at(-1)?.sprintPlanRevisionId)!);
  return result;
}
function required<T>(map: ReadonlyMap<string, T>, id: string, label: string): T {
  const value = map.get(id);
  if (!value) fail(`missing ${label} for ${id}`);
  return value;
}
function requireComplete<T>(
  entries: readonly T[],
  ids: readonly string[],
  label: string,
  id: (entry: T) => string,
) {
  if (entries.length !== ids.length || new Set(entries.map(id)).size !== ids.length)
    fail(`reference index must contain one ${label} entry per Event identity`);
}
function validateSourcedReadValue<T>(
  value: {
    readonly source: import('./productReadModels').ReadSourceAuthorityV1;
    readonly value?: T;
  },
  facts: ReadonlyMap<string, string | undefined>,
  label: string,
) {
  if (value.source.status === 'available') {
    if (value.value === undefined) fail(`${label} is available but has no value`);
    if (!value.source.sourceReferences.every((reference) => facts.has(reference)))
      fail(`${label} available source must name known facts or provenance`);
  } else if (value.value !== undefined)
    fail(`${label} cannot invent a value while ${value.source.status}`);
}
function validateAvailableSource(
  source: import('./productReadModels').ReadSourceAuthorityV1,
  facts: ReadonlyMap<string, string | undefined>,
  label: string,
) {
  if (source.status !== 'available') fail(`${label} requires an available source authority`);
  if (!source.sourceReferences.every((reference) => facts.has(reference)))
    fail(`${label} source must name known facts or provenance`);
}
function sameMembers(left: readonly string[], right: readonly string[]) {
  return left.length === right.length && left.every((item) => right.includes(item));
}
function fail(message: string): never {
  throw new Error(`Invalid product read-model composition: ${message}`);
}
