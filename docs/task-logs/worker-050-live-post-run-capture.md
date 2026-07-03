# Worker 050 Live Post-Run Capture

Branch: `worker/050-live-post-run-capture`
Worktree: `C:\Users\user\.codex\worktrees\94b3\Codex Orchestrator`
Base: `57b46faa5d6d4e23eb839ce9367d1ae0a743ac0f` (`Log Worker 049 merge`)

## Summary

- Extended the browser-safe `start_codex_task_run` contract with explicit optional
  `postRunCapture` input for tracked diff capture and one array-argument validation command.
- Wired the Rust/Tauri live `start_codex_task_run` path to run post-run capture only after a
  completed Codex run, preserving omitted-option behavior and failed Codex run semantics.
- Added fakeable Rust Git diff and validation command runner boundaries. The production validation
  runner uses `std::process::Command` with argument arrays and no shell parsing.
- Persisted captured diffs as `diff` artifacts with `artifact_created` events.
- Persisted validation capture as one `validation_runs` row, one linked `validation_log` artifact,
  and `validation_started`, `artifact_created`, and `validation_completed` events.
- Kept visible React/UI controls deferred; existing UI still omits capture options.

## Changed Files

- `docs/architecture.md`
- `docs/first-slice-completion-plan.md`
- `docs/task-logs/worker-050-live-post-run-capture.md`
- `src/application/runtimeCommandClient.ts`
- `src/infrastructure/localRuntimeCommands.ts`
- `src/infrastructure/tauriCommands.test.ts`
- `src-tauri/src/lib.rs`

## Verification

- `cargo test post_run --lib` passed: 3 focused fake-runner post-run capture tests.
- `cargo fmt --check` passed via `C:\Users\user\.cargo\bin\cargo.exe fmt --check`.
- `cargo test` passed: 14 Rust tests.
- `npm run lint` passed.
- `npm run format:check` passed.
- `npm run test` passed: 47 files, 283 tests. Node emitted the existing experimental SQLite
  warning.
- `npm run build` passed.
- `npm run build:tauri` passed through the Visual Studio developer environment and produced MSI and
  NSIS bundles. Tauri emitted the existing bundle identifier warning about `.app`.

## Review Notes

- Capture is caller-configured only. No default validation command or visible UI control was added.
- Capture failures are returned in `postRunCapture` and do not convert a completed Codex run into a
  failed run.
- Validation launch failures still create a failed validation run and linked validation log artifact.
- Rust tests inject fake Codex, Git diff, and validation runners; no live Codex, Git, or validation
  command is required for capture tests.

## Blockers

None.
