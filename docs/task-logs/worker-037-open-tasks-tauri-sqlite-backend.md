# Worker 037: Open Tasks Tauri SQLite Backend

Date: 2026-07-02

Branch: `worker/037-open-tasks-tauri-sqlite-backend`

Base verified: `aa4d68799f0679776e0ebe75671ecb7f468e216d` (`Log Worker 035 merge`)

## Goal

Implement durable Open Tasks persistence for the default Tauri WebView path by replacing the
backend-pending Rust command stubs for:

- `load_open_task_dashboard`
- `create_open_task`
- `update_open_task`
- `archive_open_task`

## Summary

- Added a narrow Rust SQLite backend in `src-tauri/src/lib.rs`.
- The backend chooses `codex-orchestrator.sqlite` under the Tauri app data directory.
- Added Rust-side schema migration application for the same ordered app schema used by the
  TypeScript SQLite infrastructure.
- Added Rust-side task create/update/archive handlers that preserve the existing command payloads
  from `src/infrastructure/tauriCommands.ts`.
- Added Rust-side dashboard snapshot projection returning the existing `TaskDashboardSnapshot`
  shape.
- Added focused Rust unit tests for empty load, create/update/archive, technical-anchor projection,
  closed-task omission, and missing-task errors.
- Updated `docs/architecture.md` to describe the new Tauri/Rust SQLite boundary.

## Design Notes

- This slice intentionally does not import Node-only TypeScript SQLite modules into browser or Rust
  code.
- The Rust backend duplicates only the small projection required to return `TaskDashboardSnapshot`
  from the command boundary.
- Archived and abandoned tasks are omitted from dashboard groups.
- `archive_open_task` persists by setting `execution_state = 'archived'`.
- Created tasks default to `execution_state = 'draft'`, `attention_state = 'needs_action_now'`,
  and `priority = 'normal'`, matching the application dashboard client defaults.
- The Rust snapshot returns all persisted projects, not only projects referenced by existing tasks,
  so a real persisted project can be selected before its first task is created. No seed/demo project
  fallback was added.

## Verification

Rust/Cargo are not available on `PATH` in this environment:

- `cargo --version`: unavailable
- `rustc --version`: unavailable

Verification results:

- `git diff --check main...worker/037-open-tasks-tauri-sqlite-backend`: passed
- Focused Rust tests: not run because Cargo/Rust are unavailable
- `npm run lint`: passed
- `npm run format:check`: passed
- `npm run test`: passed, 42 files / 258 tests
- `npm run build`: passed
- `npm run build:tauri`: failed before build because `cargo metadata` could not be launched
  (`program not found`)

Final command results are recorded in the completion report.

## Deferred

- React UI redesign or new screens.
- Codex execution and run controls.
- Repo/worktree UI or Git runtime wiring.
- Diff/validation runtime triggers.
- Workflow-engine behavior and cleanup policy.
- Seed/demo fallback for the default Tauri path.
