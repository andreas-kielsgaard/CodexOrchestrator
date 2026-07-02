import { DatabaseSync } from 'node:sqlite';

import type {
  Artifact,
  Branch,
  Conversation,
  Event,
  Project,
  Repo,
  Task,
  TaskRun,
  ValidationRun,
  Worktree,
} from '../../domain/model';
import { artifactToRow, validationRunToRow } from './artifactValidationSchema';
import { eventFromRow, eventSqliteMigrations, eventToRow, type EventRow } from './eventSchema';
import { applyAppSqliteMigrations, enableAppSqliteForeignKeys } from './migrationCoordinator';
import { branchToRow, projectToRow, repoToRow, worktreeToRow } from './repoSyncSchema';
import { conversationToRow, taskRunToRow } from './runConversationSchema';
import { taskToRow } from './taskSchema';

const now = '2026-07-02T10:00:00.000Z';

describe('event SQLite schema', () => {
  it('creates the events table through the app migration coordinator', () => {
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

  it('keeps event migrations separate from parent table definitions', () => {
    const migrationSql = eventSqliteMigrations.map((migration) => migration.sql).join('\n');

    expect(migrationSql).toContain('CREATE TABLE IF NOT EXISTS events');
    expect(migrationSql).not.toContain('CREATE TABLE IF NOT EXISTS artifacts');
    expect(migrationSql).not.toContain('CREATE TABLE IF NOT EXISTS validation_runs');
  });

  it('enforces event-kind check constraints', () => {
    const db = openMigratedDatabase();

    try {
      insertFullParentGraph(db);

      expect(() =>
        insertRow(db, 'events', {
          ...eventToRow(event()),
          kind: 'repo_scanned',
        }),
      ).toThrow();
    } finally {
      db.close();
    }
  });

  it('sets optional links to NULL when related records are cleaned up', () => {
    const db = openMigratedDatabase();

    try {
      insertFullParentGraph(db);
      insertRow(db, 'events', eventToRow(event()));

      db.prepare('DELETE FROM projects WHERE id = ?').run('project-1');

      expect(selectOne<EventRow>(db, 'events', 'event-1')).toMatchObject({
        project_id: null,
        task_id: null,
        task_run_id: null,
      });

      db.prepare('DELETE FROM conversations WHERE id = ?').run('conversation-1');
      expect(selectOne<EventRow>(db, 'events', 'event-1').conversation_id).toBeNull();

      db.prepare('DELETE FROM artifacts WHERE id = ?').run('artifact-1');
      expect(selectOne<EventRow>(db, 'events', 'event-1').artifact_id).toBeNull();

      db.prepare('DELETE FROM validation_runs WHERE id = ?').run('validation-1');
      expect(selectOne<EventRow>(db, 'events', 'event-1').validation_run_id).toBeNull();
    } finally {
      db.close();
    }
  });

  it('round-trips optional fields as NULL and payload JSON through row mappers', () => {
    const db = openMigratedDatabase();
    const minimalEvent: Event = {
      id: 'event-1',
      kind: 'run_event',
      occurredAt: now,
      payload: {
        zeta: true,
        alpha: {
          second: 2,
          first: 1,
        },
        list: [{ b: 'two', a: 'one' }],
      },
    };

    try {
      insertRow(db, 'events', eventToRow(minimalEvent));

      const row = selectOne<EventRow>(db, 'events', 'event-1');

      expect(row.project_id).toBeNull();
      expect(row.task_id).toBeNull();
      expect(row.task_run_id).toBeNull();
      expect(row.conversation_id).toBeNull();
      expect(row.artifact_id).toBeNull();
      expect(row.validation_run_id).toBeNull();
      expect(row.payload_json).toBe(
        '{"alpha":{"first":1,"second":2},"list":[{"a":"one","b":"two"}],"zeta":true}',
      );
      expect(eventFromRow(row)).toEqual(minimalEvent);
    } finally {
      db.close();
    }
  });

  it('throws clear mapper errors for invalid JSON payload rows', () => {
    expect(() =>
      eventFromRow({
        ...eventToRow(event()),
        payload_json: '{not-json',
      }),
    ).toThrow('Invalid JSON payload for event event-1');

    expect(() =>
      eventFromRow({
        ...eventToRow(event()),
        payload_json: '["not", "an", "object"]',
      }),
    ).toThrow('Invalid JSON payload for event event-1: expected a JSON object');
  });
});

function openMigratedDatabase(): DatabaseSync {
  const db = new DatabaseSync(':memory:');
  enableAppSqliteForeignKeys(db);
  applyAppSqliteMigrations(db, { appliedAt: (_migration, position) => `${now}:${position}` });
  return db;
}

function insertFullParentGraph(db: DatabaseSync): void {
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
  insertRow(db, 'artifacts', artifactToRow(artifact()));
  insertRow(db, 'validation_runs', validationRunToRow(validationRun()));
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
    title: 'Persist event schema',
    summary: 'Add the event table and mapper foundation.',
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
    title: 'Worker 018 schema foundation',
    summary: 'Event schema work.',
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
    title: 'Event schema validation',
    uri: 'file:///tmp/event-schema.log',
    content: 'event schema tests pass',
    createdAt: now,
    ...overrides,
  };
}

function validationRun(overrides: Partial<ValidationRun> = {}): ValidationRun {
  return {
    id: 'validation-1',
    taskId: 'task-1',
    taskRunId: 'run-1',
    command: 'npm run test -- src/infrastructure/sqlite/eventSchema.test.ts',
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

function event(overrides: Partial<Event> = {}): Event {
  return {
    id: 'event-1',
    kind: 'run_event',
    occurredAt: now,
    projectId: 'project-1',
    taskId: 'task-1',
    taskRunId: 'run-1',
    conversationId: 'conversation-1',
    artifactId: 'artifact-1',
    validationRunId: 'validation-1',
    payload: {
      message: 'schema foundation created',
      sequence: 1,
    },
    ...overrides,
  };
}
