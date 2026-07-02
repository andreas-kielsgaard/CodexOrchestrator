import type { DomainRecords } from '../../domain/model';
import type {
  RepoSyncStore,
  RepoSyncStoreLoadInput,
  RepoSyncStorePersistInput,
} from '../../domain/repoSyncStore';
import { normalizeDomainPath } from '../../domain/repoSyncPlanning';
import {
  branchFromRow,
  branchToRow,
  projectFromRow,
  repoFromRow,
  repoToRow,
  type BranchRow,
  type ProjectRow,
  type RepoRow,
  type WorktreeRow,
  worktreeFromRow,
  worktreeToRow,
} from './repoSyncSchema';

export interface SqliteStatement {
  all(...params: unknown[]): unknown[];
  get(...params: unknown[]): unknown | undefined;
  run(...params: unknown[]): unknown;
}

export interface RepoSyncSqliteDatabase {
  prepare(sql: string): SqliteStatement;
  exec?(sql: string): unknown;
}

export class SqliteRepoSyncStore implements RepoSyncStore {
  constructor(private readonly db: RepoSyncSqliteDatabase) {}

  async loadRepoSyncRecords(input: RepoSyncStoreLoadInput): Promise<DomainRecords> {
    const rootPath = normalizeDomainPath(input.rootPath);
    const projectRow = this.db
      .prepare('SELECT * FROM projects WHERE id = ?')
      .get(input.projectId) as ProjectRow | undefined;
    const repoRow = this.db
      .prepare('SELECT * FROM repos WHERE project_id = ? AND root_path = ?')
      .get(input.projectId, rootPath) as RepoRow | undefined;

    if (repoRow === undefined) {
      return emptyDomainRecords({
        projects: projectRow === undefined ? [] : [projectFromRow(projectRow)],
      });
    }

    const branchRows = this.db
      .prepare('SELECT * FROM branches WHERE repo_id = ? ORDER BY name')
      .all(repoRow.id) as BranchRow[];
    const worktreeRows = this.db
      .prepare('SELECT * FROM worktrees WHERE repo_id = ? ORDER BY path')
      .all(repoRow.id) as WorktreeRow[];

    return emptyDomainRecords({
      projects: projectRow === undefined ? [] : [projectFromRow(projectRow)],
      repos: [repoFromRow(repoRow)],
      branches: branchRows.map(branchFromRow),
      worktrees: worktreeRows.map(worktreeFromRow),
    });
  }

  async persistRepoSyncRecords(input: RepoSyncStorePersistInput): Promise<void> {
    this.runInTransaction(() => {
      for (const repo of input.records.repos) {
        const row = repoToRow(repo);
        this.db
          .prepare(
            `
INSERT INTO repos (
  id, project_id, name, root_path, default_branch, remote_url, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(id) DO UPDATE SET
  project_id = excluded.project_id,
  name = excluded.name,
  root_path = excluded.root_path,
  default_branch = excluded.default_branch,
  remote_url = excluded.remote_url,
  created_at = excluded.created_at,
  updated_at = excluded.updated_at
`,
          )
          .run(
            row.id,
            row.project_id,
            row.name,
            row.root_path,
            row.default_branch,
            row.remote_url,
            row.created_at,
            row.updated_at,
          );
      }

      for (const branch of input.records.branches) {
        const row = branchToRow(branch);
        this.db
          .prepare(
            `
INSERT INTO branches (
  id, repo_id, name, base_branch, head_sha, intent, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(id) DO UPDATE SET
  repo_id = excluded.repo_id,
  name = excluded.name,
  base_branch = excluded.base_branch,
  head_sha = excluded.head_sha,
  intent = excluded.intent,
  created_at = excluded.created_at,
  updated_at = excluded.updated_at
`,
          )
          .run(
            row.id,
            row.repo_id,
            row.name,
            row.base_branch,
            row.head_sha,
            row.intent,
            row.created_at,
            row.updated_at,
          );
      }

      for (const worktree of input.records.worktrees) {
        const row = worktreeToRow(worktree);
        this.db
          .prepare(
            `
INSERT INTO worktrees (
  id, repo_id, branch_id, path, is_main, is_dirty, lock_reason, last_scanned_at, created_at,
  updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
ON CONFLICT(id) DO UPDATE SET
  repo_id = excluded.repo_id,
  branch_id = excluded.branch_id,
  path = excluded.path,
  is_main = excluded.is_main,
  is_dirty = excluded.is_dirty,
  lock_reason = excluded.lock_reason,
  last_scanned_at = excluded.last_scanned_at,
  created_at = excluded.created_at,
  updated_at = excluded.updated_at
`,
          )
          .run(
            row.id,
            row.repo_id,
            row.branch_id,
            row.path,
            row.is_main,
            row.is_dirty,
            row.lock_reason,
            row.last_scanned_at,
            row.created_at,
            row.updated_at,
          );
      }
    });
  }

  private runInTransaction(write: () => void): void {
    if (this.db.exec === undefined) {
      write();
      return;
    }

    this.db.exec('BEGIN');
    try {
      write();
      this.db.exec('COMMIT');
    } catch (error) {
      this.db.exec('ROLLBACK');
      throw error;
    }
  }
}

function emptyDomainRecords(overrides: Partial<DomainRecords> = {}): DomainRecords {
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
    ...overrides,
  };
}
