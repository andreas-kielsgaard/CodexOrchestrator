import type {
  AgentInvocationDetailsDto,
  AgentSessionDetailsDto,
  AgentSessionUpdateDto,
} from './contracts';
import { applySteeringEvent } from './steeringProjection';

export interface AgentSessionHistorySnapshot {
  readonly details: AgentSessionDetailsDto | null;
  readonly loading: boolean;
  readonly refreshing: boolean;
  readonly error: string | null;
  readonly revision: number;
  readonly changes: readonly AgentSessionUpdateDto[];
}

export interface AgentSessionHistorySource {
  getSnapshot(sessionId: string): AgentSessionHistorySnapshot;
  subscribe(sessionId: string, listener: () => void): () => void;
  ensure(sessionId: string): Promise<AgentSessionDetailsDto>;
  refresh(sessionId: string): Promise<AgentSessionDetailsDto>;
}

export type AgentSessionHistoryReduction =
  | { readonly kind: 'unchanged'; readonly details: AgentSessionDetailsDto }
  | { readonly kind: 'changed'; readonly details: AgentSessionDetailsDto }
  | { readonly kind: 'reconcile' };

const replaceInvocation = (
  details: AgentSessionDetailsDto,
  invocationId: string,
  replace: (entry: AgentInvocationDetailsDto) => AgentInvocationDetailsDto,
): AgentSessionDetailsDto | null => {
  const index = details.invocations.findIndex((entry) => entry.invocation.id === invocationId);
  if (index < 0) return null;
  const invocations = [...details.invocations];
  invocations[index] = replace(invocations[index]);
  return { ...details, invocations };
};

/** Applies updates that already carry their durable payload; only missing durable data reconciles. */
export function reduceAgentSessionHistory(
  details: AgentSessionDetailsDto,
  update: AgentSessionUpdateDto,
): AgentSessionHistoryReduction {
  if (update.sessionId !== details.session.id) return { kind: 'unchanged', details };
  if (update.kind === 'replica_updated') return { kind: 'reconcile' };
  if (update.kind === 'queue_updated') {
    if (!update.timeline && !update.session && !update.removedInvocationIds?.length)
      return { kind: 'unchanged', details };
    const removed = new Set(update.removedInvocationIds);
    return {
      kind: 'changed',
      details: {
        ...details,
        session: update.session ?? details.session,
        timeline: update.timeline ?? details.timeline,
        invocations: details.invocations.filter((entry) => !removed.has(entry.invocation.id)),
      },
    };
  }
  if (update.kind === 'event_persisted') {
    const entry = details.invocations.find(
      (candidate) => candidate.invocation.id === update.invocationId,
    );
    if (!entry) return { kind: 'reconcile' };
    if (entry.events.some((event) => event.id === update.event.id)) {
      return { kind: 'unchanged', details };
    }
    const lastSequence = entry.lastEventSequence ?? entry.events.at(-1)?.sequence;
    if (lastSequence === undefined && update.event.sequence > 1) return { kind: 'reconcile' };
    if (lastSequence !== undefined && update.event.sequence !== lastSequence + 1)
      return { kind: 'reconcile' };
    const next = replaceInvocation(details, update.invocationId, (current) => ({
      ...current,
      lastEventSequence: update.event.sequence,
      invocation:
        current.invocation.status === 'pending' &&
        update.event.normalized?.kind === 'processing_started'
          ? {
              ...current.invocation,
              status: 'running',
              startedAt: current.invocation.startedAt ?? update.event.recordedAt,
            }
          : current.invocation,
      events: [...current.events, update.event],
    }));
    const interactions = applySteeringEvent(details.interactions, update.event);
    return next
      ? { kind: 'changed', details: interactions ? { ...next, interactions } : next }
      : { kind: 'reconcile' };
  }
  if (update.kind === 'invocation_terminal' || update.kind === 'diagnostic_recorded') {
    const next = replaceInvocation(details, update.invocationId, (current) => ({
      ...current,
      invocation: update.invocation,
    }));
    return next
      ? {
          kind: 'changed',
          details:
            update.kind === 'invocation_terminal'
              ? {
                  ...next,
                  interactions: next.interactions?.map((i) =>
                    i.invocationId === update.invocationId &&
                    i.kind === 'steering' &&
                    i.state === 'queued'
                      ? { ...i, state: 'expired' as const }
                      : i,
                  ),
                }
              : next,
        }
      : { kind: 'reconcile' };
  }
  if (update.kind === 'preparation_updated') {
    return { kind: 'unchanged', details };
  }
  return { kind: 'reconcile' };
}
