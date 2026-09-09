import type { HarnessVersionRef } from '../harnesses';
import type { AssignedAgentIdentity } from '../identities';

export type AgentSessionIdDto = string;
export type AgentInvocationIdDto = string;
export type AgentRuntimeEventIdDto = string;
export type ExternalRuntimeContextIdDto = string;
export type IsoDateTimeDto = string;

export type AgentSessionAvailabilityDto = 'available' | 'archived';
export type RuntimeSandboxModeDto = 'read_only' | 'workspace_write' | 'danger_full_access';
export type AgentIdentityShape = 'circle' | 'square' | 'hexagon';

/** Session-owned presentation identity. Assignment and persistence belong outside the view. */
export interface AgentIdentity {
  readonly name: string;
  readonly harnessRole: string;
  readonly visualIdentityToken: string;
  readonly visualIdentityAccent?: string;
  readonly visualIdentityShape?: AgentIdentityShape;
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
  workspaceOrigin?: 'explicit' | 'allocated' | 'native_metadata' | null;
  id: AgentSessionIdDto;
  title: string;
  availability: AgentSessionAvailabilityDto;
  runtimeBinding: AgentRuntimeBindingDto;
  workingDirectory: string | null;
  requestedOptions: AgentRuntimeOptionsDto;
  /** Present on canonical Session reads; optional only for recorded/legacy clients. */
  harnessVersion?: HarnessVersionRef | null;
  /** Present on canonical Session reads; optional only for recorded/legacy clients. */
  assignedIdentity?: AssignedAgentIdentity | null;
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
    status: AgentInvocationStatusDto;
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
  interactions?: SessionInteractionDto[];
  session: AgentSessionDto;
  invocations: AgentInvocationDetailsDto[];
}

export interface AgentSessionSummaryDto {
  pendingRequestCount: number;
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
  harnessVersion?: HarnessVersionRef | null;
  assignedIdentity?: AssignedAgentIdentity | null;
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
  | { kind: 'steering_accepted'; sessionId: string; invocationId: string; inputId: string }
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
  resolveWorkingDirectory?(sessionId: string, directory: string): Promise<void>;
  steerSession?(input: {
    sessionId: string;
    invocationId: string;
    inputId: string;
    text: string;
  }): Promise<SessionInteractionDto>;
  respondToRuntimeRequest?(input: {
    sessionId: string;
    invocationId: string;
    requestId: string;
    response: unknown;
  }): Promise<void>;

  createSession(command: CreateAgentSessionCommandDto): Promise<AgentSessionDto>;
  updateHarness?(command: UpdateAgentSessionHarnessCommandDto): Promise<AgentSessionDto>;
  updateIdentity?(command: UpdateAgentSessionIdentityCommandDto): Promise<AgentSessionDto>;
  updateModelOverride?(
    command: UpdateAgentSessionModelOverrideCommandDto,
  ): Promise<AgentSessionDto>;
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

export interface UpdateAgentSessionHarnessCommandDto {
  readonly sessionId: AgentSessionIdDto;
  readonly harnessVersion: HarnessVersionRef | null;
}

export interface UpdateAgentSessionIdentityCommandDto {
  readonly sessionId: AgentSessionIdDto;
  readonly assignedIdentity: AssignedAgentIdentity | null;
}

export interface UpdateAgentSessionModelOverrideCommandDto {
  readonly sessionId: AgentSessionIdDto;
  readonly model: string | null;
}

export interface SessionInteractionDto {
  id: string;
  invocationId: string;
  sequence: number;
  kind: 'steering' | 'request';
  state:
    | 'pending'
    | 'accepted'
    | 'rejected'
    | 'uncertain'
    | 'responding'
    | 'answered'
    | 'expired'
    | 'unsupported';
  content: {
    permissions?: unknown;
    grantRoot?: string;
    text?: string;
    title?: string;
    command?: string;
    cwd?: string;
    url?: string;
    supported?: boolean;
    kind?: string;
    choices?: Array<{
      label: string;
      description?: string;
      scope?: string | null;
      response: unknown;
    }>;
    questions?: Array<{
      id: string;
      question: string;
      isSecret?: boolean;
      isOther?: boolean;
      options?: Array<{ label: string; description: string }>;
    }>;
  };
  result: string | null;
}
