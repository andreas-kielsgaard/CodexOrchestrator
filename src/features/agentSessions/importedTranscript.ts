import type { AgentInvocationDetailsDto } from '../../application/agentSessions';
export interface ImportedTranscript {
  ordinal: number;
  sourceStartedAt: number | null;
  items: { eventId: string; kind: 'user' | 'assistant' | 'activity'; text: string }[];
}
export function importedTranscript(
  entry: Pick<AgentInvocationDetailsDto, 'events' | 'importProvenance'>,
): ImportedTranscript | undefined {
  const provenance = entry.importProvenance;
  if (!provenance) return undefined;
  return {
    ordinal: provenance.ordinal,
    sourceStartedAt: provenance.sourceStartedAt,
    items: [...entry.events]
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
