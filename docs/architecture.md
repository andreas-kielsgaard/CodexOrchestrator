# Architecture Notes

Updated: 2026-07-10

This document describes the current code architecture. It should explain where new work belongs and
which boundaries should stay intact.

## Runtime Shape

- Desktop shell: Tauri v2.
- UI: React, TypeScript, Vite.
- The legacy task/run model remains TypeScript-first with a parallel Rust Tauri implementation.
- The core Agent Session lifecycle is Rust-first: durable records, SQLite history, application
  coordination, Codex protocol handling, and process supervision live behind Tauri commands.
- Rust also retains the older command boundary for app metadata, Open Tasks persistence, manual
  repo/worktree registration, task/run detail reads, and task-scoped Codex execution.
- SQLite infrastructure is written as pure TypeScript over injected SQLite-like interfaces.

Current limitation: the app can open a local runtime database file through a Node-facing
infrastructure boundary, compose the TypeScript application services over that opened store bundle,
and inject concrete local Git, Codex, and validation adapters for Node-side callers. The Open Tasks
UI now consumes injected async dashboard, task/run detail, and runtime command clients. The default
Tauri WebView path has a narrow Rust-side SQLite backend for Open Tasks dashboard
load/create/update/archive commands, manual repo/worktree registration, the read-only
`load_task_run_detail` command, and live `start_codex_task_run` execution. The Rust run command
invokes `codex exec --json`, persists raw stdout JSONL before deriving summaries, and updates
task-run, conversation, artifact, event, execution-state, and attention-state tables. When callers
explicitly pass post-run capture options, the same Rust run command can also collect a tracked Git
diff and/or run one validation command after a completed Codex run. Visible UI controls for those
options are still deferred.

## Boundary Rules

- React should consume application/domain facades, not parse Git, open SQLite, or execute Codex.
- Git output parsing stays under `src/infrastructure/git/`.
- SQLite schema/store code stays under `src/infrastructure/sqlite/`.
- Task lifecycle state changes stay in application services, not UI components.
- Agent Session execution enters through the provider-neutral `AgentRuntime` port and its current
  Codex-specific adapter; Codex credentials remain owned by Codex.
- Persist raw runtime output before deriving transcript presentation. The legacy task-run path
  still stores its raw stream as an artifact; Agent Sessions store ordered raw runtime events.

## Agent Session Vertical Slice

Agent Sessions are the default application surface and are independent from the legacy task
dashboard. The responsibility flow is:

```text
React Agent Session screen
  -> TypeScript AgentSessionClient
    -> Tauri commands and persisted update event
      -> Rust AgentSessionApplication
        -> SQLite AgentSessionRepository
        -> CodexCliRuntime
          -> ProcessSupervisor
```

The Rust backend is authoritative. It persists a submitted invocation before launch, persists each
ordered runtime event before notifying the WebView, separately captures the external Codex context
ID, and persists terminal state idempotently. The frontend projects durable records into a
conversation: live work is open, completed work is collapsed, and the final response remains
prominent. Reload and short-interval active reconciliation repair missed transient events.

Primary module map:

- `src/application/agentSessions/`: serializable client contract and DTOs
- `src/infrastructure/agentSessions/`: Tauri client and persisted update subscription
- `src/features/agentSessions/`: controller, transcript projector, and focused UI components
- `src-tauri/src/agent_sessions/`: domain, ports, repository, lifecycle, and Tauri transport
- `src-tauri/src/runtime/codex/`: Codex command capability mapping and JSONL normalization
- `src-tauri/src/runtime/processes/`: direct-child ownership, streaming, cancellation, and shutdown

The older task `Conversation` and Codex run path are intentionally not foundations for Agent
Sessions. Task, goal, repo, and orchestration relationships remain deferred edges.

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
itself. The browser-safe dashboard snapshot now includes registered worktree anchors, and the Tauri
client supplies an optional `registerWorktree` operation for the Rust-backed manual setup command.

`taskRunDetailClient.ts` is the read-only task/run detail application boundary. It composes
`OpenTaskDashboardStore`, `TaskRunStore`, `ArtifactStore`, `EventStore`, and `ValidationRunStore`
into one serializable detail snapshot for a single task: task/project/repo/branch/worktree anchors,
run history ordered for review, grouped final-response/raw-event/diff/validation artifacts,
validation output links, unlinked task-level artifacts and validation runs, and a chronological
event timeline. It does not mutate stores, execute runtime commands, add UI behavior, or open
SQLite directly.

