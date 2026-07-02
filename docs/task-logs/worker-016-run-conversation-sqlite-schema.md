# Worker 016 TaskRun and Conversation SQLite Schema Foundation

Date: 2026-07-02

## Summary

Added a pure TypeScript SQLite schema foundation for TaskRun and Conversation provenance records.
This slice does not add CRUD stores, runtime database file opening, Tauri/Rust commands, Codex
runtime integration, Git execution, React/UI work, package dependencies, or artifact/validation/event
schemas.

## Behavior

- Added ordered migration SQL for `task_runs` and `conversations`.
- Composed the migration into the app-level migration coordinator after the existing Open Tasks
  schema migration.
- `task_runs.task_id` references `tasks(id)` with `ON DELETE CASCADE` so task-owned run records are
  cleaned up with their task.
- Optional `task_runs.worktree_id`, `task_runs.conversation_id`, `conversations.task_id`, and
  `conversations.task_run_id` links use `ON DELETE SET NULL` so provenance rows survive related
  cleanup where the link is optional.
- Preserved practical insertion for the optional TaskRun/Conversation relationship by allowing both
  foreign-key columns to be nullable: insert a task run without a conversation, insert the
  conversation linked to that run, then update the task run with the conversation ID.
- Added row types and mapper helpers for `TaskRun` and `Conversation`, preserving optional fields as
  SQL `NULL`.
- Constrained `TaskRun.executionState` and `Conversation.provider` as checked text values matching
  the TypeScript domain unions.

## Changed Files

- `src/infrastructure/sqlite/runConversationSchema.ts`: TaskRun/Conversation migration SQL, row
  types, and mapper helpers.
- `src/infrastructure/sqlite/runConversationSchema.test.ts`: in-memory `node:sqlite` coverage for
  coordinator table creation, constraints, foreign-key behavior, nullable round-trips, and practical
  optional link insertion.
- `src/infrastructure/sqlite/migrationCoordinator.ts`: added the TaskRun/Conversation migration
  family after Open Tasks.
- `src/infrastructure/sqlite/migrationCoordinator.test.ts`: updated app table/idempotency
  expectations for the new tables.
- `docs/architecture.md`: documented the TaskRun/Conversation SQLite schema boundary.
- `docs/task-logs/worker-016-run-conversation-sqlite-schema.md`: recorded this worker result.

## Verification

- `npm run test -- src/infrastructure/sqlite/runConversationSchema.test.ts src/infrastructure/sqlite/migrationCoordinator.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None.

## Review Notes

- Review the bidirectional optional foreign-key decision in
  `src/infrastructure/sqlite/runConversationSchema.ts`; tests cover the intended insert/update flow.
- The tests use `node:sqlite`, which emits Node's experimental feature warning during test runs.

## Orchestrator Review Addendum

The orchestrator made two small review corrections before merge:

- Clarified `docs/architecture.md` so `task_conversation_links.conversation_id` is described as an
  intentionally text-only dashboard link even after the new `conversations` table exists, pending a
  future link-integrity/backfill migration.
- Added a cleanup assertion proving that deleting a task cascades its task runs and also clears both
  `conversations.task_id` and `conversations.task_run_id`.
