import type { EntityId, ValidationRun } from '../../domain/model';
import {
  applyValidationRunUpdate,
  cloneValidationRun,
  createValidationRunRecord,
  queryStoredValidationRuns,
  ValidationRunNotFoundError,
  type CreateValidationRunInput,
  type UpdateValidationRunInput,
  type ValidationRunQuery,
  type ValidationRunStore,
  type ValidationRunStoreIdProvider,
  type ValidationRunStoreTimeProvider,
} from '../../domain/validationRunStore';
import {
  validationRunFromRow,
  validationRunToRow,
  type ValidationRunRow,
} from './artifactValidationSchema';

export interface ValidationRunSqliteStatement {
  all(...params: unknown[]): unknown[];
  get(...params: unknown[]): unknown | undefined;
  run(...params: unknown[]): unknown;
}

export interface ValidationRunSqliteDatabase {
  prepare(sql: string): ValidationRunSqliteStatement;
  exec?(sql: string): unknown;
}

export class SqliteValidationRunStore implements ValidationRunStore {
  constructor(
    private readonly db: ValidationRunSqliteDatabase,
    private readonly ids: ValidationRunStoreIdProvider,
    private readonly clock: ValidationRunStoreTimeProvider,
  ) {}

  async createValidationRun(input: CreateValidationRunInput): Promise<ValidationRun> {
    return this.runInTransaction(() => {
      const validationRun = createValidationRunRecord(input, this.ids.nextId(), this.clock.now());

      this.insertValidationRun(validationRun);

      return validationRunFromRow(validationRunToRow(validationRun));
    });
  }

  async updateValidationRun(
    validationRunId: EntityId,
    input: UpdateValidationRunInput,
  ): Promise<ValidationRun> {
    return this.runInTransaction(() => {
      const existingValidationRun = this.loadValidationRun(validationRunId);
      const updatedValidationRun = applyValidationRunUpdate(
        existingValidationRun,
        input,
        this.clock.now(),
      );

      this.updateValidationRunRow(updatedValidationRun);

      return cloneValidationRun(updatedValidationRun);
    });
  }

  async queryValidationRuns(query: ValidationRunQuery = {}): Promise<ValidationRun[]> {
    const rows = this.db
      .prepare('SELECT * FROM validation_runs ORDER BY created_at, id')
      .all() as ValidationRunRow[];

    return queryStoredValidationRuns(rows.map(validationRunFromRow), query).map(cloneValidationRun);
  }

  private loadValidationRun(validationRunId: EntityId): ValidationRun {
    const row = this.db
      .prepare('SELECT * FROM validation_runs WHERE id = ?')
      .get(validationRunId) as ValidationRunRow | undefined;

    if (row === undefined) {
      throw new ValidationRunNotFoundError(validationRunId);
    }

    return validationRunFromRow(row);
  }

  private insertValidationRun(validationRun: ValidationRun): void {
    const row = validationRunToRow(validationRun);
    this.db
      .prepare(
        `
INSERT INTO validation_runs (
  id, task_id, task_run_id, command, status, started_at, completed_at, exit_code,
  output_artifact_id, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
`,
      )
      .run(
        row.id,
        row.task_id,
        row.task_run_id,
        row.command,
        row.status,
        row.started_at,
        row.completed_at,
        row.exit_code,
        row.output_artifact_id,
        row.created_at,
        row.updated_at,
      );
  }

  private updateValidationRunRow(validationRun: ValidationRun): void {
    const row = validationRunToRow(validationRun);
    this.db
      .prepare(
        `
UPDATE validation_runs SET
  task_id = ?,
  task_run_id = ?,
  status = ?,
  started_at = ?,
  completed_at = ?,
  exit_code = ?,
  output_artifact_id = ?,
  updated_at = ?
WHERE id = ?
`,
      )
      .run(
        row.task_id,
        row.task_run_id,
        row.status,
        row.started_at,
        row.completed_at,
        row.exit_code,
        row.output_artifact_id,
        row.updated_at,
        row.id,
      );
  }

  private runInTransaction<T>(write: () => T): T {
    if (this.db.exec === undefined) {
      return write();
    }

    this.db.exec('BEGIN');
    try {
      const result = write();
      this.db.exec('COMMIT');
      return result;
    } catch (error) {
      this.db.exec('ROLLBACK');
      throw error;
    }
  }
}
