export const SPRINT_RUNNER_TRANSITION_CONTRACT = 'sprint-runner-transition-query/v1';

export interface SprintRunnerTransitionV1 {
  readonly sprintId: string;
  readonly epicId: string;
  readonly requestId: string;
  readonly epicRunnerInvocationId: string;
  readonly sprintRunnerSessionId: string;
  readonly sprintRunnerInvocationId: string;
  readonly requestedAt: string;
  readonly authorizedAt: string;
  readonly sessionCreatedAt?: string;
  readonly harnessAppliedAt?: string;
  readonly launchAcceptedAt?: string;
  readonly preStartReady: boolean;
  readonly lifecycleObserved: boolean;
  readonly accepted: boolean;
  readonly preStartSemanticOutcomeRecordedAt?: string;
  readonly preStartLifecycleObservedAt?: string;
  readonly preStartOutcomeAcceptedAt?: string;
  readonly parentContinuationDeliveryRequestedAt?: string;
  readonly parentContinuationDeliveryPersistedAt?: string;
  readonly epicContinuationInvocationId?: string;
  readonly epicContinuationLaunchAcceptedAt?: string;
  readonly providerReceiverActivationObservedAt?: string;
  readonly sprintStartAuthorizedAt?: string;
  readonly sprintStartPersistedAt?: string;
  readonly sprintContinuationInvocationId?: string;
  readonly sprintContinuationLaunchAcceptedAt?: string;
  readonly repositoryBranchReevaluationRecordedAt?: string;
  readonly startedReevaluationLifecycleObservedAt?: string;
  readonly planningControlDeliveryRequestedAt?: string;
  readonly planningControlDeliveryPersistedAt?: string;
  readonly planningControlInvocationId?: string;
  readonly planningControlLaunchAcceptedAt?: string;
  readonly planningReadyAt?: string;
  readonly workSlicePlannerRequestId?: string;
  readonly workSlicePlanningPointId?: string;
  readonly workSlicePlannerRepositoryWorktreeRoute?: string;
  readonly workSlicePlannerHarnessKey?: string;
  readonly workSlicePlannerHarnessVersion?: number;
  readonly workSlicePlannerSessionId?: string;
  readonly workSlicePlannerInvocationId?: string;
  readonly workSlicePlannerSessionCreatedAt?: string;
  readonly workSlicePlannerInvocationCreatedAt?: string;
  readonly workSlicePlannerHarnessAppliedAt?: string;
  readonly workSlicePlannerLaunchRequestedAt?: string;
  readonly workSlicePlannerLaunchAcceptedAt?: string;
  readonly workSlicePlannerReadyAt?: string;
  readonly workSlicePlannerProviderActivationObservedAt?: string;
  readonly workSlicePlannerLifecycleObservedAt?: string;
  readonly downstreamNotStarted?: boolean;
}

export interface SprintRunnerTransitionQueryV1 {
  readonly contract: typeof SPRINT_RUNNER_TRANSITION_CONTRACT;
  readonly transitions: readonly SprintRunnerTransitionV1[];
}

