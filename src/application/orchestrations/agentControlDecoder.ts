import {
  projectContinuationEligibility,
  AGENT_CONTROL_CONTRACTS_V1,
  type ContinuationEligibilityEvaluationV1,
  type ContinuationPolicyV1,
  type AgentControlCommandV1,
  type AgentControlContractsV1,
} from './agentControl';

const forbiddenFieldFragments = [
  'provider',
  'thread',
  'token',
  'credential',
  'database',
  'storage',
  'schema',
  'protocol',
];

/** Decodes Agent Control commands and rejects cross-level, collision, and leakage shortcuts. */
export function decodeAgentControlContractsV1(value: unknown): AgentControlContractsV1 {
  rejectForbiddenFields(value);
  const root = record(value, 'Agent Control contracts');
  equal(required(root, 'version'), AGENT_CONTROL_CONTRACTS_V1, 'version');
  const promptProvenance = list(required(root, 'promptProvenance'), 'prompt provenance');
  const policies = list(required(root, 'continuationPolicies'), 'continuation policies');
  const evaluations = list(
    required(root, 'continuationEligibilityEvaluations'),
    'continuation eligibility evaluations',
  );
  const commands = list(required(root, 'commands'), 'commands');
  const results = list(required(root, 'results'), 'results');
  const promptIds = identifiers(promptProvenance, 'promptProvenanceId', 'prompt provenance');
  identifiers(policies, 'continuationPolicyId', 'continuation policy');
  identifiers(
    evaluations,
    'continuationEligibilityEvaluationId',
    'continuation eligibility evaluation',
  );
  const commandIds = identifiers(commands, 'agentControlCommandId', 'command');
  identifiers(results, 'agentControlResultId', 'result');

  for (const item of promptProvenance) validateSource(record(item, 'prompt provenance'));
  const policyById = new Map<string, ContinuationPolicyV1>();
  for (const item of policies) {
    const policy = record(item, 'continuation policy');
    const id = string(required(policy, 'continuationPolicyId'), 'continuation policy id');
    const level = literal(required(policy, 'level'), ['sprint', 'epic'], 'policy level');
    boolean(required(policy, 'autoFlowEnabled'), 'policy auto-flow');
    if (level === 'sprint') string(required(policy, 'sprintId'), 'policy sprint id');
    else string(required(policy, 'epicId'), 'policy Epic id');
    policyById.set(id, policy as ContinuationPolicyV1);
  }
  const evaluationById = new Map<string, ContinuationEligibilityEvaluationV1>();
  for (const item of evaluations) {
    const evaluation = record(item, 'continuation eligibility evaluation');
    const policy = policyById.get(
      string(required(evaluation, 'continuationPolicyId'), 'evaluation policy'),
    );
    if (!policy) fail('dangling continuation eligibility policy');
    const level = literal(required(evaluation, 'level'), ['sprint', 'epic'], 'evaluation level');
    if (level !== policy.level) fail('continuation eligibility level must match its policy');
    validateTarget(record(required(evaluation, 'target'), 'eligibility target'), level);
    ensurePolicyTarget(policy, record(required(evaluation, 'target'), 'eligibility target'));
    boolean(required(evaluation, 'requiredConditionsSatisfied'), 'required conditions');
    boolean(required(evaluation, 'designedForFeedback'), 'designed feedback');
    boolean(required(evaluation, 'allPendingDevelopmentTechnicallyBlocked'), 'all pending blocked');
    timestamp(required(evaluation, 'recordedAt'), 'evaluation time');
    const result = record(required(evaluation, 'result'), 'eligibility result');
    const projected = projectContinuationEligibility(
      policy,
      evaluation as unknown as ContinuationEligibilityEvaluationV1,
    );
    if (
      result.status !== projected.status ||
      result.feedbackBoundary !== projected.feedbackBoundary
    )
      fail('continuation eligibility result must equal its policy projection');
    evaluationById.set(
      string(required(evaluation, 'continuationEligibilityEvaluationId'), 'evaluation id'),
      evaluation as unknown as ContinuationEligibilityEvaluationV1,
    );
  }
  const idempotencySignatures = new Map<string, string>();
  for (const item of commands) {
    const command = record(item, 'command');
    const kind = literal(
      required(command, 'commandKind'),
      [
        'request_next_ready_work_unit_planner',
        'request_next_sprint_planner',
        'request_agent_session_prompt',
      ],
      'command kind',
    );
    const target = record(required(command, 'target'), 'command target');
    validateRequestTarget(kind, target);
    const recipientAgentSessionRefId = string(
      required(command, 'recipientAgentSessionRefId'),
      'recipient Agent Session reference',
    );
    if (
      kind === 'request_agent_session_prompt' &&
      target.agentSessionRefId !== recipientAgentSessionRefId
    )
      fail('Agent Session prompt target must equal its command recipient');
    const idempotency = record(required(command, 'idempotency'), 'idempotency');
    const scopeKind = literal(
      required(idempotency, 'scopeKind'),
      ['sprint', 'epic', 'agent_session'],
      'idempotency scope',
    );
    const scopeId = string(required(idempotency, 'scopeId'), 'idempotency scope id');
    const key = string(required(idempotency, 'key'), 'idempotency key');
    validateIdempotencyScope(kind, target, scopeKind, scopeId);
    validateSource(record(required(command, 'initiatedBy'), 'initiating source'));
    if (!promptIds.has(string(required(command, 'promptProvenanceId'), 'prompt provenance id')))
      fail('dangling prompt provenance');
    timestamp(required(command, 'recordedAt'), 'command time');
    string(required(command, 'preconditionEvidenceReference'), 'precondition evidence');
    validateContinuation(command as unknown as AgentControlCommandV1, policyById, evaluationById);
    const idempotencyId = `${scopeKind}:${scopeId}:${key}`;
    const signature = JSON.stringify({
      kind,
      recipientAgentSessionRefId,
      target,
      promptProvenanceId: command.promptProvenanceId ?? null,
    });
    const existing = idempotencySignatures.get(idempotencyId);
    if (existing !== undefined && existing !== signature)
      fail('idempotency key cannot represent different command, target, or prompt semantics');
    idempotencySignatures.set(idempotencyId, signature);
  }
  for (const item of results) {
    const result = record(item, 'result');
    if (!commandIds.has(string(required(result, 'agentControlCommandId'), 'result command id')))
      fail('dangling result command');
    const state = literal(
      required(result, 'state'),
      [
        'requested',
        'acknowledged',
        'unsupported',
        'denied_ineligible',
        'failed',
        'orchestration_event_recorded',
      ],
      'result state',
    );
    timestamp(required(result, 'recordedAt'), 'result time');
    if (state === 'orchestration_event_recorded')
      string(required(result, 'orchestrationEventReference'), 'Orchestration Event reference');
    else if (result.orchestrationEventReference !== undefined)
      fail('only an event-recorded result may carry an Orchestration Event reference');
  }
  return root as unknown as AgentControlContractsV1;
}

