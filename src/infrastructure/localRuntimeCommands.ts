import {
  composeCodexTaskRun,
  type ComposeCodexTaskRunInput,
  type ComposeCodexTaskRunResult,
} from '../application/commands/runComposition';
import {
  composeCodexTaskRunWithPostRunCapture,
  type ComposeCodexTaskRunWithPostRunCaptureInput,
  type PostRunCaptureResult,
} from '../application/commands/postRunCaptureComposition';
import type {
  RuntimeCommandClient,
  StartCodexTaskRunCommandInput,
  StartCodexTaskRunPostRunCaptureResult,
  StartCodexTaskRunCommandResult,
  StartCodexTaskRunTaskRunState,
  StartCodexTaskRunTaskState,
} from '../application/commands/runtimeCommandClient';
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
  if (input.postRunCapture !== undefined) {
    const result = await composeCodexTaskRunWithPostRunCapture(
      composition.services.postRunCaptureCompositionService,
      toComposeCodexTaskRunWithPostRunCaptureInput(input, options),
    );

    return {
      ...toStartCodexTaskRunCommandResult(input.taskId, result.run),
      postRunCapture: toStartCodexTaskRunPostRunCaptureResult(result.postRunCapture),
    };
  }

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

function toComposeCodexTaskRunWithPostRunCaptureInput(
  input: StartCodexTaskRunCommandInput,
  options: LocalRuntimeCommandHandlerOptions,
): ComposeCodexTaskRunWithPostRunCaptureInput {
  return {
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
    postRunCapture: {
      ...(input.postRunCapture?.collectDiff === true ? { diff: {} } : {}),
      ...(input.postRunCapture?.validationCommand === undefined
        ? {}
        : {
            validation: {
              command: input.postRunCapture.validationCommand.command,
              ...(input.postRunCapture.validationCommand.args === undefined
                ? {}
                : { args: input.postRunCapture.validationCommand.args }),
              ...(input.postRunCapture.validationCommand.cwd === undefined
                ? {}
                : { cwd: input.postRunCapture.validationCommand.cwd }),
              ...(input.postRunCapture.validationCommand.env === undefined
                ? {}
                : { env: input.postRunCapture.validationCommand.env }),
            },
          }),
    },
  };
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

function toStartCodexTaskRunPostRunCaptureResult(
  result: PostRunCaptureResult,
): StartCodexTaskRunPostRunCaptureResult {
  return {
    ...(result.skippedReason === undefined ? {} : { skippedReason: result.skippedReason }),
    ...(result.diff === undefined
      ? {}
      : {
          diff:
            result.diff.status === 'captured'
              ? {
                  status: 'captured',
                  artifactId: result.diff.result.artifact.id,
                  eventId: result.diff.result.event.id,
                  diffLength: result.diff.result.diff.length,
                  isEmptyDiff: result.diff.result.isEmptyDiff,
                  worktreePath: result.diff.result.worktreePath,
                }
              : {
                  status: 'failed',
                  error: result.diff.error,
                },
        }),
    ...(result.validation === undefined
      ? {}
      : { validation: toStartCodexTaskRunValidationCaptureResult(result.validation) }),
  };
}

function toStartCodexTaskRunValidationCaptureResult(
  result: PostRunCaptureResult['validation'],
): NonNullable<StartCodexTaskRunPostRunCaptureResult['validation']> {
  if (result === undefined) {
    return { status: 'failed', error: 'Validation capture result is missing.' };
  }

  const validationResult = result.result;

  if (validationResult === undefined) {
    return {
      status: 'failed',
      ...(result.status === 'failed' && result.error !== undefined ? { error: result.error } : {}),
    };
  }

  const validationError =
    result.status === 'failed'
      ? (result.error ??
        (validationResult.status === 'failed' ? validationResult.error : undefined))
      : undefined;

  return {
    status: validationResult.status,
    validationRunId: validationResult.validationRun.id,
    outputArtifactId: validationResult.outputArtifact.id,
    startedEventId: validationResult.startedEvent.id,
    artifactCreatedEventId: validationResult.artifactCreatedEvent.id,
    completedEventId: validationResult.completedEvent.id,
    ...(validationResult.runtimeResult?.exitCode === undefined ||
    validationResult.runtimeResult.exitCode === null
      ? {}
      : { exitCode: validationResult.runtimeResult.exitCode }),
    ...(validationResult.runtimeResult?.signal === undefined ||
    validationResult.runtimeResult.signal === null
      ? {}
      : { signal: validationResult.runtimeResult.signal }),
    ...(validationError === undefined ? {} : { error: validationError }),
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
