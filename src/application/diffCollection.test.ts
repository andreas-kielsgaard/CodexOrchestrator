import { InMemoryArtifactStore } from '../domain/artifactStore';
import { InMemoryEventStore } from '../domain/eventStore';
import type { DomainRecords, IsoDateTime, Task, TaskRun, Worktree } from '../domain/model';
import { InMemoryOpenTaskDashboardStore } from '../domain/openTaskDashboardStore';
import {
  collectTaskDiff,
  DiffCollectionTaskNotFoundError,
  DiffCollectionTaskRunNotFoundForTaskError,
  DiffCollectionWorktreeNotFoundError,
  DiffCollectionWorktreeNotResolvedError,
  type DiffCollectionService,
  type GitDiffProvider,
  type GitDiffProviderInput,
} from './diffCollection';

const projectId = 'project-orchestrator';
const taskId = 'task-diff';
const taskRunId = 'run-diff';
const worktreeId = 'worktree-worker';
const worktreePath = 'C:/Repos/Codex Orchestrator Worktrees/033';

describe('diff collection service', () => {
  it('collects a non-empty linked worktree diff, stores it as an artifact, and emits metadata', async () => {
    const diff = [
      'diff --git a/src/example.ts b/src/example.ts',
      'index 1111111..2222222 100644',
      '--- a/src/example.ts',
      '+++ b/src/example.ts',
      '@@ -1 +1 @@',
      '-old',
      '+new',
    ].join('\n');
    const fixture = createFixture({
      provider: new FakeGitDiffProvider(diff),
    });

    const result = await collectTaskDiff(fixture.service, {
      taskId,
      taskRunId,
    });

    expect(fixture.provider.inputs).toEqual([{ worktreePath }]);
    expect(result).toMatchObject({
      task: expect.objectContaining({ id: taskId }),
      taskRun: expect.objectContaining({ id: taskRunId }),
      worktree: expect.objectContaining({ id: worktreeId }),
      worktreePath,
      diff,
      isEmptyDiff: false,
    });
    expect(result.artifact).toMatchObject({
      id: 'artifact-001',
      kind: 'diff',
      title: 'Worktree diff',
      taskId,
      taskRunId,
      content: diff,
    });
    expect(result.event).toMatchObject({
      id: 'event-001',
      kind: 'artifact_created',
      projectId,
      taskId,
      taskRunId,
      artifactId: 'artifact-001',
      payload: {
        artifactKind: 'diff',
        artifactId: 'artifact-001',
        diffLength: diff.length,
        isEmptyDiff: false,
        worktreeId,
        worktreePath,
      },
    });
  });

  it('preserves an empty explicit-path diff as an empty artifact body plus explicit metadata', async () => {
    const explicitWorktreePath = 'C:\\Repos\\Explicit Worker';
    const fixture = createFixture({
      provider: new FakeGitDiffProvider(''),
    });

    const result = await collectTaskDiff(fixture.service, {
      taskId,
      worktreePath: explicitWorktreePath,
      title: 'Post-run diff',
    });

    expect(fixture.provider.inputs).toEqual([{ worktreePath: explicitWorktreePath }]);
    expect(result.isEmptyDiff).toBe(true);
    expect(result.worktree).toBeUndefined();
    expect(result.artifact).toMatchObject({
      kind: 'diff',
      title: 'Post-run diff',
      taskId,
      content: '',
    });
    expect(result.artifact).not.toHaveProperty('taskRunId');
    expect(result.event.payload).toEqual({
      artifactKind: 'diff',
      artifactId: 'artifact-001',
      diffLength: 0,
      isEmptyDiff: true,
      worktreePath: 'C:/Repos/Explicit Worker',
    });
  });

  it('preflights missing tasks and worktree resolution before calling the provider', async () => {
    const provider = new FakeGitDiffProvider('diff');
    const missingTaskFixture = createFixture({
      records: recordsWith({ tasks: [] }),
      provider,
    });

    await expect(
      collectTaskDiff(missingTaskFixture.service, {
        taskId: 'task-missing',
      }),
    ).rejects.toThrow(DiffCollectionTaskNotFoundError);

    const mismatchedTaskRunFixture = createFixture({
      records: recordsWith({
        taskRuns: [baseTaskRun({ taskId: 'task-other' })],
      }),
      provider,
    });

    await expect(
      collectTaskDiff(mismatchedTaskRunFixture.service, {
        taskId,
        taskRunId,
      }),
    ).rejects.toThrow(DiffCollectionTaskRunNotFoundForTaskError);

    const noWorktreeFixture = createFixture({
      records: recordsWith({
        tasks: [baseTask({ worktreeId: undefined })],
        taskRuns: [],
        worktrees: [],
      }),
      provider,
    });

    await expect(
      collectTaskDiff(noWorktreeFixture.service, {
        taskId,
      }),
    ).rejects.toThrow(DiffCollectionWorktreeNotResolvedError);

    const missingWorktreeFixture = createFixture({
      records: recordsWith({
        tasks: [baseTask({ worktreeId: 'worktree-missing' })],
        worktrees: [],
      }),
      provider,
    });

    await expect(
      collectTaskDiff(missingWorktreeFixture.service, {
        taskId,
      }),
    ).rejects.toThrow(DiffCollectionWorktreeNotFoundError);

    expect(provider.inputs).toEqual([]);
    expect(missingTaskFixture.artifactStore.snapshot()).toEqual([]);
    expect(mismatchedTaskRunFixture.artifactStore.snapshot()).toEqual([]);
    expect(noWorktreeFixture.artifactStore.snapshot()).toEqual([]);
    expect(missingWorktreeFixture.artifactStore.snapshot()).toEqual([]);
    expect(missingTaskFixture.eventStore.snapshot()).toEqual([]);
    expect(mismatchedTaskRunFixture.eventStore.snapshot()).toEqual([]);
    expect(noWorktreeFixture.eventStore.snapshot()).toEqual([]);
    expect(missingWorktreeFixture.eventStore.snapshot()).toEqual([]);
  });

  it('does not create artifacts or events when diff collection fails', async () => {
    const provider = new FakeGitDiffProvider(new Error('git diff failed'));
    const fixture = createFixture({ provider });

    await expect(
      collectTaskDiff(fixture.service, {
        taskId,
        taskRunId,
      }),
    ).rejects.toThrow('git diff failed');

    expect(provider.inputs).toEqual([{ worktreePath }]);
    expect(fixture.artifactStore.snapshot()).toEqual([]);
    expect(fixture.eventStore.snapshot()).toEqual([]);
  });
});

