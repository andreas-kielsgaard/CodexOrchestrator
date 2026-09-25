import {
  isSessionPreparing,
  type SessionPreparationDto,
} from '../../application/agentSessions/preparation';
import { importedTranscript, type ImportedTranscript } from './importedTranscript';
import type { SessionInteractionDto } from '../../application/agentSessions';
import type {
  AgentDiagnosticDto,
  AgentInvocationDetailsDto,
  AgentInvocationStatusDto,
  AgentRuntimeFailureDto,
  AgentRuntimeEventDto,
  AgentSessionDetailsDto,
  IsoDateTimeDto,
  NormalizedToolActivityDto,
  ToolActivityKindDto,
} from '../../application/agentSessions';
import { runtimeDiagnosticText } from './runtimeDiagnostics';

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
  safeDetail: TranscriptActivitySafeDetail | null;
}

export type TranscriptActivitySafeDetail =
  | {
      kind: 'tool';
      activity: ToolActivityKindDto;
      phase: NormalizedToolActivityDto['phase'];
      itemId: string | null;
      server: string | null;
      tool: string | null;
      status: string | null;
      resultClassification: NormalizedToolActivityDto['resultClassification'];
    }
  | {
      kind: 'usage';
      inputTokens: number | null;
      cachedInputTokens: number | null;
      outputTokens: number | null;
    };

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
  preparing?: boolean;
  imported?: ImportedTranscript;
  interactions?: readonly SessionInteractionDto[];
  id: string;
  submittedText: string;
  inputProvenance: 'user' | 'application';
  status: AgentInvocationStatusDto;
  isActive: boolean;
  createdAt: IsoDateTimeDto;
  startedAt: IsoDateTimeDto | null;
  completedAt: IsoDateTimeDto | null;
  processing: TranscriptActivity[];
  technical: TranscriptActivity[];
  diagnostics: AgentDiagnosticDto[];
  finalResponse: TranscriptFinalResponse | null;
  runtimeFailure: AgentRuntimeFailureDto | null;
  outcome: TranscriptOutcome;
}

export interface ProjectedTranscript {
  sessionId: string;
  invocations: ProjectedInvocation[];
  activeInvocationId: string | null;
  presentationRevision: number;
}

const activeStatuses = new Set<AgentInvocationStatusDto>(['pending', 'running']);
let nextPresentationRevision = 0;

export function projectAgentSessionTranscript(
  details: AgentSessionDetailsDto,
  preparation?: SessionPreparationDto | null,
): ProjectedTranscript {
  const invocations = orderedInvocationEntries(details).map(({ entry, imported }) =>
    projectInvocation(
      details.session.id,
      entry,
      imported,
      preparation,
      interactionsFor(details, entry.invocation.id),
    ),
  );

  return transcript(details.session.id, invocations);
}

interface CachedInvocationProjection {
  readonly entry: AgentInvocationDetailsDto;
  readonly interactions: readonly SessionInteractionDto[] | undefined;
  readonly preparing: boolean;
  readonly projected: ProjectedInvocation;
}

/** Retains unchanged invocation projections and only folds newly appended live events. */
export class AgentSessionTranscriptProjectionCache {
  private sessionId: string | null = null;
  private invocations = new Map<string, CachedInvocationProjection>();

  project(
    details: AgentSessionDetailsDto,
    preparation?: SessionPreparationDto | null,
  ): ProjectedTranscript {
    if (this.sessionId !== details.session.id) {
      this.sessionId = details.session.id;
      this.invocations.clear();
    }

    const nextCache = new Map<string, CachedInvocationProjection>();
    const projected = orderedInvocationEntries(details).map(({ entry, imported }) => {
      const invocationId = entry.invocation.id;
      const interactions = interactionsFor(details, invocationId);
      const preparing =
        preparation?.invocationId === invocationId && isSessionPreparing(preparation);
      const cached = this.invocations.get(invocationId);
      const next = cached
        ? updateInvocationProjection(
            details.session.id,
            cached,
            entry,
            imported,
            preparing,
            interactions,
          )
        : projectInvocation(details.session.id, entry, imported, preparation, interactions);
      nextCache.set(invocationId, { entry, interactions, preparing, projected: next });
      return next;
    });
    this.invocations = nextCache;
    return transcript(details.session.id, projected);
  }
}

