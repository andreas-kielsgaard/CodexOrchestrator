import {
  ORCHESTRATION_EVENTS_V1,
  type AgentSessionAssociationTargetKind,
  type AgentSessionSemanticRole,
  type OrchestrationEventsV1,
} from './orchestrationEvents';

const forbiddenFieldFragments = [
  'label',
  'title',
  'summary',
  'detail',
  'geometry',
  'layout',
  'fixture',
  'selection',
  'transcript',
  'controlbehavior',
  'provider',
  'thread',
  'token',
  'credential',
  'runtimebinding',
  'externalcontext',
  'database',
  'storage',
  'path',
];

/** Decodes unknown input and verifies the durable relationship/cardinality contract. */
export function decodeOrchestrationEventsV1(value: unknown): OrchestrationEventsV1 {
  rejectNonContractFields(value);
  const facts = object(value, 'orchestration events');
  literal(required(facts, 'version'), ORCHESTRATION_EVENTS_V1, 'version');
  const arrays = requiredArrays(facts, [
    'epics',
    'sprints',
    'sprintPlans',
    'sprintPlanRevisions',
    'workUnits',
    'workUnitScopes',
    'sprintPlannerActivities',
    'workUnitExecutions',
    'attempts',
    'agentSessions',
    'agentSessionReferences',
    'gates',
    'gateCriteriaRevisions',
    'feedbackRecords',
    'policyEligibilityFacts',
    'executionRequests',
    'observedLaunches',
    'observedReturns',
    'reviews',
    'observedIntegrations',
    'observedCompletions',
    'continuationRequests',
    'observedContinuations',
    'observedHandoffs',
    'internalArtifacts',
    'documentReferences',
    'provenance',
  ]);
  const allIds = new Set<string>();
  const ids = (key: keyof typeof arrays, field: string) =>
    collectIds(arrays[key], field, key, allIds);
  const epicIds = ids('epics', 'epicId');
  const sprintIds = ids('sprints', 'sprintId');
  const planIds = ids('sprintPlans', 'sprintPlanId');
  const revisionIds = ids('sprintPlanRevisions', 'sprintPlanRevisionId');
  const workUnitIds = ids('workUnits', 'workUnitId');
  const scopeIds = ids('workUnitScopes', 'workUnitScopeId');
  const plannerActivityIds = ids('sprintPlannerActivities', 'sprintPlannerActivityId');
  const executionIds = ids('workUnitExecutions', 'workUnitExecutionId');
  const attemptIds = ids('attempts', 'attemptId');
  const sessionIds = ids('agentSessions', 'agentSessionId');
  const sessionRefIds = ids('agentSessionReferences', 'agentSessionRefId');
  const gateIds = ids('gates', 'gateId');
  ids('gateCriteriaRevisions', 'gateCriteriaRevisionId');
  ids('feedbackRecords', 'feedbackRecordId');
  const policyFactIds = ids('policyEligibilityFacts', 'policyEligibilityFactId');
  const requestIds = ids('executionRequests', 'executionRequestId');
  const launchIds = ids('observedLaunches', 'observedLaunchId');
  ids('observedReturns', 'observedReturnId');
  const documentIds = ids('documentReferences', 'documentRefId');
  const artifactIds = ids('internalArtifacts', 'artifactId');
  ids('observedIntegrations', 'observedIntegrationId');
  ids('observedCompletions', 'observedCompletionId');
  const continuationRequestIds = ids('continuationRequests', 'continuationRequestId');
  ids('observedContinuations', 'observedContinuationId');
  ids('observedHandoffs', 'observedHandoffId');
  const provenanceIds = ids('provenance', 'provenanceId');
  ids('reviews', 'reviewId');

  const sprintById = new Map<string, Record<string, unknown>>();
  for (const item of arrays.sprints) {
    const sprint = object(item, 'sprint');
    const sprintId = identifier(required(sprint, 'sprintId'), 'sprintId');
    reference(required(sprint, 'epicId'), epicIds, 'Sprint Epic owner');
    sprintById.set(sprintId, sprint);
  }
  for (const epicId of epicIds) {
    if (![...sprintById.values()].some((sprint) => sprint.epicId === epicId))
      fail('each epic must own at least one sprint');
  }
  const planById = new Map<string, Record<string, unknown>>();
  for (const item of arrays.sprintPlans) {
    const plan = object(item, 'sprint plan');
    const planId = identifier(required(plan, 'sprintPlanId'), 'sprintPlanId');
    reference(required(plan, 'sprintId'), sprintIds, 'sprint plan sprint');
    planById.set(planId, plan);
  }
  for (const sprintId of sprintIds) {
    if ([...planById.values()].filter((plan) => plan.sprintId === sprintId).length !== 1)
      fail('each sprint must own exactly one sprint plan');
  }
  const revisionById = new Map<string, Record<string, unknown>>();
  const revisionNumbers = new Set<string>();
  for (const item of arrays.sprintPlanRevisions) {
    const revision = object(item, 'sprint plan revision');
    const revisionId = identifier(
      required(revision, 'sprintPlanRevisionId'),
      'sprintPlanRevisionId',
    );
    const planId = identifier(required(revision, 'sprintPlanId'), 'sprint plan revision plan');
    reference(planId, planIds, 'sprint plan revision plan');
    const number = positiveInteger(required(revision, 'revision'), 'sprint plan revision number');
    if (revisionNumbers.has(`${planId}:${number}`)) fail('duplicate sprint plan revision number');
    revisionNumbers.add(`${planId}:${number}`);
    revisionById.set(revisionId, revision);
  }
  for (const planId of planIds) {
    if (![...revisionById.values()].some((revision) => revision.sprintPlanId === planId))
      fail('each sprint plan must have at least one revision');
  }
  for (const [revisionId, revision] of revisionById) {
    if (revision.supersedesSprintPlanRevisionId !== undefined) {
      const priorId = identifier(
        revision.supersedesSprintPlanRevisionId,
        'superseded sprint plan revision',
      );
      reference(priorId, revisionIds, 'superseded sprint plan revision');
      if (revisionById.get(priorId)?.sprintPlanId !== revision.sprintPlanId)
        fail('a revision may supersede only a revision of its own sprint plan');
    }
    const visited = new Set([revisionId]);
    let cursor = revision.supersedesSprintPlanRevisionId;
    while (cursor !== undefined) {
      const prior = revisionById.get(identifier(cursor, 'superseded sprint plan revision'));
      if (!prior || visited.has(identifier(cursor, 'superseded sprint plan revision')))
        fail('invalid sprint plan revision supersession chain');
      visited.add(identifier(cursor, 'superseded sprint plan revision'));
      cursor = prior.supersedesSprintPlanRevisionId;
    }
  }
  validateRevisionChains(revisionById, planIds);
  const scopeById = new Map<string, Record<string, unknown>>();
  const scopedMemberships = new Set<string>();
  for (const item of arrays.workUnitScopes) {
    const scope = object(item, 'work unit scope');
    const scopeId = identifier(required(scope, 'workUnitScopeId'), 'workUnitScopeId');
    reference(required(scope, 'sprintPlanRevisionId'), revisionIds, 'scope sprint plan revision');
    reference(required(scope, 'workUnitId'), workUnitIds, 'scope work unit');
    const membership = `${scope.sprintPlanRevisionId}:${scope.workUnitId}`;
    if (scopedMemberships.has(membership))
      fail('a work unit may have only one scoped definition in an sprint plan revision');
    scopedMemberships.add(membership);
    scopeById.set(scopeId, scope);
  }
  for (const workUnitId of workUnitIds) {
    if (![...scopeById.values()].some((scope) => scope.workUnitId === workUnitId))
      fail('each work unit must have revision-specific scope membership');
  }
  for (const scope of scopeById.values()) {
    for (const dependency of array(
      required(scope, 'dependsOnWorkUnitScopeIds'),
      'scope dependencies',
    )) {
      reference(dependency, scopeIds, 'scope dependency');
      if (
        scopeById.get(identifier(dependency, 'scope dependency'))?.sprintPlanRevisionId !==
        scope.sprintPlanRevisionId
      )
        fail('scope dependencies must belong to the same sprint plan revision');
    }
    for (const gateId of array(required(scope, 'gateIds'), 'scope gates')) {
      reference(gateId, gateIds, 'scope gate');
      const gate = arrays.gates.find((candidate) => object(candidate, 'gate').gateId === gateId);
      if (!gate || object(gate, 'gate').sprintPlanRevisionId !== scope.sprintPlanRevisionId)
        fail('scope gates must belong to the same sprint plan revision');
    }
  }
  for (const item of arrays.gates) {
    const gate = object(item, 'gate');
    reference(required(gate, 'sprintPlanRevisionId'), revisionIds, 'gate sprint plan revision');
  }
  const criteriaNumbers = new Set<string>();
  for (const item of arrays.gateCriteriaRevisions) {
    const criteria = object(item, 'gate criteria revision');
    const gateId = identifier(required(criteria, 'gateId'), 'gate criteria gate');
    reference(gateId, gateIds, 'gate criteria gate');
    const revision = positiveInteger(
      required(criteria, 'revision'),
      'gate criteria revision number',
    );
    if (criteriaNumbers.has(`${gateId}:${revision}`))
      fail('duplicate gate criteria revision number');
    criteriaNumbers.add(`${gateId}:${revision}`);
  }
  for (const item of arrays.sprintPlannerActivities) {
    const activity = object(item, 'sprint planner activity');
    const planId = identifier(required(activity, 'sprintPlanId'), 'planner activity sprint plan');
    reference(planId, planIds, 'planner activity sprint plan');
    for (const revisionId of array(
      required(activity, 'assessedSprintPlanRevisionIds'),
      'planner activity assessments',
    )) {
      reference(revisionId, revisionIds, 'planner activity assessed revision');
      if (
        revisionById.get(identifier(revisionId, 'planner activity assessed revision'))
          ?.sprintPlanId !== planId
      )
        fail('planner activity may assess only revisions of its sprint plan');
    }
  }
  const executionById = new Map<string, Record<string, unknown>>();
  for (const item of arrays.workUnitExecutions) {
    const execution = object(item, 'work unit execution');
    const executionId = identifier(
      required(execution, 'workUnitExecutionId'),
      'workUnitExecutionId',
    );
    const workUnitId = identifier(required(execution, 'workUnitId'), 'execution work unit');
    const scopeId = identifier(
      required(execution, 'fixedWorkUnitScopeId'),
      'execution fixed scope',
    );
    reference(workUnitId, workUnitIds, 'execution work unit');
    reference(scopeId, scopeIds, 'execution fixed scope');
    if (scopeById.get(scopeId)?.workUnitId !== workUnitId)
      fail('execution fixed scope must belong to its work unit');
    executionById.set(executionId, execution);
  }
  const attemptById = new Map<string, Record<string, unknown>>();
  for (const item of arrays.attempts) {
    const attempt = object(item, 'attempt');
    const attemptId = identifier(required(attempt, 'attemptId'), 'attemptId');
    const executionId = identifier(required(attempt, 'workUnitExecutionId'), 'attempt execution');
    const scopeId = identifier(required(attempt, 'fixedWorkUnitScopeId'), 'attempt fixed scope');
    reference(executionId, executionIds, 'attempt execution');
    reference(scopeId, scopeIds, 'attempt fixed scope');
    if (executionById.get(executionId)?.fixedWorkUnitScopeId !== scopeId)
      fail('attempt scope must equal its execution fixed scope');
    attemptById.set(attemptId, attempt);
  }
  for (const item of arrays.agentSessionReferences) {
    const referenceFact = object(item, 'agent session reference');
    reference(
      required(referenceFact, 'agentSessionId'),
      sessionIds,
      'agent session reference session',
    );
    const targetKind = literal(
      required(referenceFact, 'targetKind'),
      ['epic', 'sprint', 'work_slice_planning_point', 'work_unit_execution', 'other'],
      'agent session reference target kind',
    ) as AgentSessionAssociationTargetKind;
    const targetId = required(referenceFact, 'targetId');
    if (targetKind === 'other') {
      identifier(targetId, 'agent session reference target');
      identifier(required(referenceFact, 'otherTargetType'), 'other agent session target type');
    } else {
      const targetIds =
        targetKind === 'epic'
          ? epicIds
          : targetKind === 'sprint'
            ? sprintIds
            : targetKind === 'work_slice_planning_point'
              ? plannerActivityIds
              : executionIds;
      reference(targetId, targetIds, 'agent session reference target');
      if (referenceFact.otherTargetType !== undefined)
        fail('other target type is allowed only for an extensible association target');
    }
    const semanticRole = literal(
      required(referenceFact, 'semanticRole'),
      [
        'epic',
        'sprint',
        'work_slice_planner',
        'work_unit_handler',
        'work_unit_implementer',
      ],
      'agent session reference role',
    ) as AgentSessionSemanticRole;
    if (referenceFact.otherSemanticRole !== undefined)
      fail('agent session references use one of the five product roles');
    const allowedRoles: Partial<
      Record<AgentSessionAssociationTargetKind, readonly AgentSessionSemanticRole[]>
    > = {
      epic: ['epic'],
      sprint: ['sprint'],
      work_slice_planning_point: ['work_slice_planner'],
      work_unit_execution: [
        'work_unit_handler',
        'work_unit_implementer',
      ],
    };
    if (targetKind !== 'other' && !allowedRoles[targetKind]?.includes(semanticRole))
      fail('agent session reference role must match its association target');
  }
  for (const item of arrays.provenance) {
    const provenance = object(item, 'provenance');
    literal(
      required(provenance, 'sourceKind'),
      ['user', 'agent_session', 'application', 'repository', 'system', 'other'],
      'provenance source',
    );
    timestamp(required(provenance, 'recordedAt'), 'provenance recordedAt');
    for (const cause of array(required(provenance, 'causalFactIds'), 'provenance causal facts'))
      reference(cause, allIds, 'provenance causal fact');
    if (provenance.actorAgentSessionRefId !== undefined)
      reference(provenance.actorAgentSessionRefId, sessionRefIds, 'provenance actor session');
  }
  const provenance = (value: unknown, label: string) => reference(value, provenanceIds, label);
  for (const item of arrays.feedbackRecords) {
    const feedback = object(item, 'feedback record');
    reference(required(feedback, 'gateId'), gateIds, 'feedback gate');
    literal(
      required(feedback, 'boundary'),
      ['auto_flow_off', 'designed_feedback_flow', 'all_pending_work_blocked'],
      'feedback boundary',
    );
    provenance(required(feedback, 'provenanceId'), 'feedback provenance');
  }
  for (const item of arrays.policyEligibilityFacts) {
    const policy = object(item, 'policy eligibility fact');
    const level = literal(required(policy, 'level'), ['sprint', 'epic'], 'policy level');
    reference(
      required(policy, 'targetId'),
      level === 'sprint' ? sprintIds : epicIds,
      'policy target',
    );
    boolean(required(policy, 'autoFlowEnabled'), 'policy autoFlowEnabled');
    boolean(required(policy, 'eligible'), 'policy eligible');
    provenance(required(policy, 'provenanceId'), 'policy provenance');
  }
  const requestById = new Map<string, Record<string, unknown>>();
  for (const item of arrays.executionRequests) {
    const request = object(item, 'execution request');
    const requestId = identifier(required(request, 'executionRequestId'), 'executionRequestId');
    reference(
      required(request, 'workUnitExecutionId'),
      executionIds,
      'execution request execution',
    );
    provenance(required(request, 'provenanceId'), 'execution request provenance');
    requestById.set(requestId, request);
  }
  const launchById = new Map<string, Record<string, unknown>>();
  for (const item of arrays.observedLaunches) {
    const launch = object(item, 'observed launch');
    const launchId = identifier(required(launch, 'observedLaunchId'), 'observedLaunchId');
    const requestId = identifier(required(launch, 'executionRequestId'), 'launch request');
    const executionId = identifier(required(launch, 'workUnitExecutionId'), 'launch execution');
    const attemptId = identifier(required(launch, 'attemptId'), 'launch attempt');
    reference(requestId, requestIds, 'launch request');
    reference(executionId, executionIds, 'launch execution');
    reference(attemptId, attemptIds, 'launch attempt');
    if (requestById.get(requestId)?.workUnitExecutionId !== executionId)
      fail('observed launch must use its request execution');
    if (attemptById.get(attemptId)?.workUnitExecutionId !== executionId)
      fail('observed launch attempt must belong to its execution');
    provenance(required(launch, 'provenanceId'), 'observed launch provenance');
    launchById.set(launchId, launch);
  }
  for (const item of arrays.observedReturns) {
    const returned = object(item, 'observed return');
    const launchId = identifier(required(returned, 'observedLaunchId'), 'return launch');
    const attemptId = identifier(required(returned, 'attemptId'), 'return attempt');
    reference(launchId, launchIds, 'return launch');
    reference(attemptId, attemptIds, 'return attempt');
    if (launchById.get(launchId)?.attemptId !== attemptId)
      fail('observed return must belong to its observed launch attempt');
    provenance(required(returned, 'provenanceId'), 'observed return provenance');
  }
  const requestedExecutionIds = new Set(
    arrays.executionRequests.map(
      (item) => object(item, 'execution request').workUnitExecutionId as string,
    ),
  );
  const observedExecutionIds = new Set(
    arrays.observedLaunches.map(
      (item) => object(item, 'observed launch').workUnitExecutionId as string,
    ),
  );
  for (const execution of executionById.values()) {
    const scopeId = execution.fixedWorkUnitScopeId as string;
    const revisionId = scopeById.get(scopeId)?.sprintPlanRevisionId as string;
    const isSuperseded = [...revisionById.values()].some(
      (revision) => revision.supersedesSprintPlanRevisionId === revisionId,
    );
    const executionId = execution.workUnitExecutionId as string;
    if (
      isSuperseded &&
      !requestedExecutionIds.has(executionId) &&
      !observedExecutionIds.has(executionId)
    )
      fail('a superseded scope may retain only explicit request or observed execution history');
  }
  for (const item of arrays.reviews) {
    const review = object(item, 'review');
    const kind = literal(
      required(review, 'subjectKind'),
      ['work_unit_execution', 'attempt', 'sprint_plan_revision', 'document_reference'],
      'review subject kind',
    );
    const available =
      kind === 'work_unit_execution'
        ? executionIds
        : kind === 'attempt'
          ? attemptIds
          : kind === 'sprint_plan_revision'
            ? revisionIds
            : documentIds;
    reference(required(review, 'subjectId'), available, 'review subject');
    if (review.outcome !== undefined)
      literal(review.outcome, ['accepted', 'needs_correction', 'blocked'], 'review outcome');
    if (review.rationaleArtifactId !== undefined)
      reference(review.rationaleArtifactId, artifactIds, 'review rationale artifact');
    provenance(required(review, 'provenanceId'), 'review provenance');
  }
  for (const item of arrays.observedIntegrations) {
    const integration = object(item, 'observed integration');
    reference(required(integration, 'workUnitExecutionId'), executionIds, 'integration execution');
    provenance(required(integration, 'provenanceId'), 'integration provenance');
  }
  for (const item of arrays.observedCompletions) {
    const completion = object(item, 'observed completion');
    const kind = literal(
      required(completion, 'subjectKind'),
      ['work_unit_execution', 'sprint'],
      'completion subject kind',
    );
    reference(
      required(completion, 'subjectId'),
      kind === 'sprint' ? sprintIds : executionIds,
      'completion subject',
    );
    boolean(required(completion, 'responsibilityAccepted'), 'completion responsibilityAccepted');
    provenance(required(completion, 'provenanceId'), 'completion provenance');
  }
  for (const item of arrays.continuationRequests) {
    const request = object(item, 'continuation request');
    identifier(required(request, 'continuationRequestId'), 'continuationRequestId');
    const policyId = identifier(
      required(request, 'policyEligibilityFactId'),
      'continuation policy',
    );
    const kind = literal(
      required(request, 'targetKind'),
      ['next_work_unit', 'next_sprint_planner'],
      'continuation target kind',
    );
    reference(policyId, policyFactIds, 'continuation policy');
    const policy = arrays.policyEligibilityFacts.find(
      (candidate) =>
        object(candidate, 'policy eligibility fact').policyEligibilityFactId === policyId,
    );
    if (
      !policy ||
      object(policy, 'policy eligibility fact').level !==
        (kind === 'next_work_unit' ? 'sprint' : 'epic')
    )
      fail('continuation target kind must match its policy level');
    const targetId = identifier(required(request, 'targetId'), 'continuation target');
    reference(targetId, kind === 'next_work_unit' ? sprintIds : epicIds, 'continuation target');
    if (object(policy, 'policy eligibility fact').targetId !== targetId)
      fail('continuation request target must equal its policy target');
    provenance(required(request, 'provenanceId'), 'continuation request provenance');
  }
  for (const item of arrays.observedContinuations) {
    const continuation = object(item, 'observed continuation');
    reference(
      required(continuation, 'continuationRequestId'),
      continuationRequestIds,
      'observed continuation request',
    );
    provenance(required(continuation, 'provenanceId'), 'observed continuation provenance');
  }
  for (const item of arrays.observedHandoffs) {
    const handoff = object(item, 'observed handoff');
    reference(required(handoff, 'sprintId'), sprintIds, 'observed handoff sprint');
    provenance(required(handoff, 'provenanceId'), 'observed handoff provenance');
  }
  for (const item of arrays.internalArtifacts) {
    const artifact = object(item, 'internal artifact');
    provenance(required(artifact, 'provenanceId'), 'artifact provenance');
  }
  for (const item of arrays.documentReferences) {
    const document = object(item, 'document reference');
    for (const artifactId of array(required(document, 'artifactIds'), 'document artifacts'))
      reference(artifactId, artifactIds, 'document artifact');
    provenance(required(document, 'provenanceId'), 'document provenance');
  }
  return value as OrchestrationEventsV1;
}

