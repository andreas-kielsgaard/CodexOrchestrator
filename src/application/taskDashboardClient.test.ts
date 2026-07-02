import { mkdtempSync, rmSync } from 'node:fs';
import { tmpdir } from 'node:os';
import { join } from 'node:path';

import type { EntityId, IsoDateTime, Project } from '../domain/model';
import {
  InMemoryOpenTaskWriteStore,
  type IdProvider,
  type TimeProvider,
} from '../domain/openTaskWriteStore';
import { seedDomainRecords } from '../domain/seedData';
import {
  createStoreBackedTaskDashboardClient,
  type TaskDashboardSnapshot,
} from './taskDashboardClient';
import type {
  AppSqliteStoreBundleProviders,
  AppSqliteWriteStoreProviders,
} from '../infrastructure/sqlite/appStore';
import { openLocalAppSqliteDatabase } from '../infrastructure/sqlite/localAppDatabase';
import type { AppSqliteMigrationDatabase } from '../infrastructure/sqlite/migrationCoordinator';

const now = '2026-07-02T12:00:00.000Z';

describe('task dashboard client', () => {
  it('loads, creates, updates, and archives tasks through injected store boundaries', async () => {
    const store = new InMemoryOpenTaskWriteStore(
      {
        ...seedDomainRecords,
        tasks: [],
      },
      deterministicIds('task'),
      deterministicClock(),
    );
    const client = createStoreBackedTaskDashboardClient({ dashboard: store, write: store });

    expect((await client.loadDashboard()).totalOpenTasks).toBe(0);

    const created = await client.createTask({
      projectId: 'project-orchestrator',
      title: 'Persist dashboard interactions',
      summary: 'Route dashboard writes through a client boundary.',
    });

    expect(findDashboardTask(created, 'task-1')).toMatchObject({
      title: 'Persist dashboard interactions',
      attentionState: 'needs_action_now',
      executionState: 'draft',
    });

    const updated = await client.updateTask('task-1', {
      title: 'Updated dashboard task',
      summary: 'Edits are persisted through the write store.',
      attentionState: 'needs_review',
      executionState: 'completed',
      priority: 'high',
    });

    expect(updated.groups.find((group) => group.id === 'review_decide')?.tasks[0]).toMatchObject({
      id: 'task-1',
      title: 'Updated dashboard task',
      executionState: 'completed',
      attentionState: 'needs_review',
    });

    const archived = await client.archiveTask('task-1');

    expect(archived.totalOpenTasks).toBe(0);
    expect(archived.groups.flatMap((group) => group.tasks)).toEqual([]);
  });

  it('works against the local SQLite app database bundle without importing SQLite into UI code', async () => {
    const tempDir = mkdtempSync(join(tmpdir(), 'codex-orchestrator-task-dashboard-'));
    const databasePath = join(tempDir, 'app.sqlite');
    const localDatabase = openLocalAppSqliteDatabase(databasePath, {
      initialize: { migrations: { appliedAt: deterministicAppliedAt } },
      providers: createProviders(),
    });

    try {
      seedProject(localDatabase.db, {
        id: 'project-1',
        name: 'Codex Orchestrator',
        createdAt: now,
        updatedAt: now,
      });

      const client = createStoreBackedTaskDashboardClient({
        dashboard: localDatabase.stores.openTaskDashboard,
        write: localDatabase.stores.openTaskWrite,
      });

      const created = await client.createTask({
        projectId: 'project-1',
        title: 'SQLite-backed dashboard task',
        summary: 'Create through the application client into local SQLite.',
      });
      const updated = await client.updateTask('task-1', {
        executionState: 'running',
        attentionState: 'waiting_on_agent',
      });

      expect(created.projects).toEqual([{ id: 'project-1', name: 'Codex Orchestrator' }]);
      expect(findDashboardTask(updated, 'task-1')).toMatchObject({
        id: 'task-1',
        title: 'SQLite-backed dashboard task',
        executionState: 'running',
      });

      localDatabase.close();

      const reopened = openLocalAppSqliteDatabase(databasePath, {
        initialize: { migrations: { appliedAt: deterministicAppliedAt } },
        providers: createProviders(),
      });

      try {
        const persisted = await createStoreBackedTaskDashboardClient({
          dashboard: reopened.stores.openTaskDashboard,
          write: reopened.stores.openTaskWrite,
        }).loadDashboard();

        expect(findDashboardTask(persisted, 'task-1')).toMatchObject({
          title: 'SQLite-backed dashboard task',
          executionState: 'running',
          attentionState: 'waiting_on_agent',
        });
      } finally {
        reopened.close();
      }
    } finally {
      localDatabase.close();
      rmSync(tempDir, { recursive: true, force: true });
    }
  });
});

function findDashboardTask(snapshot: TaskDashboardSnapshot, taskId: EntityId) {
  return snapshot.groups.flatMap((group) => group.tasks).find((task) => task.id === taskId);
}

function deterministicIds(prefix: string): IdProvider {
  let index = 0;

  return {
    nextId: () => `${prefix}-${++index}`,
  };
}

function deterministicClock(): TimeProvider {
  let index = 0;

  return {
    now: () => `2026-07-02T12:00:${(index++).toString().padStart(2, '0')}.000Z`,
  };
}

function deterministicAppliedAt(_migration: unknown, position: number): string {
  return `2026-07-02T12:01:${position.toString().padStart(2, '0')}.000Z`;
}

function seedProject(db: AppSqliteMigrationDatabase, project: Project): void {
  db.prepare(
    `
INSERT INTO projects (id, name, description, created_at, updated_at)
VALUES (?, ?, ?, ?, ?)
`,
  ).run(
    project.id,
    project.name,
    project.description ?? null,
    project.createdAt,
    project.updatedAt,
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
        `2026-07-02T12:02:${(timeIndex++).toString().padStart(2, '0')}.000Z` as IsoDateTime,
    },
  };
}
