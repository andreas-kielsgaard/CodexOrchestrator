import { InMemoryArtifactStore } from '../domain/artifactStore';
import { InMemoryEventStore } from '../domain/eventStore';
import type { DomainRecords, IsoDateTime, Task, Worktree } from '../domain/model';
import { InMemoryOpenTaskDashboardStore } from '../domain/openTaskDashboardStore';
import { InMemoryValidationRunStore } from '../domain/validationRunStore';
import {
  runTaskValidationCommand,
  ValidationCommandTaskNotFoundError,
  ValidationCommandWorktreeNotFoundError,
  type ValidationCommandRunnerService,
  type ValidationCommandRuntime,
  type ValidationCommandRuntimeInput,
  type ValidationCommandRuntimeResult,
} from './validationCommandRunner';

const taskId = 'task-validation';
const taskRunId = 'task-run-validation';
const projectId = 'project-orchestrator';
const worktreeId = 'worktree-validation';
const worktreePath = 'C:/Repos/Codex Orchestrator Worktrees/034';

const baseTask: Task = {
  id: taskId,
  projectId,
  repoId: 'repo-orchestrator',
  branchId: 'branch-worker',
  worktreeId,
  conversationIds: [],
  title: 'Validation task',
  summary: 'Run configured validation commands.',
  executionState: 'completed',
  attentionState: 'needs_review',
  priority: 'normal',
  createdAt: '2026-07-02T08:00:00.000Z',
  updatedAt: '2026-07-02T08:00:00.000Z',
};

const baseWorktree: Worktree = {
  id: worktreeId,
  repoId: 'repo-orchestrator',
  branchId: 'branch-worker',
  path: worktreePath,
  isMain: false,
  isDirty: false,
  lastScannedAt: '2026-07-02T09:00:00.000Z',
  createdAt: '2026-07-02T08:30:00.000Z',
  updatedAt: '2026-07-02T09:00:00.000Z',
};

