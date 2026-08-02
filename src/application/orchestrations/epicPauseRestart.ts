/** Durable dispatch facts only; no provider receipt, compliance, or work progress is implied. */
export interface EpicPauseRestartOutcome {
  readonly actionId: string;
  readonly kind: 'pause' | 'restart';
  readonly status: 'pending' | 'partial' | 'attention' | 'completed';
  readonly targetCount: number;
  readonly launchedCount: number;
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
  async load() { throw new Error('Epic controls are unavailable.'); },
  async requestPause() { throw new Error('Epic pause is unavailable.'); },
  async requestRestart() { throw new Error('Epic restart is unavailable.'); },
};

/** Reject unknown/malformed native data before it can become product control state. */
export function decodeEpicPauseRestartQuery(value: unknown): EpicPauseRestartQuery {
  const root = record(value, 'Epic control query');
  exact(root, ['epicId', 'pause', 'restart'], 'Epic control query');
  return { epicId: text(root.epicId, 'epicId'), pause: read(root.pause, 'pause'), restart: read(root.restart, 'restart') };
}
export function decodeEpicPauseRestartOutcome(value: unknown): EpicPauseRestartOutcome {
  return outcome(value, 'Epic control outcome');
}
function read(value: unknown, label: string): EpicControlRead {
  const item = record(value, label);
  exact(item, ['availability', 'reason', 'current'], label);
  const availability = item.availability;
  if (availability !== 'available' && availability !== 'busy' && availability !== 'unavailable') throw new Error(`${label}.availability is invalid`);
  return { availability, reason: text(item.reason, `${label}.reason`), ...(item.current === undefined || item.current === null ? {} : { current: outcome(item.current, `${label}.current`) }) };
}
function outcome(value: unknown, label: string): EpicPauseRestartOutcome {
  const item = record(value, label);
  exact(item, ['actionId', 'kind', 'status', 'targetCount', 'launchedCount'], label);
  if (item.kind !== 'pause' && item.kind !== 'restart') throw new Error(`${label}.kind is invalid`);
  if (!['pending', 'partial', 'attention', 'completed'].includes(String(item.status))) throw new Error(`${label}.status is invalid`);
  const targetCount = count(item.targetCount, `${label}.targetCount`);
  const launchedCount = count(item.launchedCount, `${label}.launchedCount`);
  if (launchedCount > targetCount) throw new Error(`${label}.launchedCount exceeds targetCount`);
  return { actionId: text(item.actionId, `${label}.actionId`), kind: item.kind, status: item.status as EpicPauseRestartOutcome['status'], targetCount, launchedCount };
}
function record(value: unknown, label: string): Record<string, unknown> { if (!value || typeof value !== 'object' || Array.isArray(value)) throw new Error(`${label} is invalid`); return value as Record<string, unknown>; }
function exact(value: Record<string, unknown>, allowed: readonly string[], label: string) { if (Object.keys(value).some((key) => !allowed.includes(key)) || allowed.some((key) => key !== 'current' && !(key in value))) throw new Error(`${label} has an invalid shape`); }
function text(value: unknown, label: string) { if (typeof value !== 'string' || !value.trim()) throw new Error(`${label} is invalid`); return value; }
function count(value: unknown, label: string) { if (!Number.isInteger(value) || (value as number) < 0) throw new Error(`${label} is invalid`); return value as number; }
