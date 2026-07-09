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

  it('records migration IDs in stable order with deterministic timestamps', () => {
    const db = openDatabase();

    try {
      applyAppSqliteMigrations(db, { appliedAt: deterministicAppliedAt });

      expect(loadSchemaMigrationRows(db)).toEqual(
        appSqliteMigrations.map((migration) => ({
          id: migration.id,
          applied_at: deterministicAppliedAt(migration, migration.position),
          position: migration.position,
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

  it('rejects duplicate migration IDs before applying migrations', () => {
    const db = openDatabase();
    const duplicateMigrations: SqliteMigration[] = [
      {
        id: '001_duplicate',
        position: 0,
        sql: 'CREATE TABLE one (id TEXT PRIMARY KEY);',
      },
      {
        id: '001_duplicate',
        position: 1,
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

  it('uses explicit positions when filling a migration-ledger gap', () => {
    const db = openDatabase();
    const firstAndThird: SqliteMigration[] = [
      {
        id: '003_third',
        position: 2,
        sql: 'CREATE TABLE third_table (id TEXT PRIMARY KEY);',
      },
      {
        id: '001_first',
        position: 0,
        sql: 'CREATE TABLE first_table (id TEXT PRIMARY KEY);',
      },
    ];
    const second: SqliteMigration = {
      id: '002_second',
      position: 1,
      sql: 'CREATE TABLE second_table (id TEXT PRIMARY KEY);',
    };

    try {
      applyAppSqliteMigrations(db, {
        appliedAt: deterministicAppliedAt,
        migrations: firstAndThird,
      });
      applyAppSqliteMigrations(db, {
        appliedAt: deterministicAppliedAt,
        migrations: [...firstAndThird, second],
      });

      expect(loadSchemaMigrationRows(db)).toEqual([
        {
          id: '001_first',
          applied_at: deterministicAppliedAt(firstAndThird[1], 0),
          position: 0,
        },
        {
          id: '002_second',
          applied_at: deterministicAppliedAt(second, 1),
          position: 1,
        },
        {
          id: '003_third',
          applied_at: deterministicAppliedAt(firstAndThird[0], 2),
          position: 2,
        },
      ]);
    } finally {
      db.close();
    }
  });

  it('initializes current schema around archived prototype ledger positions', () => {
    const db = openDatabase();

    try {
      createMigrationLedger(db);
      const insert = db.prepare(
        'INSERT INTO schema_migrations (id, applied_at, position) VALUES (?, ?, ?)',
      );
      insert.run('006_orchestration_drafts_schema', 'prototype-006', 5);
      insert.run('007_orchestration_stage_runs_schema', 'prototype-007', 6);
      insert.run('008_agent_sessions_schema', 'prototype-008', 7);

      applyAppSqliteMigrations(db, { appliedAt: deterministicAppliedAt });

      expect(loadSchemaMigrationRows(db)).toEqual([
        ...appSqliteMigrations.map((migration) => ({
          id: migration.id,
          applied_at: deterministicAppliedAt(migration, migration.position),
          position: migration.position,
        })),
        {
          id: '006_orchestration_drafts_schema',
          applied_at: 'prototype-006',
          position: 5,
        },
        {
          id: '007_orchestration_stage_runs_schema',
          applied_at: 'prototype-007',
          position: 6,
        },
        {
          id: '008_agent_sessions_schema',
          applied_at: 'prototype-008',
          position: 7,
        },
      ]);
    } finally {
      db.close();
    }
  });

  it('rejects reuse of an archived prototype migration position', () => {
    const db = openDatabase();
    const migration: SqliteMigration = {
      id: '009_new_schema_at_wrong_position',
      position: 5,
      sql: 'CREATE TABLE wrong_position (id TEXT PRIMARY KEY);',
    };

    try {
      expect(() => applyAppSqliteMigrations(db, { migrations: [migration] })).toThrow(
        'reuses archived prototype migration 006_orchestration_drafts_schema at position 5',
      );
      expect(tableNames(db)).toEqual([]);
    } finally {
      db.close();
    }
  });

  it('rejects reuse of an archived prototype migration version', () => {
    const db = openDatabase();
    const migration: SqliteMigration = {
      id: '008_replacement_agent_sessions_schema',
      position: 8,
      sql: 'CREATE TABLE wrong_version (id TEXT PRIMARY KEY);',
    };

    try {
      expect(() => applyAppSqliteMigrations(db, { migrations: [migration] })).toThrow(
        'reuses archived prototype migration 008_agent_sessions_schema at position 7',
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
        position: 0,
        sql: 'CREATE TABLE valid_table (id TEXT PRIMARY KEY);',
      },
      {
        id: '002_fail_after_ddl',
        position: 1,
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

function createMigrationLedger(db: DatabaseSync): void {
  db.exec(`
CREATE TABLE schema_migrations (
  id TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  UNIQUE (position)
);
`);
}

function deterministicAppliedAt(_migration: SqliteMigration, position: number): string {
  return `2026-07-02T10:00:0${position}.000Z`;
}
