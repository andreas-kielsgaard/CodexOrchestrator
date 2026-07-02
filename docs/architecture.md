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

Current limitation: the app does not yet open a real runtime database file or execute Git/Codex from
the UI.

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

## Infrastructure Layer

### Git

Location: `src/infrastructure/git/`

The Git infrastructure parses raw command output for status, branch, and worktree facts and
assembles normalized scan results. It does not execute Git yet. Future command execution should feed
raw output into these parsers rather than duplicating parsing in UI or application code.

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

The SQLite adapters do not open database files and do not import `node:sqlite` in production code.
Runtime wiring still needs to choose the local database path, open the connection, enable foreign
keys, apply migrations, and create the app store bundle.

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

1. Review/merge the Codex JSONL parser currently on Worker 026.
2. Runtime database opening and store-bundle construction.
3. A `CodexRuntime` adapter that invokes `codex exec --json` and streams raw JSONL.
4. A composition service that stores raw JSONL, extracts final response/thread metadata, updates
   lifecycle state, and appends events.
5. Diff and validation runners that store their outputs as artifacts/validation runs.
6. UI surfaces for starting runs and reviewing final response, diff, validation, and event history.

## Testing And Verification

The reliable verification set today is:

- `npm run lint`
- `npm run format:check`
- `npm run test`
- `npm run build`

`npm run build:tauri` remains blocked in the current environment until Rust/Cargo are installed or
available on `PATH`.
