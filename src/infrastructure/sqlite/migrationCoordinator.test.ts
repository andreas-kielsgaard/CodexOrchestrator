import { DatabaseSync } from 'node:sqlite';

import type { SqliteMigration } from './repoSyncSchema';
import {
  appSqliteMigrations,
  applyAppSqliteMigrations,
  enableAppSqliteForeignKeys,
  loadSchemaMigrationRows,
} from './migrationCoordinator';

describe('SQLite migration coordinator', () => {
  it('creates all current app tables through the coordinated migration list', () => {
    const db = openDatabase();

    try {
      applyAppSqliteMigrations(db, { appliedAt: deterministicAppliedAt });

      expect(tableNames(db)).toEqual([
        'branches',
        'conversations',
        'projects',
        'repos',
        'schema_migrations',
        'task_conversation_links',
        'task_runs',
        'tasks',
        'worktrees',
      ]);
    } finally {
      db.close();
    }
  });

  it('records migration IDs in stable order with deterministic timestamps', () => {
    const db = openDatabase();

    try {
      applyAppSqliteMigrations(db, { appliedAt: deterministicAppliedAt });

      expect(loadSchemaMigrationRows(db)).toEqual(
        appSqliteMigrations.map((migration, position) => ({
          id: migration.id,
          applied_at: deterministicAppliedAt(migration, position),
          position,
        })),
      );
    } finally {
      db.close();
    }
  });

  it('is idempotent when migrations are applied more than once', () => {
    const db = openDatabase();

    try {
      applyAppSqliteMigrations(db, { appliedAt: deterministicAppliedAt });
      const firstRows = loadSchemaMigrationRows(db);

      applyAppSqliteMigrations(db, {
        appliedAt: (migration, position) => `rerun-${position}-${migration.id}`,
      });

      expect(loadSchemaMigrationRows(db)).toEqual(firstRows);
      expect(tableNames(db)).toEqual([
        'branches',
        'conversations',
        'projects',
        'repos',
        'schema_migrations',
        'task_conversation_links',
        'task_runs',
        'tasks',
        'worktrees',
      ]);
    } finally {
      db.close();
    }
  });

  it('rejects duplicate migration IDs before applying migrations', () => {
    const db = openDatabase();
    const duplicateMigrations: SqliteMigration[] = [
      {
        id: '001_duplicate',
        sql: 'CREATE TABLE one (id TEXT PRIMARY KEY);',
      },
      {
        id: '001_duplicate',
        sql: 'CREATE TABLE two (id TEXT PRIMARY KEY);',
      },
    ];

    try {
      expect(() => applyAppSqliteMigrations(db, { migrations: duplicateMigrations })).toThrow(
        'Duplicate SQLite migration id: 001_duplicate',
      );
      expect(tableNames(db)).toEqual([]);
    } finally {
      db.close();
    }
  });

  it('does not record failed migrations', () => {
    const db = openDatabase();
    const migrations: SqliteMigration[] = [
      {
        id: '001_create_valid_table',
        sql: 'CREATE TABLE valid_table (id TEXT PRIMARY KEY);',
      },
      {
        id: '002_fail_after_ddl',
        sql: `
CREATE TABLE should_roll_back (id TEXT PRIMARY KEY);
INSERT INTO missing_table (id) VALUES ('missing');
`,
      },
    ];

    try {
      expect(() =>
        applyAppSqliteMigrations(db, {
          appliedAt: deterministicAppliedAt,
          migrations,
        }),
      ).toThrow();

      expect(loadSchemaMigrationRows(db)).toEqual([
        {
          id: '001_create_valid_table',
          applied_at: deterministicAppliedAt(migrations[0], 0),
          position: 0,
        },
      ]);
      expect(tableNames(db)).toEqual(['schema_migrations', 'valid_table']);
    } finally {
      db.close();
    }
  });

  it('enables foreign-key enforcement for coordinated app schema setup', () => {
    const db = openDatabase();

    try {
      enableAppSqliteForeignKeys(db);
      applyAppSqliteMigrations(db, { appliedAt: deterministicAppliedAt });

      expect(() =>
        db
          .prepare(
            `
INSERT INTO repos (
  id, project_id, name, root_path, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?)
`,
          )
          .run(
            'repo-without-project',
            'missing-project',
            'Repo without project',
            'C:/Repos/Missing',
            '2026-07-02T10:00:00.000Z',
            '2026-07-02T10:00:00.000Z',
          ),
      ).toThrow();
    } finally {
      db.close();
    }
  });
});

function openDatabase(): DatabaseSync {
  return new DatabaseSync(':memory:');
}

function tableNames(db: DatabaseSync): string[] {
  return db
    .prepare("SELECT name FROM sqlite_master WHERE type = 'table' ORDER BY name")
    .all()
    .map((row) => (row as { name: string }).name);
}

function deterministicAppliedAt(_migration: SqliteMigration, position: number): string {
  return `2026-07-02T10:00:0${position}.000Z`;
}
