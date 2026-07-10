import type {
  AgentDiagnosticDto,
  AgentInvocationStatusDto,
  AgentRuntimeEventDto,
  AgentRuntimeUsageDto,
  AgentSessionDetailsDto,
  IsoDateTimeDto,
} from '../../application/agentSessions';

export type TranscriptActivityKind =
  'processing' | 'tool' | 'agent_intermediate' | 'usage' | 'technical';

export interface TranscriptActivity {
  id: string;
  kind: TranscriptActivityKind;
  text: string;
  source: AgentRuntimeEventDto['source'];
  recordedAt: IsoDateTimeDto;
  rawPayload: unknown;
}

export interface TranscriptOutcome {
  status: AgentInvocationStatusDto;
  label: string;
  message: string | null;
}

export interface ProjectedInvocation {
  id: string;
  submittedText: string;
  status: AgentInvocationStatusDto;
  isActive: boolean;
  createdAt: IsoDateTimeDto;
  processing: TranscriptActivity[];
  technical: TranscriptActivity[];
  diagnostics: AgentDiagnosticDto[];
  finalResponse: string | null;
  outcome: TranscriptOutcome;
}

export interface ProjectedTranscript {
  sessionId: string;
  invocations: ProjectedInvocation[];
  activeInvocationId: string | null;
}

const activeStatuses = new Set<AgentInvocationStatusDto>(['pending', 'running']);

export function projectAgentSessionTranscript(
  details: AgentSessionDetailsDto,
): ProjectedTranscript {
  const invocations = [...details.invocations]
    .sort((left, right) =>
      compareOrdered(
        left.invocation.createdAt,
        left.invocation.id,
        right.invocation.createdAt,
        right.invocation.id,
      ),
    )
    .map(({ invocation, events }): ProjectedInvocation => {
      const orderedEvents = [...events].sort(
        (left, right) =>
          left.sequence - right.sequence ||
          compareOrdered(left.recordedAt, left.id, right.recordedAt, right.id),
      );
      const finalEvent = findLastFinalAgentMessage(orderedEvents);
      const processing: TranscriptActivity[] = [];
      const technical: TranscriptActivity[] = [];

      for (const event of orderedEvents) {
        if (event === finalEvent) {
          continue;
        }

        const activity = projectActivity(event);
        if (!activity) {
          continue;
        }
        if (activity.kind === 'technical') {
          technical.push(activity);
        } else {
          processing.push(activity);
        }
      }

      return {
        id: invocation.id,
        submittedText: invocation.submittedText,
        status: invocation.status,
        isActive: activeStatuses.has(invocation.status),
        createdAt: invocation.createdAt,
        processing,
        technical,
        diagnostics: [...invocation.diagnostics].sort((left, right) =>
          left.recordedAt.localeCompare(right.recordedAt),
        ),
        finalResponse: finalEvent?.normalized?.text?.trim() || null,
        outcome: projectOutcome(invocation.status, invocation.runtimeError?.message ?? null),
      };
    });

  return {
    sessionId: details.session.id,
    invocations,
    activeInvocationId: invocations.find((invocation) => invocation.isActive)?.id ?? null,
  };
}

function findLastFinalAgentMessage(
  events: AgentRuntimeEventDto[],
): AgentRuntimeEventDto | undefined {
  for (let index = events.length - 1; index >= 0; index -= 1) {
    const event = events[index];
    if (
      event.normalized?.kind === 'agent_message' &&
      event.normalized.text?.trim() &&
      hasDetail(event.normalized.details, 'role', 'final')
    ) {
      return event;
    }
  }
  return undefined;
}

function projectActivity(event: AgentRuntimeEventDto): TranscriptActivity | null {
  const normalized = event.normalized;
  const kind = normalized?.kind;
  const base = {
    id: event.id,
    source: event.source,
    recordedAt: event.recordedAt,
    rawPayload: event.rawPayload,
  };

  if (event.source === 'stderr' || !normalized || kind === 'unknown' || kind === 'runtime_error') {
    return {
      ...base,
      kind: 'technical',
      text: normalized?.text?.trim() || technicalLabel(event),
    };
  }

  if (kind === 'processing_started') {
    return { ...base, kind: 'processing', text: normalized.text?.trim() || 'Processing started' };
  }
  if (kind === 'processing_update') {
    return { ...base, kind: 'processing', text: normalized.text?.trim() || 'Processing update' };
  }
  if (kind === 'tool_activity') {
    return {
      ...base,
      kind: 'tool',
      text: normalized.text?.trim() || toolLabel(normalized.details),
    };
  }
  if (kind === 'agent_message') {
    return {
      ...base,
      kind: 'agent_intermediate',
      text: normalized.text?.trim() || 'Agent message',
    };
  }
  if (kind === 'usage') {
    return { ...base, kind: 'usage', text: usageLabel(normalized.usage) };
  }
  if (kind === 'runtime_context_established' || kind === 'invocation_completed') {
    return null;
  }

  return { ...base, kind: 'technical', text: technicalLabel(event) };
}

function projectOutcome(
  status: AgentInvocationStatusDto,
  runtimeError: string | null,
): TranscriptOutcome {
  switch (status) {
    case 'pending':
      return { status, label: 'Queued', message: null };
    case 'running':
      return { status, label: 'Working', message: null };
    case 'completed':
      return { status, label: 'Completed', message: null };
    case 'failed':
      return { status, label: 'Failed', message: runtimeError || 'The agent invocation failed.' };
    case 'canceled':
      return { status, label: 'Canceled', message: 'This invocation was canceled.' };
    case 'interrupted':
      return {
        status,
        label: 'Interrupted',
        message: 'This invocation was interrupted before completion.',
      };
  }
}

function compareOrdered(
  leftDate: string,
  leftId: string,
  rightDate: string,
  rightId: string,
): number {
  return leftDate.localeCompare(rightDate) || leftId.localeCompare(rightId);
}

function hasDetail(value: unknown, key: string, expected: string): boolean {
  return Boolean(
    value && typeof value === 'object' && (value as Record<string, unknown>)[key] === expected,
  );
}

function toolLabel(details: unknown): string {
  if (details && typeof details === 'object') {
    const itemType = (details as Record<string, unknown>).itemType;
    if (typeof itemType === 'string') {
      return itemType.replaceAll('_', ' ');
    }
  }
  return 'Tool activity';
}

function technicalLabel(event: AgentRuntimeEventDto): string {
  if (typeof event.rawPayload === 'string' && event.rawPayload.trim()) {
    return event.rawPayload.trim();
  }
  return `${event.source} event (${event.normalized?.kind ?? 'unparsed'})`;
}

function usageLabel(usage: AgentRuntimeUsageDto | null): string {
  if (!usage) return 'Usage recorded';
  const parts = [
    usage.inputTokens === null ? null : `${usage.inputTokens} input`,
    usage.cachedInputTokens === null ? null : `${usage.cachedInputTokens} cached`,
    usage.outputTokens === null ? null : `${usage.outputTokens} output`,
  ].filter(Boolean);
  return parts.length ? `Usage: ${parts.join(', ')}` : 'Usage recorded';
}
