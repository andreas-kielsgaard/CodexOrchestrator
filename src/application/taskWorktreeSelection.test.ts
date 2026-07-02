import type { DomainRecords, Task } from '../domain/model';
import type { RepoSyncPlanIdProvider } from '../domain/repoSyncPlanApplier';
import { InMemoryRepoSyncStore } from '../domain/repoSyncStore';
import {
  InMemoryOpenTaskWriteStore,
  type IdProvider,
  type TimeProvider,
} from '../domain/openTaskWriteStore';
import type { GitRepoScanInput, GitRepoScanner } from '../infrastructure/git/gitAdapter';
import type { GitRepoScanResult } from '../infrastructure/git/types';
import {
  selectOrCreateTaskWorktree,
  TaskWorktreeNotFoundAfterScanError,
  TaskWorktreeTaskNotFoundError,
  type GitCreateWorktreeInput,
  type GitCreateWorktreeResult,
  type GitWorktreeCreator,
} from './taskWorktreeSelection';

const projectId = 'project-1';
const taskId = 'task-1';
const now = '2026-07-02T10:00:00.000Z';
const repoRootPath = 'C:/Repos/Codex Orchestrator';
const workerWorktreePath = 'C:/Repos/Codex Orchestrator Worktrees/031';
const workerBranchName = 'worker/031-task-worktree-service';

describe('selectOrCreateTaskWorktree', () => {
  it('preflights task existence before scanning or creating worktrees', async () => {
    const scanner = new FakeGitRepoScanner(repoScan());
    const creator = new FakeGitWorktreeCreator();

    await expect(
      selectOrCreateTaskWorktree(
        service({
          records: recordsWithTasks([]),
          scanner,
          creator,
        }),
        {
          taskId: 'task-missing',
          repoRootPath,
          worktree: {
            mode: 'create',
            worktreePath: workerWorktreePath,
            branchName: workerBranchName,
          },
        },
      ),
    ).rejects.toThrow(TaskWorktreeTaskNotFoundError);

    expect(creator.inputs).toEqual([]);
    expect(scanner.inputs).toEqual([]);
  });

  it('reports a typed error when the requested worktree is still absent after scan', async () => {
    const store = writeStore(recordsWithTasks([baseTask()]));

    await expect(
      selectOrCreateTaskWorktree(
        service({
          taskStore: store,
          repoSyncStore: new InMemoryRepoSyncStore(emptyRecords()),
          scanner: new FakeGitRepoScanner(
            repoScan({
              branches: repoScan().branches.filter((branch) => branch.name === 'main'),
              worktrees: repoScan().worktrees.filter((worktree) => worktree.branchName === 'main'),
            }),
          ),
        }),
        {
          taskId,
          repoRootPath,
          worktree: {
            mode: 'select',
            worktreePath: workerWorktreePath,
          },
        },
      ),
    ).rejects.toThrow(TaskWorktreeNotFoundAfterScanError);

    expect(store.snapshot().tasks[0]).toMatchObject({
      id: taskId,
      repoId: 'repo-old',
      branchId: 'branch-old',
      worktreeId: 'worktree-old',
    });
  });

  it('selects an existing scanned worktree by branch name and links the task anchors', async () => {
    const scanner = new FakeGitRepoScanner(repoScan());
    const store = writeStore(recordsWithTasks([baseTask()]));

    const result = await selectOrCreateTaskWorktree(
      service({
        taskStore: store,
        repoSyncStore: new InMemoryRepoSyncStore(emptyRecords()),
        scanner,
      }),
      {
        taskId,
        repoRootPath: 'C:\\Repos\\Codex Orchestrator',
        defaultBranch: 'main',
        scannedAt: now,
        worktree: {
          mode: 'select',
          branchName: workerBranchName,
        },
      },
    );

    expect(scanner.inputs).toEqual([
      {
        rootPath: 'C:\\Repos\\Codex Orchestrator',
        defaultBranch: 'main',
        scannedAt: now,
      },
    ]);
    expect(result.repo.id).toBe('repo:C:/Repos/Codex Orchestrator');
    expect(result.branch?.id).toBe('branch:worker/031-task-worktree-service');
    expect(result.worktree.id).toBe('worktree:C:/Repos/Codex Orchestrator Worktrees/031');
    expect(result.task).toMatchObject({
      id: taskId,
      repoId: 'repo:C:/Repos/Codex Orchestrator',
      branchId: 'branch:worker/031-task-worktree-service',
      worktreeId: 'worktree:C:/Repos/Codex Orchestrator Worktrees/031',
    });
    expect(result.scan.worktreeCount).toBe(2);
    expect(result.sync.changeCounts.worktree.insert).toBe(2);
  });

  it('creates through the injected boundary before scanning and selecting by path', async () => {
    const creator = new FakeGitWorktreeCreator({
      repoRootPath,
      worktreePath: workerWorktreePath,
      branchName: workerBranchName,
      baseBranch: 'main',
    });

    const result = await selectOrCreateTaskWorktree(
      service({
        records: recordsWithTasks([baseTask()]),
        scanner: new FakeGitRepoScanner(repoScan()),
        creator,
      }),
      {
        taskId,
        repoRootPath,
        worktree: {
          mode: 'create',
          worktreePath: 'C:\\Repos\\Codex Orchestrator Worktrees\\031',
          branchName: workerBranchName,
          baseBranch: 'main',
        },
      },
    );

    expect(creator.inputs).toEqual([
      {
        repoRootPath,
        worktreePath: 'C:\\Repos\\Codex Orchestrator Worktrees\\031',
        branchName: workerBranchName,
        baseBranch: 'main',
      },
    ]);
    expect(result.creation).toEqual({
      repoRootPath,
      worktreePath: workerWorktreePath,
      branchName: workerBranchName,
      baseBranch: 'main',
    });
    expect(result.worktree.path).toBe(workerWorktreePath);
    expect(result.task.worktreeId).toBe('worktree:C:/Repos/Codex Orchestrator Worktrees/031');
  });
});