function validateRevisionChains(
  revisions: ReadonlyMap<string, Record<string, unknown>>,
  planIds: ReadonlySet<string>,
) {
  for (const planId of planIds) {
    const planRevisions = [...revisions.entries()].filter(
      ([, revision]) => revision.sprintPlanId === planId,
    );
    const successors = new Map<string, string>();
    const roots = planRevisions.filter(
      ([, revision]) => revision.supersedesSprintPlanRevisionId === undefined,
    );
    if (roots.length !== 1) fail('each sprint plan must have exactly one revision root');
    for (const [revisionId, revision] of planRevisions) {
      const priorId = revision.supersedesSprintPlanRevisionId;
      if (priorId === undefined) continue;
      const prior = revisions.get(identifier(priorId, 'superseded sprint plan revision'));
      if (prior?.sprintPlanId === planId && successors.has(priorId as string))
        fail('each sprint plan revision may have at most one direct successor');
      if (prior?.sprintPlanId === planId) successors.set(priorId as string, revisionId);
      if (
        prior?.sprintPlanId === planId &&
        (revision.revision as number) <= (prior.revision as number)
      )
        fail('sprint plan revision numbers must increase along supersession');
    }
    const reachable = new Set<string>();
    let cursor: string | undefined = roots[0]?.[0];
    while (cursor !== undefined) {
      if (reachable.has(cursor)) fail('invalid sprint plan revision supersession chain');
      reachable.add(cursor);
      cursor = successors.get(cursor) as string | undefined;
    }
    if (reachable.size !== planRevisions.length)
      fail('every sprint plan revision must be reachable from its root');
  }
}

