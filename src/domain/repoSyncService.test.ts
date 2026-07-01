import type { GitRepoScanDomainFacts } from '../infrastructure/git/types';
import type { DomainRecords } from './model';
import type { RepoSyncPlanIdProvider } from './repoSyncPlanApplier';
import { syncRepoFromScan } from './repoSyncService';

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

describe('syncRepoFromScan', () => {
  it('returns the generated plan alongside inserted repo, branches, and worktrees', () => {
    const facts: GitRepoScanDomainFacts = {
      repo: {
        name: 'Codex Orchestrator',
        rootPath: 'C:\\Repos\\Codex Orchestrator',
        defaultBranch: 'main',
        remoteUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
      },
      branches: [
        {
          name: 'main',
          headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          isCurrent: true,
        },
        {
          name: 'worker/007-repo-sync-service',
          headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
          isCurrent: false,
          worktreePath: 'C:\\Repos\\Codex Orchestrator Worktrees\\007',
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
        {
          path: 'C:\\Repos\\Codex Orchestrator Worktrees\\007',
          branchName: 'worker/007-repo-sync-service',
          headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
          isMain: false,
          dirtyState: 'dirty',
          isDirty: true,
          isBare: false,
          isDetached: false,
          isLocked: true,
          lockReason: 'active worker',
          isPrunable: false,
          lastScannedAt: now,
        },
      ],
    };

    const result = syncRepoFromScan({
      records: emptyRecords(),
      projectId,
      facts,
      plannedAt: new Date(now),
      ids: deterministicIds(),
    });

    expect(result.plan.repo.action).toBe('insert');
    expect(result.plan.repo.match.rootPath).toBe('C:/Repos/Codex Orchestrator');
    expect(result.applied.records.repos).toEqual([
      {
        id: 'repo:C:/Repos/Codex Orchestrator',
        projectId,
        name: 'Codex Orchestrator',
        rootPath: 'C:/Repos/Codex Orchestrator',
        defaultBranch: 'main',
        remoteUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
        createdAt: now,
        updatedAt: now,
      },
    ]);
    expect(result.applied.records.branches.map((branch) => branch.id)).toEqual([
      'branch:main',
      'branch:worker/007-repo-sync-service',
    ]);
    expect(result.applied.records.worktrees.map((worktree) => worktree.branchId)).toEqual([
      'branch:main',
      'branch:worker/007-repo-sync-service',
    ]);
    expect(result.applied.records.worktrees[1]).toMatchObject({
      id: 'worktree:C:/Repos/Codex Orchestrator Worktrees/007',
      repoId: 'repo:C:/Repos/Codex Orchestrator',
      isDirty: true,
      lockReason: 'active worker',
    });
    expect(result.applied.changes.map((change) => `${change.kind}:${change.action}`)).toEqual([
      'repo:insert',
      'branch:insert',
      'branch:insert',
      'worktree:insert',
      'worktree:insert',
    ]);
  });

  it('updates existing records while preserving unrelated records and app-owned branch fields', () => {
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

    const result = syncRepoFromScan({
      records,
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
            lockReason: 'maintenance',
            isPrunable: false,
            lastScannedAt: now,
          },
        ],
      },
    });

    expect(result.plan.repo.action).toBe('update');
    expect(result.applied.records.tasks).toBe(records.tasks);
    expect(result.applied.records.repos[0]).toMatchObject({
      id: 'repo-1',
      name: 'Codex Orchestrator',
      defaultBranch: 'main',
      remoteUrl: 'git@github.com:new/remote.git',
      createdAt: yesterday,
      updatedAt: now,
    });
    expect(result.applied.records.branches[0]).toMatchObject({
      id: 'branch-1',
      baseBranch: 'trunk',
      headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      intent: 'Preserve app notes',
      createdAt: yesterday,
      updatedAt: now,
    });
    expect(result.applied.records.worktrees[0]).toMatchObject({
      id: 'worktree-1',
      branchId: 'branch-1',
      isDirty: true,
      lockReason: 'maintenance',
      lastScannedAt: now,
    });
  });

  it('clears omitted worktree lock and branch through the composed service path', () => {
    const records = recordsWithExistingRepo();

    const result = syncRepoFromScan({
      records,
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
    expect(result.applied.records.worktrees[0].lockReason).toBeUndefined();
    expect(result.applied.records.worktrees[0].branchId).toBeUndefined();
  });

  it('reports missing scan worktrees without deleting existing records', () => {
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

    const result = syncRepoFromScan({
      records,
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
      },
    });

    expect(result.applied.records.worktrees).toHaveLength(2);
    expect(result.applied.records.worktrees[1]).toBe(records.worktrees[1]);
    expect(result.plan.staleWorktrees).toHaveLength(1);
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
  });

  it('does not invent main when default branch facts are missing', () => {
    const result = syncRepoFromScan({
      records: emptyRecords(),
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

    expect(result.plan.repo.values.defaultBranch).toBeUndefined();
    expect(result.applied.records.repos[0]).toEqual({
      id: 'repo:C:/Repos/Detached Only',
      projectId,
      name: 'Detached Only',
      rootPath: 'C:/Repos/Detached Only',
      createdAt: now,
      updatedAt: now,
    });
  });
});

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
