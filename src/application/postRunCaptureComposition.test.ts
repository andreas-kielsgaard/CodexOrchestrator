import { InMemoryArtifactStore } from '../domain/artifactStore';
import { InMemoryConversationStore } from '../domain/conversationStore';
import { InMemoryEventStore } from '../domain/eventStore';
import type { DomainRecords, EntityId, IsoDateTime, Task, Worktree } from '../domain/model';
import type { OpenTaskDashboardStore } from '../domain/openTaskDashboardStore';
import { InMemoryOpenTaskWriteStore } from '../domain/openTaskWriteStore';
import { seedDomainRecords } from '../domain/seedData';
import { InMemoryTaskRunStore } from '../domain/taskRunStore';
import { InMemoryValidationRunStore } from '../domain/validationRunStore';
import type { GitDiffProvider, GitDiffProviderInput } from './diffCollection';
import {
  composeCodexTaskRunWithPostRunCapture,
  type PostRunCaptureCompositionService,
} from './postRunCaptureComposition';
import type { CodexRunRuntime, CodexRunRuntimeResult } from './runComposition';
import type { TaskRunLifecycleRecorder } from './taskRunLifecycle';
import type {
  ValidationCommandRuntime,
  ValidationCommandRuntimeInput,
  ValidationCommandRuntimeResult,
} from './validationCommandRunner';

const taskId = 'task-post-run';
const projectId = 'project-orchestrator';
const worktreeId = 'worktree-post-run';
const worktreePath = 'C:/Repos/Codex Orchestrator Worktrees/047';

describe('post-run capture composition', () => {
  it('runs Codex, then captures a diff and one validation command for the same task run', async () => {
    const fixture = createFixture({
      codexRuntime: new FakeCodexRuntime(createRuntimeResult()),
      diffProvider: new FakeGitDiffProvider('diff --git a/file.ts b/file.ts\n+done'),
      validationRuntime: new FakeValidationRuntime({
        stdout: 'tests passed\n',
        stderr: '',
        exitCode: 0,
        signal: null,
      }),
    });

    const result = await composeCodexTaskRunWithPostRunCapture(fixture.service, {
      taskId,
      prompt: 'Implement post-run capture',
      cwd: worktreePath,
      worktreeId,
      startedAt: '2026-07-03T10:00:00.000Z',
      completedAt: '2026-07-03T10:10:00.000Z',
      postRunCapture: {
        diff: {},
        validation: {
          command: 'npm',
          args: ['run', 'test'],
          env: { CI: '1' },
          startedAt: '2026-07-03T10:11:00.000Z',
          completedAt: '2026-07-03T10:15:00.000Z',
        },
      },
    });

    expect(result.run.status).toBe('completed');
    expect(result.postRunCapture.diff?.status).toBe('captured');
    expect(result.postRunCapture.validation?.status).toBe('completed');
    expect(fixture.diffProvider.inputs).toEqual([{ worktreePath }]);
    expect(fixture.validationRuntime.calls).toEqual([
      {
        command: 'npm',
        args: ['run', 'test'],
        cwd: worktreePath,
        env: { CI: '1' },
      },
    ]);
    expect(result.postRunCapture.diff).toMatchObject({
      result: {
        taskRun: { id: 'run-001' },
        artifact: {
          id: 'artifact-003',
          kind: 'diff',
          title: 'Post-run diff',
          taskId,
          taskRunId: 'run-001',
        },
        isEmptyDiff: false,
      },
    });
    expect(result.postRunCapture.validation).toMatchObject({
      result: {
        validationRun: {
          id: 'validation-001',
          taskId,
          taskRunId: 'run-001',
          command: 'npm run test',
          status: 'passed',
        },
        outputArtifact: {
          id: 'artifact-004',
          kind: 'validation_log',
          taskRunId: 'run-001',
        },
      },
    });
  });

  it('does not run post-run capture when no capture options are configured', async () => {
    const fixture = createFixture({
      codexRuntime: new FakeCodexRuntime(createRuntimeResult()),
    });

    const result = await composeCodexTaskRunWithPostRunCapture(fixture.service, {
      taskId,
      prompt: 'Run only Codex',
      cwd: worktreePath,
      worktreeId,
    });

    expect(result.run.status).toBe('completed');
    expect(result.postRunCapture).toEqual({});
    expect(fixture.diffProvider.inputs).toEqual([]);
    expect(fixture.validationRuntime.calls).toEqual([]);
    expect(fixture.artifactStore.snapshot().map((artifact) => artifact.kind)).toEqual([
      'raw_event_stream',
      'final_response',
    ]);
  });

  it('preserves the completed run result when diff capture fails after Codex succeeds', async () => {
    const fixture = createFixture({
      codexRuntime: new FakeCodexRuntime(createRuntimeResult()),
      diffProvider: new FakeGitDiffProvider(new Error('git diff failed')),
    });

    const result = await composeCodexTaskRunWithPostRunCapture(fixture.service, {
      taskId,
      prompt: 'Run with failing diff',
      cwd: worktreePath,
      worktreeId,
      postRunCapture: {
        diff: {},
      },
    });

    expect(result.run.status).toBe('completed');
    expect(result.postRunCapture.diff).toEqual({
      status: 'failed',
      error: 'git diff failed',
    });
    expect(fixture.artifactStore.snapshot().map((artifact) => artifact.kind)).toEqual([
      'raw_event_stream',
      'final_response',
    ]);
  });

  it('preserves the completed run result when validation exits non-zero after Codex succeeds', async () => {
    const fixture = createFixture({
      codexRuntime: new FakeCodexRuntime(createRuntimeResult()),
      validationRuntime: new FakeValidationRuntime({
        stdout: '',
        stderr: 'lint failed\n',
        exitCode: 1,
        signal: null,
      }),
    });

    const result = await composeCodexTaskRunWithPostRunCapture(fixture.service, {
      taskId,
      prompt: 'Run with failing validation',
      cwd: worktreePath,
      worktreeId,
      postRunCapture: {
        validation: {
          command: 'npm',
          args: ['run', 'lint'],
        },
      },
    });

    expect(result.run.status).toBe('completed');
    expect(result.postRunCapture.validation).toMatchObject({
      status: 'failed',
      result: {
        status: 'failed',
        validationRun: {
          id: 'validation-001',
          taskRunId: 'run-001',
          status: 'failed',
          exitCode: 1,
        },
      },
    });
  });

  it('does not run post-run capture after a failed Codex run', async () => {
    const fixture = createFixture({
      codexRuntime: new FakeCodexRuntime(
        createRuntimeResult({
          status: 'failed',
          statusReason: 'Codex emitted a turn.failed event',
          stdoutJsonl: failedJsonl,
          stderr: 'permission denied',
          exitCode: 1,
        }),
      ),
    });

    const result = await composeCodexTaskRunWithPostRunCapture(fixture.service, {
      taskId,
      prompt: 'Run with failed Codex',
      cwd: worktreePath,
      worktreeId,
      postRunCapture: {
        diff: {},
        validation: {
          command: 'npm',
          args: ['run', 'test'],
        },
      },
    });

    expect(result.run.status).toBe('failed');
    expect(result.postRunCapture).toEqual({ skippedReason: 'run_failed' });
    expect(fixture.diffProvider.inputs).toEqual([]);
    expect(fixture.validationRuntime.calls).toEqual([]);
  });
});