function orderedInvocationEntries(details: AgentSessionDetailsDto) {
  return details.invocations
    .map((entry) => ({ entry, imported: importedTranscript(entry) }))
    .sort((left, right) =>
      left.imported && right.imported
        ? left.imported.ordinal - right.imported.ordinal
        : compareOrdered(
            left.entry.invocation.createdAt,
            left.entry.invocation.id,
            right.entry.invocation.createdAt,
            right.entry.invocation.id,
          ),
    );
}

function projectInvocation(
  sessionId: string,
  entry: AgentInvocationDetailsDto,
  imported: ImportedTranscript | undefined,
  preparation: SessionPreparationDto | null | undefined,
  interactions: readonly SessionInteractionDto[] | undefined,
): ProjectedInvocation {
  const { invocation, events } = entry;
  const orderedEvents = [...events].sort(
    (left, right) =>
      left.sequence - right.sequence ||
      compareOrdered(left.recordedAt, left.id, right.recordedAt, right.id),
  );
  const finalEvent = findLastFinalAgentMessage(orderedEvents);
  const processing: TranscriptActivity[] = [];
  const technical: TranscriptActivity[] = [];

  for (const event of orderedEvents) {
    if (event === finalEvent) continue;
    const activity = projectActivity(event);
    if (!activity) continue;
    if (activity.kind === 'technical') technical.push(activity);
    else processing.push(activity);
  }

  return {
    preparing: preparation?.invocationId === invocation.id && isSessionPreparing(preparation),
    imported,
    id: invocation.id,
    interactions,
    submittedText: invocation.submittedText,
    inputProvenance: invocation.inputProvenance,
    status: invocation.status,
    isActive: activeStatuses.has(invocation.status),
    createdAt: invocation.createdAt,
    startedAt: invocation.startedAt,
    completedAt: invocation.completedAt,
    processing: coalesceLifecycleActivities(processing),
    technical,
    diagnostics: sortedDiagnostics(invocation.diagnostics),
    finalResponse: finalEvent?.normalized?.text?.trim()
      ? {
          anchor: eventAnchor(sessionId, invocation.id, 'final_response', finalEvent.id),
          eventId: finalEvent.id,
          text: finalEvent.normalized.text.trim(),
        }
      : null,
    runtimeFailure: invocation.runtimeError,
    outcome: projectOutcome(invocation.status, invocation.runtimeError?.message ?? null),
  };
}

function updateInvocationProjection(
  sessionId: string,
  cached: CachedInvocationProjection,
  entry: AgentInvocationDetailsDto,
  imported: ImportedTranscript | undefined,
  preparing: boolean,
  interactions: readonly SessionInteractionDto[] | undefined,
): ProjectedInvocation {
  const previousEvents = cached.entry.events;
  const events = entry.events;
  const appendOnly =
    previousEvents === events ||
    (previousEvents.length < events.length &&
      (previousEvents.length === 0 ||
        previousEvents.at(-1) === events[previousEvents.length - 1]));
  const appended = appendOnly ? events.slice(previousEvents.length) : [];
  if (
    !appendOnly ||
    imported ||
    cached.projected.imported ||
    Boolean(findLastFinalAgentMessage(appended))
  ) {
    return {
      ...projectInvocation(sessionId, entry, imported, undefined, interactions),
      preparing,
    };
  }

  const unchanged =
    cached.entry === entry &&
    cached.preparing === preparing &&
    sameInteractionList(cached.interactions, interactions);
  if (unchanged) return cached.projected;

  const additions = appended.map(projectActivity).filter((item) => item !== null);
  const processing = coalesceLifecycleActivities([
    ...cached.projected.processing,
    ...additions.filter((item) => item.kind !== 'technical'),
  ]);
  const technical = [
    ...cached.projected.technical,
    ...additions.filter((item) => item.kind === 'technical'),
  ];
  const invocation = entry.invocation;
  return {
    ...cached.projected,
    preparing,
    interactions,
    status: invocation.status,
    isActive: activeStatuses.has(invocation.status),
    startedAt: invocation.startedAt,
    completedAt: invocation.completedAt,
    processing,
    technical,
    diagnostics: sortedDiagnostics(invocation.diagnostics),
    runtimeFailure: invocation.runtimeError,
    outcome: projectOutcome(invocation.status, invocation.runtimeError?.message ?? null),
  };
}

function interactionsFor(details: AgentSessionDetailsDto, invocationId: string) {
  return details.interactions?.filter((item) => item.invocationId === invocationId);
}

