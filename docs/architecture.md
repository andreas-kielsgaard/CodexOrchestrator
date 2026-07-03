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

Current limitation: the app can open a local runtime database file through a Node-facing
infrastructure boundary, compose the TypeScript application services over that opened store bundle,
and inject concrete local Git, Codex, and validation adapters for Node-side callers. The Open Tasks
UI now consumes injected async dashboard and runtime command clients. The default Tauri WebView path
has a narrow Rust-side SQLite backend for Open Tasks dashboard load/create/update/archive commands.
A TypeScript runtime command contract and browser-safe `start_codex_task_run` client now exist for
starting one Codex task run, and the UI has compact run controls that call the injected runtime
client when a task exposes a worktree path. The Rust/Tauri command registration for that runtime
command remains a later slice.

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

`postRunCaptureComposition.ts` is the first explicit run-plus-capture application boundary. It
wraps the existing run composition service and, only after a completed Codex run, invokes caller
configured post-run diff capture and/or one caller configured validation command through the
existing diff and validation services. It returns the original run result alongside optional
post-run outcomes, preserving partial failures such as a successful Codex run followed by failed
diff collection or failed validation. It does not choose default validation commands, schedule
workflows, supervise processes, open databases, or add UI/Tauri behavior.

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

`diffCollection.ts` coordinates post-run worktree diff capture over existing stores and an injected
`GitDiffProvider`. It preflights the task, optional task run, and worktree path through
`OpenTaskDashboardStore` records, stores the diff body as a `diff` artifact even when empty, and
emits one compact `artifact_created` event with diff size/empty metadata. It does not execute Git
directly, mutate task/run lifecycle state, run validation commands, or wire UI/Tauri behavior.

`taskDashboardClient.ts` is the Open Tasks application/client boundary. It composes
`OpenTaskDashboardStore` and `OpenTaskWriteStore` into async load/create/update/archive operations
that return a dashboard snapshot for UI callers. The client is verified against in-memory stores
and the local SQLite app store bundle, but it does not open SQLite files or import Node-only modules
itself.

`taskRunDetailClient.ts` is the read-only task/run detail application boundary. It composes
`OpenTaskDashboardStore`, `TaskRunStore`, `ArtifactStore`, `EventStore`, and `ValidationRunStore`
into one serializable detail snapshot for a single task: task/project/repo/branch/worktree anchors,
run history ordered for review, grouped final-response/raw-event/diff/validation artifacts,
validation output links, unlinked task-level artifacts and validation runs, and a chronological
event timeline. It does not mutate stores, execute runtime commands, add UI behavior, or open
SQLite directly.

`runtimeCommandClient.ts` is the browser-safe runtime command contract for starting one Codex task
run. It defines serializable input and compact output shapes for task/run IDs, conversation and
artifact IDs, terminal metadata, and updated task/run state. It does not import local runtime
composition, execute Codex, open stores, or own React behavior.

`validationCommandRunner.ts` coordinates one configured validation command over injected task,
validation-run, artifact, event, and command-runtime boundaries. It preflights the task and
worktree/cwd, creates a running `ValidationRun`, executes the injected runtime, stores a
`validation_log` artifact with stdout/stderr/process metadata, updates the validation outcome, and
emits validation lifecycle events. It does not execute processes directly, collect diffs, compose
Codex runs, or wire UI behavior.

## Infrastructure Layer

### Git

Location: `src/infrastructure/git/`

The Git infrastructure parses raw command output for status, branch, and worktree facts and
assembles normalized scan results. It also includes Node-only local runtime adapters for Git command
execution, repo scanning, worktree creation, and tracked-file diff collection. These adapters use
the parser-compatible command outputs and remain outside React/browser imports; future runtime
composition can inject them into application services without changing UI code.

`GitRepoScanner` is the current scanner interface consumed by application services. Concrete local
runtime wiring can use the local scanner factory to gather raw Git command outputs and feed them
into the existing parser/mapper functions.

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

### Validation

Location: `src/infrastructure/validation/`

`validationCommandRuntime.ts` is the Node-side runtime adapter for configured validation commands.
It structurally satisfies the application-layer `ValidationCommandRuntime` contract through
type-only imports, invokes exactly one configured command with argument arrays, cwd, and inherited
environment plus caller overrides, and preserves raw stdout/stderr plus optional chunk callbacks.
The default runner uses `node:child_process.spawn` with `shell: false` and hidden Windows process
windows. It returns exit code and signal metadata without deciding whether validation passed.