describe('validation command runner', () => {
  it('runs a passing command in the linked worktree, stores output, and links the validation artifact', async () => {
    const runtimeResult: ValidationCommandRuntimeResult = {
      stdout: 'all tests passed\n',
      stderr: '',
      exitCode: 0,
      signal: null,
    };
    const fixture = createFixture({ runtime: new FakeValidationRuntime(runtimeResult) });

    const result = await runTaskValidationCommand(fixture.service, {
      taskId,
      taskRunId,
      command: 'npm',
      args: ['run', 'test'],
      env: { CI: '1' },
      startedAt: '2026-07-02T10:00:00.000Z',
      completedAt: '2026-07-02T10:05:00.000Z',
    });

    expect(result.status).toBe('passed');
    expect(fixture.runtime.calls).toEqual([
      {
        command: 'npm',
        args: ['run', 'test'],
        cwd: worktreePath,
        env: { CI: '1' },
      },
    ]);
    expect(result.validationRun).toMatchObject({
      id: 'validation-001',
      taskId,
      taskRunId,
      command: 'npm run test',
      status: 'passed',
      startedAt: '2026-07-02T10:00:00.000Z',
      completedAt: '2026-07-02T10:05:00.000Z',
      exitCode: 0,
      outputArtifactId: 'artifact-001',
    });
    expect(result.outputArtifact).toMatchObject({
      id: 'artifact-001',
      kind: 'validation_log',
      title: 'Validation log: npm run test',
      taskId,
      taskRunId,
    });
    expect(JSON.parse(result.outputArtifact.content ?? '{}')).toEqual({
      taskId,
      validationRunId: 'validation-001',
      status: 'passed',
      command: 'npm',
      args: ['run', 'test'],
      cwd: worktreePath,
      worktreeId,
      startedAt: '2026-07-02T10:00:00.000Z',
      completedAt: '2026-07-02T10:05:00.000Z',
      process: runtimeResult,
    });
    expect(result.startedEvent).toMatchObject({
      id: 'event-001',
      kind: 'validation_started',
      projectId,
      taskId,
      taskRunId,
      validationRunId: 'validation-001',
      payload: {
        taskId,
        validationRunId: 'validation-001',
        command: 'npm',
        args: ['run', 'test'],
        cwd: worktreePath,
        worktreeId,
        startedAt: '2026-07-02T10:00:00.000Z',
      },
    });
    expect(result.artifactCreatedEvent).toMatchObject({
      id: 'event-002',
      kind: 'artifact_created',
      artifactId: 'artifact-001',
      validationRunId: 'validation-001',
      payload: {
        artifactKind: 'validation_log',
        artifactId: 'artifact-001',
        validationRunId: 'validation-001',
        validationStatus: 'passed',
        stdoutLength: runtimeResult.stdout.length,
        stderrLength: 0,
        exitCode: 0,
      },
    });
    expect(result.completedEvent).toMatchObject({
      id: 'event-003',
      kind: 'validation_completed',
      artifactId: 'artifact-001',
      validationRunId: 'validation-001',
      payload: {
        outcome: 'passed',
        taskId,
        validationRunId: 'validation-001',
        artifactId: 'artifact-001',
        completedAt: '2026-07-02T10:05:00.000Z',
        exitCode: 0,
      },
    });
    expect(fixture.validationRunStore.snapshot()[0]?.outputArtifactId).toBe('artifact-001');
    expect(fixture.eventStore.snapshot().map((event) => event.kind)).toEqual([
      'validation_started',
      'artifact_created',
      'validation_completed',
    ]);
  });

  it('records a failed validation run when the command exits non-zero', async () => {
    const runtimeResult: ValidationCommandRuntimeResult = {
      stdout: '',
      stderr: 'lint failed\n',
      exitCode: 1,
      signal: null,
    };
    const fixture = createFixture({
      records: recordsWith({ worktrees: [] }),
      runtime: new FakeValidationRuntime(runtimeResult),
    });

    const result = await runTaskValidationCommand(fixture.service, {
      taskId,
      taskRunId,
      command: 'npm',
      args: ['run', 'lint'],
      cwd: 'C:/explicit/worktree',
      completedAt: '2026-07-02T10:10:00.000Z',
    });

    expect(result.status).toBe('failed');
    expect(fixture.runtime.calls[0]).toMatchObject({
      command: 'npm',
      args: ['run', 'lint'],
      cwd: 'C:/explicit/worktree',
    });
    expect(result.validationRun).toMatchObject({
      status: 'failed',
      exitCode: 1,
      outputArtifactId: 'artifact-001',
    });
    expect(JSON.parse(result.outputArtifact.content ?? '{}')).toMatchObject({
      status: 'failed',
      command: 'npm',
      args: ['run', 'lint'],
      cwd: 'C:/explicit/worktree',
      process: runtimeResult,
    });
    expect(result.completedEvent).toMatchObject({
      payload: {
        outcome: 'failed',
        exitCode: 1,
        artifactId: 'artifact-001',
      },
    });
  });

  it('stores a failed validation log when the runtime throws before returning process output', async () => {
    const fixture = createFixture({
      runtime: new FakeValidationRuntime(new Error('spawn npm ENOENT')),
    });

    const result = await runTaskValidationCommand(fixture.service, {
      taskId,
      command: 'npm',
      args: ['run', 'test'],
      completedAt: '2026-07-02T10:15:00.000Z',
    });

    expect(result.status).toBe('failed');
    if (result.status !== 'failed') {
      throw new Error('Expected failed validation command result');
    }
    expect(result.error).toBe('spawn npm ENOENT');
    expect(result.runtimeResult).toBeUndefined();
    expect(result.validationRun).toMatchObject({
      status: 'failed',
      outputArtifactId: 'artifact-001',
    });
    expect(result.validationRun).not.toHaveProperty('exitCode');
    expect(JSON.parse(result.outputArtifact.content ?? '{}')).toMatchObject({
      status: 'failed',
      command: 'npm',
      args: ['run', 'test'],
      cwd: worktreePath,
      process: {
        stdout: '',
        stderr: '',
        exitCode: null,
        signal: null,
        error: 'spawn npm ENOENT',
      },
    });
    expect(result.artifactCreatedEvent).toMatchObject({
      payload: {
        artifactKind: 'validation_log',
        validationStatus: 'failed',
        error: 'spawn npm ENOENT',
      },
    });
    expect(result.completedEvent).toMatchObject({
      payload: {
        outcome: 'failed',
        artifactId: 'artifact-001',
        error: 'spawn npm ENOENT',
      },
    });
    expect(fixture.eventStore.snapshot().map((event) => event.kind)).toEqual([
      'validation_started',
      'artifact_created',
      'validation_completed',
    ]);
  });

  it('preflights missing tasks before creating validation records or invoking the runtime', async () => {
    const fixture = createFixture({
      records: recordsWith({ tasks: [] }),
      runtime: new FakeValidationRuntime({
        stdout: 'unused',
        stderr: '',
        exitCode: 0,
        signal: null,
      }),
    });

    await expect(
      runTaskValidationCommand(fixture.service, {
        taskId: 'task-missing',
        command: 'npm',
        args: ['run', 'test'],
      }),
    ).rejects.toThrow(ValidationCommandTaskNotFoundError);

    expect(fixture.runtime.calls).toEqual([]);
    expect(fixture.validationRunStore.snapshot()).toEqual([]);
    expect(fixture.artifactStore.snapshot()).toEqual([]);
    expect(fixture.eventStore.snapshot()).toEqual([]);
  });

  it('preflights missing linked worktrees before creating validation records or invoking the runtime', async () => {
    const fixture = createFixture({
      records: recordsWith({ tasks: [baseTask], worktrees: [] }),
      runtime: new FakeValidationRuntime({
        stdout: 'unused',
        stderr: '',
        exitCode: 0,
        signal: null,
      }),
    });

    await expect(
      runTaskValidationCommand(fixture.service, {
        taskId,
        command: 'npm',
        args: ['run', 'test'],
      }),
    ).rejects.toThrow(ValidationCommandWorktreeNotFoundError);

    expect(fixture.runtime.calls).toEqual([]);
    expect(fixture.validationRunStore.snapshot()).toEqual([]);
    expect(fixture.artifactStore.snapshot()).toEqual([]);
    expect(fixture.eventStore.snapshot()).toEqual([]);
  });
});

