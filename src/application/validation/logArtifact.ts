import type { Artifact } from '../../domain/model';
import type { CreateValidationLogArtifactInput, ValidationCommandRunnerService } from './types';

export async function createValidationLogArtifact(
  service: ValidationCommandRunnerService,
  input: CreateValidationLogArtifactInput,
): Promise<Artifact> {
  return service.artifactStore.createArtifact({
    kind: 'validation_log',
    title: `Validation log: ${input.displayCommand}`,
    taskId: input.task.id,
    ...(input.taskRunId === undefined ? {} : { taskRunId: input.taskRunId }),
    content: JSON.stringify(createValidationLogPayload(input), null, 2),
  });
}

function createValidationLogPayload(
  input: CreateValidationLogArtifactInput,
): Record<string, unknown> {
  return {
    taskId: input.task.id,
    validationRunId: input.validationRun.id,
    status: input.status,
    command: input.command,
    args: [...(input.args ?? [])],
    cwd: input.cwd,
    ...(input.worktree === undefined ? {} : { worktreeId: input.worktree.id }),
    ...(input.startedAt === undefined ? {} : { startedAt: input.startedAt }),
    ...(input.completedAt === undefined ? {} : { completedAt: input.completedAt }),
    process:
      input.runtimeResult === undefined
        ? {
            stdout: '',
            stderr: '',
            exitCode: null,
            signal: null,
            error: input.error ?? 'Validation command did not return a process result',
          }
        : {
            stdout: input.runtimeResult.stdout,
            stderr: input.runtimeResult.stderr,
            exitCode: input.runtimeResult.exitCode,
            signal: input.runtimeResult.signal,
          },
  };
}
