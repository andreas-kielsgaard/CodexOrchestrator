import type { Branch, Project, Repo, Worktree } from '../../domain/model';

export interface SqliteMigration {
  id: string;
  position: number;
  sql: string;
}

export interface SqliteMigrationDatabase {
  exec(sql: string): unknown;
}

export const repoSyncSqliteMigrations: SqliteMigration[] = [
  {
    id: '001_repo_sync_schema',
    position: 0,
    sql: `
CREATE TABLE IF NOT EXISTS projects (
  id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  description TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS repos (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  name TEXT NOT NULL,
  root_path TEXT NOT NULL,
  default_branch TEXT,
  remote_url TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
  UNIQUE (project_id, root_path)
);

CREATE TABLE IF NOT EXISTS branches (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL,
  name TEXT NOT NULL,
  base_branch TEXT,
  head_sha TEXT,
  intent TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE CASCADE,
  UNIQUE (repo_id, name)
);

CREATE TABLE IF NOT EXISTS worktrees (
  id TEXT PRIMARY KEY,
  repo_id TEXT NOT NULL,
  branch_id TEXT,
  path TEXT NOT NULL,
  is_main INTEGER NOT NULL CHECK (is_main IN (0, 1)),
  is_dirty INTEGER NOT NULL CHECK (is_dirty IN (0, 1)),
  lock_reason TEXT,
  last_scanned_at TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE CASCADE,
  FOREIGN KEY (branch_id) REFERENCES branches(id) ON DELETE SET NULL,
  UNIQUE (repo_id, path)
);
`,
  },
];

export function repoSyncSqliteSchemaSql(): string {
  return repoSyncSqliteMigrations.map((migration) => migration.sql).join('\n');
}

export function enableRepoSyncSqliteForeignKeys(db: SqliteMigrationDatabase): void {
  db.exec('PRAGMA foreign_keys = ON;');
}

export function applyRepoSyncSqliteMigrations(db: SqliteMigrationDatabase): void {
  for (const migration of repoSyncSqliteMigrations) {
    db.exec(migration.sql);
  }
}

export interface ProjectRow {
  id: string;
  name: string;
  description: string | null;
  created_at: string;
  updated_at: string;
}

export interface RepoRow {
  id: string;
  project_id: string;
  name: string;
  root_path: string;
  default_branch: string | null;
  remote_url: string | null;
  created_at: string;
  updated_at: string;
}

export interface BranchRow {
  id: string;
  repo_id: string;
  name: string;
  base_branch: string | null;
  head_sha: string | null;
  intent: string | null;
  created_at: string;
  updated_at: string;
}

export interface WorktreeRow {
  id: string;
  repo_id: string;
  branch_id: string | null;
  path: string;
  is_main: 0 | 1;
  is_dirty: 0 | 1;
  lock_reason: string | null;
  last_scanned_at: string | null;
  created_at: string;
  updated_at: string;
}

export function projectToRow(project: Project): ProjectRow {
  return {
    id: project.id,
    name: project.name,
    description: project.description ?? null,
    created_at: project.createdAt,
    updated_at: project.updatedAt,
  };
}

export function projectFromRow(row: ProjectRow): Project {
  return {
    id: row.id,
    name: row.name,
    ...(row.description === null ? {} : { description: row.description }),
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

export function repoToRow(repo: Repo): RepoRow {
  return {
    id: repo.id,
    project_id: repo.projectId,
    name: repo.name,
    root_path: repo.rootPath,
    default_branch: repo.defaultBranch ?? null,
    remote_url: repo.remoteUrl ?? null,
    created_at: repo.createdAt,
    updated_at: repo.updatedAt,
  };
}

export function repoFromRow(row: RepoRow): Repo {
  return {
    id: row.id,
    projectId: row.project_id,
    name: row.name,
    rootPath: row.root_path,
    ...(row.default_branch === null ? {} : { defaultBranch: row.default_branch }),
    ...(row.remote_url === null ? {} : { remoteUrl: row.remote_url }),
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

export function branchToRow(branch: Branch): BranchRow {
  return {
    id: branch.id,
    repo_id: branch.repoId,
    name: branch.name,
    base_branch: branch.baseBranch ?? null,
    head_sha: branch.headSha ?? null,
    intent: branch.intent ?? null,
    created_at: branch.createdAt,
    updated_at: branch.updatedAt,
  };
}

export function branchFromRow(row: BranchRow): Branch {
  return {
    id: row.id,
    repoId: row.repo_id,
    name: row.name,
    ...(row.base_branch === null ? {} : { baseBranch: row.base_branch }),
    ...(row.head_sha === null ? {} : { headSha: row.head_sha }),
    ...(row.intent === null ? {} : { intent: row.intent }),
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

export function worktreeToRow(worktree: Worktree): WorktreeRow {
  return {
    id: worktree.id,
    repo_id: worktree.repoId,
    branch_id: worktree.branchId ?? null,
    path: worktree.path,
    is_main: booleanToSqlite(worktree.isMain),
    is_dirty: booleanToSqlite(worktree.isDirty),
    lock_reason: worktree.lockReason ?? null,
    last_scanned_at: worktree.lastScannedAt ?? null,
    created_at: worktree.createdAt,
    updated_at: worktree.updatedAt,
  };
}

export function worktreeFromRow(row: WorktreeRow): Worktree {
  return {
    id: row.id,
    repoId: row.repo_id,
    ...(row.branch_id === null ? {} : { branchId: row.branch_id }),
    path: row.path,
    isMain: sqliteBoolean(row.is_main),
    isDirty: sqliteBoolean(row.is_dirty),
    ...(row.lock_reason === null ? {} : { lockReason: row.lock_reason }),
    ...(row.last_scanned_at === null ? {} : { lastScannedAt: row.last_scanned_at }),
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

function booleanToSqlite(value: boolean): 0 | 1 {
  return value ? 1 : 0;
}

function sqliteBoolean(value: 0 | 1): boolean {
  return value === 1;
}