export interface ProductSprintRunnerTransitionStatusV1 {
  readonly label: string;
  readonly requestedAt: string;
  readonly authorizedAt: string;
  readonly sessionCreatedAt?: string;
  readonly harnessAppliedAt?: string;
  readonly launchAcceptedAt?: string;
  readonly preStartReady: boolean;
  readonly lifecycleObserved: boolean;
  readonly accepted: boolean;
  readonly preStartSemanticOutcomeRecordedAt?: string;
  readonly preStartLifecycleObservedAt?: string;
  readonly preStartOutcomeAcceptedAt?: string;
  readonly parentContinuationDeliveryRequestedAt?: string;
  readonly parentContinuationDeliveryPersistedAt?: string;
  readonly epicContinuationLaunchAcceptedAt?: string;
  readonly providerReceiverActivationObservedAt?: string;
  readonly sprintStartAuthorizedAt?: string;
  readonly sprintStartPersistedAt?: string;
  readonly sprintContinuationLaunchAcceptedAt?: string;
  readonly repositoryBranchReevaluationRecordedAt?: string;
  readonly startedReevaluationLifecycleObservedAt?: string;
  readonly planningControlDeliveryRequestedAt?: string;
  readonly planningControlDeliveryPersistedAt?: string;
  readonly planningControlInvocationId?: string;
  readonly planningControlLaunchAcceptedAt?: string;
  readonly planningReadyAt?: string;
  readonly workSlicePlannerRequestId?: string;
  readonly workSlicePlanningPointId?: string;
  readonly workSlicePlannerRepositoryWorktreeRoute?: string;
  readonly workSlicePlannerHarnessKey?: string;
  readonly workSlicePlannerHarnessVersion?: number;
  readonly workSlicePlannerSessionId?: string;
  readonly workSlicePlannerInvocationId?: string;
  readonly workSlicePlannerSessionCreatedAt?: string;
  readonly workSlicePlannerInvocationCreatedAt?: string;
  readonly workSlicePlannerHarnessAppliedAt?: string;
  readonly workSlicePlannerLaunchRequestedAt?: string;
  readonly workSlicePlannerLaunchAcceptedAt?: string;
  readonly workSlicePlannerReadyAt?: string;
  readonly workSlicePlannerProviderActivationObservedAt?: string;
  readonly workSlicePlannerLifecycleObservedAt?: string;
  readonly downstreamNotStarted?: boolean;
}

