# Architecture Notes

Date: 2026-07-02

This document describes the current code architecture. It should explain where new work belongs and
which boundaries should stay intact.

## Runtime Shape

- Desktop shell: Tauri v2.
- UI: React, TypeScript, Vite.
- Domain/application/infrastructure layers are TypeScript-first today.
- Rust currently only proves the Tauri command boundary with `app_metadata`.
- SQLite infrastructure is written as pure TypeScript over injected SQLite-like interfaces.

Current limitation: the app can open a local runtime database file through an infrastructure
boundary, but it does not yet compose that database into the UI or execute Git/Codex from the UI.

## Boundary Rules

- React should consume application/domain facades, not parse Git, open SQLite, or execute Codex.
- Git output parsing stays under `src/infrastructure/git/`.
- SQLite schema/store code stays under `src/infrastructure/sqlite/`.
- Task lifecycle state changes stay in application services, not UI components.
- Codex execution should enter through a future runtime adapter; Codex credentials remain owned by
  Codex.
- Store raw runtime output as artifacts before deriving summaries.

## Domain Layer

Location: `src/domain/`

The domain layer owns product records and pure rules:

- `model.ts`: `Project`, `Repo`, `Branch`, `Worktree`, `Task`, `Conversation`, `TaskRun`,
  `Artifact`, `ValidationRun`, and `Event`.
- `dashboardProjection.ts`: derives Open Tasks groups from `DomainRecords`.
- `openTaskDashboardStore.ts` and `openTaskWriteStore.ts`: dashboard read/write contracts plus
  in-memory helpers.
- `repoSyncPlanning.ts`, `repoSyncPlanApplier.ts`, `repoSyncService.ts`, and `repoSyncStore.ts`:
  persistence-neutral repo scan reconciliation.
- `eventStore.ts`, `taskRunStore.ts`, `conversationStore.ts`, `artifactStore.ts`, and
  `validationRunStore.ts`: durable store contracts plus in-memory helpers.
- `seedData.ts` and `taskDashboard.ts`: demo records and seed dashboard export.

Important domain choices:

- `Task.executionState` says what the work is doing.
- `Task.attentionState` says what the human needs to do.
- Closed-task omission and dashboard grouping belong in `dashboardProjection.ts`.
- Store boundaries use explicit ID/time providers for deterministic tests and runtime wiring.

## Application Layer

Location: `src/application/`

`taskRunLifecycle.ts` coordinates existing stores for task-run start, success, and failure paths.
It:

- preflights task existence
- creates and updates `TaskRun` records
- optionally creates Codex `Conversation` records
- preserves task conversation links
- updates task execution/attention state
- optionally stores final-response artifacts
- appends lifecycle events

It intentionally does not execute Codex, parse Codex output, open SQLite, run validations, or own UI
behavior. Future runtime composition should call this service after Codex events are captured and
parsed.

`runComposition.ts` coordinates one persisted non-interactive Codex run over injected boundaries. It
starts the lifecycle with Codex conversation metadata, invokes an injected Codex runtime, stores the
raw stdout JSONL as a `raw_event_stream` artifact, updates conversation thread metadata and summary
when the runtime returns structured output, and then completes or fails the lifecycle with exit-code
and final-response details when available. The service is intentionally non-atomic and explicit
about that coordination boundary: it does not open the app database, import concrete runtime
infrastructure, execute child processes directly, choose database paths, collect diffs, run
validation commands, manage worktrees, or wire UI behavior.

`repoRegistryScan.ts` coordinates repo registration/scanning over injected Git and persistence
boundaries. It calls an injected `GitRepoScanner`, maps the scan through the Git infrastructure
facts mapper, persists the resulting repo/branch/worktree state through `RepoSyncStore`, and returns
the records touched by the current scan plus compact scan/change metadata suitable for future
UI/runtime wiring. Stale same-repo worktrees are reported separately rather than returned as current
scan worktrees. It does not list or remove registered repos, create worktrees, choose database paths,
execute Git directly, link tasks to worktrees, or expose raw command output as its primary API.

`taskWorktreeSelection.ts` coordinates selecting or creating the technical worktree anchor for a
task. It preflights task existence through `OpenTaskDashboardStore`, optionally asks an injected
`GitWorktreeCreator` to create one narrow Git worktree, scans/syncs the repo through
`repoRegistryScan.ts`, selects a scanned worktree by normalized path and/or branch name, and links
the task through `OpenTaskWriteStore.updateTask` with repo, worktree, and available branch records.
It intentionally does not run Git commands directly, delete worktrees, start Codex runs, collect
diffs, run validations, or own UI behavior.

