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
import { eventFromRow, eventToRow, type EventRow } from './eventSchema';
import { SqliteEventStore } from './eventStore';
import { applyAppSqliteMigrations, enableAppSqliteForeignKeys } from './migrationCoordinator';
import { branchToRow, projectToRow, repoToRow, worktreeToRow } from './repoSyncSchema';
import { conversationToRow, taskRunToRow } from './runConversationSchema';
import { taskToRow } from './taskSchema';

const now = '2026-07-02T10:00:00.000Z';
const createdAt = '2026-07-01T09:00:00.000Z';

describe('SqliteEventStore', () => {
  it('appends and round-trips an event through the app-migrated events table', async () => {
    const db = openMigratedDatabase();

    try {
      insertFullParentGraph(db);

      const appended = await createStore(db).appendEvent({
        kind: 'run_event',
        projectId: 'project-1',
        taskId: 'task-1',
        taskRunId: 'run-1',
        conversationId: 'conversation-1',
        artifactId: 'artifact-1',
        validationRunId: 'validation-1',
        payload: {
          zeta: true,
          alpha: {
            second: 2,
            first: 1,
          },
        },
      });

      expect(appended).toEqual({
        id: 'event-created',
        kind: 'run_event',
        occurredAt: now,
        projectId: 'project-1',
        taskId: 'task-1',
        taskRunId: 'run-1',
        conversationId: 'conversation-1',
        artifactId: 'artifact-1',
        validationRunId: 'validation-1',
        payload: {
          alpha: {
            first: 1,
            second: 2,
          },
          zeta: true,
        },
      });
      expect(loadEvent(db, 'event-created')).toEqual(appended);
      expect(selectOne<EventRow>(db, 'events', 'event-created').payload_json).toBe(
        '{"alpha":{"first":1,"second":2},"zeta":true}',
      );
    } finally {
      db.close();
    }
  });

  it('persists omitted optional links as SQL NULL and default payload as an object', async () => {
    const db = openMigratedDatabase();

    try {
      const appended = await createStore(db).appendEvent({
        kind: 'task_created',
      });
      const row = selectOne<EventRow>(db, 'events', 'event-created');

      expect(appended.payload).toEqual({});
      expect(row.project_id).toBeNull();
      expect(row.task_id).toBeNull();
      expect(row.task_run_id).toBeNull();
      expect(row.conversation_id).toBeNull();
      expect(row.artifact_id).toBeNull();
      expect(row.validation_run_id).toBeNull();
      expect(row.payload_json).toBe('{}');
    } finally {
      db.close();
    }
  });

  it('serializes payloads at append time so later caller mutation cannot affect storage', async () => {
    const db = openMigratedDatabase();
    const payload = {
      nested: {
        status: 'created',
      },
    };

    try {
      await createStore(db).appendEvent({
        kind: 'task_created',
        payload,
      });

      payload.nested.status = 'mutated-after-append';

      await expect(createStore(db).queryEvents()).resolves.toEqual([
        {
          id: 'event-created',
          kind: 'task_created',
          occurredAt: now,
          payload: {
            nested: {
              status: 'created',
            },
          },
        },
      ]);
    } finally {
      db.close();
    }
  });

  it('queries by kind and optional linked ids with chronological ordering and limits', async () => {
    const db = openMigratedDatabase();

    try {
      insertFullParentGraph(db);
      insertRow(
        db,
        'events',
        eventToRow(
          event({
            id: 'event-b',
            kind: 'run_event',
            occurredAt: '2026-07-02T10:00:01.000Z',
            taskId: 'task-1',
            conversationId: 'conversation-1',
          }),
        ),
      );
      insertRow(
        db,
        'events',
        eventToRow(
          event({
            id: 'event-a',
            kind: 'run_event',
            occurredAt: '2026-07-02T10:00:01.000Z',
            taskId: 'task-1',
            conversationId: 'conversation-1',
          }),
        ),
      );
      insertRow(
        db,
        'events',
        eventToRow(
          event({
            id: 'event-c',
            kind: 'run_completed',
            occurredAt: '2026-07-02T09:59:59.000Z',
            taskId: 'task-1',
            conversationId: 'conversation-1',
          }),
        ),
      );
      insertRow(
        db,
        'events',
        eventToRow(
          event({
            id: 'event-d',
            kind: 'run_event',
            occurredAt: '2026-07-02T10:00:02.000Z',
            taskId: 'task-1',
          }),
        ),
      );

      await expect(
        createStore(db).queryEvents({
          kind: 'run_event',
          taskId: 'task-1',
          conversationId: 'conversation-1',
          limit: 1,
        }),
      ).resolves.toEqual([
        event({
          id: 'event-a',
          kind: 'run_event',
          occurredAt: '2026-07-02T10:00:01.000Z',
          taskId: 'task-1',
          conversationId: 'conversation-1',
        }),
      ]);
    } finally {
      db.close();
    }
  });

  it('returns empty results for unmatched filters', async () => {
    const db = openMigratedDatabase();

    try {
      insertRow(db, 'events', eventToRow(event()));

      await expect(createStore(db).queryEvents({ validationRunId: 'missing' })).resolves.toEqual(
        [],
      );
    } finally {
      db.close();
    }
  });

  it('rolls back appended rows when transaction-backed persistence fails', async () => {
    const db = openMigratedDatabase();

    try {
      insertRow(db, 'events', eventToRow(event({ id: 'event-created' })));

      await expect(
        createStore(db).appendEvent({
          kind: 'task_updated',
          payload: {
            duplicate: true,
          },
        }),
      ).rejects.toThrow();

      expect(selectAll<EventRow>(db, 'events')).toHaveLength(1);
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

function createStore(db: DatabaseSync): SqliteEventStore {
  return new SqliteEventStore(
    db,
    {
      nextId: () => 'event-created',
    },
    {
      now: () => now,
    },
  );
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

function loadEvent(db: DatabaseSync, eventId: string): Event {
  return eventFromRow(selectOne<EventRow>(db, 'events', eventId));
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
    conversationIds: ['conversation-1'],
    title: 'Persist event store',
    summary: 'Add event append and query behavior.',
    executionState: 'running',
    attentionState: 'waiting_on_agent',
    priority: 'high',
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
    executionState: 'running',
    startedAt: createdAt,
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
    externalThreadId: '019f22c0-b8fa-7092-8a7f-0171f72455c5',
    title: 'Worker 019 event store boundary',
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
    title: 'Event store validation',
    createdAt,
    ...overrides,
  };
}

function validationRun(overrides: Partial<ValidationRun> = {}): ValidationRun {
  return {
    id: 'validation-1',
    taskId: 'task-1',
    taskRunId: 'run-1',
    command: 'npm run test -- src/infrastructure/sqlite/eventStore.test.ts',
    status: 'queued',
    outputArtifactId: 'artifact-1',
    createdAt,
    updatedAt: createdAt,
    ...overrides,
  };
}

function event(overrides: Partial<Event> = {}): Event {
  return {
    id: 'event-1',
    kind: 'run_event',
    occurredAt: now,
    payload: {
      status: 'running',
    },
    ...overrides,
  };
}
