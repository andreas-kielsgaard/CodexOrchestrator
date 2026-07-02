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

Known blocker:

- `npm run build:tauri` is blocked until Rust/Cargo are installed or on `PATH`.

## Remaining Tasks

| ID    | Task                           | Output                                                                                                                                                          | Depends On                                |
| ----- | ------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- | ----------------------------------------- |
| FS-03 | Repo registry UI/service path  | Add/list/remove repos through persisted stores; scan selected repo into repo/branch/worktree records                                                            | Existing Git parsers and repo sync stores |
| FS-04 | Worktree creation for a task   | Create/select app-managed task worktree and link it to task/branch/repo records                                                                                 | FS-03                                     |
| FS-05 | Persisted Open Tasks dashboard | Dashboard reads/writes SQLite-backed tasks instead of seed data; supports create/edit/archive/state changes                                                     | Merged database opener                    |
| FS-06 | Codex exec runtime adapter     | Narrow `CodexRuntime` implementation that runs `codex exec --json`, streams raw JSONL, parses events, and exposes terminal result metadata                      | Merged Codex parser                       |
| FS-07 | Run composition service        | Start/complete/fail task runs by combining runtime adapter, JSONL parser, lifecycle recorder, raw artifact storage, final-response artifact storage, and events | Merged database opener, FS-06             |
| FS-08 | Run controls in UI             | User can start a Codex run for a task in a selected worktree and see running/completed/failed state                                                             | FS-04, FS-05, FS-07                       |
| FS-09 | Task/run detail view           | Show task anchors, run history, final response, raw JSONL artifact link/summary, and event timeline                                                             | FS-05, FS-07                              |
| FS-10 | Diff collector                 | Capture worktree diff after a run and store it as an artifact                                                                                                   | FS-04, FS-07                              |
| FS-11 | Validation command runner      | Run configured validation command(s), store output artifact and validation run status, and surface failures                                                     | FS-04, FS-07                              |
| FS-12 | Review surface MVP             | Show final response, diff state, validation status, and next action for completed/failed runs                                                                   | FS-09, FS-10, FS-11                       |
| FS-13 | Tauri build environment        | Rust/Cargo available; `npm run build:tauri` can be verified                                                                                                     | External environment                      |

## Dependency Shape

Critical path:

1. FS-06: run and parse Codex.
2. FS-07: persist a full task-run lifecycle.
3. FS-08 and FS-09: expose run start and run review in the UI.
4. FS-10, FS-11, FS-12: add review-grade diff and validation.

Repo/worktree path:

1. FS-03 registers and scans repos.
2. FS-04 creates or selects worktrees for tasks.
3. FS-08, FS-10, and FS-11 rely on a concrete worktree path.

Dashboard path:

1. FS-05 replaces seed data with persisted task CRUD using the merged database opener.
2. FS-08 and FS-09 add runtime-specific controls and detail.

Review path:

1. FS-07 creates run/artifact/event records.
2. FS-10 adds diffs.
3. FS-11 adds validation output.
4. FS-12 turns those records into the first review surface.

## Parallelization Opportunities

Safe immediately:

- FS-03 can run in parallel with FS-06 if it stays at the service/store boundary.
- FS-05 can start now that the database opener is merged, if it keeps browser/runtime boundaries
  explicit.
- FS-06 can proceed without waiting for UI work.
- FS-13 can run anytime.

Safe after FS-03:

- FS-04 can proceed while FS-05/FS-06 continue.

Should wait:

- FS-07 should wait for FS-06.
- FS-08 should wait for FS-04, FS-05, and FS-07.
- FS-09 should wait for FS-07 unless built as a UI shell only.
- FS-10 and FS-11 should wait for FS-04 and FS-07.
- FS-12 should wait for real run, diff, and validation records.

## Recommended Worker Sequencing

1. Launch FS-06 as the next critical runtime boundary.
2. Launch FS-03 as a service/UI-boundary worker in parallel if review capacity allows.
3. Launch FS-05 once the runtime/database boundary for UI consumption is clear.
4. Launch FS-04 after FS-03.
5. Launch FS-07 as the main vertical integration worker.
6. Launch FS-08 and FS-09 as UI workers after FS-07.
7. Launch FS-10 and FS-11 in parallel after FS-04 and FS-07.
8. Launch FS-12 to pull final response, diff, validation, and next action into one review view.

## Orchestration Notes

- Prefer small workers that own one boundary. Do not combine FS-06 and FS-07 unless the runtime
  adapter is tiny after review.
- Keep Codex credentials owned by Codex. The app should invoke Codex, not inspect its auth state.
- Store raw Codex JSONL as an artifact before relying on normalized summaries.
- Treat state transitions as lifecycle-recorder behavior, not UI behavior.
- Update `docs/active-task-map.md` as the last step before ending an orchestration operation, after
  it is clear which tasks remain blocked, active, or complete-but-unreviewed.
