import type { AgentInvocationObservationDto } from '../agentSessions/contracts';

/** Durable control dispatch facts only; provider receipt, compliance, and work progress stay separate. */
export interface EpicPauseRestartOutcome {
  readonly actionId: string;
  readonly kind: 'pause' | 'restart';
  readonly status: 'pending' | 'partial' | 'attention' | 'completed';
  readonly targetCount: number;
  readonly launchedCount: number;
  readonly targets: readonly EpicControlTargetObservation[];
}
/** One durable target selection and only its source/control invocation evidence. */
export interface EpicControlTargetObservation {
  readonly sessionId: string;
  readonly sourceInvocationId: string;
  readonly cancelRequestedAt: string | null;
  readonly interruptionStatus:
    'awaiting_cancel' | 'canceled' | 'interrupted' | 'failed' | 'completed';
  readonly interruptionObservedAt: string | null;
  readonly sourceObservation: AgentInvocationObservationDto | null;
  readonly controlInvocation: {
    readonly invocationId: string;
    readonly persistedAt: string;
    readonly launchAcceptedAt: string | null;
    readonly observation: AgentInvocationObservationDto | null;
  } | null;
  readonly failure: { readonly category: string; readonly detail: string } | null;
}
export interface EpicControlRead {
  readonly availability: 'available' | 'busy' | 'unavailable';
  readonly reason: string;
  readonly current?: EpicPauseRestartOutcome;
}
export interface EpicPauseRestartQuery {
  readonly epicId: string;
  readonly pause: EpicControlRead;
  readonly restart: EpicControlRead;
}
export interface EpicPauseRestartController {
  load(epicId: string): Promise<EpicPauseRestartQuery>;
  requestPause(epicId: string): Promise<EpicPauseRestartOutcome>;
  requestRestart(epicId: string): Promise<EpicPauseRestartOutcome>;
}
export const unsupportedEpicPauseRestartController: EpicPauseRestartController = {
  async load() {
    throw new Error('Epic controls are unavailable.');
  },
  async requestPause() {
    throw new Error('Epic pause is unavailable.');
  },
  async requestRestart() {
    throw new Error('Epic restart is unavailable.');
  },
};