interface Fixture {
  service: PostRunCaptureCompositionService;
  diffProvider: FakeGitDiffProvider;
  validationRuntime: FakeValidationRuntime;
  artifactStore: InMemoryArtifactStore;
}

function createFixture(input: {
  codexRuntime: FakeCodexRuntime;
  diffProvider?: FakeGitDiffProvider;
  validationRuntime?: FakeValidationRuntime;
}): Fixture {
  const taskStore = new InMemoryOpenTaskWriteStore(
    baseRecords(),
    sequenceIds('task-unused'),
    fixedClock('2026-07-03T10:05:00.000Z'),
  );
  const taskRunStore = new InMemoryTaskRunStore(
    sequenceIds('run'),
    fixedClock('2026-07-03T10:01:00.000Z'),
  );
  const conversationStore = new InMemoryConversationStore(
    sequenceIds('conversation'),
    fixedClock('2026-07-03T10:02:00.000Z'),
  );
  const artifactStore = new InMemoryArtifactStore(
    sequenceIds('artifact'),
    fixedClock('2026-07-03T10:03:00.000Z'),
  );
  const eventStore = new InMemoryEventStore(
    sequenceIds('event'),
    fixedClock('2026-07-03T10:04:00.000Z'),
  );
  const validationRunStore = new InMemoryValidationRunStore(
    sequenceIds('validation'),
    fixedClock('2026-07-03T10:06:00.000Z'),
  );
  const dashboardStore = new CombinedDashboardStore(taskStore, taskRunStore);
  const diffProvider = input.diffProvider ?? new FakeGitDiffProvider('');
  const validationRuntime =
    input.validationRuntime ??
    new FakeValidationRuntime({
      stdout: '',
      stderr: '',
      exitCode: 0,
      signal: null,
    });
  const recorder: TaskRunLifecycleRecorder = {
    openTaskDashboardStore: taskStore,
    openTaskWriteStore: taskStore,
    taskRunStore,
    conversationStore,
    artifactStore,
    eventStore,
  };

  return {
    service: {
      runCompositionService: {
        recorder,
        runtime: input.codexRuntime,
      },
      diffCollectionService: {
        dashboardStore,
        artifactStore,
        eventStore,
        diffProvider,
      },
      validationCommandRunnerService: {
        dashboardStore,
        validationRunStore,
        artifactStore,
        eventStore,
        runtime: validationRuntime,
      },
    },
    diffProvider,
    validationRuntime,
    artifactStore,
  };
}

