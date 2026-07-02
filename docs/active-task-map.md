# Active Task Map

Updated: 2026-07-02

Purpose: fast recovery and orchestration continuity. This file tracks only work that still needs
attention: blockers, active workers, complete-but-unreviewed branches, pending corrections, and
cleanup that affects current work.

Update this file as the last step before ending an orchestration operation. Do not add a task here
just because it was launched if the same operation will immediately complete, review, merge, or
otherwise resolve it.

## Active Tasks

- Worker 035: Local Git Runtime Adapters Bundle
  - Status: launched; awaiting worker completion report
  - Pending worktree id: `local:c9793b9d-cdfd-4f2c-9333-b28ed6a99ed6`
  - Worktree: `C:\Users\user\.codex\worktrees\46d5\Codex Orchestrator`
  - Branch: `worker/035-local-git-runtime-adapters`
  - Launch base: `a54248c41d617b0c799c09b02d4f41d63864c6c5`
  - Result log target: `docs/task-logs/worker-035-local-git-runtime-adapters.md`
  - Next orchestration action: inspect worker report/worktree, review branch, independently
    verify, then merge or request correction.
- Worker 036: Validation Command Runtime Adapter
  - Status: launched; awaiting worker completion report
  - Pending worktree id: `local:52ac78b9-0540-429a-b109-6255e1a13362`
  - Worktree: `C:\Users\user\.codex\worktrees\478e\Codex Orchestrator`
  - Branch: `worker/036-validation-command-runtime-adapter`
  - Launch base: `a54248c41d617b0c799c09b02d4f41d63864c6c5`
  - Result log target: `docs/task-logs/worker-036-validation-command-runtime-adapter.md`
  - Next orchestration action: inspect worker report/worktree, review branch, independently
    verify, then merge or request correction.

## Pending Blockers / Follow-Up

- FS-05 backend persistence gap: the Open Tasks dashboard client/UI boundary is merged, but the
  registered Tauri commands still return an explicit backend-pending error. Durable default WebView
  persistence needs a Rust-side SQLite command adapter.
- `npm run build:tauri` is blocked until Rust/Cargo are installed or available on `PATH`.

## Recently Archived

- Worker 032: reviewed, corrected, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-032-persisted-open-tasks-dashboard.md`.
- Worker 033: reviewed, corrected, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-033-diff-collector-service.md`.
- Worker 034: reviewed, corrected, merged, verified, and Git-cleaned. See
  `docs/task-logs/worker-034-validation-command-runner.md`.

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