/** Reject unknown/malformed native data before it can become product control state. */
export function decodeEpicPauseRestartQuery(value: unknown): EpicPauseRestartQuery {
  const root = record(value, 'Epic control query');
  exact(root, ['epicId', 'pause', 'restart'], 'Epic control query');
  return {
    epicId: text(root.epicId, 'epicId'),
    pause: read(root.pause, 'pause'),
    restart: read(root.restart, 'restart'),
  };
}
export function decodeEpicPauseRestartOutcome(value: unknown): EpicPauseRestartOutcome {
  return outcome(value, 'Epic control outcome');
}
function read(value: unknown, label: string): EpicControlRead {
  const item = record(value, label);
  exact(item, ['availability', 'reason', 'current'], label);
  const availability = item.availability;
  if (availability !== 'available' && availability !== 'busy' && availability !== 'unavailable')
    throw new Error(`${label}.availability is invalid`);
  return {
    availability,
    reason: text(item.reason, `${label}.reason`),
    ...(item.current === undefined || item.current === null
      ? {}
      : { current: outcome(item.current, `${label}.current`) }),
  };
}
function outcome(value: unknown, label: string): EpicPauseRestartOutcome {
  const item = record(value, label);
  exact(item, ['actionId', 'kind', 'status', 'targetCount', 'launchedCount', 'targets'], label);
  if (item.kind !== 'pause' && item.kind !== 'restart') throw new Error(`${label}.kind is invalid`);
  if (!['pending', 'partial', 'attention', 'completed'].includes(String(item.status)))
    throw new Error(`${label}.status is invalid`);
  const targetCount = count(item.targetCount, `${label}.targetCount`);
  const launchedCount = count(item.launchedCount, `${label}.launchedCount`);
  if (launchedCount > targetCount) throw new Error(`${label}.launchedCount exceeds targetCount`);
  const targets = list(item.targets, `${label}.targets`).map((target, index) =>
    targetObservation(target, `${label}.targets[${index}]`),
  );
  if (targets.length !== targetCount)
    throw new Error(`${label}.targets does not match targetCount`);
  return {
    actionId: text(item.actionId, `${label}.actionId`),
    kind: item.kind,
    status: item.status as EpicPauseRestartOutcome['status'],
    targetCount,
    launchedCount,
    targets,
  };
}
function targetObservation(value: unknown, label: string): EpicControlTargetObservation {
  const item = record(value, label);
  exact(
    item,
    [
      'sessionId',
      'sourceInvocationId',
      'cancelRequestedAt',
      'interruptionStatus',
      'interruptionObservedAt',
      'sourceObservation',
      'controlInvocation',
      'failure',
    ],
    label,
  );
  if (
    !['awaiting_cancel', 'canceled', 'interrupted', 'failed', 'completed'].includes(
      String(item.interruptionStatus),
    )
  )
    throw new Error(`${label}.interruptionStatus is invalid`);
  return {
    sessionId: text(item.sessionId, `${label}.sessionId`),
    sourceInvocationId: text(item.sourceInvocationId, `${label}.sourceInvocationId`),
    cancelRequestedAt: nullableText(item.cancelRequestedAt, `${label}.cancelRequestedAt`),
    interruptionStatus:
      item.interruptionStatus as EpicControlTargetObservation['interruptionStatus'],
    interruptionObservedAt: nullableText(
      item.interruptionObservedAt,
      `${label}.interruptionObservedAt`,
    ),
    sourceObservation: observation(item.sourceObservation, `${label}.sourceObservation`),
    controlInvocation: controlInvocation(item.controlInvocation, `${label}.controlInvocation`),
    failure: failure(item.failure, `${label}.failure`),
  };
}
function controlInvocation(
  value: unknown,
  label: string,
): EpicControlTargetObservation['controlInvocation'] {
  if (value === null) return null;
  const item = record(value, label);
  exact(item, ['invocationId', 'persistedAt', 'launchAcceptedAt', 'observation'], label);
  return {
    invocationId: text(item.invocationId, `${label}.invocationId`),
    persistedAt: text(item.persistedAt, `${label}.persistedAt`),
    launchAcceptedAt: nullableText(item.launchAcceptedAt, `${label}.launchAcceptedAt`),
    observation: observation(item.observation, `${label}.observation`),
  };
}
function failure(value: unknown, label: string): EpicControlTargetObservation['failure'] {
  if (value === null) return null;
  const item = record(value, label);
  exact(item, ['category', 'detail'], label);
  return {
    category: text(item.category, `${label}.category`),
    detail: text(item.detail, `${label}.detail`),
  };
}
/** Agent Session remains the owner of this model; strict outer shape prevents foreign control data. */
function observation(value: unknown, label: string): AgentInvocationObservationDto | null {
  if (value === null) return null;
  const item = record(value, label);
  exact(
    item,
    [
      'launchAcceptedAt',
      'externalContext',
      'providerActivity',
      'providerTerminal',
      'processTerminal',
      'mcpToolActivities',
      'mcpToolActivityPartial',
    ],
    label,
  );
  if (typeof item.mcpToolActivityPartial !== 'boolean' || !Array.isArray(item.mcpToolActivities))
    throw new Error(`${label} is invalid`);
  return item as unknown as AgentInvocationObservationDto;
}
function record(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error(`${label} is invalid`);
  return value as Record<string, unknown>;
}
function exact(value: Record<string, unknown>, allowed: readonly string[], label: string) {
  if (
    Object.keys(value).some((key) => !allowed.includes(key)) ||
    allowed.some((key) => key !== 'current' && !(key in value))
  )
    throw new Error(`${label} has an invalid shape`);
}
function text(value: unknown, label: string) {
  if (typeof value !== 'string' || !value.trim()) throw new Error(`${label} is invalid`);
  return value;
}
function count(value: unknown, label: string) {
  if (!Number.isInteger(value) || (value as number) < 0) throw new Error(`${label} is invalid`);
  return value as number;
}
function nullableText(value: unknown, label: string) {
  return value === null ? null : text(value, label);
}
function list(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) throw new Error(`${label} is invalid`);
  return value;
}