This adapter does not compose validation-run lifecycle state, persist artifacts/events, choose
commands, run multiple commands, collect diffs, or wire UI/Tauri behavior.

### Local Runtime Composition

Location: `src/infrastructure/localRuntimeComposition.ts`

The local runtime composition boundary is Node-only. It opens the local app SQLite database through
`openLocalAppSqliteDatabase`, reuses the resulting app store bundle, constructs local Git adapters
with `createLocalGitRuntimeAdapters`, constructs the Codex runtime with `createCodexRuntime`,
constructs the validation command runtime with `createValidationCommandRuntime`, and exposes the
application service objects needed by upcoming run-control and review slices:

- store-backed Open Tasks dashboard client
- task-run lifecycle recorder
- task/run detail client
- Codex run composition service
- repo registry scan service
- task worktree selection service
- diff collection service
- validation command runner service
- post-run capture composition service

The composition keeps database opening, process runners, adapter instances, repo-sync ID providers,
and clocks injectable so tests and future callers can exercise the boundary without running live
Git, Codex, or validation commands. It also exposes `close`/`dispose` by forwarding the opened
database lifecycle. Browser/React entrypoints must not import this module directly.

`localRuntimeCommands.ts` is also Node-only. It adapts the browser-safe runtime command contract to
the composed local runtime services by calling `composeCodexTaskRun` through
`composition.services.runCompositionService`, then maps the rich composition result to compact
serializable command output. It does not register Tauri/Rust commands or import React/browser
entrypoints.

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

The pure SQLite adapters still do not open database files or import `node:sqlite`; Node
runtime-facing opening is isolated in `localAppDatabase.ts`. Browser/React modules must not import
this opener. The Tauri/Rust backend independently applies the same ordered app schema migrations to
the app data database for the Open Tasks command path.

### Tauri

Location: `src/infrastructure/tauriCommands.ts` and `src-tauri/`

The Tauri bridge exposes browser-safe TypeScript functions over `@tauri-apps/api/core`. The Open
Tasks dashboard command contract is:

- `load_open_task_dashboard`
- `create_open_task`
- `update_open_task`
- `archive_open_task`

Those commands are implemented in Rust over a local SQLite database under the Tauri app data
directory. The Rust backend applies the app schema migrations, uses UUID/time providers on the Rust
side, writes only task fields exposed by the current command contract, archives by setting
`execution_state = 'archived'`, omits archived/abandoned tasks from the dashboard, and returns the
existing `TaskDashboardSnapshot` shape. Its dashboard query duplicates only the small projection
needed for the command response; unlike the earlier TypeScript task read store, it returns all
persisted projects so the dashboard can create the first task for a real project.

The TypeScript Tauri bridge also exposes a browser-safe runtime command client for
`start_codex_task_run`. That client is intentionally only a typed `invoke` facade today; no Rust
runtime command implementation is registered in this slice.

## UI Layer

Location: `src/app/`, `src/main.tsx`, `src/styles.css`

The Open Tasks UI consumes injected `TaskDashboardClient` and `RuntimeCommandClient` instances,
loads asynchronously, and provides visible create, edit, state-change, archive, and per-task Codex
run controls. Run controls use the projected task `worktreePath` as the command `cwd`, stay
unavailable when no worktree is linked, show compact running/completed/failed feedback, and reload
the dashboard after a run attempt so persisted state can be reflected once the backend command
exists. The default `src/main.tsx` wiring injects the Tauri command clients, keeping React/browser
code away from SQLite and Node-only modules. Tests exercise the UI against fake clients; durable
behavior is covered at the application client/store boundary.

## Pending Runtime Architecture

The first usable runtime loop still needs:

1. Rust/Tauri registration that backs `start_codex_task_run` with the local runtime command handler
   without importing Node-only modules into React entrypoints.
2. Repo list/remove behavior once a UI/runtime caller needs that registry management surface.
3. Runtime triggers that call the composed diff and validation services after a Codex run completes.
4. UI surfaces for starting runs and reviewing final response, diff, validation, and event history.

## Testing And Verification

The reliable verification set today is:

- `npm run lint`
- `npm run format:check`
- `npm run test`
- `npm run build`

Rust/Cargo verification is environment-dependent. When Rust is installed, run the Rust
format/test/build checks in `src-tauri/` plus `npm run build:tauri`; otherwise the TypeScript
verification set remains available.
