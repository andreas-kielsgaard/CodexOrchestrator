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
