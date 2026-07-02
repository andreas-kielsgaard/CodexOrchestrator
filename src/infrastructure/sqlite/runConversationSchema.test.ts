import { DatabaseSync } from 'node:sqlite';

import type {
  Branch,
  Conversation,
  Project,
  Repo,
  Task,
  TaskRun,
  Worktree,
} from '../../domain/model';
import { applyAppSqliteMigrations, enableAppSqliteForeignKeys } from './migrationCoordinator';
import { branchToRow, projectToRow, repoToRow, worktreeToRow } from './repoSyncSchema';
import {
  conversationFromRow,
  type ConversationRow,
  conversationToRow,
  runConversationSqliteMigrations,
  taskRunFromRow,
  type TaskRunRow,
  taskRunToRow,
} from './runConversationSchema';
import { taskToRow } from './taskSchema';

const now = '2026-07-02T10:00:00.000Z';

describe('task run and conversation SQLite schema', () => {
  it('creates task run and conversation tables through the app migration coordinator', () => {
    const db = openMigratedDatabase();

    try {
      expect(tableNames(db)).toEqual([
        'artifacts',
        'branches',
        'conversations',
        'events',
        'projects',
        'repos',
        'schema_migrations',
        'task_conversation_links',
        'task_runs',
        'tasks',
        'validation_runs',
        'worktrees',
      ]);
    } finally {
      db.close();
    }
  });

  it('keeps run/conversation migrations separate from task parent table definitions', () => {
    const migrationSql = runConversationSqliteMigrations
      .map((migration) => migration.sql)
      .join('\n');

    expect(migrationSql).not.toContain('CREATE TABLE IF NOT EXISTS tasks');
    expect(migrationSql).toContain('CREATE TABLE IF NOT EXISTS task_runs');
    expect(migrationSql).toContain('CREATE TABLE IF NOT EXISTS conversations');
  });

  it('enforces execution-state and provider check constraints', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktreeTask(db);

      expect(() =>
        insertRow(db, 'task_runs', {
          ...taskRunToRow(taskRun({ conversationId: undefined })),
          execution_state: 'paused',
        }),
      ).toThrow();
      expect(() =>
        insertRow(db, 'conversations', {
          ...conversationToRow(conversation({ taskRunId: undefined })),
          provider: 'slack',
        }),
      ).toThrow();
    } finally {
      db.close();
    }
  });

  it('requires an existing task for task runs and cascades task-owned cleanup', () => {
    const db = openMigratedDatabase();

    try {
      expect(() => insertRow(db, 'task_runs', taskRunToRow(taskRun()))).toThrow();

      insertProjectRepoBranchWorktreeTask(db);
      insertTaskRunAndConversation(db);

      db.prepare('DELETE FROM tasks WHERE id = ?').run('task-1');

      expect(selectAll<TaskRunRow>(db, 'task_runs')).toEqual([]);
      expect(selectOne<ConversationRow>(db, 'conversations', 'conversation-1')).toMatchObject({
        task_id: null,
        task_run_id: null,
      });
    } finally {
      db.close();
    }
  });

  it('sets optional worktree, conversation, and task-run links to NULL when parents are deleted', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktreeTask(db);
      insertTaskRunAndConversation(db);

      db.prepare('DELETE FROM worktrees WHERE id = ?').run('worktree-main');
      expect(selectOne<TaskRunRow>(db, 'task_runs', 'run-1').worktree_id).toBeNull();

      db.prepare('DELETE FROM conversations WHERE id = ?').run('conversation-1');
      expect(selectOne<TaskRunRow>(db, 'task_runs', 'run-1').conversation_id).toBeNull();

      insertRow(
        db,
        'conversations',
        conversationToRow(
          conversation({
            id: 'conversation-2',
            taskRunId: 'run-1',
          }),
        ),
      );
      db.prepare('DELETE FROM task_runs WHERE id = ?').run('run-1');
      expect(
        selectOne<ConversationRow>(db, 'conversations', 'conversation-2').task_run_id,
      ).toBeNull();
    } finally {
      db.close();
    }
  });

  it('round-trips optional fields as NULL through row mappers', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktreeTask(db);
      const minimalTaskRun = taskRun({
        conversationId: undefined,
        worktreeId: undefined,
        startedAt: undefined,
        completedAt: undefined,
        exitCode: undefined,
      });
      const minimalConversation = conversation({
        taskId: undefined,
        taskRunId: undefined,
        externalThreadId: undefined,
        summary: undefined,
      });

      insertRow(db, 'task_runs', taskRunToRow(minimalTaskRun));
      insertRow(db, 'conversations', conversationToRow(minimalConversation));

      const taskRunRow = selectOne<TaskRunRow>(db, 'task_runs', 'run-1');
      const conversationRow = selectOne<ConversationRow>(db, 'conversations', 'conversation-1');

      expect(taskRunRow.conversation_id).toBeNull();
      expect(taskRunRow.worktree_id).toBeNull();
      expect(taskRunRow.started_at).toBeNull();
      expect(taskRunRow.completed_at).toBeNull();
      expect(taskRunRow.exit_code).toBeNull();
      expect(conversationRow.task_id).toBeNull();
      expect(conversationRow.task_run_id).toBeNull();
      expect(conversationRow.external_thread_id).toBeNull();
      expect(conversationRow.summary).toBeNull();
      expect(taskRunFromRow(taskRunRow)).toEqual(minimalTaskRun);
      expect(conversationFromRow(conversationRow)).toEqual(minimalConversation);
    } finally {
      db.close();
    }
  });

  it('supports practical insertion before connecting optional task-run and conversation links', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktreeTask(db);

      insertRow(db, 'task_runs', taskRunToRow(taskRun({ conversationId: undefined })));
      insertRow(db, 'conversations', conversationToRow(conversation()));
      db.prepare('UPDATE task_runs SET conversation_id = ? WHERE id = ?').run(
        'conversation-1',
        'run-1',
      );

      expect(taskRunFromRow(selectOne<TaskRunRow>(db, 'task_runs', 'run-1'))).toEqual(taskRun());
      expect(
        conversationFromRow(selectOne<ConversationRow>(db, 'conversations', 'conversation-1')),
      ).toEqual(conversation());
    } finally {
      db.close();
    }
  });
});