export function projectSprintRunnerTransitionStatus(
  transition: SprintRunnerTransitionV1,
): ProductSprintRunnerTransitionStatusV1 {
  return {
    label: transition.workSlicePlannerReadyAt
      ? plannerReadyLabel(transition)
      : transition.workSlicePlannerLaunchAcceptedAt
        ? 'Work Slice Planner runtime launch accepted; readiness pending'
        : transition.workSlicePlannerLaunchRequestedAt
          ? 'Work Slice Planner launch requested; runtime acceptance pending'
          : transition.workSlicePlannerHarnessAppliedAt
            ? 'Work Slice Planner Harness applied; launch request pending'
            : transition.workSlicePlannerInvocationCreatedAt
              ? 'Work Slice Planner invocation prepared; Harness application pending'
              : transition.workSlicePlannerSessionCreatedAt
                ? 'Work Slice Planner Session created; invocation pending'
                : transition.workSlicePlanningPointId
                  ? 'Work Slice Planner planning point authorized; Session pending'
                  : transition.workSlicePlannerRequestId
                    ? 'Work Slice Planner request authorized; planning point pending'
      : transition.planningControlLaunchAcceptedAt
        ? 'Sprint planning control launch accepted; Planner request available'
      : transition.startedReevaluationLifecycleObservedAt
        ? 'Started reevaluation completed; planning control delivery pending'
      : transition.planningReadyAt
      ? 'Sprint planning control ready; downstream not started'
      : transition.sprintStartPersistedAt
        ? 'Sprint start authorized; repository reevaluation pending'
        : transition.preStartSemanticOutcomeRecordedAt && !transition.preStartOutcomeAcceptedAt
          ? 'Pre-start outcome recorded; matching lifecycle pending'
        : transition.preStartOutcomeAcceptedAt && !transition.parentContinuationDeliveryPersistedAt
          ? 'Pre-start outcome accepted; Epic continuation delivery pending'
          : transition.parentContinuationDeliveryPersistedAt && !transition.epicContinuationLaunchAcceptedAt
            ? 'Epic continuation invocation persisted; launch acceptance pending'
        : transition.epicContinuationLaunchAcceptedAt
          ? 'Waiting for Epic Runner start authorization'
          : transition.launchAcceptedAt
      ? 'Sprint Runner launch accepted — pre-start ready'
      : transition.harnessAppliedAt
        ? 'Sprint Runner launch acceptance pending'
        : transition.sessionCreatedAt
          ? 'Sprint Runner session created; applying Harness'
          : 'Sprint Runner request authorized',
    requestedAt: transition.requestedAt,
    authorizedAt: transition.authorizedAt,
    ...(transition.sessionCreatedAt ? { sessionCreatedAt: transition.sessionCreatedAt } : {}),
    ...(transition.harnessAppliedAt ? { harnessAppliedAt: transition.harnessAppliedAt } : {}),
    ...(transition.launchAcceptedAt ? { launchAcceptedAt: transition.launchAcceptedAt } : {}),
    preStartReady: transition.preStartReady,
    lifecycleObserved: transition.lifecycleObserved,
    accepted: transition.accepted,
    ...optionalProjection(transition, 'preStartSemanticOutcomeRecordedAt'),
    ...optionalProjection(transition, 'preStartLifecycleObservedAt'),
    ...optionalProjection(transition, 'preStartOutcomeAcceptedAt'),
    ...optionalProjection(transition, 'parentContinuationDeliveryRequestedAt'),
    ...optionalProjection(transition, 'parentContinuationDeliveryPersistedAt'),
    ...optionalProjection(transition, 'epicContinuationLaunchAcceptedAt'),
    ...optionalProjection(transition, 'providerReceiverActivationObservedAt'),
    ...optionalProjection(transition, 'sprintStartAuthorizedAt'),
    ...optionalProjection(transition, 'sprintStartPersistedAt'),
    ...optionalProjection(transition, 'sprintContinuationLaunchAcceptedAt'),
    ...optionalProjection(transition, 'repositoryBranchReevaluationRecordedAt'),
    ...optionalProjection(transition, 'startedReevaluationLifecycleObservedAt'),
    ...optionalProjection(transition, 'planningControlDeliveryRequestedAt'),
    ...optionalProjection(transition, 'planningControlDeliveryPersistedAt'),
    ...optionalProjection(transition, 'planningControlInvocationId'),
    ...optionalProjection(transition, 'planningControlLaunchAcceptedAt'),
    ...optionalProjection(transition, 'planningReadyAt'),
    ...optionalProjection(transition, 'workSlicePlannerRequestId'),
    ...optionalProjection(transition, 'workSlicePlanningPointId'),
    ...optionalProjection(transition, 'workSlicePlannerRepositoryWorktreeRoute'),
    ...optionalProjection(transition, 'workSlicePlannerHarnessKey'),
    ...optionalProjection(transition, 'workSlicePlannerHarnessVersion'),
    ...optionalProjection(transition, 'workSlicePlannerSessionId'),
    ...optionalProjection(transition, 'workSlicePlannerInvocationId'),
    ...optionalProjection(transition, 'workSlicePlannerSessionCreatedAt'),
    ...optionalProjection(transition, 'workSlicePlannerInvocationCreatedAt'),
    ...optionalProjection(transition, 'workSlicePlannerHarnessAppliedAt'),
    ...optionalProjection(transition, 'workSlicePlannerLaunchRequestedAt'),
    ...optionalProjection(transition, 'workSlicePlannerLaunchAcceptedAt'),
    ...optionalProjection(transition, 'workSlicePlannerReadyAt'),
    ...optionalProjection(transition, 'workSlicePlannerProviderActivationObservedAt'),
    ...optionalProjection(transition, 'workSlicePlannerLifecycleObservedAt'),
    downstreamNotStarted: transition.downstreamNotStarted,
  };
}

