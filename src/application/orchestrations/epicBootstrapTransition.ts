export const EPIC_BOOTSTRAP_TRANSITION_CONTRACT = 'epic-bootstrap-transition-query/v2';

export type BootstrapLifecycleStatus =
  'pending' | 'running' | 'completed' | 'failed' | 'canceled' | 'interrupted';
export type BootstrapRetryState = 'active' | 'retryable' | 'retried' | 'blocked' | 'accepted';

export interface EpicBootstrapAttemptV2 {
  readonly attemptId: string;
  readonly ordinal: number;
  readonly agentSessionId: string;
  readonly agentInvocationId: string;
  readonly launchedAt?: string;
  readonly lifecycleStatus?: BootstrapLifecycleStatus;
  readonly lifecycleObservedAt?: string;
  readonly semanticCompletionFactId?: string;
  readonly semanticCompletedAt?: string;
  readonly retryDisposition: BootstrapRetryState;
  readonly retryReason?: string;
  readonly retryAttemptId?: string;
  readonly acceptedAt?: string;
}

export interface EpicBootstrapTransitionV2 {
  readonly initiationId: string;
  readonly epicId: string;
  readonly preparationId: string;
  readonly preparedRoot: string;
  readonly approvedPlanPath: string;
  readonly manifestPath: string;
  readonly overviewPath: string;
  readonly runnerBriefPath: string;
  readonly bootstrapSessionId: string;
  readonly bootstrapInvocationId: string;
  readonly preparedAt?: string;
  readonly bootstrapSessionCreatedAt?: string;
  readonly bootstrapLaunchedAt?: string;
  readonly bootstrapLifecycleStatus?: BootstrapLifecycleStatus;
  readonly bootstrapLifecycleObservedAt?: string;
  readonly semanticCompletionFactId?: string;
  readonly semanticCompletedAt?: string;
  readonly materialAcceptedAt?: string;
  readonly runnerSessionId: string;
  readonly runnerInvocationId: string;
  readonly runnerSessionCreatedAt?: string;
  readonly runnerLaunchedAt?: string;
  readonly runnerLifecycleStatus?: BootstrapLifecycleStatus;
  readonly runnerLifecycleObservedAt?: string;
  readonly currentAttemptId: string;
  readonly retryState: BootstrapRetryState;
  readonly blockedReason?: string;
  readonly acceptedAttemptId?: string;
  readonly bootstrapAttempts: readonly EpicBootstrapAttemptV2[];
}

export interface EpicBootstrapTransitionQueryV2 {
  readonly contract: typeof EPIC_BOOTSTRAP_TRANSITION_CONTRACT;
  readonly schemaVersion: 2;
  readonly transitions: readonly EpicBootstrapTransitionV2[];
}

export type ProductBootstrapTransitionStatusV2 =
  | { readonly kind: 'preparing'; readonly label: string }
  | { readonly kind: 'bootstrap_running'; readonly label: string }
  | { readonly kind: 'awaiting_matching_lifecycle'; readonly label: string }
  | { readonly kind: 'retrying'; readonly label: string }
  | { readonly kind: 'blocked'; readonly label: string; readonly reason: string }
  | { readonly kind: 'material_accepted'; readonly label: string }
  | { readonly kind: 'runner_launching'; readonly label: string }
  | { readonly kind: 'runner_launched'; readonly label: string };

export function projectBootstrapTransitionStatus(
  transition: EpicBootstrapTransitionV2,
): ProductBootstrapTransitionStatusV2 {
  if (!transition.preparedAt) return { kind: 'preparing', label: 'Preparing approved Epic inputs' };
  if (!transition.bootstrapSessionCreatedAt)
    return { kind: 'preparing', label: 'Preparation recorded; creating Bootstrap session' };
  if (transition.retryState === 'blocked')
    return {
      kind: 'blocked',
      label: 'Bootstrap transition blocked',
      reason:
        transition.blockedReason ?? 'The durable transition did not provide a blocked reason.',
    };
  if (transition.retryState === 'retryable' || transition.retryState === 'retried')
    return {
      kind: 'retrying',
      label: `Retrying Bootstrap generation (attempt ${currentAttempt(transition).ordinal + 1} of 3)`,
    };
  if (transition.semanticCompletionFactId && !transition.acceptedAttemptId)
    return {
      kind: 'awaiting_matching_lifecycle',
      label: 'Bootstrap completion recorded; awaiting matching successful lifecycle',
    };
  if (!transition.materialAcceptedAt)
    return {
      kind: 'bootstrap_running',
      label: transition.bootstrapLaunchedAt
        ? `Bootstrap attempt ${currentAttempt(transition).ordinal + 1} running`
        : 'Bootstrap session created; launch acknowledgement pending',
    };
  if (!transition.runnerSessionCreatedAt)
    return { kind: 'material_accepted', label: 'Bootstrap material accepted' };
  if (!transition.runnerLaunchedAt)
    return {
      kind: 'runner_launching',
      label: 'Epic Runner session created; launch acknowledgement pending',
    };
  return { kind: 'runner_launched', label: 'Epic Runner launched; no Sprint has started' };
}

