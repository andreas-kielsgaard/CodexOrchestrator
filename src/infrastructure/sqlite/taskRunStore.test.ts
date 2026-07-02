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
import { TaskRunNotFoundError } from '../../domain/taskRunStore';
import { applyAppSqliteMigrations, enableAppSqliteForeignKeys } from './migrationCoordinator';
import { branchToRow, projectToRow, repoToRow, worktreeToRow } from './repoSyncSchema';
import {
  conversationToRow,
  taskRunFromRow,
  taskRunToRow,
  type TaskRunRow,
} from './runConversationSchema';
import { taskToRow } from './taskSchema';
import { SqliteTaskRunStore } from './taskRunStore';

const now = '2026-07-02T10:00:00.000Z';
const updatedAt = '2026-07-02T10:05:00.000Z';
const createdAt = '2026-07-01T09:00:00.000Z';

describe('SqliteTaskRunStore', () => {
  it('creates and round-trips a task run through the app-migrated task_runs table', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);

      const created = await createStore(db, [now]).createTaskRun({
        taskId: 'task-1',
        conversationId: 'conversation-1',
        worktreeId: 'worktree-main',
        executionState: 'running',
        startedAt: '2026-07-02T09:59:00.000Z',
      });

      expect(created).toEqual({
        id: 'run-created',
        taskId: 'task-1',
        conversationId: 'conversation-1',
        worktreeId: 'worktree-main',
        executionState: 'running',
        startedAt: '2026-07-02T09:59:00.000Z',
        createdAt: now,
        updatedAt: now,
      });
      expect(loadTaskRun(db, 'run-created')).toEqual(created);
    } finally {
      db.close();
    }
  });

  it('persists omitted create optionals and explicit update clears as SQL NULL', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);

      const created = await createStore(db, [now, updatedAt]).createTaskRun({
        taskId: 'task-1',
        executionState: 'queued',
      });
      let row = selectOne<TaskRunRow>(db, 'task_runs', created.id);

      expect(row.conversation_id).toBeNull();
      expect(row.worktree_id).toBeNull();
      expect(row.started_at).toBeNull();
      expect(row.completed_at).toBeNull();
      expect(row.exit_code).toBeNull();

      const updated = await createStore(db, [updatedAt]).updateTaskRun(created.id, {
        conversationId: null,
        worktreeId: null,
        startedAt: null,
        completedAt: null,
        exitCode: null,
      });
      row = selectOne<TaskRunRow>(db, 'task_runs', created.id);

      expect(updated).not.toHaveProperty('conversationId');
      expect(updated).not.toHaveProperty('worktreeId');
      expect(updated).not.toHaveProperty('startedAt');
      expect(updated).not.toHaveProperty('completedAt');
      expect(updated).not.toHaveProperty('exitCode');
      expect(row.conversation_id).toBeNull();
      expect(row.worktree_id).toBeNull();
      expect(row.started_at).toBeNull();
      expect(row.completed_at).toBeNull();
      expect(row.exit_code).toBeNull();
    } finally {
      db.close();
    }
  });

  it('updates mutable fields while omitted values remain unchanged', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);
      insertRow(db, 'task_runs', taskRunToRow(taskRun()));

      const updated = await createStore(db, [updatedAt]).updateTaskRun('run-1', {
        executionState: 'completed',
        completedAt: '2026-07-02T10:04:00.000Z',
        exitCode: 0,
      });

      expect(updated).toEqual({
        ...taskRun(),
        executionState: 'completed',
        completedAt: '2026-07-02T10:04:00.000Z',
        exitCode: 0,
        updatedAt,
      });
      expect(loadTaskRun(db, 'run-1')).toEqual(updated);
    } finally {
      db.close();
    }
  });

  it('throws a typed error when updating a missing task run', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);

      await expect(
        createStore(db, [updatedAt]).updateTaskRun('run-missing', {
          executionState: 'failed',
        }),
      ).rejects.toThrow(TaskRunNotFoundError);
    } finally {
      db.close();
    }
  });

  it('queries by optional filters in created order with stable id tie-breakers and limits', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);
      insertRow(
        db,
        'task_runs',
        taskRunToRow(
          taskRun({
            id: 'run-b',
            executionState: 'running',
            createdAt: '2026-07-02T10:00:01.000Z',
          }),
        ),
      );
      insertRow(
        db,
        'task_runs',
        taskRunToRow(
          taskRun({
            id: 'run-a',
            executionState: 'running',
            createdAt: '2026-07-02T10:00:01.000Z',
          }),
        ),
      );
      insertRow(
        db,
        'task_runs',
        taskRunToRow(
          taskRun({
            id: 'run-c',
            executionState: 'completed',
            createdAt: '2026-07-02T09:59:59.000Z',
          }),
        ),
      );

      await expect(
        createStore(db).queryTaskRuns({
          taskId: 'task-1',
          conversationId: 'conversation-1',
          worktreeId: 'worktree-main',
          executionState: 'running',
          limit: 1,
        }),
      ).resolves.toEqual([
        taskRun({
          id: 'run-a',
          executionState: 'running',
          createdAt: '2026-07-02T10:00:01.000Z',
        }),
      ]);
    } finally {
      db.close();
    }
  });

  it('returns empty results, supports limit zero, and rejects invalid limits', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);
      insertRow(db, 'task_runs', taskRunToRow(taskRun()));

      await expect(createStore(db).queryTaskRuns({ conversationId: 'missing' })).resolves.toEqual(
        [],
      );
      await expect(createStore(db).queryTaskRuns({ limit: 0 })).resolves.toEqual([]);
      await expect(createStore(db).queryTaskRuns({ limit: -1 })).rejects.toThrow(
        'TaskRun query limit must be a non-negative integer',
      );
    } finally {
      db.close();
    }
  });

  it('rolls back created rows when transaction-backed persistence fails', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);

      await expect(
        createStore(db).createTaskRun({
          taskId: 'task-missing',
          executionState: 'running',
        }),
      ).rejects.toThrow();

      expect(selectAll<TaskRunRow>(db, 'task_runs')).toEqual([]);
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