function plannerReadyLabel(transition: SprintRunnerTransitionV1): string {
  const provider = Boolean(transition.workSlicePlannerProviderActivationObservedAt);
  const lifecycle = Boolean(transition.workSlicePlannerLifecycleObservedAt);
  if (provider && lifecycle) return 'Work Slice Planner ready; provider and lifecycle observations recorded';
  if (provider) return 'Work Slice Planner ready; provider observed; lifecycle observation pending';
  if (lifecycle) return 'Work Slice Planner ready; lifecycle observed; provider activation pending';
  return 'Work Slice Planner ready; provider and lifecycle observation pending';
}
function optionalProjection<T extends object, K extends keyof T>(value: T, key: K) {
  return value[key] == null ? {} : { [key]: value[key] };
}

export function decodeSprintRunnerTransitionQueryV1(value: unknown): SprintRunnerTransitionQueryV1 {
  const root = object(value, 'Sprint Runner transition query');
  exact(root, ['contract', 'transitions'], 'Sprint Runner transition query');
  if (root.contract !== SPRINT_RUNNER_TRANSITION_CONTRACT || !Array.isArray(root.transitions))
    invalid('unsupported Sprint Runner transition query contract');
  const transitions = root.transitions.map(decodeTransition);
  unique(
    transitions.map((transition) => transition.sprintId),
    'Sprint Runner transition Sprint',
  );
  return { contract: SPRINT_RUNNER_TRANSITION_CONTRACT, transitions };
}

