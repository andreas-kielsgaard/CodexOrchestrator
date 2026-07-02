import type { AttentionState, ExecutionState, Task } from '../../domain/model';
import type { SqliteMigration, SqliteMigrationDatabase } from './repoSyncSchema';

const executionStates = [
  'draft',
  'queued',
  'running',
  'blocked',
  'completed',
  'failed',
  'abandoned',
  'archived',
] as const satisfies readonly ExecutionState[];

const attentionStates = [
  'needs_action_now',
  'needs_review',
  'waiting_on_agent',
  'waiting_on_external',
  'consider_later',
  'snoozed',
  'reference_only',
] as const satisfies readonly AttentionState[];

const priorities = ['low', 'normal', 'high'] as const satisfies readonly Task['priority'][];

export const taskSqliteMigrations: SqliteMigration[] = [
  {
    id: '002_open_tasks_schema',
    sql: `
CREATE TABLE IF NOT EXISTS tasks (
  id TEXT PRIMARY KEY,
  project_id TEXT NOT NULL,
  repo_id TEXT,
  branch_id TEXT,
  worktree_id TEXT,
  title TEXT NOT NULL,
  summary TEXT NOT NULL,
  execution_state TEXT NOT NULL CHECK (execution_state IN (${sqlStringList(executionStates)})),
  attention_state TEXT NOT NULL CHECK (attention_state IN (${sqlStringList(attentionStates)})),
  priority TEXT NOT NULL CHECK (priority IN (${sqlStringList(priorities)})),
  due_at TEXT,
  snoozed_until TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE CASCADE,
  FOREIGN KEY (repo_id) REFERENCES repos(id) ON DELETE SET NULL,
  FOREIGN KEY (branch_id) REFERENCES branches(id) ON DELETE SET NULL,
  FOREIGN KEY (worktree_id) REFERENCES worktrees(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS task_conversation_links (
  task_id TEXT NOT NULL,
  conversation_id TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  created_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
  PRIMARY KEY (task_id, conversation_id),
  UNIQUE (task_id, position)
);
`,
  },
];

export function taskSqliteSchemaSql(): string {
  return taskSqliteMigrations.map((migration) => migration.sql).join('\n');
}

export function applyTaskSqliteMigrations(db: SqliteMigrationDatabase): void {
  for (const migration of taskSqliteMigrations) {
    db.exec(migration.sql);
  }
}

export interface TaskRow {
  id: string;
  project_id: string;
  repo_id: string | null;
  branch_id: string | null;
  worktree_id: string | null;
  title: string;
  summary: string;
  execution_state: ExecutionState;
  attention_state: AttentionState;
  priority: Task['priority'];
  due_at: string | null;
  snoozed_until: string | null;
  created_at: string;
  updated_at: string;
}

export interface TaskConversationLinkRow {
  task_id: string;
  conversation_id: string;
  position: number;
  created_at: string;
}

export function taskToRow(task: Task): TaskRow {
  return {
    id: task.id,
    project_id: task.projectId,
    repo_id: task.repoId ?? null,
    branch_id: task.branchId ?? null,
    worktree_id: task.worktreeId ?? null,
    title: task.title,
    summary: task.summary,
    execution_state: task.executionState,
    attention_state: task.attentionState,
    priority: task.priority,
    due_at: task.dueAt ?? null,
    snoozed_until: task.snoozedUntil ?? null,
    created_at: task.createdAt,
    updated_at: task.updatedAt,
  };
}

export function taskFromRow(
  row: TaskRow,
  conversationLinks: readonly TaskConversationLinkRow[] = [],
): Task {
  return {
    id: row.id,
    projectId: row.project_id,
    ...(row.repo_id === null ? {} : { repoId: row.repo_id }),
    ...(row.branch_id === null ? {} : { branchId: row.branch_id }),
    ...(row.worktree_id === null ? {} : { worktreeId: row.worktree_id }),
    conversationIds: [...conversationLinks]
      .filter((link) => link.task_id === row.id)
      .sort((left, right) => left.position - right.position)
      .map((link) => link.conversation_id),
    title: row.title,
    summary: row.summary,
    executionState: row.execution_state,
    attentionState: row.attention_state,
    priority: row.priority,
    ...(row.due_at === null ? {} : { dueAt: row.due_at }),
    ...(row.snoozed_until === null ? {} : { snoozedUntil: row.snoozed_until }),
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

export function taskConversationLinksToRows(
  task: Task,
  createdAt: string = task.createdAt,
): TaskConversationLinkRow[] {
  return task.conversationIds.map((conversationId, position) => ({
    task_id: task.id,
    conversation_id: conversationId,
    position,
    created_at: createdAt,
  }));
}

function sqlStringList(values: readonly string[]): string {
  return values.map((value) => `'${value}'`).join(', ');
}