class CombinedDashboardStore implements OpenTaskDashboardStore {
  constructor(
    private readonly taskStore: InMemoryOpenTaskWriteStore,
    private readonly taskRunStore: InMemoryTaskRunStore,
  ) {}

  async loadOpenTaskDashboardRecords(): Promise<DomainRecords> {
    return {
      ...this.taskStore.snapshot(),
      taskRuns: this.taskRunStore.snapshot(),
    };
  }
}

class FakeCodexRuntime implements CodexRunRuntime {
  constructor(private readonly result: CodexRunRuntimeResult | Error) {}

  async exec(): Promise<CodexRunRuntimeResult> {
    if (this.result instanceof Error) {
      throw this.result;
    }

    return this.result;
  }
}

class FakeGitDiffProvider implements GitDiffProvider {
  readonly inputs: GitDiffProviderInput[] = [];

  constructor(private readonly result: string | Error) {}

  async collectDiff(input: GitDiffProviderInput): Promise<{ diff: string }> {
    this.inputs.push({ ...input });

    if (this.result instanceof Error) {
      throw this.result;
    }

    return { diff: this.result };
  }
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

function baseRecords(): DomainRecords {
  return {
    ...seedDomainRecords,
    tasks: [baseTask()],
    taskRuns: [],
    worktrees: [baseWorktree()],
    conversations: [],
    artifacts: [],
    validationRuns: [],
    events: [],
  };
}

function baseTask(): Task {
  return {
    id: taskId,
    projectId,
    repoId: 'repo-orchestrator',
    branchId: 'branch-main',
    worktreeId,
    conversationIds: [],
    title: 'Post-run capture task',
    summary: 'Run Codex, then collect diff and validation output.',
    executionState: 'queued',
    attentionState: 'consider_later',
    priority: 'normal',
    createdAt: '2026-07-03T08:00:00.000Z',
    updatedAt: '2026-07-03T08:00:00.000Z',
  };
}

function baseWorktree(): Worktree {
  return {
    id: worktreeId,
    repoId: 'repo-orchestrator',
    branchId: 'branch-main',
    path: worktreePath,
    isMain: false,
    isDirty: true,
    createdAt: '2026-07-03T08:30:00.000Z',
    updatedAt: '2026-07-03T08:45:00.000Z',
  };
}

function createRuntimeResult(
  overrides: Partial<CodexRunRuntimeResult> = {},
): CodexRunRuntimeResult {
  return {
    command: 'codex',
    args: ['exec', '--json', 'prompt'],
    exitCode: 0,
    signal: null,
    status: 'completed',
    statusReason: 'Codex emitted a turn.completed event',
    stdoutJsonl: completedJsonl,
    stderr: '',
    summary: {
      threadId: 'thread-post-run',
      finalAgentMessageText: 'Post-run capture is wired.',
      terminalStatus: { kind: 'completed', lineNumber: 3 },
      itemCountsByType: { agent_message: 1 },
    },
    ...overrides,
  };
}

const completedJsonl = [
  JSON.stringify({ type: 'thread.started', thread_id: 'thread-post-run' }),
  JSON.stringify({ type: 'item.completed', item: { type: 'agent_message', text: 'Done' } }),
  JSON.stringify({ type: 'turn.completed' }),
].join('\n');

const failedJsonl = [
  JSON.stringify({ type: 'thread.started', thread_id: 'thread-post-run-failed' }),
  JSON.stringify({ type: 'turn.failed' }),
].join('\n');

function fixedClock(now: IsoDateTime): { now(): IsoDateTime } {
  return {
    now: () => now,
  };
}

function sequenceIds(prefix: string): { nextId(): EntityId } {
  let next = 1;

  return {
    nextId: () => `${prefix}-${String(next++).padStart(3, '0')}`,
  };
}
