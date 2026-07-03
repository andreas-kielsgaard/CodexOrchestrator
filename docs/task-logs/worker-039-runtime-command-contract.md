# Worker 039: Runtime Command Contract Boundary

Date: 2026-07-03
Branch: `worker/039-runtime-command-contract`
Worktree: `C:\Users\user\.codex\worktrees\c6f0\Codex Orchestrator`

## Summary

Added the next narrow runtime bridge after Worker 038:

- browser-safe `RuntimeCommandClient` contract for starting one Codex task run
- Tauri `invoke` facade for `start_codex_task_run`
- Node-only local command handler that calls `composeCodexTaskRun` through
  `composition.services.runCompositionService`
- focused fake-runtime tests for the local handler and Tauri facade

No React run controls, Rust/Tauri backend implementation, live Codex execution, Git execution, or
validation execution were added in this slice.

## Verification

- `git diff --check main...worker/039-runtime-command-contract` passed.
- `.\node_modules\.bin\vitest.cmd run src/infrastructure/localRuntimeCommands.test.ts src/infrastructure/tauriCommands.test.ts`
  passed: 2 files, 3 tests.
- `npm run lint` passed.
- `npm run test` passed: 45 files, 264 tests.
- `npm run build` passed.
- `npm run format:check` failed on pre-existing/orchestrator-owned
  `docs/orchestration-log.md`. This slice did not edit that file, and the worker prompt explicitly
  said not to edit it.

`npm run build:tauri` was intentionally not required because Rust/Cargo remain unavailable on
`PATH`.

## Notes

The `start_codex_task_run` TypeScript facade now exists, but Rust/Tauri command registration remains
a later slice. The local handler is Node-only and imports the local runtime composition type only at
the infrastructure boundary.
