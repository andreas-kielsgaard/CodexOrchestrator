import type { Artifact } from '../../domain/model';
import {
  cloneArtifact,
  createArtifactRecord,
  queryStoredArtifacts,
  type ArtifactQuery,
  type ArtifactStore,
  type ArtifactStoreIdProvider,
  type ArtifactStoreTimeProvider,
  type CreateArtifactInput,
} from '../../domain/artifactStore';
import { artifactFromRow, artifactToRow, type ArtifactRow } from './artifactValidationSchema';

export interface ArtifactSqliteStatement {
  all(...params: unknown[]): unknown[];
  get(...params: unknown[]): unknown | undefined;
  run(...params: unknown[]): unknown;
}

export interface ArtifactSqliteDatabase {
  prepare(sql: string): ArtifactSqliteStatement;
  exec?(sql: string): unknown;
}

export class SqliteArtifactStore implements ArtifactStore {
  constructor(
    private readonly db: ArtifactSqliteDatabase,
    private readonly ids: ArtifactStoreIdProvider,
    private readonly clock: ArtifactStoreTimeProvider,
  ) {}

  async createArtifact(input: CreateArtifactInput): Promise<Artifact> {
    return this.runInTransaction(() => {
      const artifact = createArtifactRecord(input, this.ids.nextId(), this.clock.now());

      this.insertArtifact(artifact);

      return artifactFromRow(artifactToRow(artifact));
    });
  }

  async queryArtifacts(query: ArtifactQuery = {}): Promise<Artifact[]> {
    const rows = this.db
      .prepare('SELECT * FROM artifacts ORDER BY created_at, id')
      .all() as ArtifactRow[];

    return queryStoredArtifacts(rows.map(artifactFromRow), query).map(cloneArtifact);
  }

  private insertArtifact(artifact: Artifact): void {
    const row = artifactToRow(artifact);
    this.db
      .prepare(
        `
INSERT INTO artifacts (
  id, task_id, task_run_id, conversation_id, kind, title, uri, content, created_at
) VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)
`,
      )
      .run(
        row.id,
        row.task_id,
        row.task_run_id,
        row.conversation_id,
        row.kind,
        row.title,
        row.uri,
        row.content,
        row.created_at,
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