function requiredArrays(value: Record<string, unknown>, keys: readonly string[]) {
  return Object.fromEntries(keys.map((key) => [key, array(required(value, key), key)])) as Record<
    string,
    unknown[]
  >;
}
function collectIds(
  values: readonly unknown[],
  field: string,
  label: string,
  allIds: Set<string>,
): Set<string> {
  const result = new Set<string>();
  for (const value of values) {
    const id = identifier(required(object(value, label), field), `${label} ${field}`);
    if (result.has(id) || allIds.has(id)) fail(`identity ${id} must not be shared by convenience`);
    result.add(id);
    allIds.add(id);
  }
  return result;
}
function rejectNonContractFields(value: unknown): void {
  if (Array.isArray(value)) return value.forEach(rejectNonContractFields);
  if (!value || typeof value !== 'object') return;
  for (const [key, child] of Object.entries(value as Record<string, unknown>)) {
    if (forbiddenFieldFragments.some((fragment) => key.toLowerCase().includes(fragment)))
      fail(
        `${key} is presentation, provider, or persistence data rather than an Orchestration Event`,
      );
    rejectNonContractFields(child);
  }
}
function object(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    fail(`${label} must be an object`);
  return value as Record<string, unknown>;
}
function required(value: Record<string, unknown>, key: string): unknown {
  if (!(key in value)) fail(`${key} is required`);
  return value[key];
}
function array(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  return value;
}
function identifier(value: unknown, label: string): string {
  if (typeof value !== 'string' || !value.trim()) fail(`${label} must be a non-empty string`);
  return value;
}
function reference(value: unknown, ids: ReadonlySet<string>, label: string): void {
  const id = identifier(value, label);
  if (!ids.has(id)) fail(`dangling ${label} reference`);
}
function literal(value: unknown, allowed: readonly string[] | string, label: string): string {
  const values = typeof allowed === 'string' ? [allowed] : allowed;
  if (typeof value !== 'string' || !values.includes(value)) fail(`${label} is invalid`);
  return value;
}
function positiveInteger(value: unknown, label: string): number {
  if (typeof value !== 'number' || !Number.isInteger(value) || value < 1)
    fail(`${label} must be a positive integer`);
  return value;
}
function boolean(value: unknown, label: string): void {
  if (typeof value !== 'boolean') fail(`${label} must be a boolean`);
}
function timestamp(value: unknown, label: string): void {
  if (typeof value !== 'string' || Number.isNaN(Date.parse(value)))
    fail(`${label} must be a timestamp`);
}
function fail(message: string): never {
  throw new Error(`Invalid orchestration events: ${message}`);
}
