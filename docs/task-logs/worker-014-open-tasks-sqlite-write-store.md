# Worker 014 Open Tasks SQLite Write Store Adapter

Date: 2026-07-02

## Summary

Added a pure TypeScript SQLite-backed adapter for the Open Tasks write boundary. This slice does not
add app runtime database wiring, Tauri/Rust commands, Codex runtime integration, Git command
execution, React/UI work, or broader Phase 1 stores.

## Behavior

- Added `SqliteOpenTaskWriteStore` in `src/infrastructure/sqlite/openTaskWriteStore.ts`.
- The adapter uses an injected SQLite-like `prepare` interface plus optional `exec` transaction
  support compatible with `node:sqlite` tests.
- `createTask` uses injected deterministic `IdProvider` and `TimeProvider`, applies the domain
  defaults, inserts a `tasks` row, and persists ordered `task_conversation_links`.
- `updateTask` loads the existing task, reuses the domain update helper, and preserves Worker 013
  semantics: omitted fields remain unchanged, `null` clears optional repo/branch/worktree anchors
  and due/snooze timestamps, and `conversationIds` replaces the full ordered link list when present.
- `archiveTask` sets `executionState` to `archived` and leaves closed-task omission centralized in
  the dashboard projection.
- Missing update/archive targets throw `OpenTaskNotFoundError`.
- When `exec` is available, writes run inside `BEGIN`/`COMMIT` and roll back on failures.

## Changed Files

- `src/domain/openTaskWriteStore.ts`: exported the focused domain update helper for adapter reuse.
- `src/infrastructure/sqlite/taskSchema.ts`: allowed conversation-link row creation to receive a
  deterministic link timestamp.
- `src/infrastructure/sqlite/openTaskWriteStore.ts`: SQLite write-store adapter implementation.
- `src/infrastructure/sqlite/openTaskWriteStore.test.ts`: in-memory `node:sqlite` coverage for
  create, updates, SQL `NULL` clears, ordered conversation replacement, archive/read-store
  interoperability, missing-task errors, unrelated-task preservation, and transaction rollback.
- `docs/architecture.md`: documented the SQLite write adapter boundary.
- `docs/task-logs/worker-014-open-tasks-sqlite-write-store.md`: recorded this worker result.

## Verification

- `npm run test -- src/infrastructure/sqlite/openTaskWriteStore.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None.

## Review Notes

- Review the small exported domain helper `applyTaskUpdate`; it keeps SQLite mutation behavior
  aligned with the in-memory Worker 013 contract.
- The tests use `node:sqlite`, which emits Node's experimental feature warning during test runs.
