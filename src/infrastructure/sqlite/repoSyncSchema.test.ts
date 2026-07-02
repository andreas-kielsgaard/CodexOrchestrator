import { DatabaseSync } from 'node:sqlite';

import type { Branch, Project, Repo, Worktree } from '../../domain/model';
import {
  branchFromRow,
  branchToRow,
  projectFromRow,
  projectToRow,
  repoFromRow,
  repoSyncSqliteMigrations,
  repoToRow,
  type BranchRow,
  type ProjectRow,
  type RepoRow,
  type WorktreeRow,
  worktreeFromRow,
  worktreeToRow,
} from './repoSyncSchema';

const now = '2026-07-02T10:00:00.000Z';

describe('repo sync SQLite schema', () => {
  it('applies ordered migrations to an in-memory SQLite database', () => {
    const db = openMigratedDatabase();

    try {
      const tables = db
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .all()
        .map((row) => (row as { name: string }).name);

      expect(tables).toEqual(['branches', 'projects', 'repos', 'worktrees']);
    } finally {
      db.close();
    }
  });

  it('enforces project, repo, branch, and worktree foreign keys', () => {
    const db = openMigratedDatabase();

    try {
      expect(() =>
        insertRow(db, 'repos', repoToRow(repo({ id: 'repo-without-project' }))),
      ).toThrow();

      insertRow(db, 'projects', projectToRow(project()));

      expect(() =>
        insertRow(db, 'branches', branchToRow(branch({ id: 'branch-without-repo' }))),
      ).toThrow();
      expect(() =>
        insertRow(db, 'worktrees', worktreeToRow(worktree({ id: 'worktree-without-repo' }))),
      ).toThrow();

      insertRow(db, 'repos', repoToRow(repo()));

      expect(() =>
        insertRow(
          db,
          'worktrees',
          worktreeToRow(worktree({ id: 'worktree-without-branch', branchId: 'missing-branch' })),
        ),
      ).toThrow();
    } finally {
      db.close();
    }
  });

  it('enforces repo root, branch name, and worktree path uniqueness within their parents', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);

      expect(() => insertRow(db, 'repos', repoToRow(repo({ id: 'repo-duplicate' })))).toThrow();
      expect(() =>
        insertRow(db, 'branches', branchToRow(branch({ id: 'branch-duplicate' }))),
      ).toThrow();
      expect(() =>
        insertRow(db, 'worktrees', worktreeToRow(worktree({ id: 'worktree-duplicate' }))),
      ).toThrow();

      insertRow(db, 'projects', projectToRow(project({ id: 'project-2' })));
      insertRow(db, 'repos', repoToRow(repo({ id: 'repo-2', projectId: 'project-2' })));
      insertRow(db, 'branches', branchToRow(branch({ id: 'branch-2', repoId: 'repo-2' })));
      insertRow(
        db,
        'worktrees',
        worktreeToRow(worktree({ id: 'worktree-2', repoId: 'repo-2', branchId: 'branch-2' })),
      );
    } finally {
      db.close();
    }
  });

  it('round-trips optional fields through NULL and booleans through stable integers', () => {
    const db = openMigratedDatabase();

    try {
      const projectRecord = project({ description: undefined });
      const repoRecord = repo({ defaultBranch: undefined, remoteUrl: undefined });
      const branchRecord = branch({ baseBranch: undefined, headSha: undefined, intent: undefined });
      const worktreeRecord = worktree({
        branchId: undefined,
        isMain: false,
        isDirty: true,
        lockReason: undefined,
        lastScannedAt: undefined,
      });

      insertRow(db, 'projects', projectToRow(projectRecord));
      insertRow(db, 'repos', repoToRow(repoRecord));
      insertRow(db, 'branches', branchToRow(branchRecord));
      insertRow(db, 'worktrees', worktreeToRow(worktreeRecord));

      const projectRow = selectOne<ProjectRow>(db, 'projects', 'project-1');
      const repoRow = selectOne<RepoRow>(db, 'repos', 'repo-1');
      const branchRow = selectOne<BranchRow>(db, 'branches', 'branch-main');
      const worktreeRow = selectOne<WorktreeRow>(db, 'worktrees', 'worktree-main');

      expect(projectRow.description).toBeNull();
      expect(repoRow.default_branch).toBeNull();
      expect(repoRow.remote_url).toBeNull();
      expect(branchRow.base_branch).toBeNull();
      expect(branchRow.head_sha).toBeNull();
      expect(branchRow.intent).toBeNull();
      expect(worktreeRow.branch_id).toBeNull();
      expect(worktreeRow.lock_reason).toBeNull();
      expect(worktreeRow.last_scanned_at).toBeNull();
      expect(worktreeRow.is_main).toBe(0);
      expect(worktreeRow.is_dirty).toBe(1);

      expect(projectFromRow(projectRow)).toEqual({
        id: projectRecord.id,
        name: projectRecord.name,
        createdAt: projectRecord.createdAt,
        updatedAt: projectRecord.updatedAt,
      });
      expect(repoFromRow(repoRow)).toEqual({
        id: repoRecord.id,
        projectId: repoRecord.projectId,
        name: repoRecord.name,
        rootPath: repoRecord.rootPath,
        createdAt: repoRecord.createdAt,
        updatedAt: repoRecord.updatedAt,
      });
      expect(branchFromRow(branchRow)).toEqual({
        id: branchRecord.id,
        repoId: branchRecord.repoId,
        name: branchRecord.name,
        createdAt: branchRecord.createdAt,
        updatedAt: branchRecord.updatedAt,
      });
      expect(worktreeFromRow(worktreeRow)).toEqual({
        id: worktreeRecord.id,
        repoId: worktreeRecord.repoId,
        path: worktreeRecord.path,
        isMain: worktreeRecord.isMain,
        isDirty: worktreeRecord.isDirty,
        createdAt: worktreeRecord.createdAt,
        updatedAt: worktreeRecord.updatedAt,
      });
    } finally {
      db.close();
    }
  });

  it('sets worktree branch links to NULL when a branch is deleted', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);

      db.prepare('DELETE FROM branches WHERE id = ?').run('branch-main');

      const worktreeRow = selectOne<WorktreeRow>(db, 'worktrees', 'worktree-main');
      expect(worktreeRow.branch_id).toBeNull();
      expect(worktreeFromRow(worktreeRow)).toEqual({
        id: 'worktree-main',
        repoId: 'repo-1',
        path: 'C:/Repos/Codex Orchestrator',
        isMain: true,
        isDirty: false,
        lockReason: 'active worker',
        lastScannedAt: now,
        createdAt: now,
        updatedAt: now,
      });
    } finally {
      db.close();
    }
  });
});

