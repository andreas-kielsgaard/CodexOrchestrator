import type { DomainRecords } from './model';
import { planRepoSync } from './repoSyncPlanning';

const now = '2026-07-02T10:00:00.000Z';
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

describe('planRepoSync', () => {
  it('plans repo, branch, and worktree inserts for a newly discovered repo', () => {
    const plan = planRepoSync({
      records: emptyRecords(),
      projectId,
      plannedAt: new Date(now),
      facts: {
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
            name: 'worker/005-repo-sync-planning',
            headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
            isCurrent: false,
            worktreePath: 'C:\\Repos\\Codex Orchestrator Worktrees\\005',
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
            path: 'C:\\Repos\\Codex Orchestrator Worktrees\\005',
            branchName: 'worker/005-repo-sync-planning',
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
      },
    });

    expect(plan.repo.action).toBe('insert');
    expect(plan.repo.ref).toEqual({
      kind: 'planned',
      projectId,
      rootPath: 'C:/Repos/Codex Orchestrator',
    });
    expect(plan.repo.values).toMatchObject({
      projectId,
      name: 'Codex Orchestrator',
      rootPath: 'C:/Repos/Codex Orchestrator',
      defaultBranch: 'main',
      remoteUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
      createdAt: now,
      updatedAt: now,
    });
    expect(plan.branches.map((branch) => branch.action)).toEqual(['insert', 'insert']);
    expect(plan.worktrees).toHaveLength(2);
    expect(plan.worktrees[1]).toMatchObject({
      action: 'insert',
      branchRef: {
        kind: 'planned',
        name: 'worker/005-repo-sync-planning',
      },
      values: {
        path: 'C:/Repos/Codex Orchestrator Worktrees/005',
        isDirty: true,
        lockReason: 'active worker',
      },
    });
    expect(plan.staleWorktrees).toEqual([]);
  });

  it('plans existing repo updates for Git-owned fields', () => {
    const records = emptyRecords();
    records.repos = [
      {
        id: 'repo-1',
        projectId,
        name: 'Old Name',
        rootPath: 'C:/Repos/Codex Orchestrator',
        defaultBranch: 'main',
        remoteUrl: 'git@github.com:old/remote.git',
        createdAt: '2026-07-01T10:00:00.000Z',
        updatedAt: '2026-07-01T10:00:00.000Z',
      },
    ];
    records.branches = [
      {
        id: 'branch-1',
        repoId: 'repo-1',
        name: 'main',
        headSha: 'old-sha',
        createdAt: '2026-07-01T10:00:00.000Z',
        updatedAt: '2026-07-01T10:00:00.000Z',
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
        createdAt: '2026-07-01T10:00:00.000Z',
        updatedAt: '2026-07-01T10:00:00.000Z',
      },
    ];

    const plan = planRepoSync({
      records,
      projectId,
      plannedAt: now,
      facts: {
        repo: {
          name: 'Codex Orchestrator',
          rootPath: 'C:\\Repos\\Codex Orchestrator',
          defaultBranch: 'trunk',
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
            path: 'C:\\Repos\\Codex Orchestrator',
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

    expect(plan.repo).toMatchObject({
      action: 'update',
      ref: {
        kind: 'existing',
        id: 'repo-1',
      },
      values: {
        defaultBranch: 'trunk',
        remoteUrl: 'git@github.com:new/remote.git',
        updatedAt: now,
      },
    });
    expect(plan.branches[0]).toMatchObject({
      action: 'update',
      ref: {
        kind: 'existing',
        id: 'branch-1',
      },
      values: {
        headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      },
    });
    expect(plan.worktrees[0]).toMatchObject({
      action: 'update',
      branchRef: {
        kind: 'existing',
        id: 'branch-1',
      },
      values: {
        isDirty: true,
        lockReason: 'maintenance',
        lastScannedAt: now,
      },
    });
  });

  it('preserves app-owned branch intent and base branch when Git facts update the head SHA', () => {
    const records = emptyRecords();
    records.repos = [
      {
        id: 'repo-1',
        projectId,
        name: 'Codex Orchestrator',
        rootPath: 'C:/Repos/Codex Orchestrator',
        defaultBranch: 'main',
        createdAt: now,
        updatedAt: now,
      },
    ];
    records.branches = [
      {
        id: 'branch-1',
        repoId: 'repo-1',
        name: 'worker/005-repo-sync-planning',
        baseBranch: 'main',
        headSha: 'old-sha',
        intent: 'Plan repo sync before persistence exists',
        createdAt: now,
        updatedAt: now,
      },
    ];

    const plan = planRepoSync({
      records,
      projectId,
      plannedAt: now,
      facts: {
        repo: {
          name: 'Codex Orchestrator',
          rootPath: 'C:/Repos/Codex Orchestrator',
          defaultBranch: 'main',
        },
        branches: [
          {
            name: 'worker/005-repo-sync-planning',
            headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
            isCurrent: false,
          },
        ],
        worktrees: [],
      },
    });

    expect(plan.branches[0].values).toMatchObject({
      headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
      baseBranch: 'main',
      intent: 'Plan repo sync before persistence exists',
    });
  });

  it('links worktree updates to known or planned branches by branch name', () => {
    const records = emptyRecords();
    records.repos = [
      {
        id: 'repo-1',
        projectId,
        name: 'Codex Orchestrator',
        rootPath: 'C:/Repos/Codex Orchestrator',
        defaultBranch: 'main',
        createdAt: now,
        updatedAt: now,
      },
    ];
    records.branches = [
      {
        id: 'branch-1',
        repoId: 'repo-1',
        name: 'main',
        createdAt: now,
        updatedAt: now,
      },
    ];

    const plan = planRepoSync({
      records,
      projectId,
      plannedAt: now,
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
          {
            name: 'worker/new',
            isCurrent: false,
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
          {
            path: 'C:/Repos/Codex Orchestrator Worktrees/new',
            branchName: 'worker/new',
            isMain: false,
            dirtyState: 'unknown',
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

    expect(plan.worktrees[0].branchRef).toMatchObject({
      kind: 'existing',
      id: 'branch-1',
      name: 'main',
    });
    expect(plan.worktrees[1].branchRef).toMatchObject({
      kind: 'planned',
      name: 'worker/new',
    });
  });

  it('marks existing worktrees missing from the scan without planning destructive deletion', () => {
    const records = emptyRecords();
    records.repos = [
      {
        id: 'repo-1',
        projectId,
        name: 'Codex Orchestrator',
        rootPath: 'C:/Repos/Codex Orchestrator',
        defaultBranch: 'main',
        createdAt: now,
        updatedAt: now,
      },
    ];
    records.worktrees = [
      {
        id: 'worktree-1',
        repoId: 'repo-1',
        path: 'C:/Repos/Codex Orchestrator',
        isMain: true,
        isDirty: false,
        lastScannedAt: '2026-07-01T10:00:00.000Z',
        createdAt: now,
        updatedAt: now,
      },
      {
        id: 'worktree-stale',
        repoId: 'repo-1',
        path: 'C:\\Repos\\Codex Orchestrator Worktrees\\old',
        isMain: false,
        isDirty: true,
        lastScannedAt: '2026-07-01T09:00:00.000Z',
        createdAt: now,
        updatedAt: now,
      },
    ];

    const plan = planRepoSync({
      records,
      projectId,
      plannedAt: now,
      facts: {
        repo: {
          name: 'Codex Orchestrator',
          rootPath: 'C:/Repos/Codex Orchestrator',
          defaultBranch: 'main',
        },
        branches: [],
        worktrees: [
          {
            path: 'C:/Repos/Codex Orchestrator',
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

    expect(plan.staleWorktrees).toEqual([
      {
        action: 'mark_missing_from_scan',
        existing: records.worktrees[1],
        repo: {
          kind: 'existing',
          projectId,
          rootPath: 'C:/Repos/Codex Orchestrator',
          id: 'repo-1',
        },
        reason: 'absent_from_current_git_scan',
        lastObservedAt: '2026-07-01T09:00:00.000Z',
        plannedAt: now,
      },
    ]);
  });

  it('does not invent a main default branch when scan facts do not provide one', () => {
    const plan = planRepoSync({
      records: emptyRecords(),
      projectId,
      plannedAt: now,
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

    expect(plan.repo.values.defaultBranch).toBeUndefined();
  });
});
