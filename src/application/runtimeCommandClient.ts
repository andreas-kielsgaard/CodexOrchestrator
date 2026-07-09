import type { AttentionState, EntityId, ExecutionState, IsoDateTime } from '../domain/model';

export interface RuntimeCommandClient {
  startCodexTaskRun(input: StartCodexTaskRunCommandInput): Promise<StartCodexTaskRunCommandResult>;
  startAgentSession(
    input: StartAgentSessionCommandInput,
    options?: StartAgentSessionCommandOptions,
  ): Promise<StartAgentSessionCommandResult>;
  loadAgentSession?(sessionId: EntityId): Promise<StartAgentSessionCommandResult | null>;
}

export interface StartAgentSessionCommandInput {
  streamId?: EntityId;
  sessionId?: EntityId;
  prompt: string;
  cwd?: string;
  additionalArgs?: readonly string[];
  env?: Record<string, string | undefined>;
}

export interface StartAgentSessionCommandOptions {
  onOutput?(chunk: StartAgentSessionOutputChunk): void;
}

export interface StartAgentSessionOutputChunk {
  stream: 'stdout' | 'stderr' | 'system';
  content: string;
}

export type StartAgentSessionCommandStatus = 'completed' | 'failed';

export interface StartAgentSessionCommandResult {
  sessionId: EntityId;
  status: StartAgentSessionCommandStatus;
  command: string;
  args: string[];
  stdout: string;
  stderr: string;
  outputWasStreamed?: boolean;
  startedAt: IsoDateTime;
  completedAt: IsoDateTime;
  exitCode?: number;
  signal?: string;
  error?: string;
}

export interface StartCodexTaskRunCommandInput {
  taskId: EntityId;
  prompt: string;
  cwd?: string;
  worktreeId?: EntityId;
  conversationTitle?: string;
  conversationSummary?: string;
  additionalArgs?: readonly string[];
  env?: Record<string, string | undefined>;
  postRunCapture?: StartCodexTaskRunPostRunCaptureInput;
}

export interface StartCodexTaskRunPostRunCaptureInput {
  collectDiff?: boolean;
  validationCommand?: StartCodexTaskRunValidationCommandInput;
}

export interface StartCodexTaskRunValidationCommandInput {
  command: string;
  args?: readonly string[];
  cwd?: string;
  env?: Record<string, string | undefined>;
}

export type StartCodexTaskRunCommandStatus = 'completed' | 'failed';

export interface StartCodexTaskRunCommandResult {
  status: StartCodexTaskRunCommandStatus;
  taskId: EntityId;
  taskRunId: EntityId;
  conversationId?: EntityId;
  rawEventStreamArtifactId?: EntityId;
  finalResponseArtifactId?: EntityId;
  exitCode?: number;
  statusReason?: string;
  error?: string;
  postRunCapture?: StartCodexTaskRunPostRunCaptureResult;
  task: StartCodexTaskRunTaskState;
  taskRun: StartCodexTaskRunTaskRunState;
}

export interface StartCodexTaskRunPostRunCaptureResult {
  diff?: StartCodexTaskRunDiffCaptureResult;
  validation?: StartCodexTaskRunValidationCaptureResult;
  skippedReason?: 'run_failed';
}

export type StartCodexTaskRunDiffCaptureResult =
  | {
      status: 'captured';
      artifactId: EntityId;
      eventId: EntityId;
      diffLength: number;
      isEmptyDiff: boolean;
      worktreePath: string;
    }
  | {
      status: 'failed';
      error: string;
    };

export interface StartCodexTaskRunValidationCaptureResult {
  status: 'passed' | 'failed';
  validationRunId?: EntityId;
  outputArtifactId?: EntityId;
  startedEventId?: EntityId;
  artifactCreatedEventId?: EntityId;
  completedEventId?: EntityId;
  exitCode?: number;
  signal?: string;
  error?: string;
}

export interface StartCodexTaskRunTaskState {
  id: EntityId;
  executionState: ExecutionState;
  attentionState: AttentionState;
  conversationIds: EntityId[];
  repoId?: EntityId;
  branchId?: EntityId;
  worktreeId?: EntityId;
  updatedAt: IsoDateTime;
}

export interface StartCodexTaskRunTaskRunState {
  id: EntityId;
  executionState: ExecutionState;
  conversationId?: EntityId;
  worktreeId?: EntityId;
  startedAt?: IsoDateTime;
  completedAt?: IsoDateTime;
  exitCode?: number;
  updatedAt: IsoDateTime;
}
