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
import { ConversationNotFoundError } from '../../domain/conversationStore';
import { applyAppSqliteMigrations, enableAppSqliteForeignKeys } from './migrationCoordinator';
import { branchToRow, projectToRow, repoToRow, worktreeToRow } from './repoSyncSchema';
import {
  conversationFromRow,
  conversationToRow,
  taskRunToRow,
  type ConversationRow,
} from './runConversationSchema';
import { taskToRow } from './taskSchema';
import { SqliteConversationStore } from './conversationStore';

const now = '2026-07-02T10:00:00.000Z';
const updatedAt = '2026-07-02T10:05:00.000Z';
const createdAt = '2026-07-01T09:00:00.000Z';

describe('SqliteConversationStore', () => {
  it('creates and round-trips a conversation through the app-migrated conversations table', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);

      const created = await createStore(db, [now]).createConversation({
        taskId: 'task-1',
        taskRunId: 'run-1',
        provider: 'codex',
        externalThreadId: 'thread-created',
        title: 'Worker 023 Conversation store boundary',
        summary: 'Created from the store.',
      });

      expect(created).toEqual({
        id: 'conversation-created',
        taskId: 'task-1',
        taskRunId: 'run-1',
        provider: 'codex',
        externalThreadId: 'thread-created',
        title: 'Worker 023 Conversation store boundary',
        summary: 'Created from the store.',
        createdAt: now,
        updatedAt: now,
      });
      expect(loadConversation(db, 'conversation-created')).toEqual(created);
    } finally {
      db.close();
    }
  });

  it('persists omitted create optionals and explicit update clears as SQL NULL', async () => {
    const db = openMigratedDatabase();

    try {
      const created = await createStore(db, [now]).createConversation({
        provider: 'manual',
        title: 'Manual note',
      });
      let row = selectOne<ConversationRow>(db, 'conversations', created.id);

      expect(created).toEqual({
        id: 'conversation-created',
        provider: 'manual',
        title: 'Manual note',
        createdAt: now,
        updatedAt: now,
      });
      expect(row.task_id).toBeNull();
      expect(row.task_run_id).toBeNull();
      expect(row.external_thread_id).toBeNull();
      expect(row.summary).toBeNull();

      const updated = await createStore(db, [updatedAt]).updateConversation(created.id, {
        taskId: null,
        taskRunId: null,
        externalThreadId: null,
        summary: null,
      });
      row = selectOne<ConversationRow>(db, 'conversations', created.id);

      expect(updated).not.toHaveProperty('taskId');
      expect(updated).not.toHaveProperty('taskRunId');
      expect(updated).not.toHaveProperty('externalThreadId');
      expect(updated).not.toHaveProperty('summary');
      expect(row.task_id).toBeNull();
      expect(row.task_run_id).toBeNull();
      expect(row.external_thread_id).toBeNull();
      expect(row.summary).toBeNull();
    } finally {
      db.close();
    }
  });

  it('updates mutable fields while omitted values remain unchanged', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);

      const updated = await createStore(db, [updatedAt]).updateConversation('conversation-1', {
        title: 'Renamed conversation',
        summary: 'Updated summary',
      });

      expect(updated).toEqual({
        ...conversation(),
        title: 'Renamed conversation',
        summary: 'Updated summary',
        updatedAt,
      });
      expect(updated.id).toBe('conversation-1');
      expect(updated.provider).toBe('codex');
      expect(updated.createdAt).toBe(createdAt);
      expect(loadConversation(db, 'conversation-1')).toEqual(updated);
    } finally {
      db.close();
    }
  });

  it('throws a typed error when updating a missing conversation', async () => {
    const db = openMigratedDatabase();

    try {
      await expect(
        createStore(db, [updatedAt]).updateConversation('conversation-missing', {
          title: 'Missing',
        }),
      ).rejects.toThrow(ConversationNotFoundError);
    } finally {
      db.close();
    }
  });

  it('queries by optional filters in created order with stable id tie-breakers and limits', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);
      db.prepare('UPDATE conversations SET external_thread_id = ? WHERE id = ?').run(
        'thread-existing',
        'conversation-1',
      );
      insertRow(
        db,
        'conversations',
        conversationToRow(
          conversation({
            id: 'conversation-b',
            createdAt: '2026-07-02T10:00:01.000Z',
          }),
        ),
      );
      insertRow(
        db,
        'conversations',
        conversationToRow(
          conversation({
            id: 'conversation-a',
            createdAt: '2026-07-02T10:00:01.000Z',
          }),
        ),
      );
      insertRow(
        db,
        'conversations',
        conversationToRow(
          conversation({
            id: 'conversation-c',
            externalThreadId: 'thread-other',
            createdAt: '2026-07-02T09:59:59.000Z',
          }),
        ),
      );

      await expect(
        createStore(db).queryConversations({
          provider: 'codex',
          taskId: 'task-1',
          taskRunId: 'run-1',
          externalThreadId: 'thread-1',
          limit: 1,
        }),
      ).resolves.toEqual([
        conversation({
          id: 'conversation-a',
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

      await expect(createStore(db).queryConversations({ taskId: 'missing' })).resolves.toEqual([]);
      await expect(createStore(db).queryConversations({ limit: 0 })).resolves.toEqual([]);
      await expect(createStore(db).queryConversations({ limit: -1 })).rejects.toThrow(
        'Conversation query limit must be a non-negative integer',
      );
      await expect(createStore(db).queryConversations({ limit: 1.5 })).rejects.toThrow(
        'Conversation query limit must be a non-negative integer',
      );
    } finally {
      db.close();
    }
  });

  it('returns cloned query results so callers cannot mutate loaded conversations', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);

      const [firstLoad] = await createStore(db).queryConversations();
      firstLoad.title = 'Caller mutation';

      await expect(createStore(db).queryConversations()).resolves.toEqual([conversation()]);
    } finally {
      db.close();
    }
  });

  it('rolls back created rows when transaction-backed persistence fails', async () => {
    const db = openMigratedDatabase();

    try {
      await expect(
        createStore(db).createConversation({
          taskId: 'task-missing',
          provider: 'codex',
          title: 'Invalid parent',
        }),
      ).rejects.toThrow();

      expect(selectAll<ConversationRow>(db, 'conversations')).toEqual([]);
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

function createStore(db: DatabaseSync, times: readonly string[] = [now]): SqliteConversationStore {
  let callCount = 0;

  return new SqliteConversationStore(
    db,
    {
      nextId: () => 'conversation-created',
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
  insertRow(db, 'conversations', conversationToRow(conversation({ taskRunId: undefined })));
  insertRow(db, 'task_runs', taskRunToRow(taskRun()));
  db.prepare('UPDATE conversations SET task_run_id = ? WHERE id = ?').run(
    'run-1',
    'conversation-1',
  );
}

function loadConversation(db: DatabaseSync, conversationId: string): Conversation {
  return conversationFromRow(selectOne<ConversationRow>(db, 'conversations', conversationId));
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
    taskRunId: 'run-1',
    provider: 'codex',
    externalThreadId: 'thread-1',
    title: 'Worker conversation',
    summary: 'Initial summary',
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
    executionState: 'completed',
    startedAt: '2026-07-02T09:58:00.000Z',
    completedAt: '2026-07-02T10:03:00.000Z',
    exitCode: 0,
    createdAt,
    updatedAt: createdAt,
    ...overrides,
  };
}
