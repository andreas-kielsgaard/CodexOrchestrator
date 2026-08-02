export type AgentSessionIdDto = string;
export type AgentInvocationIdDto = string;
export type AgentRuntimeEventIdDto = string;
export type ExternalRuntimeContextIdDto = string;
export type IsoDateTimeDto = string;

export type AgentSessionAvailabilityDto = 'available' | 'archived';
export type RuntimeSandboxModeDto = 'read_only' | 'workspace_write' | 'danger_full_access';

/** Session-owned presentation identity. Assignment and persistence belong outside the view. */
export interface AgentIdentity {
  readonly name: string;
  readonly harnessRole: string;
  readonly visualIdentityToken: string;
}

export interface AgentRuntimeOptionsDto {
  model: string | null;
  sandbox: RuntimeSandboxModeDto | null;
}

export interface AgentRuntimeBindingDto {
  externalContextId: ExternalRuntimeContextIdDto | null;
  runtimeVersion: string | null;
}

export interface AgentSessionDto {
  id: AgentSessionIdDto;
  title: string;
  availability: AgentSessionAvailabilityDto;
  runtimeBinding: AgentRuntimeBindingDto;
  workingDirectory: string | null;
  requestedOptions: AgentRuntimeOptionsDto;
  createdAt: IsoDateTimeDto;
  updatedAt: IsoDateTimeDto;
}

export type AgentInvocationStatusDto =
  'pending' | 'running' | 'completed' | 'failed' | 'canceled' | 'interrupted';
export type AgentInvocationTerminalStatusDto = Extract<
  AgentInvocationStatusDto,
  'completed' | 'failed' | 'canceled' | 'interrupted'
>;
export type AgentInvocationInputProvenanceDto = 'user' | 'application';

export interface AgentRuntimeFailureDto {
  code: string;
  message: string;
  details: unknown | null;
}

export type AgentDiagnosticSourceDto = 'repository' | 'runtime' | 'transport';
export type AgentDiagnosticSeverityDto = 'warning' | 'error';

export interface AgentDiagnosticDto {
  source: AgentDiagnosticSourceDto;
  severity: AgentDiagnosticSeverityDto;
  code: string;
  message: string;
  details: unknown | null;
  recordedAt: IsoDateTimeDto;
}

export interface AgentInvocationDto {
  id: AgentInvocationIdDto;
  sessionId: AgentSessionIdDto;
  submittedText: string;
  inputProvenance: AgentInvocationInputProvenanceDto;
  status: AgentInvocationStatusDto;
  requestedOptions: AgentRuntimeOptionsDto;
  effectiveOptions: AgentRuntimeOptionsDto | null;
  startedAt: IsoDateTimeDto | null;
  completedAt: IsoDateTimeDto | null;
  exitCode: number | null;
  signal: string | null;
  runtimeError: AgentRuntimeFailureDto | null;
  diagnostics: AgentDiagnosticDto[];
  createdAt: IsoDateTimeDto;
  updatedAt: IsoDateTimeDto;
}

export type AgentRuntimeEventSourceDto = 'stdout' | 'stderr' | 'runtime';

export type NormalizedRuntimeEventKindDto =
  | 'runtime_context_established'
  | 'processing_started'
  | 'processing_update'
  | 'tool_activity'
  | 'agent_message'
  | 'usage'
  | 'invocation_completed'
  | 'runtime_error'
  | 'unknown';

export interface AgentRuntimeUsageDto {
  inputTokens: number | null;
  cachedInputTokens: number | null;
  outputTokens: number | null;
}

export interface NormalizedRuntimeEventDto {
  kind: NormalizedRuntimeEventKindDto;
  text: string | null;
  externalContextId: ExternalRuntimeContextIdDto | null;
  usage: AgentRuntimeUsageDto | null;
  details: unknown | null;
  toolActivity: NormalizedToolActivityDto | null;
}

export type ToolActivityPhaseDto = 'started' | 'completed' | 'unknown';
export type ToolResultClassificationDto = 'succeeded' | 'failed' | 'unknown';

export interface NormalizedToolActivityDto {
  phase: ToolActivityPhaseDto;
  itemId: string | null;
  server: string | null;
  tool: string | null;
  status: string | null;
  resultClassification: ToolResultClassificationDto;
}

export interface RuntimeObservationCorrelationDto {
  eventId: AgentRuntimeEventIdDto;
  sequence: number;
  recordedAt: IsoDateTimeDto;
}

export interface AgentInvocationObservationDto {
  launchAcceptedAt: IsoDateTimeDto | null;
  externalContext: {
    externalContextId: ExternalRuntimeContextIdDto;
    correlation: RuntimeObservationCorrelationDto;
  } | null;
  providerActivity: RuntimeObservationCorrelationDto | null;
  providerTerminal: {
    status: 'completed' | 'failed' | 'error';
    correlation: RuntimeObservationCorrelationDto;
  } | null;
  processTerminal: {
    status: AgentInvocationTerminalStatusDto;
    completedAt: IsoDateTimeDto;
    exitCode: number | null;
    signal: string | null;
  } | null;
  mcpToolActivities: Array<{
    activity: NormalizedToolActivityDto;
    correlation: RuntimeObservationCorrelationDto;
  }>;
  mcpToolActivityPartial: boolean;
}