interface Fixture {
  service: ValidationCommandRunnerService;
  runtime: FakeValidationRuntime;
  validationRunStore: InMemoryValidationRunStore;
  artifactStore: InMemoryArtifactStore;
  eventStore: InMemoryEventStore;
}

function createFixture(input: {
  records?: DomainRecords;
  runtime: FakeValidationRuntime;
}): Fixture {
  const validationRunStore = new InMemoryValidationRunStore(
    sequenceIds('validation'),
    fixedClock('2026-07-02T10:01:00.000Z'),
  );
  const artifactStore = new InMemoryArtifactStore(
    sequenceIds('artifact'),
    fixedClock('2026-07-02T10:02:00.000Z'),
  );
  const eventStore = new InMemoryEventStore(
    sequenceIds('event'),
    fixedClock('2026-07-02T10:03:00.000Z'),
  );

  return {
    service: {
      dashboardStore: new InMemoryOpenTaskDashboardStore(input.records ?? recordsWith({})),
      validationRunStore,
      artifactStore,
      eventStore,
      runtime: input.runtime,
    },
    runtime: input.runtime,
    validationRunStore,
    artifactStore,
    eventStore,
  };
}

class FakeValidationRuntime implements ValidationCommandRuntime {
  readonly calls: ValidationCommandRuntimeInput[] = [];

  constructor(private readonly result: ValidationCommandRuntimeResult | Error) {}

  async run(input: ValidationCommandRuntimeInput): Promise<ValidationCommandRuntimeResult> {
    this.calls.push({
      command: input.command,
      ...(input.args === undefined ? {} : { args: [...input.args] }),
      cwd: input.cwd,
      ...(input.env === undefined ? {} : { env: { ...input.env } }),
    });

    if (this.result instanceof Error) {
      throw this.result;
    }

    return this.result;
  }
}

function recordsWith(overrides: Partial<DomainRecords>): DomainRecords {
  return {
    projects: [],
    repos: [],
    branches: [],
    worktrees: [baseWorktree],
    conversations: [],
    tasks: [baseTask],
    taskRuns: [],
    artifacts: [],
    validationRuns: [],
    events: [],
    ...overrides,
  };
}

function fixedClock(now: IsoDateTime): { now(): IsoDateTime } {
  return {
    now: () => now,
  };
}

function sequenceIds(prefix: string): { nextId(): string } {
  let next = 1;

  return {
    nextId: () => `${prefix}-${String(next++).padStart(3, '0')}`,
  };
}
