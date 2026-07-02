import { DatabaseSync } from 'node:sqlite';

import type {
  Artifact,
  Branch,
  Conversation,
  Project,
  Repo,
  Task,
  TaskRun,
  Worktree,
} from '../../domain/model';
import { applyAppSqliteMigrations, enableAppSqliteForeignKeys } from './migrationCoordinator';
import { artifactFromRow, artifactToRow, type ArtifactRow } from './artifactValidationSchema';
import { branchToRow, projectToRow, repoToRow, worktreeToRow } from './repoSyncSchema';
import { conversationToRow, taskRunToRow } from './runConversationSchema';
import { SqliteArtifactStore } from './artifactStore';
import { taskToRow } from './taskSchema';

const now = '2026-07-02T10:00:00.000Z';
const createdAt = '2026-07-01T09:00:00.000Z';

describe('SqliteArtifactStore', () => {
  it('creates and round-trips an artifact through the app-migrated artifacts table', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);

      const created = await createStore(db).createArtifact({
        taskId: 'task-1',
        taskRunId: 'run-1',
        conversationId: 'conversation-1',
        kind: 'final_response',
        title: 'Worker completion report',
        uri: 'file:///tmp/report.md',
        content: 'Task complete.',
      });

      expect(created).toEqual({
        id: 'artifact-created',
        taskId: 'task-1',
        taskRunId: 'run-1',
        conversationId: 'conversation-1',
        kind: 'final_response',
        title: 'Worker completion report',
        uri: 'file:///tmp/report.md',
        content: 'Task complete.',
        createdAt: now,
      });
      expect(loadArtifact(db, 'artifact-created')).toEqual(created);
    } finally {
      db.close();
    }
  });

  it('persists omitted create optionals as SQL NULL', async () => {
    const db = openMigratedDatabase();

    try {
      const created = await createStore(db).createArtifact({
        kind: 'note',
        title: 'Loose note',
      });
      const row = selectOne<ArtifactRow>(db, 'artifacts', created.id);

      expect(created).toEqual({
        id: 'artifact-created',
        kind: 'note',
        title: 'Loose note',
        createdAt: now,
      });
      expect(row.task_id).toBeNull();
      expect(row.task_run_id).toBeNull();
      expect(row.conversation_id).toBeNull();
      expect(row.uri).toBeNull();
      expect(row.content).toBeNull();
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
        'artifacts',
        artifactToRow(
          artifact({
            id: 'artifact-b',
            kind: 'diff',
            createdAt: '2026-07-02T10:00:01.000Z',
          }),
        ),
      );
      insertRow(
        db,
        'artifacts',
        artifactToRow(
          artifact({
            id: 'artifact-a',
            kind: 'diff',
            createdAt: '2026-07-02T10:00:01.000Z',
          }),
        ),
      );
      insertRow(
        db,
        'artifacts',
        artifactToRow(
          artifact({
            id: 'artifact-c',
            kind: 'note',
            createdAt: '2026-07-02T09:59:59.000Z',
          }),
        ),
      );

      await expect(
        createStore(db).queryArtifacts({
          kind: 'diff',
          taskId: 'task-1',
          taskRunId: 'run-1',
          conversationId: 'conversation-1',
          limit: 1,
        }),
      ).resolves.toEqual([
        artifact({
          id: 'artifact-a',
          kind: 'diff',
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
      insertRow(db, 'artifacts', artifactToRow(artifact()));

      await expect(createStore(db).queryArtifacts({ conversationId: 'missing' })).resolves.toEqual(
        [],
      );
      await expect(createStore(db).queryArtifacts({ limit: 0 })).resolves.toEqual([]);
      await expect(createStore(db).queryArtifacts({ limit: -1 })).rejects.toThrow(
        'Artifact query limit must be a non-negative integer',
      );
    } finally {
      db.close();
    }
  });

  it('returns cloned query results so callers cannot mutate loaded artifacts', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);
      insertRow(db, 'artifacts', artifactToRow(artifact()));

      const [firstLoad] = await createStore(db).queryArtifacts();
      firstLoad.title = 'Mutated outside the store';

      await expect(createStore(db).queryArtifacts()).resolves.toEqual([artifact()]);
    } finally {
      db.close();
    }
  });

  it('rolls back created rows when transaction-backed persistence fails', async () => {
    const db = openMigratedDatabase();

    try {
      await expect(
        createStore(db).createArtifact({
          taskId: 'task-missing',
          kind: 'diff',
          title: 'Missing parent diff',
        }),
      ).rejects.toThrow();

      expect(selectAll<ArtifactRow>(db, 'artifacts')).toEqual([]);
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

function createStore(db: DatabaseSync, times: readonly string[] = [now]): SqliteArtifactStore {
  let callCount = 0;

  return new SqliteArtifactStore(
    db,
    {
      nextId: () => 'artifact-created',
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
  insertRow(db, 'task_runs', taskRunToRow(taskRun()));
}

function loadArtifact(db: DatabaseSync, artifactId: string): Artifact {
  return artifactFromRow(selectOne<ArtifactRow>(db, 'artifacts', artifactId));
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
    title: 'Worker 021 Artifact store boundary',
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

function artifact(overrides: Partial<Artifact> = {}): Artifact {
  return {
    id: 'artifact-1',
    taskId: 'task-1',
    taskRunId: 'run-1',
    conversationId: 'conversation-1',
    kind: 'final_response',
    title: 'Worker completion report',
    uri: 'file:///tmp/report.md',
    content: 'Task complete.',
    createdAt,
    ...overrides,
  };
}
