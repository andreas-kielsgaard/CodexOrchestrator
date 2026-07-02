import { DatabaseSync } from 'node:sqlite';

import {
  appSqliteMigrations,
  loadSchemaMigrationRows,
  type AppSqliteMigrationDatabase,
} from './migrationCoordinator';
import {
  createAppSqliteStoreBundle,
  initializeAppSqliteStoreDatabase,
  type AppSqliteStoreBundleProviders,
} from './appStore';
import { SqliteArtifactStore } from './artifactStore';
import { SqliteConversationStore } from './conversationStore';
import { SqliteEventStore } from './eventStore';
import { SqliteOpenTaskDashboardStore } from './openTaskDashboardStore';
import { SqliteOpenTaskWriteStore } from './openTaskWriteStore';
import { SqliteRepoSyncStore } from './repoSyncStore';
import { SqliteTaskRunStore } from './taskRunStore';
import { SqliteValidationRunStore } from './validationRunStore';

describe('app SQLite store bundle', () => {
  it('initializes a SQLite database by enabling foreign keys and applying app migrations idempotently', () => {
    const db = openDatabase();

    try {
      initializeAppSqliteStoreDatabase(db, {
        migrations: { appliedAt: deterministicAppliedAt },
      });

      expect(foreignKeysEnabled(db)).toBe(true);
      expect(loadSchemaMigrationRows(db)).toEqual(
        appSqliteMigrations.map((migration, position) => ({
          id: migration.id,
          applied_at: deterministicAppliedAt(migration, position),
          position,
        })),
      );

      initializeAppSqliteStoreDatabase(db, {
        migrations: {
          appliedAt: (migration, position) => `rerun-${position}-${migration.id}`,
        },
      });

      expect(loadSchemaMigrationRows(db)).toEqual(
        appSqliteMigrations.map((migration, position) => ({
          id: migration.id,
          applied_at: deterministicAppliedAt(migration, position),
          position,
        })),
      );
      expect(() =>
        db
          .prepare(
            `
INSERT INTO tasks (
  id, project_id, title, summary, execution_state, attention_state, priority, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
`,
          )
          .run(
            'task-without-project',
            'missing-project',
            'Missing parent',
            'This should fail when foreign keys are enabled.',
            'draft',
            'consider_later',
            'normal',
            '2026-07-02T12:00:00.000Z',
            '2026-07-02T12:00:00.000Z',
          ),
      ).toThrow();
    } finally {
      db.close();
    }
  });

  it('returns the expected concrete SQLite store adapters over one migrated database', () => {
    const db = openDatabase();

    try {
      initializeAppSqliteStoreDatabase(db, {
        migrations: { appliedAt: deterministicAppliedAt },
      });

      const bundle = createAppSqliteStoreBundle(db, createProviders());

      expect(bundle.repoSync).toBeInstanceOf(SqliteRepoSyncStore);
      expect(bundle.openTaskDashboard).toBeInstanceOf(SqliteOpenTaskDashboardStore);
      expect(bundle.openTaskWrite).toBeInstanceOf(SqliteOpenTaskWriteStore);
      expect(bundle.event).toBeInstanceOf(SqliteEventStore);
      expect(bundle.taskRun).toBeInstanceOf(SqliteTaskRunStore);
      expect(bundle.conversation).toBeInstanceOf(SqliteConversationStore);
      expect(bundle.artifact).toBeInstanceOf(SqliteArtifactStore);
      expect(bundle.validationRun).toBeInstanceOf(SqliteValidationRunStore);
    } finally {
      db.close();
    }
  });

  it('lets assembled stores operate against the same initialized connection', async () => {
    const db = openDatabase();

    try {
      initializeAppSqliteStoreDatabase(db, {
        migrations: { appliedAt: deterministicAppliedAt },
      });
      seedProject(db);

      const bundle = createAppSqliteStoreBundle(db, createProviders());
      const task = await bundle.openTaskWrite.createTask({
        projectId: 'project-1',
        title: 'Wire app store bundle',
        summary: 'Assemble SQLite stores from one app-level factory.',
        executionState: 'queued',
        attentionState: 'waiting_on_agent',
        priority: 'high',
      });
      const taskRun = await bundle.taskRun.createTaskRun({
        taskId: task.id,
        executionState: 'running',
        startedAt: '2026-07-02T12:02:00.000Z',
      });
      const conversation = await bundle.conversation.createConversation({
        provider: 'codex',
        taskId: task.id,
        taskRunId: taskRun.id,
        externalThreadId: 'thread-024',
        title: 'Worker 024',
      });
      await bundle.taskRun.updateTaskRun(taskRun.id, {
        conversationId: conversation.id,
        executionState: 'completed',
        completedAt: '2026-07-02T12:10:00.000Z',
        exitCode: 0,
      });
      await bundle.openTaskWrite.updateTask(task.id, {
        conversationIds: [conversation.id],
        executionState: 'completed',
        attentionState: 'needs_review',
      });
      const artifact = await bundle.artifact.createArtifact({
        taskId: task.id,
        taskRunId: taskRun.id,
        conversationId: conversation.id,
        kind: 'summary',
        title: 'Completion summary',
        content: 'The app SQLite store bundle is assembled.',
      });
      const validationRun = await bundle.validationRun.createValidationRun({
        taskId: task.id,
        taskRunId: taskRun.id,
        command: 'npm run test -- src/infrastructure/sqlite/appStore.test.ts',
        status: 'passed',
        outputArtifactId: artifact.id,
      });
      await bundle.event.appendEvent({
        kind: 'validation_completed',
        projectId: 'project-1',
        taskId: task.id,
        taskRunId: taskRun.id,
        conversationId: conversation.id,
        artifactId: artifact.id,
        validationRunId: validationRun.id,
        payload: { status: 'passed' },
      });

      const dashboardRecords = await bundle.openTaskDashboard.loadOpenTaskDashboardRecords();
      const taskRuns = await bundle.taskRun.queryTaskRuns({ taskId: task.id });
      const conversations = await bundle.conversation.queryConversations({ taskId: task.id });
      const artifacts = await bundle.artifact.queryArtifacts({ taskId: task.id });
      const validationRuns = await bundle.validationRun.queryValidationRuns({ taskId: task.id });
      const events = await bundle.event.queryEvents({ taskId: task.id });

      expect(dashboardRecords.tasks).toEqual([
        expect.objectContaining({
          id: task.id,
          conversationIds: [conversation.id],
          executionState: 'completed',
          attentionState: 'needs_review',
        }),
      ]);
      expect(taskRuns).toEqual([
        expect.objectContaining({
          id: taskRun.id,
          conversationId: conversation.id,
          executionState: 'completed',
        }),
      ]);
      expect(conversations).toEqual([expect.objectContaining({ id: conversation.id })]);
      expect(artifacts).toEqual([expect.objectContaining({ id: artifact.id })]);
      expect(validationRuns).toEqual([expect.objectContaining({ id: validationRun.id })]);
      expect(events).toEqual([
        expect.objectContaining({
          kind: 'validation_completed',
          validationRunId: validationRun.id,
          payload: { status: 'passed' },
        }),
      ]);
    } finally {
      db.close();
    }
  });
});

