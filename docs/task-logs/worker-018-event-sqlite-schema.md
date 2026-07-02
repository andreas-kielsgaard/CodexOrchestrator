# Worker 018 Event SQLite Schema Foundation

Date: 2026-07-02

## Summary

Added a pure TypeScript SQLite schema foundation for durable domain Event records. This slice does
not add an event append/query store, runtime database file opening, Tauri/Rust commands, Codex
runtime integration, Git execution, React/UI work, or package dependencies.

## Behavior

- Added ordered migration SQL for `events`.
- Composed the migration into the app-level migration coordinator after the existing
  Artifact/ValidationRun migration family.
- Optional `project_id`, `task_id`, `task_run_id`, `conversation_id`, `artifact_id`, and
  `validation_run_id` links use `ON DELETE SET NULL` so event records survive cleanup of related
  workflow, provenance, output, and validation rows.
- Added row types and mapper helpers for `Event`, preserving optional fields as SQL `NULL`.
- Constrained `Event.kind` as checked text matching the TypeScript domain union.
- Serialized payloads as deterministic JSON text with sorted object keys, and deserialized invalid
  JSON or non-object payload rows with clear errors that include the event id.

## Changed Files

- `src/infrastructure/sqlite/eventSchema.ts`: Event migration SQL, row type, and mapper helpers.
- `src/infrastructure/sqlite/eventSchema.test.ts`: in-memory `node:sqlite` coverage for
  coordinator table creation, migration separation, check constraints, foreign-key cleanup,
  nullable round-trips, deterministic payload JSON, and invalid JSON handling.
- `src/infrastructure/sqlite/migrationCoordinator.ts`: added Event migrations after
  Artifact/ValidationRun.
- `src/infrastructure/sqlite/migrationCoordinator.test.ts`: updated app table/idempotency
  expectations for `events`.
- `src/infrastructure/sqlite/artifactValidationSchema.test.ts` and
  `src/infrastructure/sqlite/runConversationSchema.test.ts`: updated downstream app-coordinator
  table expectations for `events`.
- `docs/architecture.md`: documented the Event SQLite schema boundary and updated coordinator order.
- `docs/task-logs/worker-018-event-sqlite-schema.md`: recorded this worker result.

## Verification

- `npm run test -- src/infrastructure/sqlite/eventSchema.test.ts src/infrastructure/sqlite/migrationCoordinator.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None.

## Review Notes

- Review the deterministic payload serialization choice in
  `src/infrastructure/sqlite/eventSchema.ts`: object keys are sorted recursively, array order is
  preserved, and invalid persisted payload JSON throws instead of being swallowed.
- Review the all-optional `ON DELETE SET NULL` foreign-key behavior for events. It follows the
  local-first audit requirement that events remain durable even when related rows are cleaned up.
- The tests use `node:sqlite`, which emits Node's experimental feature warning during test runs.
