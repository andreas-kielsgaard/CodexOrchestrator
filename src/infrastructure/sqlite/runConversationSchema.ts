import type { Conversation, ExecutionState, TaskRun } from '../../domain/model';
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

const conversationProviders = [
  'codex',
  'chatgpt_export',
  'manual',
] as const satisfies readonly Conversation['provider'][];

export const runConversationSqliteMigrations: SqliteMigration[] = [
  {
    id: '003_task_runs_conversations_schema',
    position: 2,
    sql: `
CREATE TABLE IF NOT EXISTS task_runs (
  id TEXT PRIMARY KEY,
  task_id TEXT NOT NULL,
  conversation_id TEXT,
  worktree_id TEXT,
  execution_state TEXT NOT NULL CHECK (execution_state IN (${sqlStringList(executionStates)})),
  started_at TEXT,
  completed_at TEXT,
  exit_code INTEGER,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE CASCADE,
  FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL,
  FOREIGN KEY (worktree_id) REFERENCES worktrees(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS conversations (
  id TEXT PRIMARY KEY,
  task_id TEXT,
  task_run_id TEXT,
  provider TEXT NOT NULL CHECK (provider IN (${sqlStringList(conversationProviders)})),
  external_thread_id TEXT,
  title TEXT NOT NULL,
  summary TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
  FOREIGN KEY (task_run_id) REFERENCES task_runs(id) ON DELETE SET NULL
);
`,
  },
];

export function runConversationSqliteSchemaSql(): string {
  return runConversationSqliteMigrations.map((migration) => migration.sql).join('\n');
}

export function applyRunConversationSqliteMigrations(db: SqliteMigrationDatabase): void {
  for (const migration of runConversationSqliteMigrations) {
    db.exec(migration.sql);
  }
}

export interface TaskRunRow {
  id: string;
  task_id: string;
  conversation_id: string | null;
  worktree_id: string | null;
  execution_state: ExecutionState;
  started_at: string | null;
  completed_at: string | null;
  exit_code: number | null;
  created_at: string;
  updated_at: string;
}

export interface ConversationRow {
  id: string;
  task_id: string | null;
  task_run_id: string | null;
  provider: Conversation['provider'];
  external_thread_id: string | null;
  title: string;
  summary: string | null;
  created_at: string;
  updated_at: string;
}

export function taskRunToRow(taskRun: TaskRun): TaskRunRow {
  return {
    id: taskRun.id,
    task_id: taskRun.taskId,
    conversation_id: taskRun.conversationId ?? null,
    worktree_id: taskRun.worktreeId ?? null,
    execution_state: taskRun.executionState,
    started_at: taskRun.startedAt ?? null,
    completed_at: taskRun.completedAt ?? null,
    exit_code: taskRun.exitCode ?? null,
    created_at: taskRun.createdAt,
    updated_at: taskRun.updatedAt,
  };
}

export function taskRunFromRow(row: TaskRunRow): TaskRun {
  return {
    id: row.id,
    taskId: row.task_id,
    ...(row.conversation_id === null ? {} : { conversationId: row.conversation_id }),
    ...(row.worktree_id === null ? {} : { worktreeId: row.worktree_id }),
    executionState: row.execution_state,
    ...(row.started_at === null ? {} : { startedAt: row.started_at }),
    ...(row.completed_at === null ? {} : { completedAt: row.completed_at }),
    ...(row.exit_code === null ? {} : { exitCode: row.exit_code }),
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

export function conversationToRow(conversation: Conversation): ConversationRow {
  return {
    id: conversation.id,
    task_id: conversation.taskId ?? null,
    task_run_id: conversation.taskRunId ?? null,
    provider: conversation.provider,
    external_thread_id: conversation.externalThreadId ?? null,
    title: conversation.title,
    summary: conversation.summary ?? null,
    created_at: conversation.createdAt,
    updated_at: conversation.updatedAt,
  };
}

export function conversationFromRow(row: ConversationRow): Conversation {
  return {
    id: row.id,
    ...(row.task_id === null ? {} : { taskId: row.task_id }),
    ...(row.task_run_id === null ? {} : { taskRunId: row.task_run_id }),
    provider: row.provider,
    ...(row.external_thread_id === null ? {} : { externalThreadId: row.external_thread_id }),
    title: row.title,
    ...(row.summary === null ? {} : { summary: row.summary }),
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

function sqlStringList(values: readonly string[]): string {
  return values.map((value) => `'${value}'`).join(', ');
}
