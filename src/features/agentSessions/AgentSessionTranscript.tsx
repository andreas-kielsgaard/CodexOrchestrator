import { useState, useEffect } from 'react';
import { AgentSessionInvocation } from './AgentSessionInvocation';
import { SessionTransitionIndicator } from './SessionTransitionIndicator';
import { SessionTransitionDetailsDialog } from './SessionTransitionDetailsDialog';
import type { SessionHistoryItem, SessionTransition } from './sessionHistoryProjector';
import {
  projectedTranscriptContent,
  type ProjectedTranscript,
  type ProjectedTranscriptContent,
} from './transcriptProjector';
import type { AgentIdentity, RuntimeInteractionResponseDto } from '../../application/agentSessions';

interface AgentSessionTranscriptProps {
  onRespondToRequest?(
    invocationId: string,
    requestId: string,
    response: RuntimeInteractionResponseDto,
  ): Promise<void>;
  transcript: ProjectedTranscript | null;
  history?: readonly SessionHistoryItem[];
  content?: readonly ProjectedTranscriptContent[];
  loading: boolean;
  expandedProcessing: ReadonlySet<string>;
  onToggleProcessing(invocationId: string): void;
  emptyState?: Readonly<{ heading: string; guidance: string }>;
  agentIdentity?: AgentIdentity;
  safeActivityDetails?: boolean;
  processingHeading?: string;
}

export function AgentSessionTranscript({
  transcript,
  history,
  content,
  loading,
  expandedProcessing,
  onToggleProcessing,
  emptyState,
  agentIdentity,
  safeActivityDetails = false,
  processingHeading,
  onRespondToRequest,
}: AgentSessionTranscriptProps) {
  const [selectedTransition, setSelectedTransition] = useState<SessionTransition | null>(null);
  useEffect(() => {
    if (
      selectedTransition?.kind === 'blocked' &&
      !history?.some((item) => item.id === selectedTransition.id)
    )
      setSelectedTransition(null);
  }, [history, selectedTransition]);
  if (loading && !transcript) {
    return (
      <p className="session-empty" role="status">
        Loading session…
      </p>
    );
  }
  const visibleInvocations = transcript
    ? projectVisibleInvocations(transcript, content ?? projectedTranscriptContent(transcript))
    : [];
  if (transcript && content && !visibleInvocations.length && !history?.length) return null;
  if (!transcript || (!visibleInvocations.length && !history?.length)) {
    return (
      <section className={`session-empty${emptyState ? ' session-empty--custom' : ''}`}>
        <h2>{emptyState?.heading ?? 'Start with a message'}</h2>
        {emptyState && <p className="session-empty__guidance">{emptyState.guidance}</p>}
        <p>Your inputs, the agent’s work, and its final responses will stay available here.</p>
      </section>
    );
  }

  const byId = new Map(visibleInvocations.map((invocation) => [invocation.id, invocation]));
  const items =
    history ??
    visibleInvocations.map((invocation) => ({ kind: 'invocation' as const, id: invocation.id }));
  return (
    <>
      <ol className="agent-transcript" aria-label="Agent Session transcript">
        {items.map((item) =>
          item.kind === 'invocation' ? (
            byId.has(item.id) && (
              <AgentSessionInvocation
                key={item.id}
                invocation={byId.get(item.id)!}
                expandedProcessing={expandedProcessing}
                onToggleProcessing={onToggleProcessing}
                safeActivityDetails={safeActivityDetails}
                processingHeading={processingHeading}
                agentIdentity={agentIdentity}
                onRespondToRequest={onRespondToRequest}
                providerChange={'providerChange' in item ? item.providerChange : undefined}
                onOpenTransition={setSelectedTransition}
              />
            )
          ) : (
            <li key={item.id}>
              <SessionTransitionIndicator transition={item} onOpen={setSelectedTransition} />
            </li>
          ),
        )}
      </ol>
      {selectedTransition && (
        <SessionTransitionDetailsDialog
          transition={selectedTransition}
          onClose={() => setSelectedTransition(null)}
        />
      )}
    </>
  );
}

export type VisibleInvocation = ProjectedTranscript['invocations'][number] & {
  showInput: boolean;
  showOutcome: boolean;
};

function projectVisibleInvocations(
  transcript: ProjectedTranscript | null,
  content: readonly ProjectedTranscriptContent[],
): VisibleInvocation[] {
  if (!transcript) return [];
  const visible = new Map<string, ProjectedTranscriptContent[]>();
  for (const item of content) {
    const items = visible.get(item.anchor.invocationId) ?? [];
    items.push(item);
    visible.set(item.anchor.invocationId, items);
  }
  return transcript.invocations.flatMap((invocation) => {
    const items = visible.get(invocation.id);
    if (!items) return [];
    const activityIds = new Set(
      items.filter((item) => item.kind === 'activity').map((item) => item.activity.id),
    );
    const final = items.find((item) => item.kind === 'final_response');
    return [
      {
        ...invocation,
        imported: invocation.imported
          ? {
              ...invocation.imported,
              items: invocation.imported.items.filter(
                (item, index) =>
                  activityIds.has(item.eventId) ||
                  (final?.kind === 'final_response' && final.response.eventId === item.eventId) ||
                  (index ===
                    invocation.imported!.items.findIndex((entry) => entry.kind === 'user') &&
                    item.kind === 'user' &&
                    items.some((i) => i.kind === 'submitted_input')),
              ),
            }
          : undefined,
        processing: invocation.processing.filter((item) => activityIds.has(item.id)),
        technical: [],
        finalResponse: final?.kind === 'final_response' ? final.response : null,
        showInput: items.some((item) => item.kind === 'submitted_input'),
        showOutcome: items.some((item) => item.kind === 'outcome'),
      },
    ];
  });
}