## Infrastructure Layer

### Git

Location: `src/infrastructure/git/`

The Git infrastructure parses raw command output for status, branch, and worktree facts and
assembles normalized scan results. It does not execute Git yet. Future command execution should feed
raw output into these parsers rather than duplicating parsing in UI or application code.

`GitRepoScanner` is the current scanner interface consumed by application services. Concrete local
runtime wiring still needs to provide an implementation that gathers the raw Git command outputs and
feeds them into the existing parser/mapper functions.

### Codex

Location: `src/infrastructure/codex/`

`jsonlEvents.ts` parses captured `codex exec --json` newline-delimited JSON streams into typed
documented event envelopes while preserving each raw JSON object for future compatibility. It
handles documented top-level events, documented item categories, unknown event/item passthrough,
line-numbered parse errors, final agent-message extraction, terminal status, token usage, and item
counts.

This boundary does not execute Codex, read credentials, write stores, create artifacts/events, or
own task lifecycle transitions. Future runtime composition should use it after capturing raw output
and before updating task-run state.

`codexRuntime.ts` is the local runtime adapter boundary for non-interactive Codex execution. It
builds `codex exec --json` arguments as an array, invokes Codex through an injectable process
runner, preserves raw JSONL stdout and stderr, parses stdout with the JSONL parser, summarizes the
event stream, and returns terminal metadata including exit code, signal, parsed events, summary,
raw output, stderr, and a completed/failed/error classification. The default runner uses
`node:child_process` and remains outside React/UI imports. Non-zero Codex exits return structured
results when stdout is parseable; launch failures and untrustworthy JSONL still throw.

This adapter does not compose task-run lifecycle state, persist artifacts, manage conversations,
read or manage Codex credentials, run validation commands, collect diffs, or wire UI behavior.

### SQLite

Location: `src/infrastructure/sqlite/`

SQLite infrastructure includes:

- ordered schema migrations for repo sync, tasks, task runs, conversations, artifacts, validation
  runs, and events
- row mappers that preserve optional fields as SQL `NULL`
- store adapters for repo sync, open tasks, events, task runs, conversations, artifacts, and
  validation runs
- `migrationCoordinator.ts` for deterministic app migration order
- `appStore.ts` for constructing the store bundle over one injected connection
- `localAppDatabase.ts` for opening a local `node:sqlite` database file, enabling foreign keys,
  applying app migrations, creating runtime default providers when needed, and returning the app
  store bundle with an explicit close/dispose path

The pure SQLite adapters still do not open database files or import `node:sqlite`; runtime-facing
opening is isolated in `localAppDatabase.ts`. UI composition still needs to choose the app database
path and consume the resulting store bundle from an application/runtime boundary.

### Tauri

Location: `src/infrastructure/tauriCommands.ts` and `src-tauri/`

The current Tauri bridge is minimal. Future filesystem/process-heavy operations may live behind
Tauri commands or another explicit local runtime boundary, but React should not call those concerns
directly.

## UI Layer

Location: `src/app/`, `src/main.tsx`, `src/styles.css`

The current UI is a seed-data Open Tasks dashboard. It demonstrates the attention-first shape but
is not yet backed by persisted stores. The next UI work should wire persisted task reads/writes
before adding rich run review surfaces.

## Pending Runtime Architecture

The first usable runtime loop still needs:

1. UI/runtime composition that chooses the local app database path and exposes the opened store
   bundle to application services.
2. Runtime wiring that passes the SQLite-backed store bundle and concrete Codex runtime adapter into
   the run composition service.
3. Runtime wiring that passes a concrete `GitRepoScanner`, `RepoSyncStore`, and repo-sync ID/clock
   providers into the repo registry scan service.
4. Repo list/remove behavior once a UI/runtime caller needs that registry management surface.
5. Concrete Git worktree creation and scanning adapters for the task worktree selection service.
6. Diff and validation runners that store their outputs as artifacts/validation runs.
7. UI surfaces for starting runs and reviewing final response, diff, validation, and event history.

## Testing And Verification

The reliable verification set today is:

- `npm run lint`
- `npm run format:check`
- `npm run test`
- `npm run build`

`npm run build:tauri` remains blocked in the current environment until Rust/Cargo are installed or
available on `PATH`.
