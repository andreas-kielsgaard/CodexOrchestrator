import type { Artifact, ArtifactKind, ValidationRun, ValidationStatus } from '../../domain/model';
import type { SqliteMigration, SqliteMigrationDatabase } from './repoSyncSchema';

const artifactKinds = [
  'final_response',
  'diff',
  'validation_log',
  'note',
  'screenshot',
  'handoff',
  'summary',
  'raw_event_stream',
] as const satisfies readonly ArtifactKind[];

const validationStatuses = [
  'queued',
  'running',
  'passed',
  'failed',
  'canceled',
] as const satisfies readonly ValidationStatus[];

export const artifactValidationSqliteMigrations: SqliteMigration[] = [
  {
    id: '004_artifacts_validation_runs_schema',
    sql: `
CREATE TABLE IF NOT EXISTS artifacts (
  id TEXT PRIMARY KEY,
  task_id TEXT,
  task_run_id TEXT,
  conversation_id TEXT,
  kind TEXT NOT NULL CHECK (kind IN (${sqlStringList(artifactKinds)})),
  title TEXT NOT NULL,
  uri TEXT,
  content TEXT,
  created_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
  FOREIGN KEY (task_run_id) REFERENCES task_runs(id) ON DELETE SET NULL,
  FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL
);

CREATE TABLE IF NOT EXISTS validation_runs (
  id TEXT PRIMARY KEY,
  task_id TEXT,
  task_run_id TEXT,
  command TEXT NOT NULL,
  status TEXT NOT NULL CHECK (status IN (${sqlStringList(validationStatuses)})),
  started_at TEXT,
  completed_at TEXT,
  exit_code INTEGER,
  output_artifact_id TEXT,
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
  FOREIGN KEY (task_run_id) REFERENCES task_runs(id) ON DELETE SET NULL,
  FOREIGN KEY (output_artifact_id) REFERENCES artifacts(id) ON DELETE SET NULL
);
`,
  },
];

export function artifactValidationSqliteSchemaSql(): string {
  return artifactValidationSqliteMigrations.map((migration) => migration.sql).join('\n');
}

export function applyArtifactValidationSqliteMigrations(db: SqliteMigrationDatabase): void {
  for (const migration of artifactValidationSqliteMigrations) {
    db.exec(migration.sql);
  }
}

export interface ArtifactRow {
  id: string;
  task_id: string | null;
  task_run_id: string | null;
  conversation_id: string | null;
  kind: ArtifactKind;
  title: string;
  uri: string | null;
  content: string | null;
  created_at: string;
}

export interface ValidationRunRow {
  id: string;
  task_id: string | null;
  task_run_id: string | null;
  command: string;
  status: ValidationStatus;
  started_at: string | null;
  completed_at: string | null;
  exit_code: number | null;
  output_artifact_id: string | null;
  created_at: string;
  updated_at: string;
}

export function artifactToRow(artifact: Artifact): ArtifactRow {
  return {
    id: artifact.id,
    task_id: artifact.taskId ?? null,
    task_run_id: artifact.taskRunId ?? null,
    conversation_id: artifact.conversationId ?? null,
    kind: artifact.kind,
    title: artifact.title,
    uri: artifact.uri ?? null,
    content: artifact.content ?? null,
    created_at: artifact.createdAt,
  };
}

export function artifactFromRow(row: ArtifactRow): Artifact {
  return {
    id: row.id,
    ...(row.task_id === null ? {} : { taskId: row.task_id }),
    ...(row.task_run_id === null ? {} : { taskRunId: row.task_run_id }),
    ...(row.conversation_id === null ? {} : { conversationId: row.conversation_id }),
    kind: row.kind,
    title: row.title,
    ...(row.uri === null ? {} : { uri: row.uri }),
    ...(row.content === null ? {} : { content: row.content }),
    createdAt: row.created_at,
  };
}

export function validationRunToRow(validationRun: ValidationRun): ValidationRunRow {
  return {
    id: validationRun.id,
    task_id: validationRun.taskId ?? null,
    task_run_id: validationRun.taskRunId ?? null,
    command: validationRun.command,
    status: validationRun.status,
    started_at: validationRun.startedAt ?? null,
    completed_at: validationRun.completedAt ?? null,
    exit_code: validationRun.exitCode ?? null,
    output_artifact_id: validationRun.outputArtifactId ?? null,
    created_at: validationRun.createdAt,
    updated_at: validationRun.updatedAt,
  };
}

export function validationRunFromRow(row: ValidationRunRow): ValidationRun {
  return {
    id: row.id,
    ...(row.task_id === null ? {} : { taskId: row.task_id }),
    ...(row.task_run_id === null ? {} : { taskRunId: row.task_run_id }),
    command: row.command,
    status: row.status,
    ...(row.started_at === null ? {} : { startedAt: row.started_at }),
    ...(row.completed_at === null ? {} : { completedAt: row.completed_at }),
    ...(row.exit_code === null ? {} : { exitCode: row.exit_code }),
    ...(row.output_artifact_id === null ? {} : { outputArtifactId: row.output_artifact_id }),
    createdAt: row.created_at,
    updatedAt: row.updated_at,
  };
}

function sqlStringList(values: readonly string[]): string {
  return values.map((value) => `'${value}'`).join(', ');
}