function openMigratedDatabase(): DatabaseSync {
  const db = new DatabaseSync(':memory:');
  enableAppSqliteForeignKeys(db);
  applyAppSqliteMigrations(db, { appliedAt: (_migration, position) => `${now}:${position}` });
  return db;
}

function insertProjectRepoBranchWorktreeTask(db: DatabaseSync): void {
  insertRow(db, 'projects', projectToRow(project()));
  insertRow(db, 'repos', repoToRow(repo()));
  insertRow(db, 'branches', branchToRow(branch()));
  insertRow(db, 'worktrees', worktreeToRow(worktree()));
  insertRow(db, 'tasks', taskToRow(task()));
}

function insertTaskRunAndConversation(db: DatabaseSync): void {
  insertRow(db, 'task_runs', taskRunToRow(taskRun({ conversationId: undefined })));
  insertRow(db, 'conversations', conversationToRow(conversation()));
  db.prepare('UPDATE task_runs SET conversation_id = ? WHERE id = ?').run(
    'conversation-1',
    'run-1',
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

function selectAll<T>(db: DatabaseSync, table: string, orderBy = 'id'): T[] {
  return db.prepare(`SELECT * FROM ${table} ORDER BY ${orderBy}`).all() as T[];
}

function tableNames(db: DatabaseSync): string[] {
  return db
    .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
    .all()
    .map((row) => (row as { name: string }).name);
}

function project(overrides: Partial<Project> = {}): Project {
  return {
    id: 'project-1',
    name: 'Codex Orchestrator',
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
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}

function task(overrides: Partial<Task> = {}): Task {
  return {
    id: 'task-1',
    projectId: 'project-1',
    repoId: 'repo-1',
    branchId: 'branch-main',
    worktreeId: 'worktree-main',
    conversationIds: ['conversation-1'],
    title: 'Review SQLite schema',
    summary: 'Confirm the run and conversation persistence subset.',
    executionState: 'running',
    attentionState: 'waiting_on_agent',
    priority: 'high',
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}

function taskRun(overrides: Partial<TaskRun> = {}): TaskRun {
  return {
    id: 'run-1',
    taskId: 'task-1',
    conversationId: 'conversation-1',
    worktreeId: 'worktree-main',
    executionState: 'completed',
    startedAt: '2026-07-02T09:30:00.000Z',
    completedAt: now,
    exitCode: 0,
    createdAt: '2026-07-02T09:30:00.000Z',
    updatedAt: now,
    ...overrides,
  };
}

function conversation(overrides: Partial<Conversation> = {}): Conversation {
  return {
    id: 'conversation-1',
    taskId: 'task-1',
    taskRunId: 'run-1',
    provider: 'codex',
    externalThreadId: '019f224e-1225-7d52-8cb5-fddd1329b53f',
    title: 'Worker 016 schema foundation',
    summary: 'TaskRun and Conversation schema work.',
    createdAt: '2026-07-02T09:30:00.000Z',
    updatedAt: now,
    ...overrides,
  };
}