function decodeTransition(value: unknown): SprintRunnerTransitionV1 {
  const item = object(value, 'Sprint Runner transition');
  exact(
    item,
    [
      'sprintId',
      'epicId',
      'requestId',
      'epicRunnerInvocationId',
      'sprintRunnerSessionId',
      'sprintRunnerInvocationId',
      'requestedAt',
      'authorizedAt',
      'sessionCreatedAt',
      'harnessAppliedAt',
      'launchAcceptedAt',
      'preStartReady',
      'lifecycleObserved',
      'accepted',
      'preStartSemanticOutcomeRecordedAt', 'preStartLifecycleObservedAt',
      'preStartOutcomeAcceptedAt', 'parentContinuationDeliveryRequestedAt',
      'parentContinuationDeliveryPersistedAt',
      'epicContinuationInvocationId', 'epicContinuationLaunchAcceptedAt',
      'providerReceiverActivationObservedAt', 'sprintStartAuthorizedAt',
      'sprintStartPersistedAt', 'sprintContinuationInvocationId',
       'sprintContinuationLaunchAcceptedAt', 'repositoryBranchReevaluationRecordedAt',
       'startedReevaluationLifecycleObservedAt', 'planningControlDeliveryRequestedAt',
       'planningControlDeliveryPersistedAt', 'planningControlInvocationId',
       'planningControlLaunchAcceptedAt',
       'planningReadyAt',
       'workSlicePlannerRequestId', 'workSlicePlanningPointId',
       'workSlicePlannerRepositoryWorktreeRoute', 'workSlicePlannerHarnessKey',
       'workSlicePlannerHarnessVersion', 'workSlicePlannerSessionId',
       'workSlicePlannerInvocationId', 'workSlicePlannerSessionCreatedAt',
       'workSlicePlannerInvocationCreatedAt', 'workSlicePlannerHarnessAppliedAt',
       'workSlicePlannerLaunchRequestedAt', 'workSlicePlannerLaunchAcceptedAt',
       'workSlicePlannerReadyAt', 'workSlicePlannerProviderActivationObservedAt',
       'workSlicePlannerLifecycleObservedAt',
      'downstreamNotStarted',
    ],
    'Sprint Runner transition',
  );
  const transition = {
    sprintId: text(item.sprintId, 'sprintId'),
    epicId: text(item.epicId, 'epicId'),
    requestId: text(item.requestId, 'requestId'),
    epicRunnerInvocationId: text(item.epicRunnerInvocationId, 'epicRunnerInvocationId'),
    sprintRunnerSessionId: text(item.sprintRunnerSessionId, 'sprintRunnerSessionId'),
    sprintRunnerInvocationId: text(item.sprintRunnerInvocationId, 'sprintRunnerInvocationId'),
    requestedAt: text(item.requestedAt, 'requestedAt'),
    authorizedAt: text(item.authorizedAt, 'authorizedAt'),
    ...optionalText(item, 'sessionCreatedAt'),
    ...optionalText(item, 'harnessAppliedAt'),
    ...optionalText(item, 'launchAcceptedAt'),
    preStartReady: bool(item.preStartReady, 'preStartReady'),
    lifecycleObserved: bool(item.lifecycleObserved, 'lifecycleObserved'),
    accepted: bool(item.accepted, 'accepted'),
    ...optionalText(item, 'preStartSemanticOutcomeRecordedAt'),
    ...optionalText(item, 'preStartLifecycleObservedAt'),
    ...optionalText(item, 'preStartOutcomeAcceptedAt'),
    ...optionalText(item, 'parentContinuationDeliveryRequestedAt'),
    ...optionalText(item, 'parentContinuationDeliveryPersistedAt'),
    ...optionalText(item, 'epicContinuationInvocationId'),
    ...optionalText(item, 'epicContinuationLaunchAcceptedAt'),
    ...optionalText(item, 'providerReceiverActivationObservedAt'),
    ...optionalText(item, 'sprintStartAuthorizedAt'),
    ...optionalText(item, 'sprintStartPersistedAt'),
    ...optionalText(item, 'sprintContinuationInvocationId'),
    ...optionalText(item, 'sprintContinuationLaunchAcceptedAt'),
    ...optionalText(item, 'repositoryBranchReevaluationRecordedAt'),
    ...optionalText(item, 'startedReevaluationLifecycleObservedAt'),
    ...optionalText(item, 'planningControlDeliveryRequestedAt'),
    ...optionalText(item, 'planningControlDeliveryPersistedAt'),
    ...optionalText(item, 'planningControlInvocationId'),
    ...optionalText(item, 'planningControlLaunchAcceptedAt'),
    ...optionalText(item, 'planningReadyAt'),
    ...optionalText(item, 'workSlicePlannerRequestId'),
    ...optionalText(item, 'workSlicePlanningPointId'),
    ...optionalText(item, 'workSlicePlannerRepositoryWorktreeRoute'),
    ...optionalText(item, 'workSlicePlannerHarnessKey'),
    ...optionalNumber(item, 'workSlicePlannerHarnessVersion'),
    ...optionalText(item, 'workSlicePlannerSessionId'),
    ...optionalText(item, 'workSlicePlannerInvocationId'),
    ...optionalText(item, 'workSlicePlannerSessionCreatedAt'),
    ...optionalText(item, 'workSlicePlannerInvocationCreatedAt'),
    ...optionalText(item, 'workSlicePlannerHarnessAppliedAt'),
    ...optionalText(item, 'workSlicePlannerLaunchRequestedAt'),
    ...optionalText(item, 'workSlicePlannerLaunchAcceptedAt'),
    ...optionalText(item, 'workSlicePlannerReadyAt'),
    ...optionalText(item, 'workSlicePlannerProviderActivationObservedAt'),
    ...optionalText(item, 'workSlicePlannerLifecycleObservedAt'),
    downstreamNotStarted: bool(item.downstreamNotStarted, 'downstreamNotStarted'),
  } as SprintRunnerTransitionV1;
  if (transition.workSlicePlanningPointId && !transition.workSlicePlannerRequestId)
    invalid('Work Slice Planner planning point requires its request');
  if (transition.workSlicePlannerSessionId && !transition.workSlicePlanningPointId)
    invalid('Work Slice Planner Session requires its planning point');
  if (transition.workSlicePlannerInvocationId && !transition.workSlicePlannerSessionId)
    invalid('Work Slice Planner invocation requires its Session');
  if (transition.workSlicePlannerSessionCreatedAt && !transition.workSlicePlannerSessionId)
    invalid('Work Slice Planner Session creation requires its Session');
  if (transition.workSlicePlannerInvocationCreatedAt && !transition.workSlicePlannerInvocationId)
    invalid('Work Slice Planner invocation creation requires its invocation');
  if (transition.workSlicePlannerHarnessAppliedAt && !transition.workSlicePlannerInvocationId)
    invalid('Work Slice Planner Harness application requires its invocation');
  if (transition.workSlicePlannerLaunchRequestedAt && !transition.workSlicePlannerHarnessAppliedAt)
    invalid('Work Slice Planner launch request requires applied Harness');
  if (transition.workSlicePlannerLaunchAcceptedAt && !transition.workSlicePlannerLaunchRequestedAt)
    invalid('Work Slice Planner launch acceptance requires requested launch');
  if (transition.workSlicePlannerReadyAt && !transition.workSlicePlannerLaunchAcceptedAt)
    invalid('Work Slice Planner readiness requires accepted launch');
  if (
    (transition.workSlicePlannerProviderActivationObservedAt ||
      transition.workSlicePlannerLifecycleObservedAt) &&
    !transition.workSlicePlannerReadyAt
  )
    invalid('Work Slice Planner observation requires readiness');
  if (
    transition.preStartReady !== Boolean(transition.launchAcceptedAt)
  )
    invalid('Sprint Runner transition evidence is inconsistent');
  if (transition.accepted && (!transition.preStartSemanticOutcomeRecordedAt || !transition.preStartLifecycleObservedAt))
    invalid('accepted pre-start outcome requires both semantic and lifecycle evidence');
  if (transition.planningReadyAt && (!transition.sprintStartPersistedAt || !transition.repositoryBranchReevaluationRecordedAt || !transition.startedReevaluationLifecycleObservedAt))
    invalid('planning-ready requires started repository/branch evidence');
  if (transition.planningControlLaunchAcceptedAt && !transition.startedReevaluationLifecycleObservedAt)
    invalid('planning control requires completed started reevaluation observation');
  if (transition.workSlicePlanningPointId && !transition.planningControlLaunchAcceptedAt)
    invalid('Work Slice Planner request requires launch-accepted planning control');
  if (transition.workSlicePlannerLaunchAcceptedAt && (!transition.workSlicePlanningPointId || !transition.workSlicePlannerHarnessAppliedAt))
    invalid('Planner launch acceptance requires an authorized applied-Harness planning point');
  if (!transition.downstreamNotStarted && !transition.workSlicePlanningPointId)
    invalid('Sprint Runner transition must not imply downstream work');
  return transition;
}
function object(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    invalid(`${label} must be an object`);
  return value as Record<string, unknown>;
}
function exact(value: Record<string, unknown>, allowed: readonly string[], label: string) {
  if (Object.keys(value).some((key) => !allowed.includes(key)))
    invalid(`${label} contains unsupported fields`);
}
function text(value: unknown, label: string): string {
  if (typeof value !== 'string' || !value.trim()) invalid(`${label} must be non-empty text`);
  return value;
}
function optionalText(value: Record<string, unknown>, key: string): Record<string, string> {
  if (value[key] == null) return {};
  return { [key]: text(value[key], key) };
}
function optionalNumber(value: Record<string, unknown>, key: string): Record<string, number> {
  if (value[key] == null) return {};
  if (typeof value[key] !== 'number' || !Number.isInteger(value[key]))
    invalid(`${key} must be an integer`);
  return { [key]: value[key] };
}
function bool(value: unknown, label: string): boolean {
  if (typeof value !== 'boolean') invalid(`${label} must be boolean`);
  return value;
}
function unique(values: readonly string[], label: string) {
  if (new Set(values).size !== values.length) invalid(`${label} must be unique`);
}
function invalid(message: string): never {
  throw new Error(`Invalid orchestration native query: ${message}`);
}
