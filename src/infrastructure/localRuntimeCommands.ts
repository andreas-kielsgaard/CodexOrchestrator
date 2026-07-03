import {
  composeCodexTaskRun,
  type ComposeCodexTaskRunInput,
  type ComposeCodexTaskRunResult,
} from '../application/runComposition';
import type {
  RuntimeCommandClient,
  StartCodexTaskRunCommandInput,
  StartCodexTaskRunCommandResult,
  StartCodexTaskRunTaskRunState,
  StartCodexTaskRunTaskState,
} from '../application/runtimeCommandClient';
import type { EntityId, Task, TaskRun } from '../domain/model';
import type { LocalRuntimeServiceComposition } from './localRuntimeComposition';

export interface LocalRuntimeCommandHandlerOptions {
  startedAt?: ComposeCodexTaskRunInput['startedAt'];
  completedAt?: ComposeCodexTaskRunInput['completedAt'];
}

export function createLocalRuntimeCommandHandler(
  composition: LocalRuntimeServiceComposition,
  options: LocalRuntimeCommandHandlerOptions = {},
): RuntimeCommandClient {
  return {
    startCodexTaskRun(input: StartCodexTaskRunCommandInput) {
      return startCodexTaskRun(composition, input, options);
    },
  };
}

export async function startCodexTaskRun(
  composition: LocalRuntimeServiceComposition,
  input: StartCodexTaskRunCommandInput,
  options: LocalRuntimeCommandHandlerOptions = {},
): Promise<StartCodexTaskRunCommandResult> {
  const result = await composeCodexTaskRun(composition.services.runCompositionService, {
    taskId: input.taskId,
    prompt: input.prompt,
    ...(input.cwd === undefined ? {} : { cwd: input.cwd }),
    ...(input.worktreeId === undefined ? {} : { worktreeId: input.worktreeId }),
    ...(input.conversationTitle === undefined
      ? {}
      : { conversationTitle: input.conversationTitle }),
    ...(input.conversationSummary === undefined
      ? {}
      : { conversationSummary: input.conversationSummary }),
    ...(input.additionalArgs === undefined ? {} : { additionalArgs: input.additionalArgs }),
    ...(input.env === undefined ? {} : { env: input.env }),
    ...(options.startedAt === undefined ? {} : { startedAt: options.startedAt }),
    ...(options.completedAt === undefined ? {} : { completedAt: options.completedAt }),
  });

  return toStartCodexTaskRunCommandResult(input.taskId, result);
}

export function toStartCodexTaskRunCommandResult(
  taskId: EntityId,
  result: ComposeCodexTaskRunResult,
): StartCodexTaskRunCommandResult {
  if (result.status === 'completed') {
    return {
      status: 'completed',
      taskId,
      taskRunId: result.completed.taskRun.id,
      conversationId: result.conversation.id,
      rawEventStreamArtifactId: result.rawEventStreamArtifact.id,
      ...(result.completed.artifact === undefined
        ? {}
        : { finalResponseArtifactId: result.completed.artifact.id }),
      ...(result.runtimeResult.exitCode === null
        ? {}
        : { exitCode: result.runtimeResult.exitCode }),
      statusReason: result.runtimeResult.statusReason,
      task: toTaskState(result.completed.task),
      taskRun: toTaskRunState(result.completed.taskRun),
    };
  }

  return {
    status: 'failed',
    taskId,
    taskRunId: result.failed.taskRun.id,
    conversationId: result.conversation.id,
    ...(result.rawEventStreamArtifact === undefined
      ? {}
      : { rawEventStreamArtifactId: result.rawEventStreamArtifact.id }),
    ...(result.runtimeResult?.exitCode === undefined || result.runtimeResult.exitCode === null
      ? {}
      : { exitCode: result.runtimeResult.exitCode }),
    ...(result.runtimeResult === undefined
      ? {}
      : { statusReason: result.runtimeResult.statusReason }),
    error: result.error,
    task: toTaskState(result.failed.task),
    taskRun: toTaskRunState(result.failed.taskRun),
  };
}

function toTaskState(task: Task): StartCodexTaskRunTaskState {
  return {
    id: task.id,
    executionState: task.executionState,
    attentionState: task.attentionState,
    conversationIds: [...task.conversationIds],
    ...(task.repoId === undefined ? {} : { repoId: task.repoId }),
    ...(task.branchId === undefined ? {} : { branchId: task.branchId }),
    ...(task.worktreeId === undefined ? {} : { worktreeId: task.worktreeId }),
    updatedAt: task.updatedAt,
  };
}

function toTaskRunState(taskRun: TaskRun): StartCodexTaskRunTaskRunState {
  return {
    id: taskRun.id,
    executionState: taskRun.executionState,
    ...(taskRun.conversationId === undefined ? {} : { conversationId: taskRun.conversationId }),
    ...(taskRun.worktreeId === undefined ? {} : { worktreeId: taskRun.worktreeId }),
    ...(taskRun.startedAt === undefined ? {} : { startedAt: taskRun.startedAt }),
    ...(taskRun.completedAt === undefined ? {} : { completedAt: taskRun.completedAt }),
    ...(taskRun.exitCode === undefined ? {} : { exitCode: taskRun.exitCode }),
    updatedAt: taskRun.updatedAt,
  };
}