function sameInteractionList(
  left: readonly SessionInteractionDto[] | undefined,
  right: readonly SessionInteractionDto[] | undefined,
) {
  return (
    left === right ||
    (left?.length === right?.length && left?.every((item, index) => item === right?.[index]))
  );
}

function sortedDiagnostics(diagnostics: readonly AgentDiagnosticDto[]) {
  return [...diagnostics].sort((left, right) => left.recordedAt.localeCompare(right.recordedAt));
}

function transcript(sessionId: string, invocations: ProjectedInvocation[]): ProjectedTranscript {
  const projected = {
    sessionId,
    invocations,
    activeInvocationId: invocations.find((invocation) => invocation.isActive)?.id ?? null,
  } as ProjectedTranscript;
  Object.defineProperty(projected, 'presentationRevision', {
    value: ++nextPresentationRevision,
    enumerable: false,
  });
  return projected;
}

/**
 * A provider reports one tool item as a started and a completed activity. Preserve both records,
 * but render the pair as one logical activity so a completed operation is not misrepresented as
 * two calls. Pairing uses only the normalized item identity and phase.
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
    if (lifecycle.phase === 'started') {
      active.set(lifecycle.key, projected.length);
      projected.push(activity);
      continue;
    }
    const index = active.get(lifecycle.key);
    if (index !== undefined) {
      const started = projected[index];
      projected[index] = {
        ...started,
        text: activity.text === toolLabelFromActivity(activity) ? started.text : activity.text,
        safeDetail: activity.safeDetail,
        rawPayload: { lifecycleEvents: [started.rawPayload, activity.rawPayload] },
      };
      active.delete(lifecycle.key);
      continue;
    }
    projected.push(activity);
  }
  return projected;
}

function lifecycleIdentity(
  activity: TranscriptActivity,
): { key: string; phase: 'started' | 'completed' } | null {
  const detail = activity.safeDetail;
  if (activity.kind !== 'tool' || detail?.kind !== 'tool' || !detail.itemId) return null;
  if (detail.phase !== 'started' && detail.phase !== 'completed') return null;
  return { key: `${detail.activity}:${detail.itemId}`, phase: detail.phase };
}

const TOOL_ACTIVITY_LABELS: Readonly<Record<ToolActivityKindDto, string>> = {
  command: 'command execution',
  file_change: 'file change',
  web_search: 'web search',
  plan: 'plan update',
  mcp_tool: 'mcp tool call',
  other: 'Tool activity',
};

export function toolActivityLabel(kind: ToolActivityKindDto | undefined): string {
  return kind ? TOOL_ACTIVITY_LABELS[kind] : 'Tool activity';
}

function toolLabelFromActivity(activity: TranscriptActivity): string {
  return activity.safeDetail?.kind === 'tool'
    ? toolActivityLabel(activity.safeDetail.activity)
    : 'Tool activity';
}

function toolSafeDetail(activity: NormalizedToolActivityDto): TranscriptActivitySafeDetail {
  return {
    kind: 'tool',
    activity: activity.kind,
    phase: activity.phase,
    itemId: activity.itemId,
    server: activity.server,
    tool: activity.tool,
    status: activity.status,
    resultClassification: activity.resultClassification,
  };
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

/** Selects one invocation only when both durable identity parts match. */
export function selectTranscriptInvocation(
  transcript: ProjectedTranscript | null,
  sessionId: string,
  invocationId: string,
): ProjectedInvocation | null {
  if (!transcript || transcript.sessionId !== sessionId) return null;
  return transcript.invocations.find((invocation) => invocation.id === invocationId) ?? null;
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
    safeDetail: null,
  };

  if (event.source === 'stderr' || !normalized || kind === 'unknown' || kind === 'runtime_error') {
    return {
      ...base,
      kind: 'technical',
      text: normalized?.text?.trim() || runtimeDiagnosticText(event),
    };
  }

  if (kind === 'processing_started') {
    return null;
  }
  if (kind === 'processing_update') {
    const text = normalized.text?.trim();
    return text ? { ...base, kind: 'processing', text } : null;
  }
  if (kind === 'tool_activity') {
    return {
      ...base,
      kind: 'tool',
      text: normalized.text?.trim() || toolActivityLabel(normalized.toolActivity?.kind),
      safeDetail: normalized.toolActivity ? toolSafeDetail(normalized.toolActivity) : null,
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
    return null;
  }
  if (kind === 'runtime_context_established' || kind === 'invocation_completed') {
    return null;
  }

  return { ...base, kind: 'technical', text: runtimeDiagnosticText(event) };
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
