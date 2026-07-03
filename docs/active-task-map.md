# Active Task Map

Updated: 2026-07-03

Purpose: fast recovery and orchestration continuity. This file tracks only work that still needs
attention: blockers, active workers, complete-but-unreviewed branches, pending corrections, and
cleanup that affects current work.

Update this file as the last step before ending an orchestration operation. Do not add a task here
just because it was launched if the same operation will immediately complete, review, merge, or
otherwise resolve it.

## Active Tasks

- Worker 048: FS-09 Rust/Tauri Task Run Detail Backend is active in
  `C:\Users\user\.codex\worktrees\ef6d\Codex Orchestrator` on target branch
  `worker/048-task-run-detail-tauri-backend`. Worker thread:
  `019f28e6-b240-79f0-8e1a-bb0a0a24740e`. Pending worktree id:
  `local:3e860c6f-fa04-434d-a321-de1f05c11d9c`. Await completion report and result log
  `docs/task-logs/worker-048-task-run-detail-tauri-backend.md`; review before merge.

## Pending Blockers / Follow-Up

- Rust/Cargo/MSVC build verification is cleared when commands are run through the Visual Studio
  developer environment. Plain shells still do not resolve `link.exe`; use `vcvars64.bat` for native
  Rust/Tauri commands.
- The merged FS-08 run controls call the browser-safe `start_codex_task_run` facade, but live
  WebView execution still needs a Rust/Tauri backend command registration.
- The merged detail UI calls the browser-safe `load_task_run_detail` facade; Worker 048 is building
  the Rust/Tauri backend command.
- Post-run diff/validation capture exists for Node/local runtime callers, but the live WebView run
  command path has not yet been wired to use it.

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
- Worker 045: completed, verified, and logged. See
  `docs/task-logs/worker-045-msvc-build-tools-setup.md`.
- Worker 046: reviewed, corrected, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-046-task-run-detail-ui-shell.md`.
- Worker 047: reviewed, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-047-post-run-capture-composition.md`.

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
- Worker 046: `C:\Users\user\.codex\worktrees\071c\Codex Orchestrator`
- Worker 047: `C:\Users\user\.codex\worktrees\13b2\Codex Orchestrator`