class FakeGitRepoScanner implements GitRepoScanner {
  readonly inputs: GitRepoScanInput[] = [];

  constructor(private readonly scan: GitRepoScanResult) {}

  async scanRepo(input: GitRepoScanInput): Promise<GitRepoScanResult> {
    this.inputs.push(input);
    return this.scan;
  }
}

class FakeGitWorktreeCreator implements GitWorktreeCreator {
  readonly inputs: GitCreateWorktreeInput[] = [];

  constructor(private readonly result?: GitCreateWorktreeResult) {}

  async createWorktree(input: GitCreateWorktreeInput): Promise<GitCreateWorktreeResult> {
    this.inputs.push(input);
    return this.result ?? { ...input };
  }
}

function service(input: {
  records?: DomainRecords;
  taskStore?: InMemoryOpenTaskWriteStore;
  repoSyncStore?: InMemoryRepoSyncStore;
  scanner: GitRepoScanner;
  creator?: GitWorktreeCreator;
}) {
  const taskStore = input.taskStore ?? writeStore(input.records ?? recordsWithTasks([baseTask()]));

  return {
    dashboardStore: taskStore,
    taskWriteStore: taskStore,
    repoRegistry: {
      scanner: input.scanner,
      store: input.repoSyncStore ?? new InMemoryRepoSyncStore(emptyRecords()),
      ids: deterministicIds(),
      clock: fixedClock(now),
    },
    ...(input.creator === undefined ? {} : { worktreeCreator: input.creator }),
  };
}

function writeStore(records: DomainRecords): InMemoryOpenTaskWriteStore {
  const ids: IdProvider = {
    nextId: () => 'task-created',
  };
  const clock: TimeProvider = {
    now: () => '2026-07-02T11:00:00.000Z',
  };

  return new InMemoryOpenTaskWriteStore(records, ids, clock);
}

function deterministicIds(): RepoSyncPlanIdProvider {
  return {
    repoId: (plan) => `repo:${plan.match.rootPath}`,
    branchId: (plan) => `branch:${plan.match.name}`,
    worktreeId: (plan) => `worktree:${plan.match.path}`,
  };
}

function fixedClock(timestamp: string) {
  return {
    now: () => timestamp,
  };
}

function baseTask(overrides: Partial<Task> = {}): Task {
  return {
    id: taskId,
    projectId,
    repoId: 'repo-old',
    branchId: 'branch-old',
    worktreeId: 'worktree-old',
    conversationIds: [],
    title: 'Link task worktree',
    summary: 'Select or create a task worktree.',
    executionState: 'draft',
    attentionState: 'consider_later',
    priority: 'normal',
    createdAt: '2026-07-01T10:00:00.000Z',
    updatedAt: '2026-07-01T10:00:00.000Z',
    ...overrides,
  };
}

function repoScan(overrides: Partial<GitRepoScanResult> = {}): GitRepoScanResult {
  return {
    rootPath: repoRootPath,
    currentBranch: 'main',
    defaultBranch: 'main',
    remotes: [
      {
        name: 'origin',
        fetchUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
        pushUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
      },
    ],
    branches: [
      {
        name: 'main',
        headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        isCurrent: true,
        upstreamName: 'origin/main',
        worktreePath: repoRootPath,
      },
      {
        name: workerBranchName,
        headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        isCurrent: false,
        worktreePath: workerWorktreePath,
      },
    ],
    status: {
      isDirty: false,
      entries: [],
    },
    worktrees: [
      {
        path: repoRootPath,
        headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
        branchName: 'main',
        state: 'branch',
        isBare: false,
        isDetached: false,
        isLocked: false,
        isPrunable: false,
      },
      {
        path: workerWorktreePath,
        headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        branchName: workerBranchName,
        state: 'branch',
        isBare: false,
        isDetached: false,
        isLocked: false,
        isPrunable: false,
      },
    ],
    scannedAt: now,
    ...overrides,
  };
}

function recordsWithTasks(tasks: Task[]): DomainRecords {
  return {
    ...emptyRecords(),
    tasks,
  };
}

function emptyRecords(): DomainRecords {
  return {
    projects: [],
    repos: [],
    branches: [],
    worktrees: [],
    conversations: [],
    tasks: [],
    taskRuns: [],
    artifacts: [],
    validationRuns: [],
    events: [],
  };
}
