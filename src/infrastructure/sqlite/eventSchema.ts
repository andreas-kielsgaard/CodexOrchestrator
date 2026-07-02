import type { Event, EventKind } from '../../domain/model';
import type { SqliteMigration, SqliteMigrationDatabase } from './repoSyncSchema';

const eventKinds = [
  'task_created',
  'task_updated',
  'attention_changed',
  'execution_changed',
  'run_started',
  'run_event',
  'run_completed',
  'artifact_created',
  'validation_started',
  'validation_completed',
] as const satisfies readonly EventKind[];

export const eventSqliteMigrations: SqliteMigration[] = [
  {
    id: '005_events_schema',
    sql: `
CREATE TABLE IF NOT EXISTS events (
  id TEXT PRIMARY KEY,
  kind TEXT NOT NULL CHECK (kind IN (${sqlStringList(eventKinds)})),
  occurred_at TEXT NOT NULL,
  project_id TEXT,
  task_id TEXT,
  task_run_id TEXT,
  conversation_id TEXT,
  artifact_id TEXT,
  validation_run_id TEXT,
  payload_json TEXT NOT NULL,
  FOREIGN KEY (project_id) REFERENCES projects(id) ON DELETE SET NULL,
  FOREIGN KEY (task_id) REFERENCES tasks(id) ON DELETE SET NULL,
  FOREIGN KEY (task_run_id) REFERENCES task_runs(id) ON DELETE SET NULL,
  FOREIGN KEY (conversation_id) REFERENCES conversations(id) ON DELETE SET NULL,
  FOREIGN KEY (artifact_id) REFERENCES artifacts(id) ON DELETE SET NULL,
  FOREIGN KEY (validation_run_id) REFERENCES validation_runs(id) ON DELETE SET NULL
);
`,
  },
];

export function eventSqliteSchemaSql(): string {
  return eventSqliteMigrations.map((migration) => migration.sql).join('\n');
}

export function applyEventSqliteMigrations(db: SqliteMigrationDatabase): void {
  for (const migration of eventSqliteMigrations) {
    db.exec(migration.sql);
  }
}

export interface EventRow {
  id: string;
  kind: EventKind;
  occurred_at: string;
  project_id: string | null;
  task_id: string | null;
  task_run_id: string | null;
  conversation_id: string | null;
  artifact_id: string | null;
  validation_run_id: string | null;
  payload_json: string;
}

export function eventToRow(event: Event): EventRow {
  return {
    id: event.id,
    kind: event.kind,
    occurred_at: event.occurredAt,
    project_id: event.projectId ?? null,
    task_id: event.taskId ?? null,
    task_run_id: event.taskRunId ?? null,
    conversation_id: event.conversationId ?? null,
    artifact_id: event.artifactId ?? null,
    validation_run_id: event.validationRunId ?? null,
    payload_json: stableJsonStringify(event.payload, `Event ${event.id} payload`),
  };
}

export function eventFromRow(row: EventRow): Event {
  return {
    id: row.id,
    kind: row.kind,
    occurredAt: row.occurred_at,
    ...(row.project_id === null ? {} : { projectId: row.project_id }),
    ...(row.task_id === null ? {} : { taskId: row.task_id }),
    ...(row.task_run_id === null ? {} : { taskRunId: row.task_run_id }),
    ...(row.conversation_id === null ? {} : { conversationId: row.conversation_id }),
    ...(row.artifact_id === null ? {} : { artifactId: row.artifact_id }),
    ...(row.validation_run_id === null ? {} : { validationRunId: row.validation_run_id }),
    payload: parsePayloadJson(row.payload_json, row.id),
  };
}

function parsePayloadJson(payloadJson: string, eventId: string): Record<string, unknown> {
  let parsed: unknown;

  try {
    parsed = JSON.parse(payloadJson);
  } catch (error) {
    throw new Error(`Invalid JSON payload for event ${eventId}: ${(error as Error).message}`);
  }

  if (parsed === null || typeof parsed !== 'object' || Array.isArray(parsed)) {
    throw new Error(`Invalid JSON payload for event ${eventId}: expected a JSON object`);
  }

  return parsed as Record<string, unknown>;
}

function stableJsonStringify(payload: Record<string, unknown>, label: string): string {
  try {
    return JSON.stringify(stableJsonValue(payload));
  } catch (error) {
    throw new Error(`${label} is not JSON serializable: ${(error as Error).message}`);
  }
}

function stableJsonValue(value: unknown): unknown {
  if (
    value === null ||
    typeof value === 'string' ||
    typeof value === 'number' ||
    typeof value === 'boolean'
  ) {
    return value;
  }

  if (Array.isArray(value)) {
    return value.map((item) => (item === undefined ? null : stableJsonValue(item)));
  }

  if (typeof value === 'object') {
    return Object.fromEntries(
      Object.entries(value as Record<string, unknown>)
        .filter(
          ([, item]) =>
            item !== undefined && typeof item !== 'function' && typeof item !== 'symbol',
        )
        .sort(([left], [right]) => compareJsonKeys(left, right))
        .map(([key, item]) => [key, stableJsonValue(item)]),
    );
  }

  return value;
}

function compareJsonKeys(left: string, right: string): number {
  if (left < right) {
    return -1;
  }

  if (left > right) {
    return 1;
  }

  return 0;
}

function sqlStringList(values: readonly string[]): string {
  return values.map((value) => `'${value}'`).join(', ');
}
