import type { AgentRuntimeEventDto } from '../../application/agentSessions';
export interface ImportedTranscript {
  ordinal: number;
  sourceStartedAt: number | null;
  items: { eventId: string; kind: 'user' | 'assistant' | 'activity'; text: string }[];
}
export function importedTranscript(
  events: readonly AgentRuntimeEventDto[],
): ImportedTranscript | undefined {
  const details = events
    .map((e) => e.normalized?.details)
    .find((d) => isRecord(d) && d.kind === 'codex_history_import');
  if (!isRecord(details) || typeof details.ordinal !== 'number') return undefined;
  return {
    ordinal: details.ordinal,
    sourceStartedAt: typeof details.sourceStartedAt === 'number' ? details.sourceStartedAt : null,
    items: [...events]
      .sort((a, b) => a.sequence - b.sequence)
      .flatMap((e) => {
        const data = e.normalized?.details;
        const item = isRecord(data) ? data.importedContent : null;
        if (
          !isRecord(item) ||
          typeof item.text !== 'string' ||
          (item.kind !== 'user' && item.kind !== 'assistant' && item.kind !== 'activity')
        )
          return [];
        return [{ eventId: e.id, kind: item.kind, text: item.text }];
      }),
  };
}
function isRecord(value: unknown): value is Record<string, unknown> {
  return typeof value === 'object' && value !== null && !Array.isArray(value);
}
