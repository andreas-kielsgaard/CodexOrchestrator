# Worker 042 Run Controls UI Shell

Date: 2026-07-03

Branch: `worker/042-run-controls-ui-shell`

## Summary

- Extended `App` to accept an injected `RuntimeCommandClient` alongside the existing
  `TaskDashboardClient`.
- Wired `src/main.tsx` to pass `tauriRuntimeCommandClient` in the default Tauri WebView path.
- Added compact per-task Codex run controls to the Open Tasks dashboard:
  - task-scoped prompt textarea
  - lucide play/start button
  - per-task in-flight disabled state
  - unavailable state when no `worktreePath` is projected
  - concise completed/failed feedback with task run id, final task/run status, exit code, and
    reason/error when provided
  - dashboard reload after each run attempt
- Added App tests with fake dashboard and runtime clients. The tests verify command payload wiring,
  `cwd` from `task.worktreePath`, dashboard reload after run, prompt clearing on completion, and
  missing-worktree gating.
- Updated `docs/architecture.md` to reflect the UI shell state while leaving Rust/Tauri runtime
  command registration as pending.

## Boundaries Kept

- Did not edit `src-tauri/`.
- Did not implement or register `start_codex_task_run` in Rust/Tauri.
- Did not add task detail pages, review surfaces, repo/worktree selection UI, workflow behavior,
  diff/validation triggers, or process supervision.
- Did not run live Codex, Git, or validation commands from tests.

## Verification

- `npm test -- src/app/App.test.tsx` - passed.
- `npm run lint` - passed.
- `npm run format:check` - passed.
- `npm run test` - passed, 45 test files and 267 tests.
- `npm run build` - passed.
- `git diff --check main...worker/042-run-controls-ui-shell` - passed.

Notes:

- Full test output includes Node's existing experimental SQLite warnings.
- `npm run build:tauri` was not run; it is not required for this UI slice.