interface DiffFixture {
  service: DiffCollectionService;
  provider: FakeGitDiffProvider;
  artifactStore: InMemoryArtifactStore;
  eventStore: InMemoryEventStore;
}

function createFixture(
  input: {
    records?: DomainRecords;
    provider?: FakeGitDiffProvider;
  } = {},
): DiffFixture {
  const provider = input.provider ?? new FakeGitDiffProvider('');
  const artifactStore = new InMemoryArtifactStore(
    sequenceIds('artifact'),
    fixedClock('2026-07-02T10:03:00.000Z'),
  );
  const eventStore = new InMemoryEventStore(
    sequenceIds('event'),
    fixedClock('2026-07-02T10:04:00.000Z'),
  );

  return {
    service: {
      dashboardStore: new InMemoryOpenTaskDashboardStore(input.records ?? baseRecords()),
      artifactStore,
      eventStore,
      diffProvider: provider,
    },
    provider,
    artifactStore,
    eventStore,
  };
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

function baseRecords(): DomainRecords {
  return recordsWith({});
}

function recordsWith(overrides: Partial<DomainRecords>): DomainRecords {
  return {
    projects: [],
    repos: [],
    branches: [],
    worktrees: [baseWorktree()],
    conversations: [],
    tasks: [baseTask()],
    taskRuns: [baseTaskRun()],
    artifacts: [],
    validationRuns: [],
    events: [],
    ...overrides,
  };
}

function baseTask(overrides: Partial<Task> = {}): Task {
  return {
    id: taskId,
    projectId,
    repoId: 'repo-orchestrator',
    branchId: 'branch-worker',
    worktreeId,
    conversationIds: [],
    title: 'Collect diff',
    summary: 'Capture the worker worktree diff.',
    executionState: 'completed',
    attentionState: 'needs_review',
    priority: 'normal',
    createdAt: '2026-07-02T08:00:00.000Z',
    updatedAt: '2026-07-02T08:00:00.000Z',
    ...overrides,
  };
}

function baseTaskRun(overrides: Partial<TaskRun> = {}): TaskRun {
  return {
    id: taskRunId,
    taskId,
    worktreeId,
    executionState: 'completed',
    startedAt: '2026-07-02T09:00:00.000Z',
    completedAt: '2026-07-02T09:30:00.000Z',
    createdAt: '2026-07-02T09:00:00.000Z',
    updatedAt: '2026-07-02T09:30:00.000Z',
    ...overrides,
  };
}

function baseWorktree(overrides: Partial<Worktree> = {}): Worktree {
  return {
    id: worktreeId,
    repoId: 'repo-orchestrator',
    branchId: 'branch-worker',
    path: worktreePath,
    isMain: false,
    isDirty: true,
    createdAt: '2026-07-02T08:30:00.000Z',
    updatedAt: '2026-07-02T08:45:00.000Z',
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
