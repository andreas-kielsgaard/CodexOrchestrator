# Worker 051: Repo/Worktree Setup Path

Date: 2026-07-03

## Summary

- Added a manual repo/worktree registration path to the Tauri command boundary so a clean app
  database can create project, repo, optional branch, and worktree anchors without manual SQLite
  seeding.
- Extended the dashboard snapshot with registered worktree anchors and let task creation carry
  optional repo/branch/worktree IDs.
- Added a compact React setup form and a worktree selector in the task composer so newly created
  tasks can immediately expose runnable Codex controls.
- Preserved existing behavior for unanchored tasks and for tasks without worktree paths.

## Changed Files

- `docs/active-task-map.md`
- `docs/architecture.md`
- `docs/first-slice-completion-plan.md`
- `docs/orchestration-log.md`
- `docs/task-logs/worker-051-repo-worktree-setup-path.md`
- `src/application/taskDashboardClient.ts`
- `src/infrastructure/tauriCommands.ts`
- `src/app/App.tsx`
- `src/app/App.test.tsx`
- `src/styles.css`
- `src-tauri/src/lib.rs`

## Verification

- `npm run build` passed.
- `npm run test -- src/app/App.test.tsx src/application/taskDashboardClient.test.ts src/infrastructure/tauriCommands.test.ts` passed.
- `npx prettier --check src/app/App.tsx src/app/App.test.tsx src/application/taskDashboardClient.ts src/infrastructure/tauriCommands.ts src/styles.css` passed.
- `cargo fmt --check` passed.
- `cargo test register_worktree --lib` passed through the Visual Studio developer environment.
- `git diff --check` passed.
- `npm run lint` passed.
- `npm run format:check` passed.
- `npm run test` passed: 47 files, 284 tests. Node emitted the existing experimental SQLite
  warnings.
- `cargo test` passed through the Visual Studio developer environment: 15 Rust tests.
- `npm run build:tauri` passed through the Visual Studio developer environment and produced MSI and
  NSIS bundles. Tauri emitted the existing bundle identifier warning about `.app`.
- Orchestrator re-verification on 2026-07-04 passed the same full suite. In this Codex shell,
  Rust/Tauri commands needed `vcvars64.bat` plus a temporary `%USERPROFILE%\.cargo\bin` PATH prepend
  so both `cargo` and MSVC `link.exe` resolved in the same `cmd` session.
- A direct smoke run of `%LOCALAPPDATA%\OpenAI\Codex\bin\codex.exe exec --json --sandbox read-only
--ephemeral "Reply with OK only."` returned a final `OK` JSONL message and exit code 0. The
  default `codex` command still resolved to the WindowsApps shim in this shell.

## Deferred

- File/folder picker integration.
- Git auto-discovery and branch scanning in the Tauri setup command.
- Repo list/remove management.
- Workflow defaults, branch policy, and cleanup policy.
- Visible post-run capture controls.
- Review-surface polish beyond making the task runnable.
