import type { AttentionState, EntityId, ExecutionState, IsoDateTime } from '../domain/model';

export interface RuntimeCommandClient {
  startCodexTaskRun(input: StartCodexTaskRunCommandInput): Promise<StartCodexTaskRunCommandResult>;
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
  task: StartCodexTaskRunTaskState;
  taskRun: StartCodexTaskRunTaskRunState;
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
