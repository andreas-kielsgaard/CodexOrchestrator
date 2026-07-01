import type { DomainRecords, EntityId } from './model';
import { applyRepoSyncPlan, type RepoSyncPlanIdProvider } from './repoSyncPlanApplier';
import type { RepoSyncPlan } from './repoSyncPlanning';

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

describe('applyRepoSyncPlan', () => {
  it('inserts a new repo, branches, and worktrees with deterministic IDs and resolved links', () => {
    const plan: RepoSyncPlan = {
      projectId,
      plannedAt: now,
      repo: {
        action: 'insert',
        match: {
          projectId,
          rootPath: 'C:/Repos/Codex Orchestrator',
        },
        ref: {
          kind: 'planned',
          projectId,
          rootPath: 'C:/Repos/Codex Orchestrator',
        },
        values: {
          projectId,
          name: 'Codex Orchestrator',
          rootPath: 'C:/Repos/Codex Orchestrator',
          defaultBranch: 'main',
          remoteUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
          createdAt: now,
          updatedAt: now,
        },
      },
      branches: [
        {
          action: 'insert',
          match: {
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            name: 'main',
          },
          ref: {
            kind: 'planned',
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            name: 'main',
          },
          values: {
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            name: 'main',
            headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            createdAt: now,
            updatedAt: now,
          },
        },
        {
          action: 'insert',
          match: {
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            name: 'worker/006-repo-sync-plan-applier',
          },
          ref: {
            kind: 'planned',
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            name: 'worker/006-repo-sync-plan-applier',
          },
          values: {
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            name: 'worker/006-repo-sync-plan-applier',
            headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
            createdAt: now,
            updatedAt: now,
          },
        },
      ],
      worktrees: [
        {
          action: 'insert',
          match: {
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            path: 'C:/Repos/Codex Orchestrator',
          },
          branchRef: {
            kind: 'planned',
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            name: 'main',
          },
          values: {
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            path: 'C:/Repos/Codex Orchestrator',
            isMain: true,
            isDirty: false,
            lockReason: null,
            lastScannedAt: now,
            branchRef: {
              kind: 'planned',
              repo: {
                kind: 'planned',
                projectId,
                rootPath: 'C:/Repos/Codex Orchestrator',
              },
              name: 'main',
            },
            createdAt: now,
            updatedAt: now,
          },
        },
        {
          action: 'insert',
          match: {
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            path: 'C:/Repos/Codex Orchestrator Worktrees/006',
          },
          branchRef: {
            kind: 'planned',
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            name: 'worker/006-repo-sync-plan-applier',
          },
          values: {
            repo: {
              kind: 'planned',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
            },
            path: 'C:/Repos/Codex Orchestrator Worktrees/006',
            isMain: false,
            isDirty: true,
            lockReason: 'active worker',
            lastScannedAt: now,
            branchRef: {
              kind: 'planned',
              repo: {
                kind: 'planned',
                projectId,
                rootPath: 'C:/Repos/Codex Orchestrator',
              },
              name: 'worker/006-repo-sync-plan-applier',
            },
            createdAt: now,
            updatedAt: now,
          },
        },
      ],
      staleWorktrees: [],
    };

    const result = applyRepoSyncPlan({
      records: emptyRecords(),
      plan,
      ids: deterministicIds(),
    });

    expect(result.records.repos).toEqual([
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
    expect(result.records.branches.map((branch) => branch.id)).toEqual([
      'branch:main',
      'branch:worker/006-repo-sync-plan-applier',
    ]);
    expect(result.records.branches.map((branch) => branch.repoId)).toEqual([
      'repo:C:/Repos/Codex Orchestrator',
      'repo:C:/Repos/Codex Orchestrator',
    ]);
    expect(result.records.worktrees.map((worktree) => worktree.branchId)).toEqual([
      'branch:main',
      'branch:worker/006-repo-sync-plan-applier',
    ]);
    expect(result.records.worktrees[1]).toMatchObject({
      id: 'worktree:C:/Repos/Codex Orchestrator Worktrees/006',
      repoId: 'repo:C:/Repos/Codex Orchestrator',
      lockReason: 'active worker',
    });
  });

  it('updates existing Git-owned fields while preserving unrelated records and app-owned fields', () => {
    const records = emptyRecords();
    records.projects = [
      {
        id: projectId,
        name: 'Project',
        createdAt: yesterday,
        updatedAt: yesterday,
      },
    ];
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
        lockReason: 'old lock',
        lastScannedAt: yesterday,
        createdAt: yesterday,
        updatedAt: yesterday,
      },
    ];
    const plan: RepoSyncPlan = {
      projectId,
      plannedAt: now,
      repo: {
        action: 'update',
        match: {
          projectId,
          rootPath: 'C:/Repos/Codex Orchestrator',
        },
        ref: {
          kind: 'existing',
          projectId,
          rootPath: 'C:/Repos/Codex Orchestrator',
          id: 'repo-1',
        },
        existing: records.repos[0],
        values: {
          projectId,
          name: 'Codex Orchestrator',
          rootPath: 'C:/Repos/Codex Orchestrator',
          remoteUrl: 'git@github.com:new/remote.git',
          updatedAt: now,
        },
      },
      branches: [
        {
          action: 'update',
          match: {
            repo: {
              kind: 'existing',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
              id: 'repo-1',
            },
            name: 'main',
          },
          ref: {
            kind: 'existing',
            repo: {
              kind: 'existing',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
              id: 'repo-1',
            },
            name: 'main',
            id: 'branch-1',
          },
          existing: records.branches[0],
          values: {
            repo: {
              kind: 'existing',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
              id: 'repo-1',
            },
            name: 'main',
            headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
            updatedAt: now,
          },
        },
      ],
      worktrees: [
        {
          action: 'update',
          match: {
            repo: {
              kind: 'existing',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
              id: 'repo-1',
            },
            path: 'C:/Repos/Codex Orchestrator',
          },
          existing: records.worktrees[0],
          branchRef: {
            kind: 'existing',
            repo: {
              kind: 'existing',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
              id: 'repo-1',
            },
            name: 'main',
            id: 'branch-1',
          },
          values: {
            repo: {
              kind: 'existing',
              projectId,
              rootPath: 'C:/Repos/Codex Orchestrator',
              id: 'repo-1',
            },
            path: 'C:/Repos/Codex Orchestrator',
            isMain: true,
            isDirty: true,
            lockReason: 'maintenance',
            lastScannedAt: now,
            branchRef: {
              kind: 'existing',
              repo: {
                kind: 'existing',
                projectId,
                rootPath: 'C:/Repos/Codex Orchestrator',
                id: 'repo-1',
              },
              name: 'main',
              id: 'branch-1',
            },
            updatedAt: now,
          },
        },
      ],
      staleWorktrees: [],
    };

    const result = applyRepoSyncPlan({
      records,
      plan,
      ids: deterministicIds(),
    });

    expect(result.records.projects).toBe(records.projects);
    expect(result.records.tasks).toBe(records.tasks);
    expect(result.records.repos[0]).toMatchObject({
      id: 'repo-1',
      name: 'Codex Orchestrator',
      defaultBranch: 'main',
      remoteUrl: 'git@github.com:new/remote.git',
      createdAt: yesterday,
      updatedAt: now,
    });
    expect(result.records.branches[0]).toMatchObject({
      id: 'branch-1',
      baseBranch: 'trunk',
      headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
      intent: 'Preserve app notes',
      createdAt: yesterday,
      updatedAt: now,
    });
    expect(result.records.worktrees[0]).toMatchObject({
      id: 'worktree-1',
      isDirty: true,
      lockReason: 'maintenance',
      lastScannedAt: now,
      createdAt: yesterday,
      updatedAt: now,
    });
  });

  it('clears lockReason when the plan explicitly sets lockReason to null', () => {
    const records = recordsWithLinkedWorktree();
    const plan = worktreeUpdatePlan(records, {
      lockReason: null,
      branchRef: {
        kind: 'existing',
        repo: existingRepoRef(),
        name: 'main',
        id: 'branch-1',
      },
    });

    const result = applyRepoSyncPlan({
      records,
      plan,
      ids: deterministicIds(),
    });

    expect(result.records.worktrees[0].lockReason).toBeUndefined();
    expect(result.records.worktrees[0].branchId).toBe('branch-1');
  });

  it('clears branchId when the plan explicitly sets branchRef to null', () => {
    const records = recordsWithLinkedWorktree();
    const plan = worktreeUpdatePlan(records, {
      lockReason: 'active worker',
      branchRef: null,
    });

    const result = applyRepoSyncPlan({
      records,
      plan,
      ids: deterministicIds(),
    });

    expect(result.records.worktrees[0].branchId).toBeUndefined();
    expect(result.records.worktrees[0].lockReason).toBe('active worker');
  });

  it('reports stale worktrees without deleting or mutating them', () => {
    const records = recordsWithLinkedWorktree();
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
    const plan = worktreeUpdatePlan(records, {
      lockReason: null,
      branchRef: null,
      staleWorktreeIds: ['worktree-stale'],
    });

    const result = applyRepoSyncPlan({
      records,
      plan,
      ids: deterministicIds(),
    });

    expect(result.records.worktrees).toHaveLength(2);
    expect(result.records.worktrees[1]).toBe(records.worktrees[1]);
    expect(result.staleWorktrees).toEqual([
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

  it('does not invent main when a new repo plan omits defaultBranch', () => {
    const plan: RepoSyncPlan = {
      projectId,
      plannedAt: now,
      repo: {
        action: 'insert',
        match: {
          projectId,
          rootPath: 'C:/Repos/Detached Only',
        },
        ref: {
          kind: 'planned',
          projectId,
          rootPath: 'C:/Repos/Detached Only',
        },
        values: {
          projectId,
          name: 'Detached Only',
          rootPath: 'C:/Repos/Detached Only',
          createdAt: now,
          updatedAt: now,
        },
      },
      branches: [],
      worktrees: [],
      staleWorktrees: [],
    };

    const result = applyRepoSyncPlan({
      records: emptyRecords(),
      plan,
      ids: deterministicIds(),
    });

    expect(result.records.repos[0]).toEqual({
      id: 'repo:C:/Repos/Detached Only',
      projectId,
      name: 'Detached Only',
      rootPath: 'C:/Repos/Detached Only',
      createdAt: now,
      updatedAt: now,
    });
  });
});

function recordsWithLinkedWorktree(): DomainRecords {
  const records = emptyRecords();
  records.repos = [
    {
      id: 'repo-1',
      projectId,
      name: 'Codex Orchestrator',
      rootPath: 'C:/Repos/Codex Orchestrator',
      defaultBranch: 'main',
      createdAt: yesterday,
      updatedAt: yesterday,
    },
  ];
  records.branches = [
    {
      id: 'branch-1',
      repoId: 'repo-1',
      name: 'main',
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

function worktreeUpdatePlan(
  records: DomainRecords,
  options: {
    lockReason: string | null;
    branchRef: RepoSyncPlan['worktrees'][number]['branchRef'];
    staleWorktreeIds?: EntityId[];
  },
): RepoSyncPlan {
  return {
    projectId,
    plannedAt: now,
    repo: {
      action: 'update',
      match: {
        projectId,
        rootPath: 'C:/Repos/Codex Orchestrator',
      },
      ref: existingRepoRef(),
      existing: records.repos[0],
      values: {
        projectId,
        name: 'Codex Orchestrator',
        rootPath: 'C:/Repos/Codex Orchestrator',
        updatedAt: now,
      },
    },
    branches: [],
    worktrees: [
      {
        action: 'update',
        match: {
          repo: existingRepoRef(),
          path: 'C:/Repos/Codex Orchestrator',
        },
        existing: records.worktrees[0],
        branchRef: options.branchRef,
        values: {
          repo: existingRepoRef(),
          path: 'C:/Repos/Codex Orchestrator',
          isMain: true,
          isDirty: false,
          lockReason: options.lockReason,
          lastScannedAt: now,
          branchRef: options.branchRef,
          updatedAt: now,
        },
      },
    ],
    staleWorktrees: (options.staleWorktreeIds ?? []).map((id) => {
      const existing = records.worktrees.find((worktree) => worktree.id === id);

      if (existing === undefined) {
        throw new Error(`Missing stale worktree fixture ${id}`);
      }

      return {
        action: 'mark_missing_from_scan',
        existing,
        repo: existingRepoRef(),
        reason: 'absent_from_current_git_scan',
        lastObservedAt: existing.lastScannedAt,
        plannedAt: now,
      };
    }),
  };
}

function existingRepoRef() {
  return {
    kind: 'existing' as const,
    projectId,
    rootPath: 'C:/Repos/Codex Orchestrator',
    id: 'repo-1',
  };
}
