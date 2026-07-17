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
  sequence: number;
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

export type TranscriptAnchorKind = 'submitted_input' | 'activity' | 'final_response' | 'outcome';

/** A durable pointer into one persisted Agent Session invocation. */
export interface TranscriptAnchor {
  sessionId: string;
  invocationId: string;
  kind: TranscriptAnchorKind;
  runtimeEventId?: string;
}

export interface TranscriptFinalResponse {
  anchor: TranscriptAnchor;
  eventId: string;
  text: string;
}

export type ProjectedTranscriptContent =
  | { anchor: TranscriptAnchor; kind: 'submitted_input'; text: string }
  | { anchor: TranscriptAnchor; kind: 'activity'; activity: TranscriptActivity }
  | { anchor: TranscriptAnchor; kind: 'final_response'; response: TranscriptFinalResponse }
  | { anchor: TranscriptAnchor; kind: 'outcome'; outcome: TranscriptOutcome };

export interface TranscriptAnchorRange {
  start: TranscriptAnchor;
  end: TranscriptAnchor;
}

export interface ProjectedInvocation {
  id: string;
  submittedText: string;
  inputProvenance: 'user' | 'application';
  status: AgentInvocationStatusDto;
  isActive: boolean;
  createdAt: IsoDateTimeDto;
  processing: TranscriptActivity[];
  technical: TranscriptActivity[];
  diagnostics: AgentDiagnosticDto[];
  finalResponse: TranscriptFinalResponse | null;
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
        inputProvenance: invocation.inputProvenance,
        status: invocation.status,
        isActive: activeStatuses.has(invocation.status),
        createdAt: invocation.createdAt,
        processing: coalesceLifecycleActivities(processing),
        technical,
        diagnostics: [...invocation.diagnostics].sort((left, right) =>
          left.recordedAt.localeCompare(right.recordedAt),
        ),
        finalResponse: finalEvent?.normalized?.text?.trim()
          ? {
              anchor: eventAnchor(
                details.session.id,
                invocation.id,
                'final_response',
                finalEvent.id,
              ),
              eventId: finalEvent.id,
              text: finalEvent.normalized.text.trim(),
            }
          : null,
        outcome: projectOutcome(invocation.status, invocation.runtimeError?.message ?? null),
      };
    });

  return {
    sessionId: details.session.id,
    invocations,
    activeInvocationId: invocations.find((invocation) => invocation.isActive)?.id ?? null,
  };
}

/**
 * Runtime JSONL emits lifecycle rows for one command or MCP item. Preserve those raw rows, but
 * render the paired start/completion as one logical activity so a completed operation is not
 * misrepresented as two calls.
 */
function coalesceLifecycleActivities(activities: TranscriptActivity[]): TranscriptActivity[] {
  const active = new Map<string, number>();
  const projected: TranscriptActivity[] = [];

  for (const activity of activities) {
    const lifecycle = lifecycleIdentity(activity);
    if (!lifecycle) {
      projected.push(activity);
      continue;
    }
    if (lifecycle.eventType === 'item.started') {
      active.set(lifecycle.key, projected.length);
      projected.push(activity);
      continue;
    }
    if (lifecycle.eventType === 'item.completed') {
      const index = active.get(lifecycle.key);
      if (index !== undefined) {
        const started = projected[index];
        projected[index] = {
          ...started,
          text: activity.text === toolLabelFromActivity(activity) ? started.text : activity.text,
          rawPayload: { lifecycleEvents: [started.rawPayload, activity.rawPayload] },
        };
        active.delete(lifecycle.key);
        continue;
      }
    }
    projected.push(activity);
  }
  return projected;
}

function lifecycleIdentity(
  activity: TranscriptActivity,
): { key: string; eventType: 'item.started' | 'item.completed' } | null {
  if (activity.kind !== 'tool' || !activity.rawPayload || typeof activity.rawPayload !== 'object') {
    return null;
  }
  const raw = activity.rawPayload as Record<string, unknown>;
  const item = raw.item;
  if (!item || typeof item !== 'object') return null;
  const itemId = (item as Record<string, unknown>).id;
  const itemType = (item as Record<string, unknown>).type;
  const eventType = raw.type;
  if (
    typeof itemId !== 'string' ||
    typeof itemType !== 'string' ||
    (eventType !== 'item.started' && eventType !== 'item.completed')
  ) {
    return null;
  }
  return { key: `${itemType}:${itemId}`, eventType };
}

