# Active Task Map

Updated: 2026-07-02

Purpose: fast recovery and orchestration continuity. This file tracks only work that still needs
attention: blockers, active workers, complete-but-unreviewed branches, pending corrections, and
cleanup that affects current work.

Update this file as the last step before ending an orchestration operation. Do not add a task here
just because it was launched if the same operation will immediately complete, review, merge, or
otherwise resolve it.

## Active Tasks

### Worker 027: App Database Opening

- Status: launched; awaiting worker completion report
- Pending worktree id: `local:ebc5e321-fadf-4c52-8106-bdfbf5cfac7c`
- Expected branch: `worker/027-app-database-opening`
- Worktree: `C:\Users\user\.codex\worktrees\04d6\Codex Orchestrator`
- Launch base: `d5a0ac19c457e4640e562f6063f679176032f862` (`Log Worker 026 merge`)
- Expected result log: `docs/task-logs/worker-027-app-database-opening.md`
- Report-back instruction: included in the worker prompt
- Next action: wait for Worker 027 completion, then inspect the worker branch/result log directly,
  review source, run independent verification, and merge only accepted work.

## Blockers

- `npm run build:tauri` is blocked until Rust/Cargo are installed or available on `PATH`.

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
