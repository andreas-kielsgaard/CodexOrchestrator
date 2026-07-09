import { DatabaseSync } from 'node:sqlite';

import type { GitRepoScanDomainFacts } from '../../domain/repoScanFacts';
import type { Branch, Project, Repo, Worktree } from '../../domain/model';
import type { RepoSyncPlanIdProvider } from '../../domain/repoSyncPlanApplier';
import { syncRepoFromScanWithStore } from '../../domain/repoSyncStore';
import {
  applyRepoSyncSqliteMigrations,
  branchFromRow,
  branchToRow,
  enableRepoSyncSqliteForeignKeys,
  projectToRow,
  repoFromRow,
  repoToRow,
  type BranchRow,
  type RepoRow,
  type WorktreeRow,
  worktreeFromRow,
  worktreeToRow,
} from './repoSyncSchema';
import { SqliteRepoSyncStore } from './repoSyncStore';

const now = '2026-07-02T10:00:00.000Z';
const yesterday = '2026-07-01T10:00:00.000Z';
const projectId = 'project-1';

describe('SqliteRepoSyncStore', () => {
  it('inserts new repo, branch, and worktree records through repo sync', async () => {
    const db = openMigratedDatabase();

    try {
      insertRow(db, 'projects', projectToRow(project()));

      await syncRepoFromScanWithStore({
        store: new SqliteRepoSyncStore(db),
        projectId,
        plannedAt: now,
        ids: deterministicIds(),
        facts: oneMainWorktreeScan(),
      });

      expect(selectAll<RepoRow>(db, 'repos').map(repoFromRow)).toEqual([
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
      expect(selectAll<BranchRow>(db, 'branches').map(branchFromRow)).toEqual([
        {
          id: 'branch:main',
          repoId: 'repo:C:/Repos/Codex Orchestrator',
          name: 'main',
          headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
          createdAt: now,
          updatedAt: now,
        },
      ]);
      expect(selectAll<WorktreeRow>(db, 'worktrees').map(worktreeFromRow)).toEqual([
        {
          id: 'worktree:C:/Repos/Codex Orchestrator',
          repoId: 'repo:C:/Repos/Codex Orchestrator',
          branchId: 'branch:main',
          path: 'C:/Repos/Codex Orchestrator',
          isMain: true,
          isDirty: false,
          lastScannedAt: now,
          createdAt: now,
          updatedAt: now,
        },
      ]);
    } finally {
      db.close();
    }
  });

  it('updates an existing repo and preserves existing branch annotations', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);

      const result = await syncRepoFromScanWithStore({
        store: new SqliteRepoSyncStore(db),
        projectId,
        plannedAt: now,
        ids: deterministicIds(),
        facts: oneMainWorktreeScan({
          repo: {
            name: 'Codex Orchestrator Renamed',
            rootPath: 'C:/Repos/Codex Orchestrator',
            defaultBranch: 'main',
            remoteUrl: 'git@github.com:new/remote.git',
          },
          branches: [
            {
              name: 'main',
              headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
              isCurrent: true,
            },
          ],
          worktrees: [
            {
              path: 'C:/Repos/Codex Orchestrator',
              branchName: 'main',
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
        }),
      });

      expect(result.applied.changes.map((change) => `${change.kind}:${change.action}`)).toEqual([
        'repo:update',
        'branch:update',
        'worktree:update',
      ]);
      expect(selectOne<RepoRow>(db, 'repos', 'repo-1')).toMatchObject({
        name: 'Codex Orchestrator Renamed',
        remote_url: 'git@github.com:new/remote.git',
        updated_at: now,
      });
      expect(branchFromRow(selectOne<BranchRow>(db, 'branches', 'branch-main'))).toEqual({
        id: 'branch-main',
        repoId: 'repo-1',
        name: 'main',
        baseBranch: 'trunk',
        headSha: 'bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb',
        intent: 'Primary branch',
        createdAt: yesterday,
        updatedAt: now,
      });
      expect(worktreeFromRow(selectOne<WorktreeRow>(db, 'worktrees', 'worktree-main'))).toEqual({
        id: 'worktree-main',
        repoId: 'repo-1',
        branchId: 'branch-main',
        path: 'C:/Repos/Codex Orchestrator',
        isMain: true,
        isDirty: true,
        lockReason: 'active sync',
        lastScannedAt: now,
        createdAt: yesterday,
        updatedAt: now,
      });
    } finally {
      db.close();
    }
  });

  it('persists explicit worktree branch and lock clears as NULL', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);

      await syncRepoFromScanWithStore({
        store: new SqliteRepoSyncStore(db),
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

      const row = selectOne<WorktreeRow>(db, 'worktrees', 'worktree-main');
      expect(row.branch_id).toBeNull();
      expect(row.lock_reason).toBeNull();
      expect(worktreeFromRow(row)).toEqual({
        id: 'worktree-main',
        repoId: 'repo-1',
        path: 'C:/Repos/Codex Orchestrator',
        isMain: true,
        isDirty: false,
        lastScannedAt: now,
        createdAt: yesterday,
        updatedAt: now,
      });
    } finally {
      db.close();
    }
  });

  it('preserves stale worktrees and reports them without deletion', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertRow(
        db,
        'worktrees',
        worktreeToRow(
          worktree({
            id: 'worktree-stale',
            branchId: undefined,
            path: 'C:/Repos/Codex Orchestrator Worktrees/old',
            isMain: false,
            isDirty: true,
            lockReason: undefined,
            lastScannedAt: yesterday,
          }),
        ),
      );

      const result = await syncRepoFromScanWithStore({
        store: new SqliteRepoSyncStore(db),
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
      expect(selectAll<WorktreeRow>(db, 'worktrees')).toHaveLength(2);
      expect(worktreeFromRow(selectOne<WorktreeRow>(db, 'worktrees', 'worktree-stale'))).toEqual({
        id: 'worktree-stale',
        repoId: 'repo-1',
        path: 'C:/Repos/Codex Orchestrator Worktrees/old',
        isMain: false,
        isDirty: true,
        lastScannedAt: yesterday,
        createdAt: yesterday,
        updatedAt: yesterday,
      });
    } finally {
      db.close();
    }
  });

  it('keeps a missing default branch missing', async () => {
    const db = openMigratedDatabase();

    try {
      insertRow(db, 'projects', projectToRow(project()));

      await syncRepoFromScanWithStore({
        store: new SqliteRepoSyncStore(db),
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

      expect(selectOne<RepoRow>(db, 'repos', 'repo:C:/Repos/Detached Only')).toMatchObject({
        default_branch: null,
      });
    } finally {
      db.close();
    }
  });

  it('scopes loads to the requested repo and leaves unrelated repos untouched', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertUnrelatedRepo(db);

      const result = await syncRepoFromScanWithStore({
        store: new SqliteRepoSyncStore(db),
        projectId,
        plannedAt: now,
        ids: deterministicIds(),
        facts: oneMainWorktreeScan(),
      });

      expect(result.applied.staleWorktrees).toEqual([]);
      expect(selectAll<RepoRow>(db, 'repos')).toHaveLength(2);
      expect(repoFromRow(selectOne<RepoRow>(db, 'repos', 'repo-unrelated'))).toEqual({
        id: 'repo-unrelated',
        projectId,
        name: 'Other Repo',
        rootPath: 'C:/Repos/Other Repo',
        defaultBranch: 'main',
        createdAt: yesterday,
        updatedAt: yesterday,
      });
      expect(selectOne<WorktreeRow>(db, 'worktrees', 'worktree-unrelated')).toMatchObject({
        repo_id: 'repo-unrelated',
        path: 'C:/Repos/Other Repo',
      });
    } finally {
      db.close();
    }
  });

  it('rolls back all persisted rows when a transaction-backed upsert fails', async () => {
    const db = openMigratedDatabase();

    try {
      insertRow(db, 'projects', projectToRow(project()));

      await expect(
        new SqliteRepoSyncStore(db).persistRepoSyncRecords({
          records: {
            repos: [repo()],
            branches: [],
            worktrees: [worktree({ branchId: 'missing-branch' })],
          },
          result: {} as never,
        }),
      ).rejects.toThrow();

      expect(selectAll<RepoRow>(db, 'repos')).toEqual([]);
      expect(selectAll<WorktreeRow>(db, 'worktrees')).toEqual([]);
    } finally {
      db.close();
    }
  });
});

