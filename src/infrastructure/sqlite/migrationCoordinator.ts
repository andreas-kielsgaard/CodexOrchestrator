import { artifactValidationSqliteMigrations } from './artifactValidationSchema';
import { eventSqliteMigrations } from './eventSchema';
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

const archivedPrototypeMigrations = [
  { version: '006', id: '006_orchestration_drafts_schema', position: 5 },
  { version: '007', id: '007_orchestration_stage_runs_schema', position: 6 },
  { version: '008', id: '008_agent_sessions_schema', position: 7 },
] as const;

export const appSqliteMigrations: readonly SqliteMigration[] = [
  ...repoSyncSqliteMigrations,
  ...taskSqliteMigrations,
  ...runConversationSqliteMigrations,
  ...artifactValidationSqliteMigrations,
  ...eventSqliteMigrations,
];

export function enableAppSqliteForeignKeys(db: AppSqliteMigrationDatabase): void {
  db.exec('PRAGMA foreign_keys = ON;');
}

export function applyAppSqliteMigrations(
  db: AppSqliteMigrationDatabase,
  options: ApplyAppSqliteMigrationsOptions = {},
): void {
  const migrations = [...(options.migrations ?? appSqliteMigrations)].sort(
    (left, right) => left.position - right.position,
  );
  assertValidMigrationRegistration(migrations);

  db.exec(`
CREATE TABLE IF NOT EXISTS schema_migrations (
  id TEXT PRIMARY KEY,
  applied_at TEXT NOT NULL,
  position INTEGER NOT NULL CHECK (position >= 0),
  UNIQUE (position)
);
`);

  const appliedRows = loadSchemaMigrationRows(db);
  const appliedById = new Map(appliedRows.map((row) => [row.id, row]));
  const appliedByPosition = new Map(appliedRows.map((row) => [row.position, row]));

  for (const migration of migrations) {
    const applied = appliedById.get(migration.id);
    if (applied && applied.position !== migration.position) {
      throw new Error(
        `SQLite migration ${migration.id} is recorded at position ${applied.position}; expected ${migration.position}`,
      );
    }

    const positionOwner = appliedByPosition.get(migration.position);
    if (positionOwner && positionOwner.id !== migration.id) {
      throw new Error(
        `SQLite migration position ${migration.position} is already recorded for ${positionOwner.id}; cannot apply ${migration.id}`,
      );
    }

    if (applied) {
      continue;
    }

    const appliedAt = migrationAppliedAt(migration, migration.position, options.appliedAt);
    db.exec('BEGIN');
    try {
      db.exec(migration.sql);
      db.prepare(
        `
INSERT INTO schema_migrations (id, applied_at, position)
VALUES (?, ?, ?)
`,
      ).run(migration.id, appliedAt, migration.position);
      db.exec('COMMIT');
    } catch (error) {
      db.exec('ROLLBACK');
      throw error;
    }

    const appliedRow: SchemaMigrationRow = {
      id: migration.id,
      applied_at: appliedAt,
      position: migration.position,
    };
    appliedById.set(appliedRow.id, appliedRow);
    appliedByPosition.set(appliedRow.position, appliedRow);
  }
}

export function loadSchemaMigrationRows(db: AppSqliteMigrationDatabase): SchemaMigrationRow[] {
  return db
    .prepare('SELECT id, applied_at, position FROM schema_migrations ORDER BY position')
    .all() as SchemaMigrationRow[];
}

function assertValidMigrationRegistration(migrations: readonly SqliteMigration[]): void {
  const seenIds = new Set<string>();
  const seenPositions = new Set<number>();

  for (const migration of migrations) {
    if (seenIds.has(migration.id)) {
      throw new Error(`Duplicate SQLite migration id: ${migration.id}`);
    }

    if (!Number.isSafeInteger(migration.position) || migration.position < 0) {
      throw new Error(
        `Invalid SQLite migration position for ${migration.id}: ${migration.position}`,
      );
    }

    if (seenPositions.has(migration.position)) {
      throw new Error(`Duplicate SQLite migration position: ${migration.position}`);
    }

    const reserved = archivedPrototypeMigrations.find(
      (prototype) =>
        migration.id.startsWith(`${prototype.version}_`) ||
        prototype.position === migration.position,
    );
    if (reserved) {
      throw new Error(
        `SQLite migration ${migration.id} at position ${migration.position} reuses archived prototype migration ${reserved.id} at position ${reserved.position}`,
      );
    }

    seenIds.add(migration.id);
    seenPositions.add(migration.position);
  }
}

function migrationAppliedAt(
  migration: SqliteMigration,
  position: number,
  appliedAt: ApplyAppSqliteMigrationsOptions['appliedAt'],
): string {
  return appliedAt?.(migration, position) ?? new Date().toISOString();
}
