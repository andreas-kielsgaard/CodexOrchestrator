import {
  collectTaskDiff,
  type CollectTaskDiffInput,
  type CollectTaskDiffResult,
  type DiffCollectionService,
} from './diffCollection';
import {
  composeCodexTaskRun,
  type ComposeCodexTaskRunInput,
  type ComposeCodexTaskRunResult,
  type RunCompositionService,
} from './runComposition';
import {
  runTaskValidationCommand,
  type RunTaskValidationCommandInput,
  type RunTaskValidationCommandResult,
  type ValidationCommandRunnerService,
} from './validationCommandRunner';
import type { EntityId, IsoDateTime } from '../domain/model';

export interface PostRunCaptureCompositionService {
  readonly runCompositionService: RunCompositionService;
  readonly diffCollectionService: DiffCollectionService;
  readonly validationCommandRunnerService: ValidationCommandRunnerService;
}

export interface ComposeCodexTaskRunWithPostRunCaptureInput extends ComposeCodexTaskRunInput {
  postRunCapture?: PostRunCaptureOptions;
}

export interface PostRunCaptureOptions {
  diff?: PostRunDiffCaptureOptions;
  validation?: PostRunValidationCaptureOptions;
}

export interface PostRunDiffCaptureOptions {
  title?: string;
  worktreePath?: string;
}

export interface PostRunValidationCaptureOptions {
  command: string;
  args?: readonly string[];
  cwd?: string;
  worktreeId?: EntityId;
  env?: Record<string, string | undefined>;
  startedAt?: IsoDateTime;
  completedAt?: IsoDateTime;
  onStdoutChunk?: (chunk: string) => void;
  onStderrChunk?: (chunk: string) => void;
}

export interface ComposeCodexTaskRunWithPostRunCaptureResult {
  run: ComposeCodexTaskRunResult;
  postRunCapture: PostRunCaptureResult;
}

export interface PostRunCaptureResult {
  diff?: PostRunDiffCaptureResult;
  validation?: PostRunValidationCaptureResult;
  skippedReason?: 'run_failed';
}

export type PostRunDiffCaptureResult =
  | {
      status: 'captured';
      result: CollectTaskDiffResult;
    }
  | {
      status: 'failed';
      error: string;
    };

export type PostRunValidationCaptureResult =
  | {
      status: 'completed';
      result: RunTaskValidationCommandResult;
    }
  | {
      status: 'failed';
      result?: RunTaskValidationCommandResult;
      error?: string;
    };

export async function composeCodexTaskRunWithPostRunCapture(
  service: PostRunCaptureCompositionService,
  input: ComposeCodexTaskRunWithPostRunCaptureInput,
): Promise<ComposeCodexTaskRunWithPostRunCaptureResult> {
  const run = await composeCodexTaskRun(service.runCompositionService, input);

  if (run.status !== 'completed') {
    return {
      run,
      postRunCapture: input.postRunCapture === undefined ? {} : { skippedReason: 'run_failed' },
    };
  }

  const postRunCapture: PostRunCaptureResult = {};
  const taskRunId = run.completed.taskRun.id;

  if (input.postRunCapture?.diff !== undefined) {
    postRunCapture.diff = await collectDiffSafely(service, {
      taskId: input.taskId,
      taskRunId,
      title: input.postRunCapture.diff.title ?? 'Post-run diff',
      worktreePath: input.postRunCapture.diff.worktreePath ?? input.cwd,
    });
  }

  if (input.postRunCapture?.validation !== undefined) {
    const validation = input.postRunCapture.validation;
    postRunCapture.validation = await runValidationSafely(service, {
      taskId: input.taskId,
      taskRunId,
      command: validation.command,
      ...(validation.args === undefined ? {} : { args: validation.args }),
      ...(validation.cwd === undefined
        ? input.cwd === undefined
          ? {}
          : { cwd: input.cwd }
        : { cwd: validation.cwd }),
      ...(validation.worktreeId === undefined
        ? input.worktreeId === undefined
          ? {}
          : { worktreeId: input.worktreeId }
        : { worktreeId: validation.worktreeId }),
      ...(validation.env === undefined ? {} : { env: validation.env }),
      ...(validation.startedAt === undefined ? {} : { startedAt: validation.startedAt }),
      ...(validation.completedAt === undefined ? {} : { completedAt: validation.completedAt }),
      ...(validation.onStdoutChunk === undefined
        ? {}
        : { onStdoutChunk: validation.onStdoutChunk }),
      ...(validation.onStderrChunk === undefined
        ? {}
        : { onStderrChunk: validation.onStderrChunk }),
    });
  }

  return {
    run,
    postRunCapture,
  };
}

async function collectDiffSafely(
  service: PostRunCaptureCompositionService,
  input: CollectTaskDiffInput,
): Promise<PostRunDiffCaptureResult> {
  try {
    return {
      status: 'captured',
      result: await collectTaskDiff(service.diffCollectionService, input),
    };
  } catch (error) {
    return {
      status: 'failed',
      error: error instanceof Error ? error.message : String(error),
    };
  }
}

async function runValidationSafely(
  service: PostRunCaptureCompositionService,
  input: RunTaskValidationCommandInput,
): Promise<PostRunValidationCaptureResult> {
  try {
    const result = await runTaskValidationCommand(service.validationCommandRunnerService, input);

    if (result.status === 'failed') {
      return {
        status: 'failed',
        result,
      };
    }

    return {
      status: 'completed',
      result,
    };
  } catch (error) {
    return {
      status: 'failed',
      error: error instanceof Error ? error.message : String(error),
    };
  }
}