export function decodeEpicBootstrapTransitionQueryV2(
  value: unknown,
): EpicBootstrapTransitionQueryV2 {
  const root = object(value, 'transition query');
  exactKeys(root, ['contract', 'schemaVersion', 'transitions'], 'transition query');
  if (
    root.contract !== EPIC_BOOTSTRAP_TRANSITION_CONTRACT ||
    root.schemaVersion !== 2 ||
    !Array.isArray(root.transitions)
  )
    invalid('unsupported transition query contract');
  const transitions = root.transitions.map(decodeTransition);
  unique(
    transitions.map((item) => item.initiationId),
    'initiation',
  );
  unique(
    transitions.map((item) => item.epicId),
    'Epic',
  );
  return { contract: EPIC_BOOTSTRAP_TRANSITION_CONTRACT, schemaVersion: 2, transitions };
}

function decodeTransition(value: unknown): EpicBootstrapTransitionV2 {
  const item = object(value, 'transition');
  exactKeys(
    item,
    [
      'initiationId',
      'epicId',
      'preparationId',
      'preparedRoot',
      'approvedPlanPath',
      'manifestPath',
      'overviewPath',
      'runnerBriefPath',
      'bootstrapSessionId',
      'bootstrapInvocationId',
      'preparedAt',
      'bootstrapSessionCreatedAt',
      'bootstrapLaunchedAt',
      'bootstrapLifecycleStatus',
      'bootstrapLifecycleObservedAt',
      'semanticCompletionFactId',
      'semanticCompletedAt',
      'materialAcceptedAt',
      'runnerSessionId',
      'runnerInvocationId',
      'runnerSessionCreatedAt',
      'runnerLaunchedAt',
      'runnerLifecycleStatus',
      'runnerLifecycleObservedAt',
      'currentAttemptId',
      'retryState',
      'blockedReason',
      'acceptedAttemptId',
      'bootstrapAttempts',
    ],
    'transition',
  );
  if (!Array.isArray(item.bootstrapAttempts) || item.bootstrapAttempts.length === 0)
    invalid('transition must contain attempts');
  const attempts = item.bootstrapAttempts.map(decodeAttempt);
  const transition: EpicBootstrapTransitionV2 = {
    initiationId: text(item.initiationId, 'initiationId'),
    epicId: text(item.epicId, 'epicId'),
    preparationId: text(item.preparationId, 'preparationId'),
    preparedRoot: text(item.preparedRoot, 'preparedRoot'),
    approvedPlanPath: text(item.approvedPlanPath, 'approvedPlanPath'),
    manifestPath: text(item.manifestPath, 'manifestPath'),
    overviewPath: text(item.overviewPath, 'overviewPath'),
    runnerBriefPath: text(item.runnerBriefPath, 'runnerBriefPath'),
    bootstrapSessionId: text(item.bootstrapSessionId, 'bootstrapSessionId'),
    bootstrapInvocationId: text(item.bootstrapInvocationId, 'bootstrapInvocationId'),
    ...optionalTextFields(item, [
      'preparedAt',
      'bootstrapSessionCreatedAt',
      'bootstrapLaunchedAt',
      'bootstrapLifecycleObservedAt',
      'semanticCompletionFactId',
      'semanticCompletedAt',
      'materialAcceptedAt',
      'runnerSessionCreatedAt',
      'runnerLaunchedAt',
      'runnerLifecycleObservedAt',
      'blockedReason',
      'acceptedAttemptId',
    ]),
    ...(item.bootstrapLifecycleStatus == null
      ? {}
      : { bootstrapLifecycleStatus: lifecycle(item.bootstrapLifecycleStatus) }),
    runnerSessionId: text(item.runnerSessionId, 'runnerSessionId'),
    runnerInvocationId: text(item.runnerInvocationId, 'runnerInvocationId'),
    ...(item.runnerLifecycleStatus == null
      ? {}
      : { runnerLifecycleStatus: lifecycle(item.runnerLifecycleStatus) }),
    currentAttemptId: text(item.currentAttemptId, 'currentAttemptId'),
    retryState: retry(item.retryState),
    bootstrapAttempts: attempts,
  } as EpicBootstrapTransitionV2;
  if (attempts.at(-1)?.attemptId !== transition.currentAttemptId)
    invalid('current attempt is not the final ordered attempt');
  if (
    transition.acceptedAttemptId &&
    !attempts.some(
      (attempt) =>
        attempt.attemptId === transition.acceptedAttemptId && attempt.acceptedAt !== undefined,
    )
  )
    invalid('accepted attempt correlation is invalid');
  return transition;
}

