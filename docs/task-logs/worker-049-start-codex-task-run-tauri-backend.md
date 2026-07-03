# Worker 049: Start Codex Task Run Tauri Backend

Date: 2026-07-03

## Branch And Base

- Branch: `worker/049-start-codex-task-run-tauri-backend`
- Worktree: `C:\Users\user\.codex\worktrees\2b63\Codex Orchestrator`
- Expected base: `607352c00a76797d19be4e9959761c3bd6bb6285`
- Actual starting base: `607352c00a76797d19be4e9959761c3bd6bb6285`

## Summary

- Registered the Rust/Tauri `start_codex_task_run` command behind the existing browser-safe facade.
- Added a Rust Codex command runner that executes `codex exec --json` with array-style arguments,
  caller-provided `cwd`, additional args, and environment overrides.
- Persisted the task-run lifecycle from the WebView path: running/failed/completed task state,
  Codex conversation records, task conversation links, raw JSONL artifacts, final response
  artifacts when available, and lifecycle/artifact events.
- Preserved raw stdout JSONL before parsing/summarizing the stream, including parse-failure paths.
- Added focused Rust tests with an injected fake runner; tests do not require a live Codex CLI.
- Updated architecture notes to reflect that the live Tauri run backend now exists while post-run
  diff/validation capture remains deferred.

## Changed Files

- `src-tauri/src/lib.rs`
- `src/infrastructure/tauriCommands.ts`
- `docs/architecture.md`
- `docs/task-logs/worker-049-start-codex-task-run-tauri-backend.md`

## Verification

- `git diff --check main...worker/049-start-codex-task-run-tauri-backend` - passed.
- Focused Rust tests: `cargo test start_codex_task_run` - passed, 4 tests.
- `cargo fmt --check` - passed.
- `cargo test` - passed, 11 tests.
- `npm run lint` - passed.
- `npm run format:check` - passed.
- `npm run test` - passed, 47 files / 283 tests.
- `npm run build` - passed.
- `npm run build:tauri` through the Visual Studio developer environment - passed; produced MSI and
  NSIS bundles. Existing warning observed: the configured bundle identifier ends with `.app`.

## Notes

- This slice intentionally does not wire post-run diff collection or validation capture.
- This slice intentionally does not inspect or manage Codex credentials; command failures are
  surfaced through the runtime command failure result.
- The Rust backend uses an injected command-runner boundary for tests and a `std::process::Command`
  implementation for live runs. Unit tests use only the fake runner.
- The Rust backend includes a small JSONL summarizer for the Tauri command path so React/WebView
  code still avoids Node-only imports.
- No live Codex CLI smoke test was run.
- Orchestrator review correction: `docs/active-task-map.md` was restored out of the worker diff
  because it is orchestrator-owned recovery state, and app metadata now reports
  `tauri-codex-exec` instead of `adapter-pending`.

## Blockers

- None.