`runtimeCommandClient.ts` is the browser-safe runtime command contract for starting one Codex task
run. It defines serializable input and compact output shapes for task/run IDs, conversation and
artifact IDs, terminal metadata, updated task/run state, and optional caller-configured post-run
capture outcomes. Its `postRunCapture` input can request tracked diff collection and/or one
array-argument validation command. It does not import local runtime composition, execute Codex, open
stores, or own React behavior.

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
`composition.services.runCompositionService`, or `composeCodexTaskRunWithPostRunCapture` when the
caller provides explicit post-run capture options, then maps the rich composition result to compact
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
- `register_task_worktree`
- `create_open_task`
- `update_open_task`
- `archive_open_task`

Those commands are implemented in Rust over a local SQLite database under the Tauri app data
directory. The Rust backend applies the app schema migrations, uses UUID/time providers on the Rust
side, writes only task fields exposed by the current command contract, archives by setting
`execution_state = 'archived'`, omits archived/abandoned tasks from the dashboard, and returns the
existing `TaskDashboardSnapshot` shape. Its dashboard query duplicates only the small projection
needed for the command response; unlike the earlier TypeScript task read store, it returns all
persisted projects and registered worktree anchors so the dashboard can create runnable tasks for a
real project. `register_task_worktree` creates or reuses a project, repo, optional branch, and
worktree anchor without requiring manual database seeding.

The TypeScript Tauri bridge also exposes browser-safe clients for `start_codex_task_run` and
`load_task_run_detail`. The detail command is implemented in Rust as a read-only SQLite read model
over the same app data database and migration setup as the Open Tasks commands. It returns the
existing `TaskRunDetailSnapshot` shape: task/project/repo/branch/worktree anchors, run history
ordered for review, grouped artifacts, validation output links, unlinked task-level artifacts and
validation runs, and a chronological event timeline. The Rust grouping intentionally mirrors the
TypeScript read-model semantics, including validations that belong to a run through either
`validation_runs.task_run_id` or a linked output artifact. `start_codex_task_run` is implemented in
Rust over the same app data database. It starts a task-run lifecycle with a Codex conversation,
executes `codex exec --json` with array-style arguments and caller-provided `cwd`/environment,
stores the raw stdout JSONL as a `raw_event_stream` artifact before parsing, derives compact Codex
thread/final-response/terminal metadata, and completes or fails the task run with the existing
browser-safe command result shape. When `postRunCapture` is present and the Codex run completes,
the Rust backend can collect a tracked `git diff --binary HEAD --` artifact and/or run one
array-argument validation command through fakeable process-runner boundaries. Capture failures are
reported in the command result without turning the completed Codex run into a failed run.

The independent Agent Session bridge exposes:

- `create_agent_session`
- `list_agent_sessions`
- `load_agent_session`
- `send_agent_session_message`
- `cancel_agent_invocation`
- `agent-session://persisted-update`

These commands use migration `009_durable_agent_sessions_schema` and the Rust Agent Session
application/repository modules rather than the task-run tables. Tauri event listen/unlisten
permissions are explicitly scoped to the main window. Notifications are correlated by session and
invocation IDs and are never the only source of terminal truth.

## UI Layer

Location: `src/app/`, `src/features/`, `src/main.tsx`, `src/styles.css`

The thin app shell defaults to the independent Agent Session screen and mounts the old task
dashboard only when the user selects `Legacy Tasks`. Agent Session state is owned by its feature
controller, not the app shell or task screen. Its feature-owned components cover session selection,
transcript projection, processing/technical disclosures, Markdown final output, composer actions,
and deliberate follow-to-latest behavior.

The legacy task screen still consumes injected `TaskDashboardClient`, `TaskRunDetailClient`, and
`RuntimeCommandClient` instances and retains its existing repo/worktree, task-run, artifact,
validation, and detail behavior. It is preserved rather than presented as the future task model.
The default `src/main.tsx` wiring injects both sets of Tauri clients, keeping React/browser code away
from SQLite, process APIs, and raw Codex arguments.

## Pending Legacy Task Runtime Architecture

The legacy task surface still needs:

1. Repo list/remove behavior once a UI/runtime caller needs that registry management surface.
2. Review-grade UI that promotes final response, diff, validation, and next action into a focused
   decision surface.

## Testing And Verification

The reliable verification set today is:

- `npm run lint`
- `npm run format:check`
- `npm run test`
- `npm run build`

Rust/Cargo verification is environment-dependent. When Rust is installed, run the Rust
format/check/test/build checks in `src-tauri/` plus `npm run build:tauri`. The installed Codex help
compatibility probe is intentionally ignored by the ordinary suite and should be executed
explicitly for Agent Session release verification.