function decodeAttempt(value: unknown): EpicBootstrapAttemptV2 {
  const item = object(value, 'attempt');
  exactKeys(
    item,
    [
      'attemptId',
      'ordinal',
      'agentSessionId',
      'agentInvocationId',
      'launchedAt',
      'lifecycleStatus',
      'lifecycleObservedAt',
      'semanticCompletionFactId',
      'semanticCompletedAt',
      'retryDisposition',
      'retryReason',
      'retryAttemptId',
      'acceptedAt',
    ],
    'attempt',
  );
  if (!Number.isSafeInteger(item.ordinal) || (item.ordinal as number) < 0)
    invalid('attempt ordinal is invalid');
  return {
    attemptId: text(item.attemptId, 'attemptId'),
    ordinal: item.ordinal as number,
    agentSessionId: text(item.agentSessionId, 'agentSessionId'),
    agentInvocationId: text(item.agentInvocationId, 'agentInvocationId'),
    ...optionalTextFields(item, [
      'launchedAt',
      'lifecycleObservedAt',
      'semanticCompletionFactId',
      'semanticCompletedAt',
      'retryReason',
      'retryAttemptId',
      'acceptedAt',
    ]),
    ...(item.lifecycleStatus == null ? {} : { lifecycleStatus: lifecycle(item.lifecycleStatus) }),
    retryDisposition: retry(item.retryDisposition),
  };
}

function currentAttempt(transition: EpicBootstrapTransitionV2) {
  return transition.bootstrapAttempts.find(
    (item) => item.attemptId === transition.currentAttemptId,
  )!;
}
function lifecycle(value: unknown): BootstrapLifecycleStatus {
  if (
    !['pending', 'running', 'completed', 'failed', 'canceled', 'interrupted'].includes(
      String(value),
    )
  )
    invalid('unknown lifecycle status');
  return value as BootstrapLifecycleStatus;
}
function retry(value: unknown): BootstrapRetryState {
  if (!['active', 'retryable', 'retried', 'blocked', 'accepted'].includes(String(value)))
    invalid('unknown retry state');
  return value as BootstrapRetryState;
}
function optionalTextFields(value: Record<string, unknown>, fields: readonly string[]) {
  return Object.fromEntries(
    fields
      .filter((field) => value[field] != null)
      .map((field) => [field, text(value[field], field)]),
  );
}
function object(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    invalid(`${label} must be an object`);
  return value as Record<string, unknown>;
}
function text(value: unknown, label: string): string {
  if (typeof value !== 'string' || value.trim() === '') invalid(`${label} must be non-empty text`);
  return value;
}
function exactKeys(value: Record<string, unknown>, allowed: readonly string[], label: string) {
  if (Object.keys(value).some((key) => !allowed.includes(key)))
    invalid(`${label} contains an unknown field`);
  if (
    allowed
      .filter(
        (key) =>
          !key.endsWith('At') &&
          ![
            'blockedReason',
            'acceptedAttemptId',
            'bootstrapLifecycleStatus',
            'bootstrapLifecycleObservedAt',
            'semanticCompletionFactId',
            'semanticCompletedAt',
            'materialAcceptedAt',
            'runnerLifecycleStatus',
            'runnerLifecycleObservedAt',
            'retryReason',
            'retryAttemptId',
            'lifecycleStatus',
            'lifecycleObservedAt',
            'launchedAt',
          ].includes(key),
      )
      .some((key) => !(key in value))
  )
    invalid(`${label} is missing a required field`);
}
function unique(values: readonly string[], label: string) {
  if (new Set(values).size !== values.length) invalid(`duplicate ${label} transition`);
}
function invalid(message: string): never {
  throw new Error(`Invalid Epic Bootstrap transition query: ${message}`);
}