function toolLabelFromActivity(activity: TranscriptActivity): string {
  const raw = activity.rawPayload;
  if (raw && typeof raw === 'object') {
    const item = (raw as Record<string, unknown>).item;
    if (item && typeof item === 'object') {
      const itemType = (item as Record<string, unknown>).type;
      if (typeof itemType === 'string') return itemType.replaceAll('_', ' ');
    }
  }
  return 'Tool activity';
}

/**
 * Returns projected content in durable session/invocation/event order.  It deliberately never
 * compares timestamps from different sessions; callers compose those sessions explicitly.
 */
export function projectedTranscriptContent(
  transcript: ProjectedTranscript,
): ProjectedTranscriptContent[] {
  return transcript.invocations.flatMap((invocation) => {
    const input: ProjectedTranscriptContent = {
      anchor: eventAnchor(transcript.sessionId, invocation.id, 'submitted_input'),
      kind: 'submitted_input',
      text: invocation.submittedText,
    };
    const activity = [...invocation.processing, ...invocation.technical]
      .sort(
        (left, right) =>
          left.sequence - right.sequence ||
          left.recordedAt.localeCompare(right.recordedAt) ||
          left.id.localeCompare(right.id),
      )
      .map((item): ProjectedTranscriptContent => ({
        anchor: eventAnchor(transcript.sessionId, invocation.id, 'activity', item.id),
        kind: 'activity',
        activity: item,
      }));
    const final = invocation.finalResponse
      ? [
          {
            anchor: invocation.finalResponse.anchor,
            kind: 'final_response' as const,
            response: invocation.finalResponse,
          },
        ]
      : [];
    const outcome: ProjectedTranscriptContent = {
      anchor: eventAnchor(transcript.sessionId, invocation.id, 'outcome'),
      kind: 'outcome',
      outcome: invocation.outcome,
    };
    return [input, ...activity, ...final, outcome];
  });
}

/** Returns an inclusive excerpt, or an empty array when either anchor is stale or reversed. */
export function selectTranscriptRange(
  transcript: ProjectedTranscript,
  range: TranscriptAnchorRange,
): ProjectedTranscriptContent[] {
  const content = projectedTranscriptContent(transcript);
  const start = content.findIndex((item) => anchorsEqual(item.anchor, range.start));
  const end = content.findIndex((item) => anchorsEqual(item.anchor, range.end));
  return start < 0 || end < start ? [] : content.slice(start, end + 1);
}

/** Anchors the newest projected final agent response without including its input or older turns. */
export function selectLatestFinalAgentResponseRange(
  transcript: ProjectedTranscript,
): TranscriptAnchorRange | null {
  for (let index = transcript.invocations.length - 1; index >= 0; index -= 1) {
    const response = transcript.invocations[index].finalResponse;
    if (response) return { start: response.anchor, end: response.anchor };
  }
  return null;
}

export function anchorsEqual(left: TranscriptAnchor, right: TranscriptAnchor): boolean {
  return (
    left.sessionId === right.sessionId &&
    left.invocationId === right.invocationId &&
    left.kind === right.kind &&
    left.runtimeEventId === right.runtimeEventId
  );
}

function eventAnchor(
  sessionId: string,
  invocationId: string,
  kind: TranscriptAnchorKind,
  runtimeEventId?: string,
): TranscriptAnchor {
  return { sessionId, invocationId, kind, ...(runtimeEventId ? { runtimeEventId } : {}) };
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
    sequence: event.sequence,
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
  if (event.rawPayload && typeof event.rawPayload === 'object') {
    const decoded = (event.rawPayload as Record<string, unknown>).lossyUtf8;
    if (typeof decoded === 'string' && decoded.trim()) {
      return decoded.trim();
    }
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