function validateContinuation(
  request: AgentControlCommandV1,
  policies: ReadonlyMap<string, ContinuationPolicyV1>,
  evaluations: ReadonlyMap<string, ContinuationEligibilityEvaluationV1>,
) {
  const isContinuation = request.commandKind !== 'request_agent_session_prompt';
  if (!isContinuation) {
    if (request.continuation !== undefined)
      fail('Agent Session prompt request cannot carry continuation control');
    return;
  }
  if (!request.continuation) fail('continuation request requires policy and eligibility evidence');
  if (request.preconditionEvidenceReference !== request.continuation.eligibilityEvaluationId)
    fail('continuation precondition evidence must equal its eligibility evaluation');
  const policy = policies.get(request.continuation.policyId);
  const evaluation = evaluations.get(request.continuation.eligibilityEvaluationId);
  if (!policy || !evaluation) fail('dangling continuation policy or eligibility evidence');
  if (evaluation.continuationPolicyId !== request.continuation.policyId)
    fail('continuation request evidence must belong to its policy');
  const expectedLevel =
    request.commandKind === 'request_next_ready_work_unit_planner' ? 'sprint' : 'epic';
  if (policy.level !== expectedLevel || evaluation.level !== expectedLevel)
    fail('continuation request cannot use policy or eligibility from another level');
  const requestTarget = request.target as Record<string, unknown>;
  const evaluationTarget = evaluation.target as Record<string, unknown>;
  if (JSON.stringify(requestTarget) !== JSON.stringify(evaluationTarget))
    fail('continuation request target must equal its eligibility target');
  ensurePolicyTarget(policy, requestTarget);
  if (evaluation.result.status !== 'eligible')
    fail('continuation request requires an eligible evaluation, not policy state alone');
}

