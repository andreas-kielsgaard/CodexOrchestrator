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
- Task/run detail read model that composes task anchors, run history, grouped artifacts, validation
  links, and task events over existing store boundaries.

Known blockers / remaining runtime wiring:

- The `start_codex_task_run` TypeScript command facade exists, but no Rust/Tauri backend command is
  registered yet.
- Rust/Cargo are installed for the Windows user and `cargo metadata` succeeds. Full Rust/Tauri
  compilation remains environment-blocked because the MSVC linker `link.exe` is unavailable; install
  Visual Studio Build Tools with the Visual C++ option before re-running `cargo test`,
  `cargo build`, or `npm run build:tauri`.

## Remaining Tasks

| ID    | Task                           | Output                                                                                                                                                    | Depends On                          |
| ----- | ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FS-05 | Persisted Open Tasks dashboard | Application client, React boundary, and Rust SQLite command backend are merged; Rust compile/Tauri build verification remains blocked on MSVC Build Tools | Merged database opener              |
| FS-08 | Run controls in UI             | UI shell is merged; live WebView execution still needs the Rust/Tauri `start_codex_task_run` backend command                                              | Merged task worktree service, FS-05 |
| FS-09 | Task/run detail view           | Detail read model is merged; UI shell is active to show task anchors, run history, artifacts, validation summaries, and event timeline                    | FS-05, merged run composition       |
| FS-10 | Diff collector                 | Service boundary and local Git diff provider are merged; runtime trigger still needed for live post-run capture                                           | Merged task worktree service        |
| FS-11 | Validation command runner      | Service boundary and Node runtime adapter are merged; runtime trigger still needed for live validation capture                                            | Merged task worktree service        |
| FS-12 | Review surface MVP             | Show final response, diff state, validation status, and next action for completed/failed runs                                                             | FS-09, FS-10, FS-11                 |
| FS-13 | Tauri build environment        | Rust/Cargo available; Visual Studio Build Tools with `link.exe` still needed before `npm run build:tauri` can pass                                        | External environment                |

## Dependency Shape

Critical path:

1. Register a Rust/Tauri backend for `start_codex_task_run` so the merged UI controls can execute
   live Codex runs.
2. FS-09: add task/run detail UI for run review.
3. Add runtime triggers for post-run diff/validation capture through the composed services.
4. FS-12: add review-grade validation and live diff capture to the review flow.

Repo/worktree path:

1. Merged repo registry scan service registers/scans repos through injected Git and persistence
   boundaries.
2. Merged task worktree selection service links tasks to selected or created worktree records.
3. FS-08, FS-10, and FS-11 rely on a concrete worktree path.

Dashboard path:

1. FS-05 has moved the dashboard off direct seed-data imports and added Rust-side durable command
   handling.
2. Visual Studio Build Tools with the Visual C++ linker are still needed before the Tauri backend
   can be compiled locally.
3. FS-08 and FS-09 add runtime-specific controls and detail.

Review path:

1. Merged run composition creates run/artifact/event records.
2. Merged FS-10 service stores diff artifacts through an injected provider.
3. Merged FS-11 service stores validation output through an injected runtime.
4. FS-12 turns those records into the first review surface.

## Parallelization Opportunities

Safe immediately:

- Task detail data/UI slices can build against the merged stores and runtime records.
- Diff and validation trigger slices can call the composed local Git and validation services.
- FS-13 can run anytime.

Should wait:

- Live FS-08 verification should wait for the Rust/Tauri runtime command backend.
- FS-09 UI should wait for a detail read model unless built as a minimal shell only.
- FS-12 should wait for real run, diff, and validation records.

## Recommended Worker Sequencing

1. Add the FS-09 task/run detail UI shell.
2. Register the `start_codex_task_run` backend command behind the existing TypeScript facade.
3. Add composed post-run diff/validation triggers.
4. Launch FS-12 to pull final response, diff, validation, and next action into one review view.
5. Install Visual Studio Build Tools with the Visual C++ linker and re-run Rust tests plus
   `npm run build:tauri`.

## Orchestration Notes

- Prefer small workers that own one boundary. Do not fold repo/worktree setup, persisted dashboard
  wiring, run controls, diff collection, or validation execution into the same worker.
- Keep Codex credentials owned by Codex. The app should invoke Codex, not inspect its auth state.
- Store raw Codex JSONL as an artifact before relying on normalized summaries.
- Treat state transitions as lifecycle-recorder behavior, not UI behavior.
- Update `docs/active-task-map.md` as the last step before ending an orchestration operation, after
  it is clear which tasks remain blocked, active, or complete-but-unreviewed.