function openMigratedDatabase(): DatabaseSync {
  const db = new DatabaseSync(':memory:');
  enableRepoSyncSqliteForeignKeys(db);
  applyRepoSyncSqliteMigrations(db);
  return db;
}

function insertProjectRepoBranchWorktree(db: DatabaseSync): void {
  insertRow(db, 'projects', projectToRow(project()));
  insertRow(db, 'repos', repoToRow(repo()));
  insertRow(db, 'branches', branchToRow(branch()));
  insertRow(db, 'worktrees', worktreeToRow(worktree()));
}

function insertUnrelatedRepo(db: DatabaseSync): void {
  insertRow(
    db,
    'repos',
    repoToRow(
      repo({
        id: 'repo-unrelated',
        name: 'Other Repo',
        rootPath: 'C:/Repos/Other Repo',
        remoteUrl: undefined,
      }),
    ),
  );
  insertRow(
    db,
    'branches',
    branchToRow(
      branch({
        id: 'branch-unrelated',
        repoId: 'repo-unrelated',
      }),
    ),
  );
  insertRow(
    db,
    'worktrees',
    worktreeToRow(
      worktree({
        id: 'worktree-unrelated',
        repoId: 'repo-unrelated',
        branchId: 'branch-unrelated',
        path: 'C:/Repos/Other Repo',
      }),
    ),
  );
}

