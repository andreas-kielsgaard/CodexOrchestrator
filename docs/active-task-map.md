# Active Task Map

Updated: 2026-07-03

Purpose: fast recovery and orchestration continuity. This file tracks only work that still needs
attention: blockers, active workers, complete-but-unreviewed branches, pending corrections, and
cleanup that affects current work.

Update this file as the last step before ending an orchestration operation. Do not add a task here
just because it was launched if the same operation will immediately complete, review, merge, or
otherwise resolve it.

## Active Tasks

- Worker 039: Runtime Command Contract Boundary is active in
  `C:\Users\user\.codex\worktrees\c6f0\Codex Orchestrator` on target branch
  `worker/039-runtime-command-contract`. Pending worktree id:
  `local:e9f471df-c410-4056-aabb-c8238ea72a6d`. Await completion report and result log
  `docs/task-logs/worker-039-runtime-command-contract.md`; review before merge.
- Worker 040: Rust Toolchain Setup And Tauri Verification is active in
  `C:\Users\user\.codex\worktrees\83c3\Codex Orchestrator` on target branch
  `worker/040-rust-toolchain-setup`. Pending worktree id:
  `local:c3fdb1cd-62ad-43ee-8ee2-e4a06858ff2b`. Await completion report and result log
  `docs/task-logs/worker-040-rust-toolchain-setup.md`; review before merge.
- Worker 041: Orchestration Review is active in
  `C:\Users\user\.codex\worktrees\b18b\Codex Orchestrator` on target branch
  `worker/041-orchestration-review`. Pending worktree id:
  `local:3233e65c-fa11-45a6-ab94-62a5457d347c`. Await completion report and result log
  `docs/task-logs/worker-041-orchestration-review.md`; review before merge.

## Pending Blockers / Follow-Up

- Rust/Cargo are unavailable on `PATH`, so the merged Rust/Tauri backend cannot yet be compile
  verified and `npm run build:tauri` fails at `cargo metadata`.

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
