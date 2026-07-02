import { DatabaseSync } from 'node:sqlite';

import { loadOpenTaskDashboard } from '../../domain/openTaskDashboardStore';
import type { Branch, Project, Repo, Task, Worktree } from '../../domain/model';
import {
  OpenTaskNotFoundError,
  type IdProvider,
  type TimeProvider,
} from '../../domain/openTaskWriteStore';
import {
  applyRepoSyncSqliteMigrations,
  branchToRow,
  enableRepoSyncSqliteForeignKeys,
  projectToRow,
  repoToRow,
  worktreeToRow,
} from './repoSyncSchema';
import {
  applyTaskSqliteMigrations,
  taskConversationLinksToRows,
  taskFromRow,
  taskToRow,
  type TaskConversationLinkRow,
  type TaskRow,
} from './taskSchema';
import { SqliteOpenTaskDashboardStore } from './openTaskDashboardStore';
import { SqliteOpenTaskWriteStore } from './openTaskWriteStore';

const now = '2026-07-02T10:00:00.000Z';
const createdAt = '2026-07-01T09:00:00.000Z';

describe('SqliteOpenTaskWriteStore', () => {
  it('creates a task with deterministic id, timestamps, defaults, and ordered conversations', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);

      const created = await createStore(db).createTask({
        projectId: 'project-1',
        repoId: 'repo-1',
        branchId: 'branch-main',
        worktreeId: 'worktree-main',
        conversationIds: ['conversation-b', 'conversation-a'],
        title: 'Persist write adapter',
        summary: 'Create task rows from the write boundary.',
      });

      expect(created).toEqual({
        id: 'task-created',
        projectId: 'project-1',
        repoId: 'repo-1',
        branchId: 'branch-main',
        worktreeId: 'worktree-main',
        conversationIds: ['conversation-b', 'conversation-a'],
        title: 'Persist write adapter',
        summary: 'Create task rows from the write boundary.',
        executionState: 'draft',
        attentionState: 'consider_later',
        priority: 'normal',
        createdAt: now,
        updatedAt: now,
      });
      expect(loadTask(db, 'task-created')).toEqual(created);
      expect(selectLinks(db, 'task-created').map((row) => row.conversation_id)).toEqual([
        'conversation-b',
        'conversation-a',
      ]);
    } finally {
      db.close();
    }
  });

  it('updates scalar fields while omitted optional values remain unchanged', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(db, task());

      const updated = await createStore(db).updateTask('task-existing', {
        title: 'Updated task',
        summary: 'Updated summary.',
        executionState: 'running',
        attentionState: 'waiting_on_agent',
        priority: 'high',
      });

      expect(updated).toMatchObject({
        repoId: 'repo-1',
        branchId: 'branch-main',
        worktreeId: 'worktree-main',
        dueAt: '2026-07-05T09:00:00.000Z',
        snoozedUntil: '2026-07-04T09:00:00.000Z',
        title: 'Updated task',
        summary: 'Updated summary.',
        executionState: 'running',
        attentionState: 'waiting_on_agent',
        priority: 'high',
        updatedAt: now,
      });
      expect(loadTask(db, 'task-existing')).toEqual(updated);
      expect(selectLinks(db, 'task-existing').map((row) => row.conversation_id)).toEqual([
        'conversation-original',
      ]);
    } finally {
      db.close();
    }
  });

  it('persists explicit optional clears as SQL NULL', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(db, task());

      const updated = await createStore(db).updateTask('task-existing', {
        repoId: null,
        branchId: null,
        worktreeId: null,
        dueAt: null,
        snoozedUntil: null,
      });
      const row = selectOne<TaskRow>(db, 'tasks', 'task-existing');

      expect(updated).not.toHaveProperty('repoId');
      expect(updated).not.toHaveProperty('branchId');
      expect(updated).not.toHaveProperty('worktreeId');
      expect(updated).not.toHaveProperty('dueAt');
      expect(updated).not.toHaveProperty('snoozedUntil');
      expect(row.repo_id).toBeNull();
      expect(row.branch_id).toBeNull();
      expect(row.worktree_id).toBeNull();
      expect(row.due_at).toBeNull();
      expect(row.snoozed_until).toBeNull();
    } finally {
      db.close();
    }
  });

  it('replaces the full ordered conversation list without disturbing unrelated tasks', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(db, task());
      insertTaskWithLinks(
        db,
        task({
          id: 'task-unrelated',
          title: 'Unrelated',
          conversationIds: ['conversation-stable'],
        }),
      );

      const updated = await createStore(db).updateTask('task-existing', {
        conversationIds: ['conversation-z', 'conversation-a', 'conversation-m'],
      });

      expect(updated.conversationIds).toEqual([
        'conversation-z',
        'conversation-a',
        'conversation-m',
      ]);
      expect(selectLinks(db, 'task-existing').map((row) => row.conversation_id)).toEqual([
        'conversation-z',
        'conversation-a',
        'conversation-m',
      ]);
      expect(loadTask(db, 'task-unrelated')).toMatchObject({
        id: 'task-unrelated',
        title: 'Unrelated',
        conversationIds: ['conversation-stable'],
      });
    } finally {
      db.close();
    }
  });

  it('replaces conversations with an empty list', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(db, task());

      const updated = await createStore(db).updateTask('task-existing', {
        conversationIds: [],
      });

      expect(updated.conversationIds).toEqual([]);
      expect(loadTask(db, 'task-existing').conversationIds).toEqual([]);
      expect(selectLinks(db, 'task-existing')).toEqual([]);
    } finally {
      db.close();
    }
  });

  it('archives tasks by state and relies on the dashboard projection to omit them', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(db, task());
      insertTaskWithLinks(
        db,
        task({
          id: 'task-open',
          title: 'Still open',
          attentionState: 'needs_action_now',
          updatedAt: '2026-07-01T10:00:00.000Z',
        }),
      );

      const archived = await createStore(db).archiveTask('task-existing');
      const records = await new SqliteOpenTaskDashboardStore(db).loadOpenTaskDashboardRecords();
      const groups = await loadOpenTaskDashboard(new SqliteOpenTaskDashboardStore(db));

      expect(archived.executionState).toBe('archived');
      expect(records.tasks.map((record) => record.id)).toEqual(['task-existing', 'task-open']);
      expect(groups.flatMap((group) => group.tasks).map((record) => record.id)).toEqual([
        'task-open',
      ]);
    } finally {
      db.close();
    }
  });

  it('throws a typed error when updating or archiving a missing task', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      const store = createStore(db);

      await expect(store.updateTask('task-missing', { title: 'Missing' })).rejects.toThrow(
        OpenTaskNotFoundError,
      );
      await expect(store.archiveTask('task-missing')).rejects.toThrow('Open task not found');
    } finally {
      db.close();
    }
  });

  it('rolls back create rows and links when transaction-backed persistence fails', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);

      await expect(
        createStore(db).createTask({
          projectId: 'project-1',
          conversationIds: ['conversation-duplicate', 'conversation-duplicate'],
          title: 'Rollback duplicate links',
          summary: 'The second link violates the task link primary key.',
        }),
      ).rejects.toThrow();

      expect(selectAll<TaskRow>(db, 'tasks')).toEqual([]);
      expect(selectAll<TaskConversationLinkRow>(db, 'task_conversation_links')).toEqual([]);
    } finally {
      db.close();
    }
  });
});

