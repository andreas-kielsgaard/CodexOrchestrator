import type { ArtifactStore } from '../domain/artifactStore';
import type { EventStore } from '../domain/eventStore';
import type {
  Artifact,
  DomainRecords,
  EntityId,
  Event,
  IsoDateTime,
  Task,
  ValidationRun,
  ValidationStatus,
  Worktree,
} from '../domain/model';
import type { OpenTaskDashboardStore } from '../domain/openTaskDashboardStore';
import type { ValidationRunStore } from '../domain/validationRunStore';

export interface ValidationCommandRunnerService {
  readonly dashboardStore: OpenTaskDashboardStore;
  readonly validationRunStore: ValidationRunStore;
  readonly artifactStore: ArtifactStore;
  readonly eventStore: EventStore;
  readonly runtime: ValidationCommandRuntime;
}

export interface ValidationCommandRuntime {
  run(input: ValidationCommandRuntimeInput): Promise<ValidationCommandRuntimeResult>;
}

export interface ValidationCommandRuntimeInput {
  command: string;
  args?: readonly string[];
  cwd: string;
  env?: Record<string, string | undefined>;
  onStdoutChunk?: (chunk: string) => void;
  onStderrChunk?: (chunk: string) => void;
}

export interface ValidationCommandRuntimeResult {
  stdout: string;
  stderr: string;
  exitCode: number | null;
  signal: string | null;
}

export interface RunTaskValidationCommandInput {
  taskId: EntityId;
  taskRunId?: EntityId;
  worktreeId?: EntityId;
  command: string;
  args?: readonly string[];
  cwd?: string;
  env?: Record<string, string | undefined>;
  startedAt?: IsoDateTime;
  completedAt?: IsoDateTime;
  onStdoutChunk?: (chunk: string) => void;
  onStderrChunk?: (chunk: string) => void;
}

export type RunTaskValidationCommandResult =
  RunTaskValidationCommandCompletedResult | RunTaskValidationCommandFailedResult;

export interface RunTaskValidationCommandCompletedResult {
  status: 'passed';
  task: Task;
  worktree?: Worktree;
  validationRun: ValidationRun;
  outputArtifact: Artifact;
  startedEvent: Event;
  artifactCreatedEvent: Event;
  completedEvent: Event;
  runtimeResult: ValidationCommandRuntimeResult;
}

export interface RunTaskValidationCommandFailedResult {
  status: 'failed';
  task: Task;
  worktree?: Worktree;
  validationRun: ValidationRun;
  outputArtifact: Artifact;
  startedEvent: Event;
  artifactCreatedEvent: Event;
  completedEvent: Event;
  runtimeResult?: ValidationCommandRuntimeResult;
  error?: string;
}

export class ValidationCommandTaskNotFoundError extends Error {
  constructor(taskId: EntityId) {
    super(`Task not found before validation command run: ${taskId}`);
    this.name = 'ValidationCommandTaskNotFoundError';
  }
}

export class ValidationCommandWorktreeRequiredError extends Error {
  constructor(taskId: EntityId) {
    super(`Validation command run requires a cwd or linked worktree path for task: ${taskId}`);
    this.name = 'ValidationCommandWorktreeRequiredError';
  }
}

export class ValidationCommandWorktreeNotFoundError extends Error {
  constructor(worktreeId: EntityId, taskId: EntityId) {
    super(`Validation command worktree not found for task ${taskId}: ${worktreeId}`);
    this.name = 'ValidationCommandWorktreeNotFoundError';
  }
}

