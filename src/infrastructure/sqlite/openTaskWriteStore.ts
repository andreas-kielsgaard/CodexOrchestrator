import type { EntityId, Task } from '../../domain/model';
import {
  applyTaskUpdate,
  OpenTaskNotFoundError,
  type CreateOpenTaskInput,
  type IdProvider,
  type OpenTaskWriteStore,
  type TimeProvider,
  type UpdateOpenTaskInput,
} from '../../domain/openTaskWriteStore';
import {
  taskConversationLinksToRows,
  taskFromRow,
  taskToRow,
  type TaskConversationLinkRow,
  type TaskRow,
} from './taskSchema';

export interface OpenTaskWriteSqliteStatement {
  all(...params: unknown[]): unknown[];
  get(...params: unknown[]): unknown | undefined;
  run(...params: unknown[]): unknown;
}

export interface OpenTaskWriteSqliteDatabase {
  prepare(sql: string): OpenTaskWriteSqliteStatement;
  exec?(sql: string): unknown;
}

export class SqliteOpenTaskWriteStore implements OpenTaskWriteStore {
  constructor(
    private readonly db: OpenTaskWriteSqliteDatabase,
    private readonly ids: IdProvider,
    private readonly clock: TimeProvider,
  ) {}

  async createTask(input: CreateOpenTaskInput): Promise<Task> {
    return this.runInTransaction(() => {
      const now = this.clock.now();
      const task: Task = {
        id: this.ids.nextId(),
        projectId: input.projectId,
        ...(input.repoId === undefined ? {} : { repoId: input.repoId }),
        ...(input.branchId === undefined ? {} : { branchId: input.branchId }),
        ...(input.worktreeId === undefined ? {} : { worktreeId: input.worktreeId }),
        conversationIds: [...(input.conversationIds ?? [])],
        title: input.title,
        summary: input.summary,
        executionState: input.executionState ?? 'draft',
        attentionState: input.attentionState ?? 'consider_later',
        priority: input.priority ?? 'normal',
        ...(input.dueAt === undefined ? {} : { dueAt: input.dueAt }),
        ...(input.snoozedUntil === undefined ? {} : { snoozedUntil: input.snoozedUntil }),
        createdAt: now,
        updatedAt: now,
      };

      this.insertTask(task);
      this.insertConversationLinks(task, now);

      return cloneTask(task);
    });
  }

  async updateTask(taskId: EntityId, input: UpdateOpenTaskInput): Promise<Task> {
    return this.runInTransaction(() => {
      const existingTask = this.loadTask(taskId);
      const updatedTask = applyTaskUpdate(existingTask, input, this.clock.now());

      this.updateTaskRow(updatedTask);

      if (input.conversationIds !== undefined) {
        this.replaceConversationLinks(updatedTask, updatedTask.updatedAt);
      }

      return cloneTask(updatedTask);
    });
  }

  async archiveTask(taskId: EntityId): Promise<Task> {
    return this.updateTask(taskId, { executionState: 'archived' });
  }

  private loadTask(taskId: EntityId): Task {
    const row = this.db.prepare('SELECT * FROM tasks WHERE id = ?').get(taskId) as
      TaskRow | undefined;

    if (row === undefined) {
      throw new OpenTaskNotFoundError(taskId);
    }

    const linkRows = this.db
      .prepare('SELECT * FROM task_conversation_links WHERE task_id = ? ORDER BY position')
      .all(taskId) as TaskConversationLinkRow[];

    return taskFromRow(row, linkRows);
  }

  private insertTask(task: Task): void {
    const row = taskToRow(task);
    this.db
      .prepare(
        `
INSERT INTO tasks (
  id, project_id, repo_id, branch_id, worktree_id, title, summary, execution_state,
  attention_state, priority, due_at, snoozed_until, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
`,
      )
      .run(
        row.id,
        row.project_id,
        row.repo_id,
        row.branch_id,
        row.worktree_id,
        row.title,
        row.summary,
        row.execution_state,
        row.attention_state,
        row.priority,
        row.due_at,
        row.snoozed_until,
        row.created_at,
        row.updated_at,
      );
  }

  private updateTaskRow(task: Task): void {
    const row = taskToRow(task);
    this.db
      .prepare(
        `
UPDATE tasks SET
  project_id = ?,
  repo_id = ?,
  branch_id = ?,
  worktree_id = ?,
  title = ?,
  summary = ?,
  execution_state = ?,
  attention_state = ?,
  priority = ?,
  due_at = ?,
  snoozed_until = ?,
  created_at = ?,
  updated_at = ?
WHERE id = ?
`,
      )
      .run(
        row.project_id,
        row.repo_id,
        row.branch_id,
        row.worktree_id,
        row.title,
        row.summary,
        row.execution_state,
        row.attention_state,
        row.priority,
        row.due_at,
        row.snoozed_until,
        row.created_at,
        row.updated_at,
        row.id,
      );
  }

  private replaceConversationLinks(task: Task, createdAt: string): void {
    this.db.prepare('DELETE FROM task_conversation_links WHERE task_id = ?').run(task.id);
    this.insertConversationLinks(task, createdAt);
  }

  private insertConversationLinks(task: Task, createdAt: string): void {
    for (const row of taskConversationLinksToRows(task, createdAt)) {
      this.db
        .prepare(
          `
INSERT INTO task_conversation_links (task_id, conversation_id, position, created_at)
VALUES (?, ?, ?, ?)
`,
        )
        .run(row.task_id, row.conversation_id, row.position, row.created_at);
    }
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

function cloneTask(task: Task): Task {
  return {
    ...task,
    conversationIds: [...task.conversationIds],
  };
}
