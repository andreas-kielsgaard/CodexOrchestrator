# Worker 031 Task Worktree Service

Date: 2026-07-02

## Summary

Added an application-layer task worktree selection/creation boundary. The service preflights task
existence through the open task dashboard store, optionally delegates worktree creation to a narrow
injected Git boundary, reuses the repo registry scan service to scan/sync repo records, selects a
scanned worktree by path and/or branch name, and links the task to the resulting repo, branch when
available, and worktree records.

## Files Changed

- `src/application/taskWorktreeSelection.ts`
- `src/application/taskWorktreeSelection.test.ts`
- `docs/architecture.md`
- `docs/task-logs/worker-031-task-worktree-service.md`

## Verification

- `git diff --check main...worker/031-task-worktree-service` - passed
- `npm run test -- taskWorktreeSelection` - passed
- `npm run lint` - passed
- `npm run format:check` - failed because untouched `docs/first-slice-completion-plan.md`
  has existing Prettier style drift
- `npm run test` - passed, 36 files / 223 tests
- `npm run build` - passed
- `npm run build:tauri` - not run; Rust/Cargo are not available on `PATH`

## Blockers

- `npm run format:check` is blocked by existing formatting drift in
  `docs/first-slice-completion-plan.md`, which this slice did not modify.
- `npm run build:tauri` is blocked because Rust/Cargo are not available on `PATH`.

## Review Notes

- The service clears a previous task `branchId` when the selected scanned worktree has no branch,
  preventing stale branch anchors on detached or branchless worktrees.
- Creation is intentionally only a narrow injected `GitWorktreeCreator` boundary. No concrete Git
  command runner was added in this slice.
- Selection happens after the repo registry scan service syncs records, so scan reconciliation is
  not duplicated.
