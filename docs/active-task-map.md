# Active Task Map

Updated: 2026-07-03

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
  Rust/Tauri commands.
- Manual testing is the next gate before extras or subjective UI polish. The current live loop has
  dashboard task CRUD, run controls, persisted Codex execution, detail loading, and explicit
  post-run capture support in the command path.

## Blockers

- None beyond the pending blockers above.
