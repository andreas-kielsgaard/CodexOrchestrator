# Active Task Map

Updated: 2026-07-04

Purpose: fast recovery and orchestration continuity. This file tracks only work that still needs
attention: blockers, active workers, complete-but-unreviewed branches, pending corrections, and
cleanup that affects current work.

Update this file as the last step before ending an orchestration operation. Do not add a task here
just because it was launched if the same operation will immediately complete, review, merge, or
otherwise resolve it.

## Active Tasks

- None.

## Pending Blockers / Follow-Up

- Rust/Cargo/MSVC build verification is cleared when commands are run through the Visual Studio
  developer environment. Plain shells still do not resolve `link.exe`; use `vcvars64.bat` for native
  Rust/Tauri commands. This Codex shell also may need `%USERPROFILE%\.cargo\bin` prepended inside
  that `cmd` session so `cargo` resolves.
- Manual testing is the next gate before extras or subjective UI polish. The current live loop now
  has app-side project/repo/worktree registration, anchored task creation, dashboard task CRUD, run
  controls, persisted Codex execution, detail loading, and explicit post-run capture support in the
  command path.
- For live Codex runs from this shell, use the explicit local binary at
  `%LOCALAPPDATA%\OpenAI\Codex\bin\codex.exe` or put that directory before the WindowsApps packaged
  shim on `PATH`; direct execution of the WindowsApps `codex.exe` currently returns access denied.

## Blockers

- None beyond the pending blockers above.