export async function runTaskValidationCommand(
  service: ValidationCommandRunnerService,
  input: RunTaskValidationCommandInput,
): Promise<RunTaskValidationCommandResult> {
  const records = await service.dashboardStore.loadOpenTaskDashboardRecords();
  const task = requireTask(records, input.taskId);
  const resolvedCwd = resolveValidationCwd(records, task, input);
  const displayCommand = renderValidationCommand(input.command, input.args);
  const validationRun = await service.validationRunStore.createValidationRun({
    command: displayCommand,
    status: 'running',
    taskId: task.id,
    ...(input.taskRunId === undefined ? {} : { taskRunId: input.taskRunId }),
    ...(input.startedAt === undefined ? {} : { startedAt: input.startedAt }),
  });
  const startedEvent = await service.eventStore.appendEvent({
    kind: 'validation_started',
    projectId: task.projectId,
    taskId: task.id,
    ...(input.taskRunId === undefined ? {} : { taskRunId: input.taskRunId }),
    validationRunId: validationRun.id,
    payload: {
      taskId: task.id,
      validationRunId: validationRun.id,
      command: input.command,
      args: [...(input.args ?? [])],
      cwd: resolvedCwd.cwd,
      ...(resolvedCwd.worktree === undefined ? {} : { worktreeId: resolvedCwd.worktree.id }),
      ...(input.startedAt === undefined ? {} : { startedAt: input.startedAt }),
    },
  });

  try {
    const runtimeResult = await service.runtime.run({
      command: input.command,
      ...(input.args === undefined ? {} : { args: input.args }),
      cwd: resolvedCwd.cwd,
      ...(input.env === undefined ? {} : { env: input.env }),
      ...(input.onStdoutChunk === undefined ? {} : { onStdoutChunk: input.onStdoutChunk }),
      ...(input.onStderrChunk === undefined ? {} : { onStderrChunk: input.onStderrChunk }),
    });
    const status = classifyValidationStatus(runtimeResult);
    const outputArtifact = await createValidationLogArtifact(service, {
      task,
      taskRunId: input.taskRunId,
      validationRun,
      displayCommand,
      command: input.command,
      args: input.args,
      cwd: resolvedCwd.cwd,
      worktree: resolvedCwd.worktree,
      startedAt: input.startedAt,
      completedAt: input.completedAt,
      status,
      runtimeResult,
    });
    const artifactCreatedEvent = await appendArtifactCreatedEvent(service, {
      task,
      taskRunId: input.taskRunId,
      validationRun,
      outputArtifact,
      runtimeResult,
      status,
    });
    const updatedValidationRun = await service.validationRunStore.updateValidationRun(
      validationRun.id,
      {
        status,
        ...(input.completedAt === undefined ? {} : { completedAt: input.completedAt }),
        ...(numericExitCode(runtimeResult.exitCode) === undefined
          ? {}
          : { exitCode: numericExitCode(runtimeResult.exitCode) }),
        outputArtifactId: outputArtifact.id,
      },
    );
    const completedEvent = await appendValidationCompletedEvent(service, {
      task,
      taskRunId: input.taskRunId,
      validationRun: updatedValidationRun,
      outputArtifact,
      runtimeResult,
      status,
      completedAt: input.completedAt,
    });

    return {
      status,
      task,
      ...(resolvedCwd.worktree === undefined ? {} : { worktree: resolvedCwd.worktree }),
      validationRun: updatedValidationRun,
      outputArtifact,
      startedEvent,
      artifactCreatedEvent,
      completedEvent,
      runtimeResult,
    };
  } catch (error) {
    const errorMessage = error instanceof Error ? error.message : String(error);
    const status = 'failed';
    const outputArtifact = await createValidationLogArtifact(service, {
      task,
      taskRunId: input.taskRunId,
      validationRun,
      displayCommand,
      command: input.command,
      args: input.args,
      cwd: resolvedCwd.cwd,
      worktree: resolvedCwd.worktree,
      startedAt: input.startedAt,
      completedAt: input.completedAt,
      status,
      error: errorMessage,
    });
    const artifactCreatedEvent = await appendArtifactCreatedEvent(service, {
      task,
      taskRunId: input.taskRunId,
      validationRun,
      outputArtifact,
      status,
      error: errorMessage,
    });
    const updatedValidationRun = await service.validationRunStore.updateValidationRun(
      validationRun.id,
      {
        status,
        ...(input.completedAt === undefined ? {} : { completedAt: input.completedAt }),
        outputArtifactId: outputArtifact.id,
      },
    );
    const completedEvent = await appendValidationCompletedEvent(service, {
      task,
      taskRunId: input.taskRunId,
      validationRun: updatedValidationRun,
      outputArtifact,
      status,
      completedAt: input.completedAt,
      error: errorMessage,
    });

    return {
      status,
      task,
      ...(resolvedCwd.worktree === undefined ? {} : { worktree: resolvedCwd.worktree }),
      validationRun: updatedValidationRun,
      outputArtifact,
      startedEvent,
      artifactCreatedEvent,
      completedEvent,
      error: errorMessage,
    };
  }
}

