# Active Task Map

Updated: 2026-07-03

Purpose: fast recovery and orchestration continuity. This file tracks only work that still needs
attention: blockers, active workers, complete-but-unreviewed branches, pending corrections, and
cleanup that affects current work.

Update this file as the last step before ending an orchestration operation. Do not add a task here
just because it was launched if the same operation will immediately complete, review, merge, or
otherwise resolve it.

## Active Tasks

- Worker 045: MSVC Build Tools / Linker Setup is active as projectless background thread
  `019f28ba-be64-7b11-92c1-b7f4bf3d564a`. Await report on whether `link.exe` can be installed or
  enabled, and whether `cargo test` / `npm run build:tauri` can progress.
- Worker 046: FS-09 Task/Run Detail UI Shell is active in
  `C:\Users\user\.codex\worktrees\071c\Codex Orchestrator` on target branch
  `worker/046-task-run-detail-ui-shell`. Pending worktree id:
  `local:40c6b702-0071-447a-812b-181d32c3adde`. Await completion report and result log
  `docs/task-logs/worker-046-task-run-detail-ui-shell.md`; review before merge.

## Pending Blockers / Follow-Up

- Rust/Cargo are installed under `C:\Users\user\.cargo\bin`, and `cargo metadata` succeeds when
  that path is prepended in the current Codex shell. Full Rust/Tauri compilation remains blocked
  because the MSVC linker `link.exe` is unavailable; Worker 045 is investigating setup.
- The merged FS-08 run controls call the browser-safe `start_codex_task_run` facade, but live
  WebView execution still needs a Rust/Tauri backend command registration.
- The merged task/run detail read model has no UI yet; Worker 046 is building the UI shell and
  browser-safe detail facade.

## Recently Archived

- Worker 032: reviewed, corrected, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-032-persisted-open-tasks-dashboard.md`.
- Worker 033: reviewed, corrected, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-033-diff-collector-service.md`.
- Worker 034: reviewed, corrected, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-034-validation-command-runner.md`.
- Worker 035: reviewed, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-035-local-git-runtime-adapters.md`.
- Worker 036: reviewed, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-036-validation-command-runtime-adapter.md`.
- Worker 037: reviewed, merged, verified within environment limits, and Git-cleaned. See
  `docs/task-logs/worker-037-open-tasks-tauri-sqlite-backend.md`.
- Worker 038: finished without reporting back, then was reviewed, corrected, merged, verified, and
  Git-cleaned. See `docs/task-logs/worker-038-local-runtime-service-composition.md`.
- Worker 039: reviewed, merged, verified, stabilized, and Git-cleaned. See
  `docs/task-logs/worker-039-runtime-command-contract.md`.
- Worker 040: reviewed, merged, verified within environment limits, and Git-cleaned. See
  `docs/task-logs/worker-040-rust-toolchain-setup.md`.
- Worker 041: reviewed, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-041-orchestration-review.md`.
- Worker 042: reviewed, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-042-run-controls-ui-shell.md`.
- Worker 043: errored before completion; stale branch had no useful work and was deleted.
- Worker 044: reviewed, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-044-task-run-detail-read-model.md`.

## Blockers

- None beyond the pending blockers above.

## Cleanup Notes

Leave Windows-locked physical leftover worktree folders in place unless an explicit cleanup task is
launched:

- Worker 019: `C:\Users\user\.codex\worktrees\282a\Codex Orchestrator`
- Worker 021: `C:\Users\user\.codex\worktrees\fae3\Codex Orchestrator`
- Worker 022: `C:\Users\user\.codex\worktrees\cea2\Codex Orchestrator`
- Worker 023: `C:\Users\user\.codex\worktrees\14b0\Codex Orchestrator`
- Worker 024: `C:\Users\user\.codex\worktrees\ab51\Codex Orchestrator`
- Worker 025: `C:\Users\user\.codex\worktrees\dc3c\Codex Orchestrator`
- Worker 026: `C:\Users\user\.codex\worktrees\9747\Codex Orchestrator`
- Worker 027: `C:\Users\user\.codex\worktrees\04d6\Codex Orchestrator`
- Worker 028: `C:\Users\user\.codex\worktrees\c12f\Codex Orchestrator`
- Worker 029: `C:\Users\user\.codex\worktrees\a518\Codex Orchestrator`
- Worker 030: `C:\Users\user\.codex\worktrees\0d6b\Codex Orchestrator`
- Worker 031: `C:\Users\user\.codex\worktrees\93b2\Codex Orchestrator`
- Worker 032: `C:\Users\user\.codex\worktrees\9117\Codex Orchestrator`
- Worker 033: `C:\Users\user\.codex\worktrees\6610\Codex Orchestrator`
- Worker 034: `C:\Users\user\.codex\worktrees\d19f\Codex Orchestrator`
- Worker 035: `C:\Users\user\.codex\worktrees\46d5\Codex Orchestrator`
- Worker 036: `C:\Users\user\.codex\worktrees\478e\Codex Orchestrator`
- Worker 037: `C:\Users\user\.codex\worktrees\35fe\Codex Orchestrator`
- Worker 038: `C:\Users\user\.codex\worktrees\c063\Codex Orchestrator`
- Worker 039: `C:\Users\user\.codex\worktrees\c6f0\Codex Orchestrator`
- Worker 040: `C:\Users\user\.codex\worktrees\83c3\Codex Orchestrator`
- Worker 041: `C:\Users\user\.codex\worktrees\b18b\Codex Orchestrator`
- Worker 042: `C:\Users\user\.codex\worktrees\9ca8\Codex Orchestrator`
- Worker 044: `C:\Users\user\.codex\worktrees\514e\Codex Orchestrator`
