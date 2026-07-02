# Worker 035 - Local Git Runtime Adapters

Date: 2026-07-02

## Worktree And Branch

- Worktree: `C:\Users\user\.codex\worktrees\46d5\Codex Orchestrator`
- Branch: `worker/035-local-git-runtime-adapters`

## Summary

- Added a concrete Git repo scanner factory over the existing `GitCommandRunner` interface. It runs
  parser-compatible `git status`, `git branch`, `git remote`, and `git worktree list` commands and
  feeds stdout into `buildGitRepoScanResult`.
- Added `src/infrastructure/git/localGitRuntime.ts` with a Node-backed Git command runner,
  `GitCommandError` / `GitCommandLaunchError`, a local worktree creator, a tracked-file diff
  provider, and a bundle factory for future runtime composition.
- Updated branch summary formatting from literal `%xNN` escapes to Git-supported `%NN` escapes and
  made the branch parser tolerate the newline Git emits after formatted NUL-delimited records.
- Added focused coverage for command argument construction, non-zero Git exits, scanner assembly,
  worktree creation command shape, diff output, and the local adapter bundle.
- Updated `docs/architecture.md` to note that local Git adapters now exist but remain outside
  React/browser imports.

## Decisions

- Kept Node process execution isolated in `localGitRuntime.ts`; application services can keep using
  structural interfaces without importing runtime infrastructure.
- The command runner uses `node:child_process.spawn` with `shell: false` and `windowsHide: true`.
- Non-zero exits, signal exits, and launch failures reject with named errors that preserve command,
  args, cwd, and stdout/stderr when available.
- Worktree creation intentionally uses a narrow command shape:
  `git worktree add -b <branchName> <worktreePath> [baseBranch]`.
- The diff provider runs `git diff --binary HEAD --` from the worktree path. This captures tracked
  staged/unstaged changes and intentionally does not include untracked files in this slice.

## Verification

- `npm run test -- src/infrastructure/git` - passed, 3 files / 23 tests
- `npm run lint` - passed
- `npm run format:check` - passed after formatting `src/infrastructure/git/gitAdapter.test.ts`
- `npm run test` - passed, 41 files / 251 tests
- `npm run build` - passed
- `git diff --check main...worker/035-local-git-runtime-adapters` - passed
- `npm run build:tauri` - not run; per task instructions, the known Rust/Cargo availability blocker
  remains outside this slice.

## Blockers

- None for this slice.

## Review Notes

- Review the `%1f` / `%00` branch format correction: a local read-only probe showed this Git treats
  the previous `%x1f` / `%x00` form literally for `git branch --format`.
- Review whether future diff capture should add an explicit untracked-file artifact path; this
  provider intentionally stays on conservative tracked diffs.
- No UI, Tauri command wiring, validation command runtime, cleanup policy, branch naming policy, or
  workflow-engine behavior was added.

## Final Git Status

```text
## worker/035-local-git-runtime-adapters
```
