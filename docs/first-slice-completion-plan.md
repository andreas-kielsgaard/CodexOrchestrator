# First Slice Completion Plan

Date: 2026-07-02

Purpose: guide orchestration from the current foundations to the first useful local dashboard loop.

First slice definition: a user can create or select a task, connect it to a repo/worktree, start a
`codex exec --json` run, preserve raw output, update task/run state, and review the final response,
events, diff, and validation result in the app.

## Current State

Already merged:

- App skeleton, tooling, Tauri/React/Vite shell, and seed dashboard.
- Domain model for projects, repos, branches, worktrees, tasks, conversations, task runs,
  artifacts, validation runs, and events.
- Git output parsers and repo sync planning/application boundaries.
- SQLite schema and store boundaries for repo sync, open tasks, events, task runs, conversations,
  artifacts, validation runs, and the app store bundle.
- Task-run lifecycle recorder over existing store boundaries.
- Codex JSONL event parser boundary.
- Runtime-facing local SQLite database opener over the app store bundle.
- Codex exec runtime adapter for non-interactive JSONL runs.
- Application-layer run composition service over injected stores and Codex runtime.
- Application-layer repo registry scan service over injected Git scanner and repo sync store
  boundaries.
- Application-layer task worktree selection/creation service over injected repo scan, task stores,
  and Git worktree creator boundaries.
- Application-layer Open Tasks dashboard client over injected read/write task stores, plus a
  browser-safe React/Tauri command boundary and dashboard UI controls for create/edit/state/archive.
- Rust-side SQLite backend for the Open Tasks Tauri commands, choosing the app data database and
  returning the existing dashboard snapshot shape.
- Application-layer diff collection service over injected stores and `GitDiffProvider`, storing
  `diff` artifacts and compact `artifact_created` metadata.
- Application-layer validation command runner service over injected stores and command runtime,
  storing `validation_log` artifacts and validation lifecycle events.
- Node-side validation command runtime adapter over `child_process.spawn`.
- Node-only local Git runtime adapters for repo scanning, worktree creation, and tracked-file diff
  collection.
- Node-only local runtime service composition that opens the app SQLite database once, reuses the
  store bundle, and wires the merged application services to concrete local Git, Codex, and
  validation adapters.
- Browser-safe runtime command contract and Tauri `invoke` client for `start_codex_task_run`, plus
  a Node-only local command handler over the composed run service.
- Open Tasks run-control UI shell that injects the runtime command client, sends task-scoped prompts
  with `cwd` from the task worktree path, and shows running/completed/failed feedback.
- Rust/Tauri `start_codex_task_run` backend that invokes `codex exec --json`, stores raw JSONL,
  updates task-run lifecycle state, links conversations, stores final responses, and returns the
  browser-safe command result shape.
- Task/run detail read model that composes task anchors, run history, grouped artifacts, validation
  links, and task events over existing store boundaries.
- Open Tasks task/run detail UI shell that injects a detail client and opens a read-only task
  inspector with anchors, run history, artifacts, validation summaries, and event timeline.
- Browser-safe `load_task_run_detail` Tauri facade plus Rust/Tauri SQLite backend for the detail UI.
- Caller-configured post-run capture composition service that can run Codex, then optionally collect
  a diff and run one validation command through existing services while preserving partial failures.
- Browser-safe `postRunCapture` command input plus live Rust/Tauri `start_codex_task_run` wiring
  that, after a completed Codex run, can optionally store a tracked diff artifact and/or run one
  validation command with a linked validation log artifact and validation events.
- App-side project/repo/worktree setup path: the Tauri backend can register a manual worktree
  anchor, the dashboard snapshot exposes registered worktrees, and the React shell can create a task
  linked to a runnable worktree.
- Rust/Cargo/MSVC native-build path verified through the Visual Studio developer environment;
  `npm run build:tauri` produces Windows bundles.

Known blockers / remaining runtime wiring:

- `link.exe` is available through the Visual Studio developer environment, not the default shell.
  Run native Rust/Tauri verification through `vcvars64.bat`; in this Codex shell, also prepend
  `%USERPROFILE%\.cargo\bin` inside that `cmd` session if `cargo` is not already on `PATH`.
