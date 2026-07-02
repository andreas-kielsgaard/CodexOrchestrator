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
import {
  artifactFromRow,
  artifactToRow,
  artifactValidationSqliteMigrations,
  type ArtifactRow,
  validationRunFromRow,
  type ValidationRunRow,
  validationRunToRow,
} from './artifactValidationSchema';
import { applyAppSqliteMigrations, enableAppSqliteForeignKeys } from './migrationCoordinator';
import { branchToRow, projectToRow, repoToRow, worktreeToRow } from './repoSyncSchema';
import { conversationToRow, taskRunToRow } from './runConversationSchema';
import { taskToRow } from './taskSchema';

const now = '2026-07-02T10:00:00.000Z';

describe('artifact and validation-run SQLite schema', () => {
  it('creates artifact and validation-run tables through the app migration coordinator', () => {
    const db = openMigratedDatabase();

    try {
      expect(tableNames(db)).toEqual([
        'artifacts',
        'branches',
        'conversations',
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

  it('keeps artifact/validation migrations separate from parent table definitions', () => {
    const migrationSql = artifactValidationSqliteMigrations
      .map((migration) => migration.sql)
      .join('\n');

    expect(migrationSql).not.toContain('CREATE TABLE IF NOT EXISTS tasks');
    expect(migrationSql).not.toContain('CREATE TABLE IF NOT EXISTS task_runs');
    expect(migrationSql).toContain('CREATE TABLE IF NOT EXISTS artifacts');
    expect(migrationSql).toContain('CREATE TABLE IF NOT EXISTS validation_runs');
  });

  it('enforces artifact-kind and validation-status check constraints', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktreeTaskRunConversation(db);

      expect(() =>
        insertRow(db, 'artifacts', {
          ...artifactToRow(artifact()),
          kind: 'spreadsheet',
        }),
      ).toThrow();
      expect(() =>
        insertRow(db, 'validation_runs', {
          ...validationRunToRow(validationRun({ outputArtifactId: undefined })),
          status: 'paused',
        }),
      ).toThrow();
    } finally {
      db.close();
    }
  });

  it('sets optional task, task-run, conversation, and output artifact links to NULL on cleanup', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktreeTaskRunConversation(db);
      insertRow(db, 'artifacts', artifactToRow(artifact()));
      insertRow(db, 'validation_runs', validationRunToRow(validationRun()));

      db.prepare('DELETE FROM conversations WHERE id = ?').run('conversation-1');
      expect(selectOne<ArtifactRow>(db, 'artifacts', 'artifact-1').conversation_id).toBeNull();

      db.prepare('DELETE FROM artifacts WHERE id = ?').run('artifact-1');
      expect(
        selectOne<ValidationRunRow>(db, 'validation_runs', 'validation-1').output_artifact_id,
      ).toBeNull();

      insertRow(
        db,
        'artifacts',
        artifactToRow(artifact({ id: 'artifact-2', conversationId: undefined })),
      );
      db.prepare('DELETE FROM tasks WHERE id = ?').run('task-1');

      expect(selectOne<ArtifactRow>(db, 'artifacts', 'artifact-2')).toMatchObject({
        task_id: null,
        task_run_id: null,
      });
      expect(selectOne<ValidationRunRow>(db, 'validation_runs', 'validation-1')).toMatchObject({
        task_id: null,
        task_run_id: null,
      });
    } finally {
      db.close();
    }
  });

  it('round-trips optional fields as NULL through row mappers', () => {
    const db = openMigratedDatabase();

    try {
      const minimalArtifact = artifact({
        taskId: undefined,
        taskRunId: undefined,
        conversationId: undefined,
        uri: undefined,
        content: undefined,
      });
      const minimalValidationRun = validationRun({
        taskId: undefined,
        taskRunId: undefined,
        startedAt: undefined,
        completedAt: undefined,
        exitCode: undefined,
        outputArtifactId: undefined,
      });

      insertRow(db, 'artifacts', artifactToRow(minimalArtifact));
      insertRow(db, 'validation_runs', validationRunToRow(minimalValidationRun));

      const artifactRow = selectOne<ArtifactRow>(db, 'artifacts', 'artifact-1');
      const validationRunRow = selectOne<ValidationRunRow>(db, 'validation_runs', 'validation-1');

      expect(artifactRow.task_id).toBeNull();
      expect(artifactRow.task_run_id).toBeNull();
      expect(artifactRow.conversation_id).toBeNull();
      expect(artifactRow.uri).toBeNull();
      expect(artifactRow.content).toBeNull();
      expect(validationRunRow.task_id).toBeNull();
      expect(validationRunRow.task_run_id).toBeNull();
      expect(validationRunRow.started_at).toBeNull();
      expect(validationRunRow.completed_at).toBeNull();
      expect(validationRunRow.exit_code).toBeNull();
      expect(validationRunRow.output_artifact_id).toBeNull();
      expect(artifactFromRow(artifactRow)).toEqual(minimalArtifact);
      expect(validationRunFromRow(validationRunRow)).toEqual(minimalValidationRun);
    } finally {
      db.close();
    }
  });

  it('supports inserting validation runs before attaching an output artifact', () => {
    const db = openMigratedDatabase();

    try {
      insertProjectRepoBranchWorktreeTaskRunConversation(db);

      insertRow(
        db,
        'validation_runs',
        validationRunToRow(validationRun({ outputArtifactId: undefined })),
      );
      insertRow(db, 'artifacts', artifactToRow(artifact()));
      db.prepare('UPDATE validation_runs SET output_artifact_id = ? WHERE id = ?').run(
        'artifact-1',
        'validation-1',
      );

      expect(artifactFromRow(selectOne<ArtifactRow>(db, 'artifacts', 'artifact-1'))).toEqual(
        artifact(),
      );
      expect(
        validationRunFromRow(selectOne<ValidationRunRow>(db, 'validation_runs', 'validation-1')),
      ).toEqual(validationRun());
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

function insertProjectRepoBranchWorktreeTaskRunConversation(db: DatabaseSync): void {
  insertRow(db, 'projects', projectToRow(project()));
  insertRow(db, 'repos', repoToRow(repo()));
  insertRow(db, 'branches', branchToRow(branch()));
  insertRow(db, 'worktrees', worktreeToRow(worktree()));
  insertRow(db, 'tasks', taskToRow(task()));
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
    summary: 'Confirm the artifact and validation persistence subset.',
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
    title: 'Worker 017 schema foundation',
    summary: 'Artifact and ValidationRun schema work.',
    createdAt: '2026-07-02T09:30:00.000Z',
    updatedAt: now,
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
    title: 'Focused schema test output',
    uri: 'file:///tmp/validation.log',
    content: 'npm run test -- artifactValidationSchema.test.ts -> pass',
    createdAt: now,
    ...overrides,
  };
}

function validationRun(overrides: Partial<ValidationRun> = {}): ValidationRun {
  return {
    id: 'validation-1',
    taskId: 'task-1',
    taskRunId: 'run-1',
    command: 'npm run test -- src/infrastructure/sqlite/artifactValidationSchema.test.ts',
    status: 'passed',
    startedAt: '2026-07-02T09:55:00.000Z',
    completedAt: now,
    exitCode: 0,
    outputArtifactId: 'artifact-1',
    createdAt: '2026-07-02T09:55:00.000Z',
    updatedAt: now,
    ...overrides,
  };
}