/** Reject malformed native observation data before any product projection can use it. */
export function decodeAgentInvocationObservation(value: unknown): AgentInvocationObservationDto {
  const item = record(value, 'Agent invocation observation');
  exact(
    item,
    [
      'launchAcceptedAt',
      'externalContext',
      'providerActivity',
      'providerTerminal',
      'processTerminal',
      'mcpToolActivities',
      'mcpToolActivityPartial',
    ],
    'Agent invocation observation',
  );
  return {
    launchAcceptedAt: nullableText(item.launchAcceptedAt, 'launchAcceptedAt'),
    externalContext: externalContext(item.externalContext),
    providerActivity: correlation(item.providerActivity, 'providerActivity'),
    providerTerminal: providerTerminal(item.providerTerminal),
    processTerminal: processTerminal(item.processTerminal),
    mcpToolActivities: array(item.mcpToolActivities, 'mcpToolActivities').map((entry, index) =>
      mcpToolActivity(entry, `mcpToolActivities[${index}]`),
    ),
    mcpToolActivityPartial: boolean(item.mcpToolActivityPartial, 'mcpToolActivityPartial'),
  };
}

function externalContext(value: unknown): AgentInvocationObservationDto['externalContext'] {
  if (value === null) return null;
  const item = record(value, 'externalContext');
  exact(item, ['externalContextId', 'correlation'], 'externalContext');
  return {
    externalContextId: text(item.externalContextId, 'externalContextId'),
    correlation: correlationRequired(item.correlation, 'externalContext.correlation'),
  };
}

function providerTerminal(value: unknown): AgentInvocationObservationDto['providerTerminal'] {
  if (value === null) return null;
  const item = record(value, 'providerTerminal');
  exact(item, ['status', 'correlation'], 'providerTerminal');
  if (item.status !== 'completed' && item.status !== 'failed' && item.status !== 'error')
    throw new Error('providerTerminal.status is invalid');
  return {
    status: item.status,
    correlation: correlationRequired(item.correlation, 'providerTerminal.correlation'),
  };
}

function processTerminal(value: unknown): AgentInvocationObservationDto['processTerminal'] {
  if (value === null) return null;
  const item = record(value, 'processTerminal');
  exact(item, ['status', 'completedAt', 'exitCode', 'signal'], 'processTerminal');
  if (!['completed', 'failed', 'canceled', 'interrupted'].includes(String(item.status)))
    throw new Error('processTerminal.status is invalid');
  return {
    status: item.status as AgentInvocationTerminalStatusDto,
    completedAt: text(item.completedAt, 'processTerminal.completedAt'),
    exitCode: nullableSignedI32(item.exitCode, 'processTerminal.exitCode'),
    signal: nullableText(item.signal, 'processTerminal.signal'),
  };
}

function mcpToolActivity(
  value: unknown,
  label: string,
): AgentInvocationObservationDto['mcpToolActivities'][number] {
  const item = record(value, label);
  exact(item, ['activity', 'correlation'], label);
  const activity = record(item.activity, `${label}.activity`);
  exact(
    activity,
    ['phase', 'itemId', 'server', 'tool', 'status', 'resultClassification'],
    `${label}.activity`,
  );
  if (!['started', 'completed', 'unknown'].includes(String(activity.phase)))
    throw new Error(`${label}.activity.phase is invalid`);
  if (!['succeeded', 'failed', 'unknown'].includes(String(activity.resultClassification)))
    throw new Error(`${label}.activity.resultClassification is invalid`);
  return {
    activity: {
      phase: activity.phase as ToolActivityPhaseDto,
      itemId: nullableText(activity.itemId, `${label}.activity.itemId`),
      server: nullableText(activity.server, `${label}.activity.server`),
      tool: nullableText(activity.tool, `${label}.activity.tool`),
      status: nullableText(activity.status, `${label}.activity.status`),
      resultClassification: activity.resultClassification as ToolResultClassificationDto,
    },
    correlation: correlationRequired(item.correlation, `${label}.correlation`),
  };
}