function validateRequestTarget(kind: string, target: Record<string, unknown>) {
  if (kind === 'request_next_ready_work_unit_planner') validateTarget(target, 'sprint');
  else if (kind === 'request_next_sprint_planner') validateTarget(target, 'epic');
  else {
    if (target.kind !== 'agent_session') fail('Agent Session prompt request target is invalid');
    string(required(target, 'agentSessionRefId'), 'Agent Session reference');
  }
}
function validateIdempotencyScope(
  kind: string,
  target: Record<string, unknown>,
  scopeKind: string,
  scopeId: string,
) {
  const expected =
    kind === 'request_next_ready_work_unit_planner'
      ? { scopeKind: 'sprint', scopeId: target.sprintId }
      : kind === 'request_next_sprint_planner'
        ? { scopeKind: 'epic', scopeId: target.epicId }
        : { scopeKind: 'agent_session', scopeId: target.agentSessionRefId };
  if (scopeKind !== expected.scopeKind) fail('idempotency scope kind must match semantic target');
  if (scopeId !== expected.scopeId) fail('idempotency scope id must equal semantic target');
}
function validateTarget(target: Record<string, unknown>, level: 'sprint' | 'epic') {
  if (level === 'sprint') {
    if (target.kind !== 'next_ready_work_unit_planner')
      fail('Sprint continuation target is invalid');
    string(required(target, 'sprintId'), 'sprint continuation target');
  } else {
    if (target.kind !== 'next_sprint_planner') fail('Epic continuation target is invalid');
    string(required(target, 'epicId'), 'Epic continuation target');
  }
}
function ensurePolicyTarget(policy: ContinuationPolicyV1, target: Record<string, unknown>) {
  if (policy.level === 'sprint') {
    if (target.sprintId !== policy.sprintId)
      fail('continuation target must equal its policy target');
  } else if (target.epicId !== policy.epicId)
    fail('continuation target must equal its policy target');
}
function validateSource(source: Record<string, unknown>) {
  const kind = literal(
    required(source, 'sourceKind'),
    [
      'user_authored',
      'agent_session_derived',
      'application_produced',
      'repository_or_system_derived',
      'other',
    ],
    'source kind',
  );
  string(required(source, 'sourceReference'), 'source reference');
  if (kind === 'other') string(required(source, 'otherSourceType'), 'other source type');
  else if (source.otherSourceType !== undefined)
    fail('known source kind cannot carry an extensible type');
  if ('causalInputReferences' in source)
    for (const causalInput of list(source.causalInputReferences, 'causal inputs'))
      string(causalInput, 'causal input');
}
function rejectForbiddenFields(value: unknown): void {
  if (Array.isArray(value)) return value.forEach(rejectForbiddenFields);
  if (!value || typeof value !== 'object') return;
  for (const [key, nested] of Object.entries(value as Record<string, unknown>)) {
    if (forbiddenFieldFragments.some((fragment) => key.toLowerCase().includes(fragment)))
      fail(`${key} is provider, storage, or process-protocol data rather than a semantic contract`);
    rejectForbiddenFields(nested);
  }
}
function identifiers(values: unknown[], field: string, label: string) {
  const ids = new Set<string>();
  for (const value of values) {
    const id = string(required(record(value, label), field), `${label} id`);
    if (ids.has(id)) fail(`duplicate ${label} id`);
    ids.add(id);
  }
  return ids;
}
function record(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    fail(`${label} must be an object`);
  return value as Record<string, unknown>;
}
function required(value: Record<string, unknown>, key: string): unknown {
  if (!(key in value)) fail(`${key} is required`);
  return value[key];
}
function list(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) fail(`${label} must be an array`);
  return value;
}
function string(value: unknown, label: string): string {
  if (typeof value !== 'string' || !value.trim()) fail(`${label} must be a non-empty string`);
  return value;
}
function literal(value: unknown, allowed: readonly string[], label: string): string {
  if (typeof value !== 'string' || !allowed.includes(value)) fail(`${label} is invalid`);
  return value;
}
function equal(value: unknown, expected: string, label: string) {
  if (value !== expected) fail(`${label} is invalid`);
}
function boolean(value: unknown, label: string) {
  if (typeof value !== 'boolean') fail(`${label} must be a boolean`);
}
function timestamp(value: unknown, label: string) {
  if (typeof value !== 'string' || Number.isNaN(Date.parse(value)))
    fail(`${label} must be a timestamp`);
}
function fail(message: string): never {
  throw new Error(`Invalid Agent Control contracts: ${message}`);
}
