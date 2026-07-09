import type { Event } from '../../domain/model';
import { numericExitCode } from './commandFormatting';
import type {
  AppendArtifactCreatedEventInput,
  AppendValidationCompletedEventInput,
  ValidationCommandRunnerService,
} from './types';

export async function appendArtifactCreatedEvent(
  service: ValidationCommandRunnerService,
  input: AppendArtifactCreatedEventInput,
): Promise<Event> {
  return service.eventStore.appendEvent({
    kind: 'artifact_created',
    projectId: input.task.projectId,
    taskId: input.task.id,
    ...(input.taskRunId === undefined ? {} : { taskRunId: input.taskRunId }),
    artifactId: input.outputArtifact.id,
    validationRunId: input.validationRun.id,
    payload: {
      artifactKind: input.outputArtifact.kind,
      artifactId: input.outputArtifact.id,
      validationRunId: input.validationRun.id,
      validationStatus: input.status,
      ...(input.runtimeResult === undefined
        ? {}
        : {
            stdoutLength: input.runtimeResult.stdout.length,
            stderrLength: input.runtimeResult.stderr.length,
            ...(numericExitCode(input.runtimeResult.exitCode) === undefined
              ? {}
              : { exitCode: numericExitCode(input.runtimeResult.exitCode) }),
            ...(input.runtimeResult.signal === null ? {} : { signal: input.runtimeResult.signal }),
          }),
      ...(input.error === undefined ? {} : { error: input.error }),
    },
  });
}

export async function appendValidationCompletedEvent(
  service: ValidationCommandRunnerService,
  input: AppendValidationCompletedEventInput,
): Promise<Event> {
  return service.eventStore.appendEvent({
    kind: 'validation_completed',
    projectId: input.task.projectId,
    taskId: input.task.id,
    ...(input.taskRunId === undefined ? {} : { taskRunId: input.taskRunId }),
    artifactId: input.outputArtifact.id,
    validationRunId: input.validationRun.id,
    payload: {
      outcome: input.status,
      taskId: input.task.id,
      validationRunId: input.validationRun.id,
      artifactId: input.outputArtifact.id,
      ...(input.completedAt === undefined ? {} : { completedAt: input.completedAt }),
      ...(input.runtimeResult === undefined
        ? {}
        : {
            ...(numericExitCode(input.runtimeResult.exitCode) === undefined
              ? {}
              : { exitCode: numericExitCode(input.runtimeResult.exitCode) }),
            ...(input.runtimeResult.signal === null ? {} : { signal: input.runtimeResult.signal }),
          }),
      ...(input.error === undefined ? {} : { error: input.error }),
    },
  });
}
