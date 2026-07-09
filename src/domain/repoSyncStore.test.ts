import type { GitRepoScanDomainFacts } from './repoScanFacts';
import type { DomainRecords } from './model';
import type { RepoSyncPlanIdProvider } from './repoSyncPlanApplier';
import { InMemoryRepoSyncStore, syncRepoFromScanWithStore } from './repoSyncStore';

const now = '2026-07-02T10:00:00.000Z';
const yesterday = '2026-07-01T10:00:00.000Z';
const projectId = 'project-1';

const emptyRecords = (): DomainRecords => ({
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
});

const deterministicIds = (): RepoSyncPlanIdProvider => ({
  repoId: (plan) => `repo:${plan.match.rootPath}`,
  branchId: (plan) => `branch:${plan.match.name}`,
  worktreeId: (plan) => `worktree:${plan.match.path}`,
});

describe('syncRepoFromScanWithStore', () => {
  it('normalizes Windows scan root paths before loading store records', async () => {
    const store = new InMemoryRepoSyncStore(emptyRecords());

    const result = await syncRepoFromScanWithStore({
      store,
      projectId,
      plannedAt: now,
      ids: deterministicIds(),
      facts: {
        repo: {
          name: 'Codex Orchestrator',
          rootPath: 'C:\\Repos\\Codex Orchestrator',
          defaultBranch: 'main',
        },
        branches: [
          {
            name: 'main',
            headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            isCurrent: true,
          },
        ],
        worktrees: [
          {
            path: 'C:\\Repos\\Codex Orchestrator',
            branchName: 'main',
            headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            isMain: true,
            dirtyState: 'clean',
            isDirty: false,
            isBare: false,
            isDetached: false,
            isLocked: false,
            isPrunable: false,
            lastScannedAt: now,
          },
        ],
      },
    });

    expect(store.loadedInputs()).toEqual([
      {
        projectId,
        rootPath: 'C:/Repos/Codex Orchestrator',
      },
    ]);
    expect(result.plan.repo.match.rootPath).toBe('C:/Repos/Codex Orchestrator');
    expect(store.snapshot().repos[0]).toMatchObject({
      id: 'repo:C:/Repos/Codex Orchestrator',
      rootPath: 'C:/Repos/Codex Orchestrator',
      defaultBranch: 'main',
    });
    expect(store.snapshot().worktrees[0]).toMatchObject({
      id: 'worktree:C:/Repos/Codex Orchestrator',
      branchId: 'branch:main',
      path: 'C:/Repos/Codex Orchestrator',
    });
  });

  it('loads existing records, applies scan facts, and persists repo sync state', async () => {
    const records = recordsWithExistingRepo();
    const store = new InMemoryRepoSyncStore(records);

    const result = await syncRepoFromScanWithStore({
      store,
      projectId,
      plannedAt: now,
      ids: deterministicIds(),
      facts: {
        repo: {
          name: 'Codex Orchestrator',
          rootPath: 'C:/Repos/Codex Orchestrator',
          remoteUrl: 'git@github.com:new/remote.git',
        },
        branches: [
          {
            name: 'main',
            headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            isCurrent: true,
          },
        ],
        worktrees: [
          {
            path: 'C:/Repos/Codex Orchestrator',
            branchName: 'main',
            headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            isMain: true,
            dirtyState: 'dirty',
            isDirty: true,
            isBare: false,
            isDetached: false,
            isLocked: true,
            lockReason: 'active sync',
            isPrunable: false,
            lastScannedAt: now,
          },
        ],
      },
    });

    expect(result.plan.repo.action).toBe('update');
    expect(result.applied.changes.map((change) => `${change.kind}:${change.action}`)).toEqual([
      'repo:update',
      'branch:update',
      'worktree:update',
    ]);
    expect(store.persistedResults()).toHaveLength(1);
    expect(store.persistedResults()[0].result).toBe(result);
    expect(store.snapshot().repos[0]).toMatchObject({
      id: 'repo-1',
      name: 'Codex Orchestrator',
      defaultBranch: 'main',
      remoteUrl: 'git@github.com:new/remote.git',
      updatedAt: now,
    });
    expect(store.snapshot().branches[0]).toMatchObject({
      id: 'branch-1',
      baseBranch: 'trunk',
      headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      intent: 'Preserve app notes',
      updatedAt: now,
    });
    expect(store.snapshot().worktrees[0]).toMatchObject({
      id: 'worktree-1',
      branchId: 'branch-1',
      isDirty: true,
      lockReason: 'active sync',
      lastScannedAt: now,
    });
  });

  it('preserves unrelated domain records when persisting the applied sync records', async () => {
    const records = recordsWithExistingRepo();
    records.tasks = [
      {
        id: 'task-1',
        projectId,
        conversationIds: [],
        title: 'Keep me',
        summary: 'Unrelated task',
        executionState: 'draft',
        attentionState: 'consider_later',
        priority: 'normal',
        createdAt: yesterday,
        updatedAt: yesterday,
      },
    ];
    records.events = [
      {
        id: 'event-1',
        kind: 'task_created',
        occurredAt: yesterday,
        taskId: 'task-1',
        payload: {},
      },
    ];
    const store = new InMemoryRepoSyncStore(records);

    await syncRepoFromScanWithStore({
      store,
      projectId,
      plannedAt: now,
      ids: deterministicIds(),
      facts: oneMainWorktreeScan(),
    });

    expect(store.snapshot().tasks).toBe(records.tasks);
    expect(store.snapshot().events).toBe(records.events);
  });

  it('persists explicit worktree lock and branch clears', async () => {
    const store = new InMemoryRepoSyncStore(recordsWithExistingRepo());

    const result = await syncRepoFromScanWithStore({
      store,
      projectId,
      plannedAt: now,
      ids: deterministicIds(),
      facts: {
        repo: {
          name: 'Codex Orchestrator',
          rootPath: 'C:/Repos/Codex Orchestrator',
          defaultBranch: 'main',
        },
        branches: [
          {
            name: 'main',
            isCurrent: false,
          },
        ],
        worktrees: [
          {
            path: 'C:/Repos/Codex Orchestrator',
            isMain: true,
            dirtyState: 'clean',
            isDirty: false,
            isBare: false,
            isDetached: true,
            isLocked: false,
            isPrunable: false,
            lastScannedAt: now,
          },
        ],
      },
    });

    expect(result.plan.worktrees[0].values.lockReason).toBeNull();
    expect(result.plan.worktrees[0].values.branchRef).toBeNull();
    expect(store.snapshot().worktrees[0].lockReason).toBeUndefined();
    expect(store.snapshot().worktrees[0].branchId).toBeUndefined();
  });

  it('reports stale worktrees without deleting or mutating the stored worktree', async () => {
    const records = recordsWithExistingRepo();
    records.worktrees.push({
      id: 'worktree-stale',
      repoId: 'repo-1',
      path: 'C:/Repos/Codex Orchestrator Worktrees/old',
      isMain: false,
      isDirty: true,
      lastScannedAt: yesterday,
      createdAt: yesterday,
      updatedAt: yesterday,
    });
    const staleBeforeSync = records.worktrees[1];
    const store = new InMemoryRepoSyncStore(records);

    const result = await syncRepoFromScanWithStore({
      store,
      projectId,
      plannedAt: now,
      ids: deterministicIds(),
      facts: oneMainWorktreeScan(),
    });

    expect(result.applied.staleWorktrees).toEqual([
      {
        action: 'reported_missing_from_scan',
        worktreeId: 'worktree-stale',
        repoId: 'repo-1',
        path: 'C:/Repos/Codex Orchestrator Worktrees/old',
        reason: 'absent_from_current_git_scan',
        lastObservedAt: yesterday,
        plannedAt: now,
      },
    ]);
    expect(store.snapshot().worktrees).toHaveLength(2);
    expect(store.snapshot().worktrees[1]).toBe(staleBeforeSync);
  });

  it('does not invent a default branch when persisted scan facts do not provide one', async () => {
    const store = new InMemoryRepoSyncStore(emptyRecords());

    await syncRepoFromScanWithStore({
      store,
      projectId,
      plannedAt: now,
      ids: deterministicIds(),
      facts: {
        repo: {
          name: 'Detached Only',
          rootPath: 'C:/Repos/Detached Only',
        },
        branches: [],
        worktrees: [
          {
            path: 'C:/Repos/Detached Only',
            isMain: true,
            dirtyState: 'clean',
            isDirty: false,
            isBare: false,
            isDetached: true,
            isLocked: false,
            isPrunable: false,
            lastScannedAt: now,
          },
        ],
      },
    });

    expect(store.snapshot().repos[0]).toEqual({
      id: 'repo:C:/Repos/Detached Only',
      projectId,
      name: 'Detached Only',
      rootPath: 'C:/Repos/Detached Only',
      createdAt: now,
      updatedAt: now,
    });
  });
});

