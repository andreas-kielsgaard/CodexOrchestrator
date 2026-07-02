# Worker 032: Persisted Open Tasks Dashboard Boundary

Date: 2026-07-02

## Summary

- Added `TaskDashboardClient`, an application/client boundary over `OpenTaskDashboardStore` and
  `OpenTaskWriteStore` for load/create/update/archive.
- Refactored the React Open Tasks dashboard to consume an injected async client instead of importing
  seed dashboard data.
- Added dense dashboard controls for task creation, inline edit, attention/execution state changes,
  archive, refresh, loading, and backend error display.
- Added a browser-safe Tauri command client and Rust command stubs for the Open Tasks contract.
- Verified the application boundary with in-memory stores and a reopened local SQLite app database.

## Files Changed

- `src/application/taskDashboardClient.ts`
- `src/application/taskDashboardClient.test.ts`
- `src/app/App.tsx`
- `src/app/App.test.tsx`
- `src/infrastructure/tauriCommands.ts`
- `src/main.tsx`
- `src/styles.css`
- `src-tauri/src/lib.rs`
- `docs/architecture.md`
- `docs/first-slice-completion-plan.md`
- `docs/task-logs/worker-032-persisted-open-tasks-dashboard.md`

## Verification

- `npm run test -- src/application/taskDashboardClient.test.ts src/app/App.test.tsx` - passed
  (4 tests).
- `npm run lint` - passed.
- `npm run format:check` - passed.
- `npm run test` - passed (38 files, 227 tests).
- `npm run build` - passed.
- `git diff --check main...worker/032-persisted-open-tasks-dashboard` - passed.
- `npm run build:tauri` - not run; `cargo` is not available on `PATH`.

## Blockers

- The default desktop UI now has a narrow Tauri command contract, but durable WebView persistence
  still requires a Rust-side SQLite backend adapter. The registered Rust commands currently return
  an explicit backend-pending error.
- `npm run build:tauri` remains dependent on Rust/Cargo availability.

## Review Notes

- Orchestrator review correction: `DashboardTask` now carries task priority so inline edit saves do
  not silently flatten high/low priority tasks to `normal`.
- React/browser modules do not import `src/infrastructure/sqlite/localAppDatabase.ts` or Node-only
  modules.
- The SQLite-backed verification lives in application tests, keeping Node SQLite isolated from the
  UI bundle.
- The UI does not use seed/demo fallback data; when the Tauri backend is unavailable it shows the
  command error.
