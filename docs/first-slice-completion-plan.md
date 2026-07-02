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

Known blocker:

- `npm run build:tauri` is blocked until Rust/Cargo are installed or on `PATH`.

## Remaining Tasks

| ID    | Task                           | Output                                                                                                      | Depends On                          |
| ----- | ------------------------------ | ----------------------------------------------------------------------------------------------------------- | ----------------------------------- |
| FS-05 | Persisted Open Tasks dashboard | Dashboard reads/writes SQLite-backed tasks instead of seed data; supports create/edit/archive/state changes | Merged database opener              |
| FS-08 | Run controls in UI             | User can start a Codex run for a task in a selected worktree and see running/completed/failed state         | Merged task worktree service, FS-05 |
| FS-09 | Task/run detail view           | Show task anchors, run history, final response, raw JSONL artifact link/summary, and event timeline         | FS-05, merged run composition       |
| FS-10 | Diff collector                 | Capture worktree diff after a run and store it as an artifact                                               | Merged task worktree service        |
| FS-11 | Validation command runner      | Run configured validation command(s), store output artifact and validation run status, and surface failures | Merged task worktree service        |
| FS-12 | Review surface MVP             | Show final response, diff state, validation status, and next action for completed/failed runs               | FS-09, FS-10, FS-11                 |
| FS-13 | Tauri build environment        | Rust/Cargo available; `npm run build:tauri` can be verified                                                 | External environment                |

## Dependency Shape

Critical path:

1. FS-05: replace seed tasks with persisted task CRUD.
2. FS-08 and FS-09: expose run start and run review in the UI.
3. FS-10, FS-11, FS-12: add review-grade diff and validation.

Repo/worktree path:

1. Merged repo registry scan service registers/scans repos through injected Git and persistence
   boundaries.
2. Merged task worktree selection service links tasks to selected or created worktree records.
3. FS-08, FS-10, and FS-11 rely on a concrete worktree path.

Dashboard path:

1. FS-05 replaces seed data with persisted task CRUD using the merged database opener.
2. FS-08 and FS-09 add runtime-specific controls and detail.

Review path:

1. Merged run composition creates run/artifact/event records.
2. FS-10 adds diffs.
3. FS-11 adds validation output.
4. FS-12 turns those records into the first review surface.

## Parallelization Opportunities

Safe immediately:

- FS-05 can start now that the database opener is merged, if it keeps browser/runtime boundaries
  explicit.
- FS-10 and FS-11 can start at the service boundary now that task worktree selection and run
  composition are merged.
- FS-13 can run anytime.

Should wait:

- FS-08 should wait for FS-05 and concrete runtime wiring.
- FS-09 should wait for persisted tasks and real run records unless built as a UI shell only.
- FS-12 should wait for real run, diff, and validation records.

## Recommended Worker Sequencing

1. Launch FS-05 next so the visible dashboard stops depending on seed data.
2. Launch FS-10 and FS-11 as service-boundary workers in parallel if capacity allows.
3. Launch FS-08 and FS-09 as UI workers after FS-05.
4. Launch FS-12 to pull final response, diff, validation, and next action into one review view.

## Orchestration Notes

- Prefer small workers that own one boundary. Do not fold repo/worktree setup, persisted dashboard
  wiring, run controls, diff collection, or validation execution into the same worker.
- Keep Codex credentials owned by Codex. The app should invoke Codex, not inspect its auth state.
- Store raw Codex JSONL as an artifact before relying on normalized summaries.
- Treat state transitions as lifecycle-recorder behavior, not UI behavior.
- Update `docs/active-task-map.md` as the last step before ending an orchestration operation, after
  it is clear which tasks remain blocked, active, or complete-but-unreviewed.
