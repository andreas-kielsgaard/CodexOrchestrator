import type { AgentRuntimeEventDto, AgentSessionDetailsDto } from '../../application/agentSessions';

export function deferredEventIds(value: unknown): string[] {
  if (!value || typeof value !== 'object') return [];
  if ('_orchidDeferredEventId' in value && typeof value._orchidDeferredEventId === 'string')
    return [value._orchidDeferredEventId];
  return [...new Set(Object.values(value).flatMap(deferredEventIds))];
}
export function restoreDeferredPayload(
  value: unknown,
  events: ReadonlyMap<string, AgentRuntimeEventDto>,
  normalized = false,
): unknown {
  if (!value || typeof value !== 'object') return value;
  if ('_orchidDeferredEventId' in value && typeof value._orchidDeferredEventId === 'string') {
    const event = events.get(value._orchidDeferredEventId);
    if (event) return normalized ? event.normalized?.details : event.rawPayload;
  }
  if (Array.isArray(value)) return value.map((v) => restoreDeferredPayload(v, events, normalized));
  return Object.fromEntries(
    Object.entries(value).map(([key, v]) => [key, restoreDeferredPayload(v, events, normalized)]),
  );
}
export async function fullActionDetails(
  details: AgentSessionDetailsDto,
  load: (id: string) => Promise<AgentRuntimeEventDto>,
) {
  const ids = [
    ...new Set(
      details.invocations.flatMap((turn) =>
        turn.events.flatMap((event) => deferredEventIds(event.rawPayload)),
      ),
    ),
  ];
  const events = new Map<string, AgentRuntimeEventDto>();
  for (let index = 0; index < ids.length; index += 4) {
    const batch = await Promise.all(ids.slice(index, index + 4).map((id) => load(id)));
    batch.forEach((event) => events.set(event.id, event));
  }
  return {
    ...details,
    invocations: details.invocations.map((turn) => ({
      ...turn,
      events: turn.events.map((event) => events.get(event.id) ?? event),
    })),
  };
}
