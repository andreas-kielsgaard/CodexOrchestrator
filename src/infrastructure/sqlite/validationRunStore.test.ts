import { DatabaseSync } from 'node:sqlite';

import type {
  Artifact,
  Branch,
  Conversation,
  Project,
  Repo,
  Task,
  TaskRun,
  ValidationRun,
  Worktree,
} from '../../domain/model';
import { ValidationRunNotFoundError } from '../../domain/validationRunStore';
import { applyAppSqliteMigrations, enableAppSqliteForeignKeys } from './migrationCoordinator';
import {
  artifactToRow,
  validationRunFromRow,
  validationRunToRow,
  type ValidationRunRow,
} from './artifactValidationSchema';
import { branchToRow, projectToRow, repoToRow, worktreeToRow } from './repoSyncSchema';
import { conversationToRow, taskRunToRow } from './runConversationSchema';
import { taskToRow } from './taskSchema';
import { SqliteValidationRunStore } from './validationRunStore';

const now = '2026-07-02T10:00:00.000Z';
const updatedAt = '2026-07-02T10:05:00.000Z';
const createdAt = '2026-07-01T09:00:00.000Z';

describe('SqliteValidationRunStore', () => {
  it('creates and round-trips a validation run through the app-migrated validation_runs table', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);

      const created = await createStore(db, [now]).createValidationRun({
        taskId: 'task-1',
        taskRunId: 'run-1',
        command: 'npm run test',
        status: 'running',
        startedAt: '2026-07-02T09:59:00.000Z',
        outputArtifactId: 'artifact-1',
      });

      expect(created).toEqual({
        id: 'validation-created',
        taskId: 'task-1',
        taskRunId: 'run-1',
        command: 'npm run test',
        status: 'running',
        startedAt: '2026-07-02T09:59:00.000Z',
        outputArtifactId: 'artifact-1',
        createdAt: now,
        updatedAt: now,
      });
      expect(loadValidationRun(db, 'validation-created')).toEqual(created);
    } finally {
      db.close();
    }
  });

  it('persists omitted create optionals and explicit update clears as SQL NULL', async () => {
    const db = openMigratedDatabase();

    try {
      const created = await createStore(db, [now]).createValidationRun({
        command: 'npm run lint',
        status: 'queued',
      });
      let row = selectOne<ValidationRunRow>(db, 'validation_runs', created.id);

      expect(created).toEqual({
        id: 'validation-created',
        command: 'npm run lint',
        status: 'queued',
        createdAt: now,
        updatedAt: now,
      });
      expect(row.task_id).toBeNull();
      expect(row.task_run_id).toBeNull();
      expect(row.started_at).toBeNull();
      expect(row.completed_at).toBeNull();
      expect(row.exit_code).toBeNull();
      expect(row.output_artifact_id).toBeNull();

      const updated = await createStore(db, [updatedAt]).updateValidationRun(created.id, {
        taskId: null,
        taskRunId: null,
        startedAt: null,
        completedAt: null,
        exitCode: null,
        outputArtifactId: null,
      });
      row = selectOne<ValidationRunRow>(db, 'validation_runs', created.id);

      expect(updated).not.toHaveProperty('taskId');
      expect(updated).not.toHaveProperty('taskRunId');
      expect(updated).not.toHaveProperty('startedAt');
      expect(updated).not.toHaveProperty('completedAt');
      expect(updated).not.toHaveProperty('exitCode');
      expect(updated).not.toHaveProperty('outputArtifactId');
      expect(row.task_id).toBeNull();
      expect(row.task_run_id).toBeNull();
      expect(row.started_at).toBeNull();
      expect(row.completed_at).toBeNull();
      expect(row.exit_code).toBeNull();
      expect(row.output_artifact_id).toBeNull();
    } finally {
      db.close();
    }
  });

  it('updates mutable fields while omitted values remain unchanged', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);
      insertRow(db, 'validation_runs', validationRunToRow(validationRun()));

      const updated = await createStore(db, [updatedAt]).updateValidationRun('validation-1', {
        status: 'passed',
        completedAt: '2026-07-02T10:04:00.000Z',
        exitCode: 0,
      });

      expect(updated).toEqual({
        ...validationRun(),
        status: 'passed',
        completedAt: '2026-07-02T10:04:00.000Z',
        exitCode: 0,
        updatedAt,
      });
      expect(updated.command).toBe('npm run test');
      expect(updated.createdAt).toBe(createdAt);
      expect(loadValidationRun(db, 'validation-1')).toEqual(updated);
    } finally {
      db.close();
    }
  });

  it('throws a typed error when updating a missing validation run', async () => {
    const db = openMigratedDatabase();

    try {
      await expect(
        createStore(db, [updatedAt]).updateValidationRun('validation-missing', {
          status: 'failed',
        }),
      ).rejects.toThrow(ValidationRunNotFoundError);
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
        'validation_runs',
        validationRunToRow(
          validationRun({
            id: 'validation-b',
            status: 'running',
            createdAt: '2026-07-02T10:00:01.000Z',
          }),
        ),
      );
      insertRow(
        db,
        'validation_runs',
        validationRunToRow(
          validationRun({
            id: 'validation-a',
            status: 'running',
            createdAt: '2026-07-02T10:00:01.000Z',
          }),
        ),
      );
      insertRow(
        db,
        'validation_runs',
        validationRunToRow(
          validationRun({
            id: 'validation-c',
            status: 'passed',
            createdAt: '2026-07-02T09:59:59.000Z',
          }),
        ),
      );

      await expect(
        createStore(db).queryValidationRuns({
          taskId: 'task-1',
          taskRunId: 'run-1',
          status: 'running',
          outputArtifactId: 'artifact-1',
          limit: 1,
        }),
      ).resolves.toEqual([
        validationRun({
          id: 'validation-a',
          status: 'running',
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
      insertRow(db, 'validation_runs', validationRunToRow(validationRun()));

      await expect(createStore(db).queryValidationRuns({ taskId: 'missing' })).resolves.toEqual([]);
      await expect(createStore(db).queryValidationRuns({ limit: 0 })).resolves.toEqual([]);
      await expect(createStore(db).queryValidationRuns({ limit: -1 })).rejects.toThrow(
        'ValidationRun query limit must be a non-negative integer',
      );
    } finally {
      db.close();
    }
  });

  it('returns cloned query results so callers cannot mutate loaded validation runs', async () => {
    const db = openMigratedDatabase();

    try {
      insertParentGraph(db);
      insertRow(db, 'validation_runs', validationRunToRow(validationRun()));

      const [firstLoad] = await createStore(db).queryValidationRuns();
      firstLoad.status = 'passed';

      await expect(createStore(db).queryValidationRuns()).resolves.toEqual([validationRun()]);
    } finally {
      db.close();
    }
  });

  it('rolls back created rows when transaction-backed persistence fails', async () => {
    const db = openMigratedDatabase();

    try {
      await expect(
        createStore(db).createValidationRun({
          taskId: 'task-missing',
          command: 'npm run test',
          status: 'running',
        }),
      ).rejects.toThrow();

      expect(selectAll<ValidationRunRow>(db, 'validation_runs')).toEqual([]);
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

function createStore(db: DatabaseSync, times: readonly string[] = [now]): SqliteValidationRunStore {
  let callCount = 0;

  return new SqliteValidationRunStore(
    db,
    {
      nextId: () => 'validation-created',
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
  insertRow(db, 'artifacts', artifactToRow(artifact()));
}

function loadValidationRun(db: DatabaseSync, validationRunId: string): ValidationRun {
  return validationRunFromRow(selectOne<ValidationRunRow>(db, 'validation_runs', validationRunId));
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
    title: 'Worker 022 ValidationRun store boundary',
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
    kind: 'validation_log',
    title: 'Validation output',
    content: 'test output',
    createdAt,
    ...overrides,
  };
}

function validationRun(overrides: Partial<ValidationRun> = {}): ValidationRun {
  return {
    id: 'validation-1',
    taskId: 'task-1',
    taskRunId: 'run-1',
    command: 'npm run test',
    status: 'failed',
    startedAt: '2026-07-02T09:58:00.000Z',
    completedAt: '2026-07-02T10:03:00.000Z',
    exitCode: 1,
    outputArtifactId: 'artifact-1',
    createdAt,
    updatedAt: createdAt,
    ...overrides,
  };
}