function insertRow(db: DatabaseSync, table: string, row: object): void {
  const entries = Object.entries(row);
  const columns = entries.map(([column]) => column);
  const placeholders = columns.map(() => '?').join(', ');
  const values = entries.map(([, value]) => value);

  db.prepare(`INSERT INTO ${table} (${columns.join(', ')}) VALUES (${placeholders})`).run(
    ...values,
  );
}

function selectOne<T>(db: DatabaseSync, table: string, id: string): T {
  const row = db.prepare(`SELECT * FROM ${table} WHERE id = ?`).get(id);

  if (row === undefined) {
    throw new Error(`Expected ${table} row ${id}`);
  }

  return row as T;
}

function selectAll<T>(db: DatabaseSync, table: string): T[] {
  return db.prepare(`SELECT * FROM ${table} ORDER BY id`).all() as T[];
}

function deterministicIds(): RepoSyncPlanIdProvider {
  return {
    repoId: (plan) => `repo:${plan.match.rootPath}`,
    branchId: (plan) => `branch:${plan.match.name}`,
    worktreeId: (plan) => `worktree:${plan.match.path}`,
  };
}

function oneMainWorktreeScan(
  overrides: Partial<GitRepoScanDomainFacts> = {},
): GitRepoScanDomainFacts {
  return {
    repo: {
      name: 'Codex Orchestrator',
      rootPath: 'C:/Repos/Codex Orchestrator',
      defaultBranch: 'main',
      remoteUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
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
    ...overrides,
  };
}

function project(overrides: Partial<Project> = {}): Project {
  return {
    id: projectId,
    name: 'Codex Orchestrator',
    createdAt: yesterday,
    updatedAt: yesterday,
    ...overrides,
  };
}

function repo(overrides: Partial<Repo> = {}): Repo {
  return {
    id: 'repo-1',
    projectId,
    name: 'Codex Orchestrator',
    rootPath: 'C:/Repos/Codex Orchestrator',
    defaultBranch: 'main',
    remoteUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
    createdAt: yesterday,
    updatedAt: yesterday,
    ...overrides,
  };
}

function branch(overrides: Partial<Branch> = {}): Branch {
  return {
    id: 'branch-main',
    repoId: 'repo-1',
    name: 'main',
    baseBranch: 'trunk',
    headSha: 'old-sha',
    intent: 'Primary branch',
    createdAt: yesterday,
    updatedAt: yesterday,
    ...overrides,
  };
}

function worktree(overrides: Partial<Worktree> = {}): Worktree {
  return {
    id: 'worktree-main',
    repoId: 'repo-1',
    branchId: 'branch-main',
    path: 'C:/Repos/Codex Orchestrator',
    isMain: true,
    isDirty: false,
    lockReason: 'active worker',
    lastScannedAt: yesterday,
    createdAt: yesterday,
    updatedAt: yesterday,
    ...overrides,
  };
}
