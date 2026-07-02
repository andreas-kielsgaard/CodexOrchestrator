import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';
import { DatabaseSync } from 'node:sqlite';

import type { EntityId, IsoDateTime } from '../../domain/model';
import type {
  AppSqliteDatabase,
  AppSqliteStoreBundleProviders,
  AppSqliteWriteStoreProviders,
} from './appStore';
import { loadSchemaMigrationRows, type AppSqliteMigrationDatabase } from './migrationCoordinator';
import {
  createDefaultAppSqliteStoreBundleProviders,
  openLocalAppSqliteDatabase,
  type ClosableAppSqliteDatabase,
} from './localAppDatabase';

describe('local app SQLite database opener', () => {
  it('opens, initializes, and closes a local SQLite app store bundle', async () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'codex-orchestrator-db-'));
    const databasePath = join(tempDir, 'app.sqlite');
    const localDatabase = openLocalAppSqliteDatabase(databasePath, {
      initialize: { migrations: { appliedAt: deterministicAppliedAt } },
      providers: createProviders(),
    });

    try {
      expect(foreignKeysEnabled(localDatabase.db)).toBe(true);
      expect(loadSchemaMigrationRows(localDatabase.db)).toEqual(
        expect.arrayContaining([
          expect.objectContaining({
            id: '001_repo_sync_schema',
            applied_at: '2026-07-02T12:00:00.000Z',
            position: 0,
          }),
        ]),
      );

      seedProject(localDatabase.db);

      const task = await localDatabase.stores.openTaskWrite.createTask({
        projectId: 'project-1',
        title: 'Open the runtime database',
        summary: 'Return stores over an initialized local SQLite file.',
        executionState: 'queued',
        attentionState: 'waiting_on_agent',
        priority: 'high',
      });
      const dashboardRecords =
        await localDatabase.stores.openTaskDashboard.loadOpenTaskDashboardRecords();

      expect(task).toEqual(
        expect.objectContaining({
          id: 'task-1',
          createdAt: '2026-07-02T12:01:00.000Z',
        }),
      );
      expect(dashboardRecords.tasks).toEqual([expect.objectContaining({ id: task.id })]);

      localDatabase.close();
      localDatabase.dispose();

      expect(() => localDatabase.db.prepare('SELECT 1')).toThrow();
    } finally {
      localDatabase.close();
      rmSync(tempDir, { recursive: true, force: true });
    }
  });

  it('provides runtime default IDs and timestamps when providers are not injected', async () => {
    const defaultProviders = createDefaultAppSqliteStoreBundleProviders();

    expect(defaultProviders.openTask.ids.nextId()).toMatch(
      /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
    );
    expect(Date.parse(defaultProviders.openTask.clock.now())).not.toBeNaN();

    const db = new DatabaseSync(':memory:');
    const localDatabase = openLocalAppSqliteDatabase(':memory:', {
      openConnection: () => db,
      initialize: { migrations: { appliedAt: deterministicAppliedAt } },
    });

    try {
      seedProject(localDatabase.db);

      const task = await localDatabase.stores.openTaskWrite.createTask({
        projectId: 'project-1',
        title: 'Use runtime defaults',
        summary: 'Create IDs and timestamps without test providers.',
        executionState: 'draft',
        attentionState: 'consider_later',
        priority: 'normal',
      });

      expect(task.id).toMatch(
        /^[0-9a-f]{8}-[0-9a-f]{4}-[1-5][0-9a-f]{3}-[89ab][0-9a-f]{3}-[0-9a-f]{12}$/,
      );
      expect(Date.parse(task.createdAt)).not.toBeNaN();
    } finally {
      localDatabase.close();
    }
  });

  it('closes the opened connection when initialization fails', () => {
    let closed = false;
    const db: ClosableAppSqliteDatabase = {
      exec: (sql: string) => {
        if (sql === 'BROKEN SQL') {
          throw new Error('migration failed');
        }
      },
      prepare: () => ({
        all: () => [],
        get: () => undefined,
        run: () => undefined,
      }),
      close: () => {
        closed = true;
      },
    };

    expect(() =>
      openLocalAppSqliteDatabase(':memory:', {
        openConnection: () => db,
        initialize: {
          migrations: {
            migrations: [{ id: '001_broken', sql: 'BROKEN SQL' }],
            appliedAt: deterministicAppliedAt,
          },
        },
        providers: createProviders(),
      }),
    ).toThrow('migration failed');

    expect(closed).toBe(true);
  });
});

function foreignKeysEnabled(db: AppSqliteDatabase): boolean {
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

function createStoreProvider(prefix: string): AppSqliteWriteStoreProviders {
  let idIndex = 0;
  let timeIndex = 0;

  return {
    ids: {
      nextId: () => `${prefix}-${++idIndex}` as EntityId,
    },
    clock: {
      now: () =>
        `2026-07-02T12:01:${(timeIndex++).toString().padStart(2, '0')}.000Z` as IsoDateTime,
    },
  };
}
