import { DatabaseSync } from 'node:sqlite';

import { loadOpenTaskDashboard } from '../../domain/openTaskDashboardStore';
import type { Branch, Project, Repo, Task, Worktree } from '../../domain/model';
import {
  applyRepoSyncSqliteMigrations,
  branchToRow,
  enableRepoSyncSqliteForeignKeys,
  projectToRow,
  repoToRow,
  worktreeToRow,
} from './repoSyncSchema';
import { applyTaskSqliteMigrations, taskConversationLinksToRows, taskToRow } from './taskSchema';
import { SqliteOpenTaskDashboardStore } from './openTaskDashboardStore';

const now = '2026-07-02T10:00:00.000Z';

describe('SqliteOpenTaskDashboardStore', () => {
  it('produces dashboard groups through the read store facade', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(
        db,
        task({
          id: 'task-review',
          title: 'Review dashboard reader',
          summary: 'Confirm SQLite read-side projection.',
          executionState: 'completed',
          attentionState: 'needs_review',
          conversationIds: ['conversation-1'],
        }),
      );

      const groups = await loadOpenTaskDashboard(new SqliteOpenTaskDashboardStore(db));

      expect(groups.find((group) => group.id === 'review_decide')?.tasks).toEqual([
        {
          id: 'task-review',
          title: 'Review dashboard reader',
          summary: 'Confirm SQLite read-side projection.',
          project: 'Codex Orchestrator',
          executionState: 'completed',
          attentionState: 'needs_review',
          priority: 'normal',
          repo: 'Codex Orchestrator',
          branch: 'worker/012-open-tasks-sqlite-read-store',
          worktreePath: 'C:/Repos/Codex Orchestrator Worker 012',
          updatedAt: now,
        },
      ]);
    } finally {
      db.close();
    }
  });

  it('resolves technical anchor names and paths from SQLite rows', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(db, task({ id: 'task-anchored', attentionState: 'needs_action_now' }));

      const groups = await loadOpenTaskDashboard(new SqliteOpenTaskDashboardStore(db));
      const dashboardTask = groups.find((group) => group.id === 'needs_action_now')?.tasks[0];

      expect(dashboardTask).toMatchObject({
        project: 'Codex Orchestrator',
        repo: 'Codex Orchestrator',
        branch: 'worker/012-open-tasks-sqlite-read-store',
        worktreePath: 'C:/Repos/Codex Orchestrator Worker 012',
      });
    } finally {
      db.close();
    }
  });

  it('loads closed task rows and relies on the domain projection to omit them', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(db, task({ id: 'task-open', attentionState: 'needs_action_now' }));
      insertTaskWithLinks(db, task({ id: 'task-archived', executionState: 'archived' }));
      insertTaskWithLinks(db, task({ id: 'task-abandoned', executionState: 'abandoned' }));

      const store = new SqliteOpenTaskDashboardStore(db);
      const records = await store.loadOpenTaskDashboardRecords();
      const groups = await loadOpenTaskDashboard(store);

      expect(records.tasks.map((loadedTask) => loadedTask.id)).toEqual([
        'task-abandoned',
        'task-archived',
        'task-open',
      ]);
      expect(groups.flatMap((group) => group.tasks).map((loadedTask) => loadedTask.id)).toEqual([
        'task-open',
      ]);
    } finally {
      db.close();
    }
  });

  it('sorts dashboard tasks by updatedAt through the projection', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(
        db,
        task({
          id: 'task-old',
          title: 'Older task',
          attentionState: 'needs_action_now',
          updatedAt: '2026-07-02T09:00:00.000Z',
        }),
      );
      insertTaskWithLinks(
        db,
        task({
          id: 'task-new',
          title: 'Newer task',
          attentionState: 'needs_action_now',
          updatedAt: '2026-07-02T11:00:00.000Z',
        }),
      );

      const groups = await loadOpenTaskDashboard(new SqliteOpenTaskDashboardStore(db));

      expect(
        groups.find((group) => group.id === 'needs_action_now')?.tasks.map((row) => row.id),
      ).toEqual(['task-new', 'task-old']);
    } finally {
      db.close();
    }
  });

  it('allows optional anchors to be missing without crashing', async () => {
    const db = openMigratedDatabase();

    try {
      insertRow(db, 'projects', projectToRow(project()));
      insertTaskWithLinks(
        db,
        task({
          id: 'task-unanchored',
          repoId: undefined,
          branchId: undefined,
          worktreeId: undefined,
          attentionState: 'needs_action_now',
        }),
      );

      const groups = await loadOpenTaskDashboard(new SqliteOpenTaskDashboardStore(db));

      expect(groups.find((group) => group.id === 'needs_action_now')?.tasks).toEqual([
        {
          id: 'task-unanchored',
          title: 'Open task dashboard reader',
          summary: 'Load persisted task dashboard records.',
          project: 'Codex Orchestrator',
          executionState: 'running',
          attentionState: 'needs_action_now',
          priority: 'normal',
          updatedAt: now,
        },
      ]);
    } finally {
      db.close();
    }
  });

  it('preserves stored task conversation link order in loaded records', async () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktree(db);
      insertTaskWithLinks(
        db,
        task({
          id: 'task-conversations',
          conversationIds: ['conversation-c', 'conversation-a', 'conversation-b'],
        }),
      );

      const records = await new SqliteOpenTaskDashboardStore(db).loadOpenTaskDashboardRecords();

      expect(
        records.tasks.find((loadedTask) => loadedTask.id === 'task-conversations')?.conversationIds,
      ).toEqual(['conversation-c', 'conversation-a', 'conversation-b']);
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
    id: 'branch-worker-012',
    repoId: 'repo-1',
    name: 'worker/012-open-tasks-sqlite-read-store',
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}

function worktree(overrides: Partial<Worktree> = {}): Worktree {
  return {
    id: 'worktree-worker-012',
    repoId: 'repo-1',
    branchId: 'branch-worker-012',
    path: 'C:/Repos/Codex Orchestrator Worker 012',
    isMain: false,
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
    branchId: 'branch-worker-012',
    worktreeId: 'worktree-worker-012',
    conversationIds: ['conversation-1'],
    title: 'Open task dashboard reader',
    summary: 'Load persisted task dashboard records.',
    executionState: 'running',
    attentionState: 'waiting_on_agent',
    priority: 'normal',
    createdAt: now,
    updatedAt: now,
    ...overrides,
  };
}