function openMigratedDatabase(): DatabaseSync {
  const db = new DatabaseSync(':memory:');
  enableRepoSyncSqliteForeignKeys(db);
  applyRepoSyncSqliteMigrations(db);
  applyTaskSqliteMigrations(db);
  return db;
}

function createStore(db: DatabaseSync): SqliteOpenTaskWriteStore {
  const ids: IdProvider = {
    nextId: () => 'task-created',
  };
  const clock: TimeProvider = {
    now: () => now,
  };

  return new SqliteOpenTaskWriteStore(db, ids, clock);
}

function insertProjectRepoBranchWorktree(db: DatabaseSync): void {
  insertRow(db, 'projects', projectToRow(project()));
  insertRow(db, 'repos', repoToRow(repo()));
  insertRow(db, 'branches', branchToRow(branch()));
  insertRow(db, 'worktrees', worktreeToRow(worktree()));
}

function insertTaskWithLinks(db: DatabaseSync, taskRecord: Task): void {
  insertRow(db, 'tasks', taskToRow(taskRecord));

  for (const linkRow of taskConversationLinksToRows(taskRecord)) {
    insertRow(db, 'task_conversation_links', linkRow);
  }
}

function loadTask(db: DatabaseSync, taskId: string): Task {
  return taskFromRow(selectOne<TaskRow>(db, 'tasks', taskId), selectLinks(db, taskId));
}

function selectLinks(db: DatabaseSync, taskId: string): TaskConversationLinkRow[] {
  return db
    .prepare('SELECT * FROM task_conversation_links WHERE task_id = ? ORDER BY position')
    .all(taskId) as unknown as TaskConversationLinkRow[];
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
  const orderBy = table === 'task_conversation_links' ? 'task_id, position' : 'id';

  return db.prepare(`SELECT * FROM ${table} ORDER BY ${orderBy}`).all() as T[];
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
    id: 'task-existing',
    projectId: 'project-1',
    repoId: 'repo-1',
    branchId: 'branch-main',
    worktreeId: 'worktree-main',
    conversationIds: ['conversation-original'],
    title: 'Existing task',
    summary: 'Existing task summary.',
    executionState: 'draft',
    attentionState: 'consider_later',
    priority: 'normal',
    dueAt: '2026-07-05T09:00:00.000Z',
    snoozedUntil: '2026-07-04T09:00:00.000Z',
    createdAt,
    updatedAt: createdAt,
    ...overrides,
  };
}