function openDatabase(): DatabaseSync {
  return new DatabaseSync(':memory:');
}

function foreignKeysEnabled(db: DatabaseSync): boolean {
  return (db.prepare('PRAGMA foreign_keys').get() as { foreign_keys: number }).foreign_keys === 1;
}

function deterministicAppliedAt(_migration: unknown, position: number): string {
  return `2026-07-02T12:00:${position.toString().padStart(2, '0')}.000Z`;
}

function seedProject(db: AppSqliteMigrationDatabase): void {
  db.prepare(
    `
INSERT INTO projects (id, name, description, created_at, updated_at)
VALUES (?, ?, ?, ?, ?)
`,
  ).run(
    'project-1',
    'Codex Orchestrator',
    null,
    '2026-07-02T12:00:00.000Z',
    '2026-07-02T12:00:00.000Z',
  );
}

function createProviders(): AppSqliteStoreBundleProviders {
  return {
    openTask: createStoreProvider('task'),
    event: createStoreProvider('event'),
    taskRun: createStoreProvider('task-run'),
    conversation: createStoreProvider('conversation'),
    artifact: createStoreProvider('artifact'),
    validationRun: createStoreProvider('validation-run'),
  };
}

function createStoreProvider(prefix: string): AppSqliteStoreBundleProviders['openTask'] {
  let idIndex = 0;
  let timeIndex = 0;

  return {
    ids: {
      nextId: () => `${prefix}-${++idIndex}`,
    },
    clock: {
      now: () => `2026-07-02T12:01:${(timeIndex++).toString().padStart(2, '0')}.000Z`,
    },
  };
}
