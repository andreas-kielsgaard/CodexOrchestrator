import type { DomainRecords, EntityId } from '../../domain/model';
import type { OpenTaskDashboardStore } from '../../domain/openTaskDashboardStore';
import {
  branchFromRow,
  projectFromRow,
  repoFromRow,
  type BranchRow,
  type ProjectRow,
  type RepoRow,
  type WorktreeRow,
  worktreeFromRow,
} from './repoSyncSchema';
import { taskFromRow, type TaskConversationLinkRow, type TaskRow } from './taskSchema';

export interface OpenTaskDashboardSqliteStatement {
  all(...params: unknown[]): unknown[];
}

export interface OpenTaskDashboardSqliteDatabase {
  prepare(sql: string): OpenTaskDashboardSqliteStatement;
}

export class SqliteOpenTaskDashboardStore implements OpenTaskDashboardStore {
  constructor(private readonly db: OpenTaskDashboardSqliteDatabase) {}

  async loadOpenTaskDashboardRecords(): Promise<DomainRecords> {
    const taskRows = this.db
      .prepare('SELECT * FROM tasks ORDER BY updated_at DESC, id')
      .all() as TaskRow[];
    const taskIds = taskRows.map((row) => row.id);
    const linkRows = this.selectTaskConversationLinks(taskIds);
    const tasks = taskRows.map((row) => taskFromRow(row, linkRows));

    const projectIds = unique(tasks.map((task) => task.projectId));
    const repoIds = unique(tasks.flatMap((task) => optionalId(task.repoId)));
    const branchIds = unique(tasks.flatMap((task) => optionalId(task.branchId)));
    const worktreeIds = unique(tasks.flatMap((task) => optionalId(task.worktreeId)));

    return emptyDomainRecords({
      projects: this.selectByIds<ProjectRow>('projects', projectIds).map(projectFromRow),
      repos: this.selectByIds<RepoRow>('repos', repoIds).map(repoFromRow),
      branches: this.selectByIds<BranchRow>('branches', branchIds).map(branchFromRow),
      worktrees: this.selectByIds<WorktreeRow>('worktrees', worktreeIds).map(worktreeFromRow),
      tasks,
    });
  }

  private selectTaskConversationLinks(taskIds: readonly EntityId[]): TaskConversationLinkRow[] {
    if (taskIds.length === 0) {
      return [];
    }

    return this.db
      .prepare(
        `
SELECT *
FROM task_conversation_links
WHERE task_id IN (${placeholders(taskIds)})
ORDER BY task_id, position
`,
      )
      .all(...taskIds) as TaskConversationLinkRow[];
  }

  private selectByIds<Row>(table: string, ids: readonly EntityId[]): Row[] {
    if (ids.length === 0) {
      return [];
    }

    return this.db
      .prepare(`SELECT * FROM ${table} WHERE id IN (${placeholders(ids)}) ORDER BY id`)
      .all(...ids) as Row[];
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

function optionalId(id: EntityId | undefined): EntityId[] {
  return id === undefined ? [] : [id];
}

function placeholders(values: readonly unknown[]): string {
  return values.map(() => '?').join(', ');
}

function unique(values: readonly EntityId[]): EntityId[] {
  return [...new Set(values)];
}
