# Active Task Map

Updated: 2026-07-02

Purpose: fast recovery and orchestration continuity. This file tracks only work that still needs
attention: blockers, active workers, complete-but-unreviewed branches, pending corrections, and
cleanup that affects current work.

Update this file as the last step before ending an orchestration operation. Do not add a task here
just because it was launched if the same operation will immediately complete, review, merge, or
otherwise resolve it.

## Active Tasks

- Worker 033: Diff Collector Service Boundary
  - Status: launched; awaiting worker completion report
  - Pending worktree id: `local:0fa11c73-f9b6-47c9-8854-cc0ed17a77f3`
  - Worktree: `C:\Users\user\.codex\worktrees\6610\Codex Orchestrator`
  - Branch: `worker/033-diff-collector-service`
  - Launch base: `d30a046e4c1f1e43df20fbe9f3e138d85dd7214f`
  - Result log target: `docs/task-logs/worker-033-diff-collector-service.md`
  - Next orchestration action: inspect worker report/worktree, review branch, independently
    verify, then merge or request correction.
- Worker 034: Validation Command Runner Service Boundary
  - Status: launched; awaiting worker completion report
  - Pending worktree id: `local:bbb5785a-a455-45d2-8d6c-3d8673724937`
  - Worktree: `C:\Users\user\.codex\worktrees\d19f\Codex Orchestrator`
  - Branch: `worker/034-validation-command-runner`
  - Launch base: `d30a046e4c1f1e43df20fbe9f3e138d85dd7214f`
  - Result log target: `docs/task-logs/worker-034-validation-command-runner.md`
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