- Direct `codex exec --json` smoke testing works when the local OpenAI Codex binary is used. The
  WindowsApps packaged shim still returns access denied when executed directly from this shell, so
  live app launches should use `%LOCALAPPDATA%\OpenAI\Codex\bin\codex.exe` explicitly or put that
  directory earlier on `PATH`.

## Remaining Tasks

| ID    | Task                           | Output                                                                                                                        | Depends On                             |
| ----- | ------------------------------ | ----------------------------------------------------------------------------------------------------------------------------- | -------------------------------------- |
| FS-05 | Persisted Open Tasks dashboard | Application client, React boundary, Rust SQLite command backend, and Rust/Tauri build verification are merged/cleared         | Merged database opener                 |
| FS-08 | Run controls in UI             | UI shell, Rust/Tauri start-run backend, and native build verification are merged/cleared                                      | Merged task worktree service, FS-05    |
| FS-09 | Task/run detail view           | Detail read model, UI shell, Tauri facade, and Rust/Tauri backend are merged/cleared                                          | FS-05, merged run composition          |
| FS-10 | Diff collector                 | Service boundary, local Git diff provider, post-run composition trigger, and explicit live WebView run-path wiring are merged | Merged task worktree service           |
| FS-11 | Validation command runner      | Service boundary, Node runtime adapter, post-run composition trigger, and explicit live WebView run-path wiring are merged    | Merged task worktree service           |
| FS-14 | Repo/worktree setup path       | Manual worktree registration, dashboard worktree anchors, and anchored task creation are implemented                          | Merged repo scan and worktree services |
| FS-12 | Review surface MVP             | Show final response, diff state, validation status, and next action for completed/failed runs                                 | FS-09, FS-10, FS-11                    |
| FS-13 | Tauri build environment        | Cleared: Rust/Cargo/MSVC build path works through the Visual Studio developer environment, and `npm run build:tauri` passes   | External environment                   |

## Dependency Shape

Critical path:

1. Manual-test the current live loop from project/repo/worktree setup through task creation,
   `codex exec --json`, detail loading, final response, diff, and validation records.
2. FS-12: add review-grade final-response, diff, validation, and next-action flow.

Repo/worktree path:

1. Merged repo registry scan service registers/scans repos through injected Git and persistence
   boundaries.
2. Merged task worktree selection service links tasks to selected or created worktree records.
3. FS-08, FS-10, and FS-11 rely on a concrete worktree path.

Dashboard path:

1. FS-05 has moved the dashboard off direct seed-data imports and added Rust-side durable command
   handling.
2. Rust/Tauri compile and bundle verification works through the Visual Studio developer environment.
3. FS-08 and FS-09 provide runtime-specific controls and persisted detail.

Review path:

1. Merged run composition creates run/artifact/event records.
2. Merged FS-10 service stores diff artifacts through an injected provider.
3. Merged FS-11 service stores validation output through an injected runtime.
4. FS-12 turns those records into the first review surface.

## Parallelization Opportunities

Safe immediately:

- Manual live-loop testing can use the new setup path without manual database seeding.

Should wait:

- Subjective review-surface polish and visible capture controls should wait until the live loop has
  been manually tested through the setup path.

## Recommended Worker Sequencing

1. Manual-test the live loop after FS-14 before launching extras or subjective polish.
2. After feedback, launch only the review-surface or capture-control work that is clearly needed.

## Orchestration Notes

- Prefer small workers that own one boundary. Do not fold repo/worktree setup, persisted dashboard
  wiring, run controls, diff collection, or validation execution into the same worker.
- Keep Codex credentials owned by Codex. The app should invoke Codex, not inspect its auth state.
- Store raw Codex JSONL as an artifact before relying on normalized summaries.
- Treat state transitions as lifecycle-recorder behavior, not UI behavior.
- Update `docs/active-task-map.md` as the last step before ending an orchestration operation, after
  it is clear which tasks remain blocked, active, or complete-but-unreviewed.
