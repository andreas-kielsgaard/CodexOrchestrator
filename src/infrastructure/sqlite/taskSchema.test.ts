import { DatabaseSync } from 'node:sqlite';

import { projectOpenTaskDashboard } from '../../domain/dashboardProjection';
import type { Branch, DomainRecords, Project, Repo, Task, Worktree } from '../../domain/model';
import {
  applyRepoSyncSqliteMigrations,
  branchToRow,
  enableRepoSyncSqliteForeignKeys,
  projectFromRow,
  projectToRow,
  repoFromRow,
  repoToRow,
  type BranchRow,
  type ProjectRow,
  type RepoRow,
  type WorktreeRow,
  worktreeFromRow,
  worktreeToRow,
} from './repoSyncSchema';
import {
  applyTaskSqliteMigrations,
  taskConversationLinksToRows,
  taskFromRow,
  type TaskConversationLinkRow,
  type TaskRow,
  taskSqliteMigrations,
  taskToRow,
} from './taskSchema';

const now = '2026-07-02T10:00:00.000Z';

describe('open tasks SQLite schema', () => {
  it('applies task migrations after repo-sync migrations', () => {
    const db = openMigratedDatabase();

    try {
      const tables = db
        .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
        .all()
        .map((row) => (row as { name: string }).name);

      expect(tables).toEqual([
        'branches',
        'projects',
        'repos',
        'task_conversation_links',
        'tasks',
        'worktrees',
      ]);
    } finally {
      db.close();
    }
  });

  it('keeps task migrations separate from repo-sync parent table definitions', () => {
    expect(taskSqliteMigrations.map((migration) => migration.sql).join('\n')).not.toContain(
      'CREATE TABLE IF NOT EXISTS projects',
    );
    expect(taskSqliteMigrations.map((migration) => migration.sql).join('\n')).toContain(
      'CREATE TABLE IF NOT EXISTS tasks',
    );
  });

  it('enforces task enum and priority check constraints', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);

      expect(() =>
        insertRow(db, 'tasks', {
          ...taskToRow(task()),
          execution_state: 'paused',
        }),
      ).toThrow();
      expect(() =>
        insertRow(db, 'tasks', {
          ...taskToRow(task()),
          attention_state: 'needs_coffee',
        }),
      ).toThrow();
      expect(() =>
        insertRow(db, 'tasks', {
          ...taskToRow(task()),
          priority: 'urgent',
        }),
      ).toThrow();
    } finally {
      db.close();
    }
  });

  it('requires an existing project and cascades project deletion to tasks', () => {
    const db = openMigratedDatabase();

    try {
      expect(() => insertRow(db, 'tasks', taskToRow(task()))).toThrow();

      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(db, task({ conversationIds: ['conversation-1'] }));

      db.prepare('DELETE FROM projects WHERE id = ?').run('project-1');

      expect(selectAll<TaskRow>(db, 'tasks')).toEqual([]);
      expect(selectAll<TaskConversationLinkRow>(db, 'task_conversation_links', 'task_id')).toEqual(
        [],
      );
    } finally {
      db.close();
    }
  });

  it('sets optional repo, branch, and worktree anchors to NULL when parents are deleted', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(db, task({ conversationIds: [] }));

      db.prepare('DELETE FROM worktrees WHERE id = ?').run('worktree-main');
      let taskRow = selectOne<TaskRow>(db, 'tasks', 'task-1');
      expect(taskRow.worktree_id).toBeNull();

      db.prepare('DELETE FROM branches WHERE id = ?').run('branch-main');
      taskRow = selectOne<TaskRow>(db, 'tasks', 'task-1');
      expect(taskRow.branch_id).toBeNull();

      db.prepare('DELETE FROM repos WHERE id = ?').run('repo-1');
      taskRow = selectOne<TaskRow>(db, 'tasks', 'task-1');
      expect(taskRow.repo_id).toBeNull();
      expect(selectAll<TaskRow>(db, 'tasks')).toHaveLength(1);
    } finally {
      db.close();
    }
  });

  it('round-trips optional fields as NULL and preserves conversation IDs by position', () => {
    const db = openMigratedDatabase();

    try {
      insertRow(db, 'projects', projectToRow(project()));
      const taskRecord = task({
        repoId: undefined,
        branchId: undefined,
        worktreeId: undefined,
        conversationIds: ['conversation-b', 'conversation-a', 'conversation-c'],
        dueAt: undefined,
        snoozedUntil: undefined,
      });

      insertTaskWithLinks(db, taskRecord);

      const taskRow = selectOne<TaskRow>(db, 'tasks', 'task-1');
      const linkRows = selectTaskLinks(db, 'task-1');

      expect(taskRow.repo_id).toBeNull();
      expect(taskRow.branch_id).toBeNull();
      expect(taskRow.worktree_id).toBeNull();
      expect(taskRow.due_at).toBeNull();
      expect(taskRow.snoozed_until).toBeNull();
      expect(linkRows.map((link) => link.conversation_id)).toEqual([
        'conversation-b',
        'conversation-a',
        'conversation-c',
      ]);
      expect(taskFromRow(taskRow, linkRows)).toEqual(taskRecord);
    } finally {
      db.close();
    }
  });

  it('cascades task deletion to conversation links', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(db, task({ conversationIds: ['conversation-1', 'conversation-2'] }));

      db.prepare('DELETE FROM tasks WHERE id = ?').run('task-1');

      expect(selectAll<TaskConversationLinkRow>(db, 'task_conversation_links', 'task_id')).toEqual(
        [],
      );
    } finally {
      db.close();
    }
  });

  it('maps persisted task rows into the open-task dashboard projection', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(
        db,
        task({
          attentionState: 'needs_review',
          executionState: 'completed',
          conversationIds: ['conversation-1'],
        }),
      );

      const records = loadDomainRecords(db);
      const groups = projectOpenTaskDashboard(records);

      expect(groups.find((group) => group.id === 'review_decide')?.tasks).toEqual([
        {
          id: 'task-1',
          title: 'Review SQLite schema',
          summary: 'Confirm the Open Tasks persistence subset.',
          project: 'Codex Orchestrator',
          executionState: 'completed',
          attentionState: 'needs_review',
          repo: 'Codex Orchestrator',
          branch: 'main',
          worktreePath: 'C:/Repos/Codex Orchestrator',
          updatedAt: now,
        },
      ]);
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

function selectTaskLinks(db: DatabaseSync, taskId: string): TaskConversationLinkRow[] {
  return db
    .prepare('SELECT * FROM task_conversation_links WHERE task_id = ? ORDER BY position')
    .all(taskId) as unknown as TaskConversationLinkRow[];
}

function loadDomainRecords(db: DatabaseSync): DomainRecords {
  const taskRows = selectAll<TaskRow>(db, 'tasks');
  const linkRows = selectAll<TaskConversationLinkRow>(
    db,
    'task_conversation_links',
    'task_id, position',
  );

  return {
    projects: selectAll<ProjectRow>(db, 'projects').map(projectFromRow),
    repos: selectAll<RepoRow>(db, 'repos').map(repoFromRow),
    branches: selectAll<BranchRow>(db, 'branches').map((row) => ({
      id: row.id,
      repoId: row.repo_id,
      name: row.name,
      ...(row.base_branch === null ? {} : { baseBranch: row.base_branch }),
      ...(row.head_sha === null ? {} : { headSha: row.head_sha }),
      ...(row.intent === null ? {} : { intent: row.intent }),
      createdAt: row.created_at,
      updatedAt: row.updated_at,
    })),
    worktrees: selectAll<WorktreeRow>(db, 'worktrees').map(worktreeFromRow),
    conversations: [],
    tasks: taskRows.map((row) => taskFromRow(row, linkRows)),
    taskRuns: [],
    artifacts: [],
    validationRuns: [],
    events: [],
  };
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
    conversationIds: ['conversation-1', 'conversation-2'],
    title: 'Review SQLite schema',
    summary: 'Confirm the Open Tasks persistence subset.',
    executionState: 'running',
    attentionState: 'waiting_on_agent',
    priority: 'high',
    dueAt: '2026-07-03T10:00:00.000Z',
    snoozedUntil: '2026-07-02T12:00:00.000Z',
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}
