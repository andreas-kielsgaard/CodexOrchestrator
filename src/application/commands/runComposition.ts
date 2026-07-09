import type { Artifact, Conversation, EntityId, Event, IsoDateTime } from '../../domain/model';
import {
  completeTaskRunLifecycle,
  failTaskRunLifecycle,
  startTaskRunLifecycle,
  type CompletedTaskRunLifecycle,
  type FailedTaskRunLifecycle,
  type StartedTaskRunLifecycle,
  type TaskRunLifecycleRecorder,
} from './taskRunLifecycle';

export interface RunCompositionService {
  readonly recorder: TaskRunLifecycleRecorder;
  readonly runtime: CodexRunRuntime;
}

export interface CodexRunRuntime {
  exec(input: CodexRunRuntimeInput): Promise<CodexRunRuntimeResult>;
}

export interface CodexRunRuntimeInput {
  prompt: string;
  cwd?: string;
  additionalArgs?: readonly string[];
  env?: Record<string, string | undefined>;
  onStdoutChunk?: (chunk: string) => void;
  onStderrChunk?: (chunk: string) => void;
}

export type CodexRunRuntimeStatus = 'completed' | 'failed' | 'error';

export interface CodexRunRuntimeResult {
  command: string;
  args: readonly string[];
  cwd?: string;
  exitCode: number | null;
  signal: string | null;
  status: CodexRunRuntimeStatus;
  statusReason: string;
  stdoutJsonl: string;
  stderr: string;
  summary: CodexRunRuntimeSummary;
}

export interface CodexRunRuntimeSummary {
  threadId?: string;
  finalAgentMessageText?: string;
  terminalStatus?: {
    readonly kind: 'completed' | 'failed' | 'error';
    readonly lineNumber: number;
  };
  tokenUsage?: Record<string, unknown>;
  itemCountsByType: Record<string, number>;
}

export interface ComposeCodexTaskRunInput {
  taskId: EntityId;
  prompt: string;
  cwd?: string;
  worktreeId?: EntityId;
  startedAt?: IsoDateTime;
  completedAt?: IsoDateTime;
  conversationTitle?: string;
  conversationSummary?: string;
  additionalArgs?: readonly string[];
  env?: Record<string, string | undefined>;
  onStdoutChunk?: (chunk: string) => void;
  onStderrChunk?: (chunk: string) => void;
}

export type ComposeCodexTaskRunResult =
  ComposeCodexTaskRunCompletedResult | ComposeCodexTaskRunFailedResult;

export interface ComposeCodexTaskRunCompletedResult {
  status: 'completed';
  started: StartedTaskRunLifecycle;
  rawEventStreamArtifact: Artifact;
  artifactCreatedEvent: Event;
  conversation: Conversation;
  runtimeResult: CodexRunRuntimeResult;
  completed: CompletedTaskRunLifecycle;
}

export interface ComposeCodexTaskRunFailedResult {
  status: 'failed';
  started: StartedTaskRunLifecycle;
  rawEventStreamArtifact?: Artifact;
  artifactCreatedEvent?: Event;
  conversation: Conversation;
  runtimeResult?: CodexRunRuntimeResult;
  failed: FailedTaskRunLifecycle;
  error: string;
}