function createStore(db: DatabaseSync, times: readonly string[] = [now]): SqliteTaskRunStore {
  let callCount = 0;

  return new SqliteTaskRunStore(
    db,
    {
      nextId: () => 'run-created',
    },
    {
      now: () => {
        const time = times[callCount] ?? times[times.length - 1];
        callCount += 1;
        return time;
      },
    },
  );
}

function insertParentGraph(db: DatabaseSync): void {
  insertRow(db, 'projects', projectToRow(project()));
  insertRow(db, 'repos', repoToRow(repo()));
  insertRow(db, 'branches', branchToRow(branch()));
  insertRow(db, 'worktrees', worktreeToRow(worktree()));
  insertRow(db, 'tasks', taskToRow(task()));
  insertRow(db, 'conversations', conversationToRow(conversation()));
}

function loadTaskRun(db: DatabaseSync, taskRunId: string): TaskRun {
  return taskRunFromRow(selectOne<TaskRunRow>(db, 'task_runs', taskRunId));
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

function project(overrides: Partial<Project> = {}): Project {
  return {
    id: 'project-1',
    name: 'Codex Orchestrator',
    createdAt,
    updatedAt: createdAt,
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
    createdAt,
    updatedAt: createdAt,
    ...overrides,
  };
}

function branch(overrides: Partial<Branch> = {}): Branch {
  return {
    id: 'branch-main',
    repoId: 'repo-1',
    name: 'main',
    createdAt,
    updatedAt: createdAt,
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
    createdAt,
    updatedAt: createdAt,
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
    conversationIds: [],
    title: 'Run Codex task',
    summary: 'Execute a delegated worker.',
    executionState: 'running',
    attentionState: 'waiting_on_agent',
    priority: 'high',
    createdAt,
    updatedAt: createdAt,
    ...overrides,
  };
}

function conversation(overrides: Partial<Conversation> = {}): Conversation {
  return {
    id: 'conversation-1',
    taskId: 'task-1',
    provider: 'codex',
    title: 'Worker 020 TaskRun store boundary',
    createdAt,
    updatedAt: createdAt,
    ...overrides,
  };
}

function taskRun(overrides: Partial<TaskRun> = {}): TaskRun {
  return {
    id: 'run-1',
    taskId: 'task-1',
    conversationId: 'conversation-1',
    worktreeId: 'worktree-main',
    executionState: 'failed',
    startedAt: '2026-07-02T09:58:00.000Z',
    completedAt: '2026-07-02T10:03:00.000Z',
    exitCode: 1,
    createdAt,
    updatedAt: createdAt,
    ...overrides,
  };
}
