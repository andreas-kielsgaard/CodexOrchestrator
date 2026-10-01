import type { AgentRuntimeEventDto, AgentRuntimeUsageDto } from '../../application/agentSessions';

export interface ContextUsage {
  used: number;
  capacity: number | null;
}
const record = (value: unknown): Record<string, unknown> | null =>
  value !== null && typeof value === 'object' ? (value as Record<string, unknown>) : null;
const count = (value: unknown): number | null =>
  typeof value === 'number' && Number.isFinite(value) && value >= 0 ? value : null;
function snapshot(event: AgentRuntimeEventDto) {
  const raw = record(event.rawPayload);
  return record(raw?.tokenUsage ?? record(raw?.params)?.tokenUsage);
}
function tokens(value: unknown): AgentRuntimeUsageDto | null {
  const item = record(value);
  if (!item) return null;
  return {
    inputTokens: count(item.inputTokens ?? item.input_tokens),
    outputTokens: count(item.outputTokens ?? item.output_tokens),
    cachedInputTokens: count(item.cachedInputTokens ?? item.cached_input_tokens),
  };
}
/** Use the last model request, never the thread's cumulative billable total, for context use. */
export function contextUsage(events: readonly AgentRuntimeEventDto[]): ContextUsage | null {
  let latest: { at: number; usage: ContextUsage } | null = null;
  for (let i = events.length - 1; i >= 0; i--) {
    const event = events[i];
    const data = snapshot(event);
    const last = record(data?.last);
    const usage = tokens(last) ?? event.normalized?.usage;
    const used =
      count(last?.totalTokens) ??
      (usage?.inputTokens !== null && usage?.inputTokens !== undefined
        ? usage.inputTokens + (usage.outputTokens ?? 0)
        : null);
    const at = Date.parse(event.recordedAt);
    if (used !== null && Number.isFinite(at) && (!latest || at > latest.at)) {
      latest = { at, usage: { used, capacity: count(data?.modelContextWindow) } };
    }
  }
  return latest?.usage ?? null;
}
/** Estimate turn totals from cumulative differences plus the first reported request. */
export function turnUsage(events: readonly AgentRuntimeEventDto[]): AgentRuntimeUsageDto | null {
  const turnIds = new Set(
    events.flatMap((event) => {
      const native = record(record(event.rawPayload)?.native);
      const params = record(native?.params);
      const id = record(params?.turn)?.id;
      return native?.method === 'turn/started' && typeof id === 'string' ? [id] : [];
    }),
  );
  const reports = events.filter((event) => {
    if (!event.normalized?.usage) return false;
    const native = record(record(event.rawPayload)?.native);
    const turnId = record(native?.params)?.turnId;
    // A resumed thread can replay the previous turn's usage before this turn starts.
    return !turnIds.size || typeof turnId !== 'string' || turnIds.has(turnId);
  });
  if (!reports.length) return null;
  const first = reports[0];
  const last = reports[reports.length - 1];
  const initial = tokens(snapshot(first)?.total);
  const final = tokens(snapshot(last)?.total);
  const base = first.normalized!.usage!;
  if (!initial || !final) return last.normalized!.usage;
  const estimate = (key: keyof AgentRuntimeUsageDto) =>
    initial[key] !== null && final[key] !== null && base[key] !== null
      ? Math.max(0, final[key]! - initial[key]!) + base[key]!
      : last.normalized!.usage![key];
  return {
    inputTokens: estimate('inputTokens'),
    outputTokens: estimate('outputTokens'),
    cachedInputTokens: estimate('cachedInputTokens'),
  };
}