export async function composeCodexTaskRun(
  service: RunCompositionService,
  input: ComposeCodexTaskRunInput,
): Promise<ComposeCodexTaskRunResult> {
  const started = await startTaskRunLifecycle(service.recorder, {
    taskId: input.taskId,
    ...(input.worktreeId === undefined ? {} : { worktreeId: input.worktreeId }),
    ...(input.startedAt === undefined ? {} : { startedAt: input.startedAt }),
    conversation: {
      title: input.conversationTitle ?? 'Codex run',
      ...(input.conversationSummary === undefined ? {} : { summary: input.conversationSummary }),
    },
  });
  const conversation = requireStartedConversation(started);

  let runtimeResult: CodexRunRuntimeResult;
  try {
    runtimeResult = await service.runtime.exec({
      prompt: input.prompt,
      ...(input.cwd === undefined ? {} : { cwd: input.cwd }),
      ...(input.additionalArgs === undefined ? {} : { additionalArgs: input.additionalArgs }),
      ...(input.env === undefined ? {} : { env: input.env }),
      ...(input.onStdoutChunk === undefined ? {} : { onStdoutChunk: input.onStdoutChunk }),
      ...(input.onStderrChunk === undefined ? {} : { onStderrChunk: input.onStderrChunk }),
    });
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    const failed = await failTaskRunLifecycle(service.recorder, {
      taskId: input.taskId,
      taskRunId: started.taskRun.id,
      ...(input.completedAt === undefined ? {} : { completedAt: input.completedAt }),
      error: errorMessage,
    });

    return {
      status: 'failed',
      started,
      conversation,
      failed,
      error: errorMessage,
    };
  }

  const rawEventStreamArtifact = await service.recorder.artifactStore.createArtifact({
    kind: 'raw_event_stream',
    title: 'Raw Codex JSONL',
    taskId: input.taskId,
    taskRunId: started.taskRun.id,
    conversationId: conversation.id,
    content: runtimeResult.stdoutJsonl,
  });
  const artifactCreatedEvent = await service.recorder.eventStore.appendEvent({
    kind: 'artifact_created',
    projectId: started.task.projectId,
    taskId: input.taskId,
    taskRunId: started.taskRun.id,
    conversationId: conversation.id,
    artifactId: rawEventStreamArtifact.id,
    payload: {
      artifactKind: rawEventStreamArtifact.kind,
      artifactId: rawEventStreamArtifact.id,
      codexStatus: runtimeResult.status,
      stdoutJsonlLength: runtimeResult.stdoutJsonl.length,
      ...(numericExitCode(runtimeResult.exitCode) === undefined
        ? {}
        : { exitCode: numericExitCode(runtimeResult.exitCode) }),
      ...(runtimeResult.signal === null ? {} : { signal: runtimeResult.signal }),
    },
  });

  const updatedConversation = await updateConversationFromRuntimeResult(
    service.recorder,
    conversation,
    runtimeResult,
  );

  if (runtimeResult.status === 'completed') {
    const exitCode = numericExitCode(runtimeResult.exitCode);
    const completed = await completeTaskRunLifecycle(service.recorder, {
      taskId: input.taskId,
      taskRunId: started.taskRun.id,
      ...(input.completedAt === undefined ? {} : { completedAt: input.completedAt }),
      ...(exitCode === undefined ? {} : { exitCode }),
      ...(runtimeResult.summary.finalAgentMessageText === undefined
        ? {}
        : {
            finalResponse: {
              title: 'Final Codex response',
              content: runtimeResult.summary.finalAgentMessageText,
            },
          }),
    });

    return {
      status: 'completed',
      started,
      rawEventStreamArtifact,
      artifactCreatedEvent,
      conversation: updatedConversation,
      runtimeResult,
      completed,
    };
  }

  const error = codexFailureReason(runtimeResult);
  const exitCode = numericExitCode(runtimeResult.exitCode);
  const failed = await failTaskRunLifecycle(service.recorder, {
    taskId: input.taskId,
    taskRunId: started.taskRun.id,
    ...(input.completedAt === undefined ? {} : { completedAt: input.completedAt }),
    ...(exitCode === undefined ? {} : { exitCode }),
    error,
  });

  return {
    status: 'failed',
    started,
    rawEventStreamArtifact,
    artifactCreatedEvent,
    conversation: updatedConversation,
    runtimeResult,
    failed,
    error,
  };
}

function requireStartedConversation(started: StartedTaskRunLifecycle): Conversation {
  if (started.conversation === undefined) {
    throw new Error('Run composition expected lifecycle start to create a conversation');
  }

  return started.conversation;
}

async function updateConversationFromRuntimeResult(
  recorder: TaskRunLifecycleRecorder,
  conversation: Conversation,
  runtimeResult: CodexRunRuntimeResult,
): Promise<Conversation> {
  return recorder.conversationStore.updateConversation(conversation.id, {
    ...(runtimeResult.summary.threadId === undefined
      ? {}
      : { externalThreadId: runtimeResult.summary.threadId }),
    summary: summarizeConversation(runtimeResult),
  });
}

function summarizeConversation(runtimeResult: CodexRunRuntimeResult): string {
  const prefix =
    runtimeResult.status === 'completed'
      ? 'Codex completed'
      : `Codex ${runtimeResult.status}: ${runtimeResult.statusReason}`;
  const finalMessage = runtimeResult.summary.finalAgentMessageText;

  if (finalMessage === undefined || finalMessage.trim() === '') {
    return prefix;
  }

  return truncate(`${prefix}: ${finalMessage.trim()}`, 240);
}

function codexFailureReason(runtimeResult: CodexRunRuntimeResult): string {
  const stderr = runtimeResult.stderr.trim();

  if (stderr === '') {
    return runtimeResult.statusReason;
  }

  return truncate(`${runtimeResult.statusReason}: ${stderr}`, 500);
}

function numericExitCode(exitCode: number | null): number | undefined {
  return typeof exitCode === 'number' && Number.isFinite(exitCode) ? exitCode : undefined;
}

function truncate(value: string, maxLength: number): string {
  return value.length <= maxLength ? value : `${value.slice(0, maxLength - 3)}...`;
}