function oneMainWorktreeScan(): GitRepoScanDomainFacts {
  return {
    repo: {
      name: 'Codex Orchestrator',
      rootPath: 'C:/Repos/Codex Orchestrator',
      defaultBranch: 'main',
    },
    branches: [
      {
        name: 'main',
        isCurrent: true,
      },
    ],
    worktrees: [
      {
        path: 'C:/Repos/Codex Orchestrator',
        branchName: 'main',
        isMain: true,
        dirtyState: 'clean',
        isDirty: false,
        isBare: false,
        isDetached: false,
        isLocked: false,
        isPrunable: false,
        lastScannedAt: now,
      },
    ],
  };
}

function recordsWithExistingRepo(): DomainRecords {
  const records = emptyRecords();
  records.repos = [
    {
      id: 'repo-1',
      projectId,
      name: 'Old Name',
      rootPath: 'C:/Repos/Codex Orchestrator',
      defaultBranch: 'main',
      remoteUrl: 'git@github.com:old/remote.git',
      createdAt: yesterday,
      updatedAt: yesterday,
    },
  ];
  records.branches = [
    {
      id: 'branch-1',
      repoId: 'repo-1',
      name: 'main',
      baseBranch: 'trunk',
      headSha: 'old-sha',
      intent: 'Preserve app notes',
      createdAt: yesterday,
      updatedAt: yesterday,
    },
  ];
  records.worktrees = [
    {
      id: 'worktree-1',
      repoId: 'repo-1',
      branchId: 'branch-1',
      path: 'C:/Repos/Codex Orchestrator',
      isMain: true,
      isDirty: false,
      lockReason: 'active worker',
      lastScannedAt: yesterday,
      createdAt: yesterday,
      updatedAt: yesterday,
    },
  ];

  return records;
}
