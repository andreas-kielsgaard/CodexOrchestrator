import type { EntityId, TaskRun } from '../../domain/model';
import {
  applyTaskRunUpdate,
  cloneTaskRun,
  createTaskRunRecord,
  queryStoredTaskRuns,
  TaskRunNotFoundError,
  type CreateTaskRunInput,
  type TaskRunQuery,
  type TaskRunStore,
  type TaskRunStoreIdProvider,
  type TaskRunStoreTimeProvider,
  type UpdateTaskRunInput,
} from '../../domain/taskRunStore';
import { taskRunFromRow, taskRunToRow, type TaskRunRow } from './runConversationSchema';

export interface TaskRunSqliteStatement {
  all(...params: unknown[]): unknown[];
  get(...params: unknown[]): unknown | undefined;
  run(...params: unknown[]): unknown;
}

export interface TaskRunSqliteDatabase {
  prepare(sql: string): TaskRunSqliteStatement;
  exec?(sql: string): unknown;
}

export class SqliteTaskRunStore implements TaskRunStore {
  constructor(
    private readonly db: TaskRunSqliteDatabase,
    private readonly ids: TaskRunStoreIdProvider,
    private readonly clock: TaskRunStoreTimeProvider,
  ) {}

  async createTaskRun(input: CreateTaskRunInput): Promise<TaskRun> {
    return this.runInTransaction(() => {
      const taskRun = createTaskRunRecord(input, this.ids.nextId(), this.clock.now());

      this.insertTaskRun(taskRun);

      return taskRunFromRow(taskRunToRow(taskRun));
    });
  }

  async updateTaskRun(taskRunId: EntityId, input: UpdateTaskRunInput): Promise<TaskRun> {
    return this.runInTransaction(() => {
      const existingTaskRun = this.loadTaskRun(taskRunId);
      const updatedTaskRun = applyTaskRunUpdate(existingTaskRun, input, this.clock.now());

      this.updateTaskRunRow(updatedTaskRun);

      return cloneTaskRun(updatedTaskRun);
    });
  }

  async queryTaskRuns(query: TaskRunQuery = {}): Promise<TaskRun[]> {
    const rows = this.db
      .prepare('SELECT * FROM task_runs ORDER BY created_at, id')
      .all() as TaskRunRow[];

    return queryStoredTaskRuns(rows.map(taskRunFromRow), query).map(cloneTaskRun);
  }

  private loadTaskRun(taskRunId: EntityId): TaskRun {
    const row = this.db.prepare('SELECT * FROM task_runs WHERE id = ?').get(taskRunId) as
      TaskRunRow | undefined;

    if (row === undefined) {
      throw new TaskRunNotFoundError(taskRunId);
    }

    return taskRunFromRow(row);
  }

  private insertTaskRun(taskRun: TaskRun): void {
    const row = taskRunToRow(taskRun);
    this.db
      .prepare(
        `
INSERT INTO task_runs (
  id, task_id, conversation_id, worktree_id, execution_state, started_at, completed_at, exit_code,
  created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
`,
      )
      .run(
        row.id,
        row.task_id,
        row.conversation_id,
        row.worktree_id,
        row.execution_state,
        row.started_at,
        row.completed_at,
        row.exit_code,
        row.created_at,
        row.updated_at,
      );
  }

  private updateTaskRunRow(taskRun: TaskRun): void {
    const row = taskRunToRow(taskRun);
    this.db
      .prepare(
        `
UPDATE task_runs SET
  conversation_id = ?,
  worktree_id = ?,
  execution_state = ?,
  started_at = ?,
  completed_at = ?,
  exit_code = ?,
  updated_at = ?
WHERE id = ?
`,
      )
      .run(
        row.conversation_id,
        row.worktree_id,
        row.execution_state,
        row.started_at,
        row.completed_at,
        row.exit_code,
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