function correlation(value: unknown, label: string): RuntimeObservationCorrelationDto | null {
  return value === null ? null : correlationRequired(value, label);
}
function correlationRequired(value: unknown, label: string): RuntimeObservationCorrelationDto {
  const item = record(value, label);
  exact(item, ['eventId', 'sequence', 'recordedAt'], label);
  return {
    eventId: text(item.eventId, `${label}.eventId`),
    sequence: integer(item.sequence, `${label}.sequence`),
    recordedAt: text(item.recordedAt, `${label}.recordedAt`),
  };
}
function record(value: unknown, label: string): Record<string, unknown> {
  if (!value || typeof value !== 'object' || Array.isArray(value))
    throw new Error(`${label} is invalid`);
  return value as Record<string, unknown>;
}
function exact(value: Record<string, unknown>, keys: readonly string[], label: string) {
  if (Object.keys(value).some((key) => !keys.includes(key)) || keys.some((key) => !(key in value)))
    throw new Error(`${label} has an invalid shape`);
}
function text(value: unknown, label: string): string {
  if (typeof value !== 'string' || !value.trim()) throw new Error(`${label} is invalid`);
  return value;
}
function nullableText(value: unknown, label: string): string | null {
  return value === null ? null : text(value, label);
}
function integer(value: unknown, label: string): number {
  if (!Number.isSafeInteger(value) || (value as number) < 0) throw new Error(`${label} is invalid`);
  return value as number;
}
function nullableSignedI32(value: unknown, label: string): number | null {
  if (value === null) return null;
  if (
    !Number.isSafeInteger(value) ||
    (value as number) < -2147483648 ||
    (value as number) > 2147483647
  )
    throw new Error(`${label} is invalid`);
  return value as number;
}
function boolean(value: unknown, label: string): boolean {
  if (typeof value !== 'boolean') throw new Error(`${label} is invalid`);
  return value;
}
function array(value: unknown, label: string): unknown[] {
  if (!Array.isArray(value)) throw new Error(`${label} is invalid`);
  return value;
}

export interface AgentRuntimeEventDto {
  id: AgentRuntimeEventIdDto;
  invocationId: AgentInvocationIdDto;
  sequence: number;
  source: AgentRuntimeEventSourceDto;
  rawPayload: unknown;
  normalized: NormalizedRuntimeEventDto | null;
  recordedAt: IsoDateTimeDto;
}

export interface AgentInvocationDetailsDto {
  invocation: AgentInvocationDto;
  observation: AgentInvocationObservationDto;
  events: AgentRuntimeEventDto[];
}

export interface AgentSessionDetailsDto {
  session: AgentSessionDto;
  invocations: AgentInvocationDetailsDto[];
}

export interface AgentSessionSummaryDto {
  id: AgentSessionIdDto;
  title: string;
  availability: AgentSessionAvailabilityDto;
  hasActiveInvocation: boolean;
  latestInvocationStatus: AgentInvocationStatusDto | null;
  createdAt: IsoDateTimeDto;
  updatedAt: IsoDateTimeDto;
}

export interface CreateAgentSessionCommandDto {
  title?: string;
  workingDirectory?: string;
  requestedOptions?: PartialAgentRuntimeOptionsDto;
}

export interface SendAgentSessionMessageCommandDto {
  sessionId?: AgentSessionIdDto;
  submittedText: string;
  title?: string;
  workingDirectory?: string;
  requestedOptions?: PartialAgentRuntimeOptionsDto;
}

export interface CancelAgentInvocationCommandDto {
  invocationId: AgentInvocationIdDto;
}

export interface ListAgentSessionsQueryDto {
  availability?: AgentSessionAvailabilityDto;
  limit?: number;
}

export interface LoadAgentSessionQueryDto {
  sessionId: AgentSessionIdDto;
}

export interface PartialAgentRuntimeOptionsDto {
  model?: string;
  sandbox?: RuntimeSandboxModeDto;
}

export interface SendAgentSessionMessageResultDto {
  sessionId: AgentSessionIdDto;
  invocationId: AgentInvocationIdDto;
}

export type AgentSessionUpdateDto =
  | {
      kind: 'event_persisted';
      sessionId: AgentSessionIdDto;
      invocationId: AgentInvocationIdDto;
      event: AgentRuntimeEventDto;
    }
  | {
      kind: 'invocation_terminal';
      sessionId: AgentSessionIdDto;
      invocationId: AgentInvocationIdDto;
      invocation: AgentInvocationDto;
    }
  | {
      kind: 'diagnostic_recorded';
      sessionId: AgentSessionIdDto;
      invocationId: AgentInvocationIdDto;
      invocation: AgentInvocationDto;
    };

export type AgentSessionUpdateListener = (update: AgentSessionUpdateDto) => void;

export interface AgentSessionClient {
  createSession(command: CreateAgentSessionCommandDto): Promise<AgentSessionDto>;
  listSessions(query?: ListAgentSessionsQueryDto): Promise<AgentSessionSummaryDto[]>;
  loadSession(query: LoadAgentSessionQueryDto): Promise<AgentSessionDetailsDto>;
  reloadSession(query: LoadAgentSessionQueryDto): Promise<AgentSessionDetailsDto>;
  subscribeUpdates(listener: AgentSessionUpdateListener): Promise<() => void>;
  sendMessage(
    command: SendAgentSessionMessageCommandDto,
  ): Promise<SendAgentSessionMessageResultDto>;
  cancelInvocation(command: CancelAgentInvocationCommandDto): Promise<AgentInvocationDto>;
  disconnectUpdates(): Promise<void>;
}
