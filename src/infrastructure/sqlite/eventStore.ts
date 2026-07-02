import type { Event } from '../../domain/model';
import {
  cloneEvent,
  queryStoredEvents,
  type AppendEventInput,
  type EventQuery,
  type EventStore,
  type EventStoreIdProvider,
  type EventStoreTimeProvider,
} from '../../domain/eventStore';
import { eventFromRow, eventToRow, type EventRow } from './eventSchema';

export interface EventSqliteStatement {
  all(...params: unknown[]): unknown[];
  get(...params: unknown[]): unknown | undefined;
  run(...params: unknown[]): unknown;
}

export interface EventSqliteDatabase {
  prepare(sql: string): EventSqliteStatement;
  exec?(sql: string): unknown;
}

export class SqliteEventStore implements EventStore {
  constructor(
    private readonly db: EventSqliteDatabase,
    private readonly ids: EventStoreIdProvider,
    private readonly clock: EventStoreTimeProvider,
  ) {}

  async appendEvent(input: AppendEventInput): Promise<Event> {
    return this.runInTransaction(() => {
      const occurredAt = this.clock.now();
      const event: Event = {
        id: this.ids.nextId(),
        kind: input.kind,
        occurredAt,
        ...(input.projectId === undefined ? {} : { projectId: input.projectId }),
        ...(input.taskId === undefined ? {} : { taskId: input.taskId }),
        ...(input.taskRunId === undefined ? {} : { taskRunId: input.taskRunId }),
        ...(input.conversationId === undefined ? {} : { conversationId: input.conversationId }),
        ...(input.artifactId === undefined ? {} : { artifactId: input.artifactId }),
        ...(input.validationRunId === undefined ? {} : { validationRunId: input.validationRunId }),
        payload: input.payload ?? {},
      };

      this.insertEvent(event);

      return eventFromRow(eventToRow(event));
    });
  }

  async queryEvents(query: EventQuery = {}): Promise<Event[]> {
    const rows = this.db
      .prepare('SELECT * FROM events ORDER BY occurred_at, id')
      .all() as EventRow[];

    return queryStoredEvents(rows.map(eventFromRow), query).map(cloneEvent);
  }

  private insertEvent(event: Event): void {
    const row = eventToRow(event);
    this.db
      .prepare(
        `
INSERT INTO events (
  id, kind, occurred_at, project_id, task_id, task_run_id, conversation_id, artifact_id,
  validation_run_id, payload_json
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
`,
      )
      .run(
        row.id,
        row.kind,
        row.occurred_at,
        row.project_id,
        row.task_id,
        row.task_run_id,
        row.conversation_id,
        row.artifact_id,
        row.validation_run_id,
        row.payload_json,
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
