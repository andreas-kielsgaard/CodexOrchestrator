# Worker 011 Open Tasks SQLite Schema Foundation

Date: 2026-07-02

## Summary

Added a pure TypeScript SQLite schema foundation for the Open Tasks dashboard persistence subset.
This slice does not implement SQLite CRUD/store APIs, app runtime database wiring, Tauri/Rust
commands, Codex runtime integration, or UI work.

## Behavior

- Added ordered migration SQL for `tasks` and `task_conversation_links`.
- Reused the existing repo-sync `projects`, `repos`, `branches`, and `worktrees` tables as
  foreign-key parents.
- `tasks.project_id` references `projects(id)` and cascades on project deletion.
- Optional `tasks.repo_id`, `tasks.branch_id`, and `tasks.worktree_id` references use
  `ON DELETE SET NULL` so task intent survives technical cleanup.
- Execution state, attention state, and priority are constrained with SQLite `CHECK` clauses that
  match the TypeScript domain model.
- `task_conversation_links.task_id` cascades when a task is deleted.
- `task_conversation_links.conversation_id` is stored as text without a foreign key until a future
  conversation schema defines the parent table.
- `Task.conversationIds` persist through ordered link rows with deterministic `position` values.
- Row types and mapper helpers preserve optional task fields as SQL `NULL` and map rows back to
  domain `Task` records.

## Changed Files

- `src/infrastructure/sqlite/taskSchema.ts`: task schema migration SQL, row types, and mapper
  helpers.
- `src/infrastructure/sqlite/taskSchema.test.ts`: in-memory `node:sqlite` schema, constraint,
  cascade, link round-trip, and dashboard-projection coverage.
- `docs/architecture.md`: documented the Open Tasks SQLite schema boundary.
- `docs/task-logs/worker-011-open-tasks-sqlite-schema.md`: recorded this worker result.

## Verification

- `npm run test -- src/infrastructure/sqlite/taskSchema.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass after `npm run format` fixed architecture markdown wrapping
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None.

## Review Notes

- Conversation integrity is intentionally deferred because this slice does not introduce a
  conversation table.
- `task_conversation_links` has no synthetic `id`; it uses `(task_id, conversation_id)` plus a
  unique `(task_id, position)` constraint to preserve deterministic ordering.
- The tests use `node:sqlite`, which emits Node's experimental feature warning during test runs.
