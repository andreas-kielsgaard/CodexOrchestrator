import type { ArtifactStore } from '../../domain/artifactStore';
import type { EventStore } from '../../domain/eventStore';
import type {
  Artifact,
  EntityId,
  Event,
  IsoDateTime,
  Task,
  ValidationRun,
  ValidationStatus,
  Worktree,
} from '../../domain/model';
import type { OpenTaskDashboardStore } from '../../domain/openTaskDashboardStore';
import type { ValidationRunStore } from '../../domain/validationRunStore';

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

export interface ResolvedValidationCwd {
  cwd: string;
  worktree?: Worktree;
}

export interface CreateValidationLogArtifactInput {
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

export interface AppendArtifactCreatedEventInput {
  task: Task;
  taskRunId?: EntityId;
  validationRun: ValidationRun;
  outputArtifact: Artifact;
  status: ValidationStatus;
  runtimeResult?: ValidationCommandRuntimeResult;
  error?: string;
}

export interface AppendValidationCompletedEventInput {
  task: Task;
  taskRunId?: EntityId;
  validationRun: ValidationRun;
  outputArtifact: Artifact;
  status: ValidationStatus;
  runtimeResult?: ValidationCommandRuntimeResult;
  completedAt?: IsoDateTime;
  error?: string;
}
