import type { Conversation, EntityId } from '../../domain/model';
import {
  applyConversationUpdate,
  cloneConversation,
  createConversationRecord,
  queryStoredConversations,
  ConversationNotFoundError,
  type ConversationQuery,
  type ConversationStore,
  type ConversationStoreIdProvider,
  type ConversationStoreTimeProvider,
  type CreateConversationInput,
  type UpdateConversationInput,
} from '../../domain/conversationStore';
import {
  conversationFromRow,
  conversationToRow,
  type ConversationRow,
} from './runConversationSchema';

export interface ConversationSqliteStatement {
  all(...params: unknown[]): unknown[];
  get(...params: unknown[]): unknown | undefined;
  run(...params: unknown[]): unknown;
}

export interface ConversationSqliteDatabase {
  prepare(sql: string): ConversationSqliteStatement;
  exec?(sql: string): unknown;
}

export class SqliteConversationStore implements ConversationStore {
  constructor(
    private readonly db: ConversationSqliteDatabase,
    private readonly ids: ConversationStoreIdProvider,
    private readonly clock: ConversationStoreTimeProvider,
  ) {}

  async createConversation(input: CreateConversationInput): Promise<Conversation> {
    return this.runInTransaction(() => {
      const conversation = createConversationRecord(input, this.ids.nextId(), this.clock.now());

      this.insertConversation(conversation);

      return conversationFromRow(conversationToRow(conversation));
    });
  }

  async updateConversation(
    conversationId: EntityId,
    input: UpdateConversationInput,
  ): Promise<Conversation> {
    return this.runInTransaction(() => {
      const existingConversation = this.loadConversation(conversationId);
      const updatedConversation = applyConversationUpdate(
        existingConversation,
        input,
        this.clock.now(),
      );

      this.updateConversationRow(updatedConversation);

      return cloneConversation(updatedConversation);
    });
  }

  async queryConversations(query: ConversationQuery = {}): Promise<Conversation[]> {
    const rows = this.db
      .prepare('SELECT * FROM conversations ORDER BY created_at, id')
      .all() as ConversationRow[];

    return queryStoredConversations(rows.map(conversationFromRow), query).map(cloneConversation);
  }

  private loadConversation(conversationId: EntityId): Conversation {
    const row = this.db.prepare('SELECT * FROM conversations WHERE id = ?').get(conversationId) as
      ConversationRow | undefined;

    if (row === undefined) {
      throw new ConversationNotFoundError(conversationId);
    }

    return conversationFromRow(row);
  }

  private insertConversation(conversation: Conversation): void {
    const row = conversationToRow(conversation);
    this.db
      .prepare(
        `
INSERT INTO conversations (
  id, task_id, task_run_id, provider, external_thread_id, title, summary, created_at, updated_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
`,
      )
      .run(
        row.id,
        row.task_id,
        row.task_run_id,
        row.provider,
        row.external_thread_id,
        row.title,
        row.summary,
        row.created_at,
        row.updated_at,
      );
  }

  private updateConversationRow(conversation: Conversation): void {
    const row = conversationToRow(conversation);
    this.db
      .prepare(
        `
UPDATE conversations SET
  task_id = ?,
  task_run_id = ?,
  external_thread_id = ?,
  title = ?,
  summary = ?,
  updated_at = ?
WHERE id = ?
`,
      )
      .run(
        row.task_id,
        row.task_run_id,
        row.external_thread_id,
        row.title,
        row.summary,
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
