# Worker 013 Open Tasks Write Boundary

Date: 2026-07-02

## Summary

Added a pure TypeScript Open Tasks write boundary with an in-memory implementation for tests and
future UI/store adapter work. This slice does not add a SQLite write adapter, schema changes,
runtime database wiring, Tauri/Rust commands, Codex runtime integration, Git command execution, or
React/UI work.

## Behavior

- Added `OpenTaskWriteStore` for task create, update, and archive-style close operations.
- Added deterministic `IdProvider` and `TimeProvider` injection so create/update/archive behavior
  can be tested without wall-clock or random ID dependencies.
- `createTask` keeps `Task` as the user-facing unit of attention and accepts optional technical
  anchors for repo, branch, worktree, ordered conversations, due date, and snooze timestamp.
- `updateTask` treats omitted fields as unchanged and `null` as an explicit clear for optional
  repo/branch/worktree anchors plus due/snooze timestamps.
- `conversationIds` updates replace the full ordered list, preserving caller-provided order.
- `archiveTask` sets `executionState` to `archived` and leaves dashboard closed-task omission to
  `projectOpenTaskDashboard`.
- Missing task mutations throw `OpenTaskNotFoundError`.
- The in-memory implementation clones records on input/output and mutates only the task collection,
  preserving unrelated domain records.

## Changed Files

- `src/domain/openTaskWriteStore.ts`: write boundary, deterministic providers, typed missing-task
  error, and in-memory implementation.
- `src/domain/openTaskWriteStore.test.ts`: focused create/update/archive tests, explicit clear
  semantics, conversation order replacement, missing-task handling, dashboard omission through the
  projection, and unrelated record preservation.
- `docs/architecture.md`: documented the Open Tasks write-store boundary.
- `docs/task-logs/worker-013-open-tasks-write-boundary.md`: recorded this worker result.

## Verification

- `npm run test -- src/domain/openTaskWriteStore.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None.

## Review Notes

- Review whether `archiveTask` should eventually set `attentionState` too. This slice only marks
  the task archived through `executionState` so closed-task rules stay centralized in the existing
  projection.
- Review the update contract choice that `undefined` means unchanged and `null` means clear for
  optional fields. This is intended to map cleanly to future SQLite write adapters and UI controls.
