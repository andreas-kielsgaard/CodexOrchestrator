# Worker 046 Task/Run Detail UI Shell

Date: 2026-07-03

Branch: `worker/046-task-run-detail-ui-shell`

## Summary

- Extended the React app boundary to accept an injected `TaskRunDetailClient` alongside the
  dashboard and runtime command clients.
- Added a browser-safe Tauri `load_task_run_detail` facade in `src/infrastructure/tauriCommands.ts`
  and injected it from `src/main.tsx`; Rust/backend implementation remains deferred.
- Added a read-only task detail inspector opened from each Open Task card. It renders task anchors,
  run history, grouped artifacts, validation summaries, and event timelines using the Worker 044
  read model shape.
- Kept live runtime behavior unchanged: no Rust commands, no workflow engine, no diff/validation
  triggers, no repo/worktree selection UI, and no Node/SQLite imports in React.
- Expanded App tests with fake injected detail clients covering successful load, backend error,
  selecting another task, and empty/no-run detail states.
- Orchestrator review correction: guarded post-run detail reloads against stale selected-task state
  and added regression coverage for switching detail selection while a run is pending.

## Changed Files

- `src/app/App.tsx`
- `src/app/App.test.tsx`
- `src/infrastructure/tauriCommands.ts`
- `src/infrastructure/tauriCommands.test.ts`
- `src/main.tsx`
- `src/styles.css`
- `docs/architecture.md`
- `docs/task-logs/worker-046-task-run-detail-ui-shell.md`

## Verification

- `npm test -- src/app/App.test.tsx`: passed
- Orchestrator correction verification: focused App/Tauri tests, `npm run lint`,
  `npm run format:check`, `npm run test`, and `npm run build` passed.

Final full-suite verification was run before handoff; see the worker completion report for the
latest command results.

## Blockers

- None for this slice.
- The default Tauri `load_task_run_detail` facade will report a backend command error until a later
  Rust/Tauri slice registers and implements the command.