interface ResolvedValidationCwd {
  cwd: string;
  worktree?: Worktree;
}

interface CreateValidationLogArtifactInput {
  task: Task;
  taskRunId?: EntityId;
  validationRun: ValidationRun;
  displayCommand: string;
  command: string;
  args?: readonly string[];
  cwd: string;
  worktree?: Worktree;
  startedAt?: IsoDateTime;
  completedAt?: IsoDateTime;
  status: ValidationStatus;
  runtimeResult?: ValidationCommandRuntimeResult;
  error?: string;
}

interface AppendArtifactCreatedEventInput {
  task: Task;
  taskRunId?: EntityId;
  validationRun: ValidationRun;
  outputArtifact: Artifact;
  status: ValidationStatus;
  runtimeResult?: ValidationCommandRuntimeResult;
  error?: string;
}

interface AppendValidationCompletedEventInput {
  task: Task;
  taskRunId?: EntityId;
  validationRun: ValidationRun;
  outputArtifact: Artifact;
  status: ValidationStatus;
  runtimeResult?: ValidationCommandRuntimeResult;
  completedAt?: IsoDateTime;
  error?: string;
}

function requireTask(records: DomainRecords, taskId: EntityId): Task {
  const task = records.tasks.find((candidate) => candidate.id === taskId);

  if (task === undefined) {
    throw new ValidationCommandTaskNotFoundError(taskId);
  }

  return task;
}

function resolveValidationCwd(
  records: DomainRecords,
  task: Task,
  input: RunTaskValidationCommandInput,
): ResolvedValidationCwd {
  const worktreeId = input.worktreeId ?? task.worktreeId;

  if (input.cwd !== undefined) {
    if (input.worktreeId !== undefined) {
      return {
        cwd: input.cwd,
        worktree: requireWorktree(records, input.worktreeId, task.id),
      };
    }

    const linkedWorktree =
      task.worktreeId === undefined ? undefined : findWorktree(records, task.worktreeId);

    return {
      cwd: input.cwd,
      ...(linkedWorktree === undefined ? {} : { worktree: linkedWorktree }),
    };
  }

  if (worktreeId === undefined) {
    throw new ValidationCommandWorktreeRequiredError(task.id);
  }

  const worktree = requireWorktree(records, worktreeId, task.id);

  return {
    cwd: worktree.path,
    worktree,
  };
}

function findWorktree(records: DomainRecords, worktreeId: EntityId): Worktree | undefined {
  return records.worktrees.find((candidate) => candidate.id === worktreeId);
}

function requireWorktree(records: DomainRecords, worktreeId: EntityId, taskId: EntityId): Worktree {
  const worktree = findWorktree(records, worktreeId);

  if (worktree === undefined) {
    throw new ValidationCommandWorktreeNotFoundError(worktreeId, taskId);
  }

  return worktree;
}

async function createValidationLogArtifact(
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

async function appendArtifactCreatedEvent(
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

async function appendValidationCompletedEvent(
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

function classifyValidationStatus(result: ValidationCommandRuntimeResult): 'passed' | 'failed' {
  if (result.exitCode === 0 && result.signal === null) {
    return 'passed';
  }

  return 'failed';
}

function renderValidationCommand(command: string, args: readonly string[] | undefined): string {
  const renderedArgs = (args ?? []).map(renderCommandArg);

  return [command, ...renderedArgs].join(' ');
}

function renderCommandArg(arg: string): string {
  if (/^[A-Za-z0-9_./:=@+-]+$/.test(arg)) {
    return arg;
  }

  return JSON.stringify(arg);
}

function numericExitCode(exitCode: number | null): number | undefined {
  return typeof exitCode === 'number' && Number.isFinite(exitCode) ? exitCode : undefined;
}