function openMigratedDatabase(): DatabaseSync {
  const db = new DatabaseSync(':memory:');
  db.exec('PRAGMA foreign_keys = ON;');
  for (const migration of repoSyncSqliteMigrations) {
    db.exec(migration.sql);
  }
  return db;
}

function insertProjectRepoBranchWorktree(db: DatabaseSync): void {
  insertRow(db, 'projects', projectToRow(project()));
  insertRow(db, 'repos', repoToRow(repo()));
  insertRow(db, 'branches', branchToRow(branch()));
  insertRow(db, 'worktrees', worktreeToRow(worktree()));
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

function project(overrides: Partial<Project> = {}): Project {
  return {
    id: 'project-1',
    name: 'Codex Orchestrator',
    description: 'Local control plane',
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}

function repo(overrides: Partial<Repo> = {}): Repo {
  return {
    id: 'repo-1',
    projectId: 'project-1',
    name: 'Codex Orchestrator',
    rootPath: 'C:/Repos/Codex Orchestrator',
    defaultBranch: 'main',
    remoteUrl: 'git@github.com:andreas-kielsgaard/CodexOrchestrator.git',
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}

function branch(overrides: Partial<Branch> = {}): Branch {
  return {
    id: 'branch-main',
    repoId: 'repo-1',
    name: 'main',
    baseBranch: 'trunk',
    headSha: 'aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa',
    intent: 'Primary branch',
    createdAt: now,
    updatedAt: now,
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
    lastScannedAt: now,
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}
