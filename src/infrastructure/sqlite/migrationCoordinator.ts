import { artifactValidationSqliteMigrations } from './artifactValidationSchema';
import { repoSyncSqliteMigrations, type SqliteMigration } from './repoSyncSchema';
import { runConversationSqliteMigrations } from './runConversationSchema';
import { taskSqliteMigrations } from './taskSchema';

export interface AppSqliteMigrationStatement {
  all(...params: unknown[]): unknown[];
  get(...params: unknown[]): unknown | undefined;
  run(...params: unknown[]): unknown;
}

export interface AppSqliteMigrationDatabase {
  exec(sql: string): unknown;
  prepare(sql: string): AppSqliteMigrationStatement;
}

export interface SchemaMigrationRow {
  id: string;
  applied_at: string;
  position: number;
}

export interface ApplyAppSqliteMigrationsOptions {
  appliedAt?: (migration: SqliteMigration, position: number) => string;
  migrations?: readonly SqliteMigration[];
}

export const appSqliteMigrations: readonly SqliteMigration[] = [
  ...repoSyncSqliteMigrations,
  ...taskSqliteMigrations,
  ...runConversationSqliteMigrations,
  ...artifactValidationSqliteMigrations,
];

export function enableAppSqliteForeignKeys(db: AppSqliteMigrationDatabase): void {
  db.exec('PRAGMA foreign_keys = ON;');
}

export function applyAppSqliteMigrations(
  db: AppSqliteMigrationDatabase,
  options: ApplyAppSqliteMigrationsOptions = {},
): void {
  const migrations = [...(options.migrations ?? appSqliteMigrations)];
  assertUniqueMigrationIds(migrations);

  db.exec(`
CREATE TABLE IF NOT EXISTS schema_migrations (
  id TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  UNIQUE (position)
);
`);

  const appliedIds = new Set(
    db
      .prepare('SELECT id FROM schema_migrations ORDER BY position')
      .all()
      .map((row) => (row as { id: string }).id),
  );

  migrations.forEach((migration, position) => {
    if (appliedIds.has(migration.id)) {
      return;
    }

    db.exec('BEGIN');
    try {
      db.exec(migration.sql);
      db.prepare(
        `
INSERT INTO schema_migrations (id, applied_at, position)
VALUES (?, ?, ?)
`,
      ).run(migration.id, migrationAppliedAt(migration, position, options.appliedAt), position);
      db.exec('COMMIT');
    } catch (error) {
      db.exec('ROLLBACK');
      throw error;
    }
  });
}

export function loadSchemaMigrationRows(db: AppSqliteMigrationDatabase): SchemaMigrationRow[] {
  return db
    .prepare('SELECT id, applied_at, position FROM schema_migrations ORDER BY position')
    .all() as SchemaMigrationRow[];
}

function assertUniqueMigrationIds(migrations: readonly SqliteMigration[]): void {
  const seen = new Set<string>();

  for (const migration of migrations) {
    if (seen.has(migration.id)) {
      throw new Error(`Duplicate SQLite migration id: ${migration.id}`);
    }

    seen.add(migration.id);
  }
}

function migrationAppliedAt(
  migration: SqliteMigration,
  position: number,
  appliedAt: ApplyAppSqliteMigrationsOptions['appliedAt'],
): string {
  return appliedAt?.(migration, position) ?? new Date().toISOString();
}
