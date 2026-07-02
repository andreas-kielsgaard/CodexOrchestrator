# Worker 012 Open Tasks SQLite Read Store

Date: 2026-07-02

## Summary

Added the Open Tasks dashboard read-side store boundary and a pure TypeScript SQLite reader. The
slice does not add task write APIs, runtime database wiring, Tauri/Rust commands, Codex runtime
integration, Git command execution, or React/UI work.

## Behavior

- Added `OpenTaskDashboardStore` in the domain layer plus a facade that returns
  `projectOpenTaskDashboard` groups from any store.
- Added an in-memory Open Tasks dashboard store for focused domain tests.
- Added `SqliteOpenTaskDashboardStore` using an injected `prepare`/statement interface compatible
  with `node:sqlite`.
- The SQLite reader loads task rows, ordered conversation links, and only the referenced
  projects/repos/branches/worktrees required by the dashboard projection.
- Archived and abandoned tasks are deliberately loaded from SQLite and omitted by
  `projectOpenTaskDashboard` so closed-task rules remain centralized in the domain projection.
- Optional technical anchors can be absent or `NULL`; the dashboard still renders the task with
  available project data.
- Stored `task_conversation_links.position` order is preserved in loaded `Task.conversationIds`.

## Changed Files

- `src/domain/openTaskDashboardStore.ts`: read store boundary, projection facade, and in-memory
  helper.
- `src/domain/openTaskDashboardStore.test.ts`: facade coverage using the in-memory helper.
- `src/infrastructure/sqlite/openTaskDashboardStore.ts`: SQLite read implementation.
- `src/infrastructure/sqlite/openTaskDashboardStore.test.ts`: in-memory `node:sqlite` read-store
  coverage.
- `docs/architecture.md`: documented the Open Tasks dashboard read-store boundary.
- `docs/task-logs/worker-012-open-tasks-sqlite-read-store.md`: recorded this worker result.

## Verification

- `npm run test -- src/domain/openTaskDashboardStore.test.ts src/infrastructure/sqlite/openTaskDashboardStore.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None.

## Review Notes

- Review the explicit read-store choice to load archived/abandoned rows and keep closed-task
  filtering in `dashboardProjection.ts`.
- Review the SQL scoping choice: the reader loads only parent anchor rows referenced by loaded
  tasks, not a full domain snapshot.
- The tests use `node:sqlite`, which emits Node's experimental feature warning during test runs.
