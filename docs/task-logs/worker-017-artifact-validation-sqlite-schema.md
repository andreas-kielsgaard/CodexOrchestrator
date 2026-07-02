# Worker 017 Artifact and ValidationRun SQLite Schema Foundation

Date: 2026-07-02

## Summary

Added a pure TypeScript SQLite schema foundation for Artifact and ValidationRun records. This slice
does not add CRUD stores, runtime database file opening, Tauri/Rust commands, Codex runtime
integration, Git execution, React/UI work, package dependencies, or event persistence.

## Behavior

- Added ordered migration SQL for `artifacts` and `validation_runs`.
- Composed the migration into the app-level migration coordinator after the existing
  TaskRun/Conversation migration family.
- Artifact optional `task_id`, `task_run_id`, and `conversation_id` links use `ON DELETE SET NULL`
  so durable outputs survive cleanup of related workflow/provenance rows.
- ValidationRun optional `task_id`, `task_run_id`, and `output_artifact_id` links use
  `ON DELETE SET NULL` so validation history survives cleanup and output artifact deletion clears
  only the optional reference.
- Preserved practical insertion for validation output artifacts by allowing
  `validation_runs.output_artifact_id` to be nullable: callers can insert a validation run first,
  insert an artifact later, then update the validation row with the artifact ID.
- Added row types and mapper helpers for `Artifact` and `ValidationRun`, preserving optional fields
  as SQL `NULL`.
- Constrained `Artifact.kind` and `ValidationRun.status` as checked text values matching the
  TypeScript domain unions.

## Changed Files

- `src/infrastructure/sqlite/artifactValidationSchema.ts`: Artifact/ValidationRun migration SQL,
  row types, and mapper helpers.
- `src/infrastructure/sqlite/artifactValidationSchema.test.ts`: in-memory `node:sqlite` coverage
  for coordinator table creation, migration separation, check constraints, foreign-key cleanup,
  nullable round-trips, and output-artifact insert/update flow.
- `src/infrastructure/sqlite/migrationCoordinator.ts`: added the Artifact/ValidationRun migration
  family after TaskRun/Conversation.
- `src/infrastructure/sqlite/migrationCoordinator.test.ts`: updated app table/idempotency
  expectations for the new tables.
- `src/infrastructure/sqlite/runConversationSchema.test.ts`: updated app-coordinator table
  expectations for the new downstream tables.
- `docs/architecture.md`: documented the Artifact/ValidationRun SQLite schema boundary.
- `docs/task-logs/worker-017-artifact-validation-sqlite-schema.md`: recorded this worker result.

## Verification

- `npm run test -- src/infrastructure/sqlite/artifactValidationSchema.test.ts src/infrastructure/sqlite/migrationCoordinator.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None.

## Review Notes

- Review the all-optional `ON DELETE SET NULL` choice for Artifact/ValidationRun links in
  `src/infrastructure/sqlite/artifactValidationSchema.ts`; it preserves local-first provenance
  records when related task/run/conversation/output rows are cleaned up.
- The tests use `node:sqlite`, which emits Node's experimental feature warning during test runs.
