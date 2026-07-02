# Architecture Notes

Date: 2026-07-01

## Stack Choice

This bootstrap follows the roadmap stack:

- Tauri v2 for the desktop shell.
- React, TypeScript, and Vite for the frontend.
- Rust commands as the backend boundary for local filesystem, process, Git, and future Codex-adapter work.

The current Rust side exposes only an `app_metadata` command. It exists to prove the command boundary without starting Codex integration, Git scanning, or persistence prematurely.

## Local-First Boundary

The UI treats tasks as the unit of attention. Technical anchors such as repos, branches, worktrees, conversations, validation runs, events, and artifacts are modeled separately so project/repo drilldowns can explain where work lives without becoming the dashboard's primary organizing shape.

Future slices should keep filesystem/process-heavy behavior behind Tauri commands or adapter modules instead of calling Git, SQLite, or Codex directly from React components.

## Domain Boundary

The TypeScript domain layer lives under `src/domain/`:

- `model.ts` defines the core product records: Project, Repo, Branch, Worktree, Conversation, Task, TaskRun, Artifact, ValidationRun, and Event.
- `Task.executionState` tracks what the work is doing, while `Task.attentionState` tracks what kind of human attention it needs.
- `dashboardProjection.ts` derives the Open Tasks dashboard groups from domain records. React components should consume this projection instead of owning grouping rules directly.
- `openTaskDashboardStore.ts` defines the read-side boundary for the Open Tasks dashboard. Stores load the minimal `DomainRecords` snapshot needed by `projectOpenTaskDashboard`, and the facade returns projected dashboard groups so callers do not duplicate grouping, sorting, or closed-task omission rules.
- `openTaskWriteStore.ts` defines the write-side boundary for Open Tasks create, update, and archive-style close behavior. The contract keeps `Task` as the mutation unit, uses injected ID/time providers for deterministic adapters and tests, and treats omitted update fields as unchanged while `null` explicitly clears optional anchors or timestamps. Conversation IDs are replaced as an ordered list by the write boundary, while dashboard grouping, sorting, and closed-task omission remain centralized in `dashboardProjection.ts`.
- `repoSyncPlanning.ts`, `repoSyncPlanApplier.ts`, and `repoSyncService.ts` keep Git scan reconciliation persistence-neutral: scans become explicit repo/branch/worktree upsert plans, then those plans can be applied to in-memory `DomainRecords` with injected deterministic IDs before a future repository layer persists them. The service facade returns both the plan and the applied result so persistence/UI callers can inspect the plan without duplicating scan-to-plan-to-record choreography.
- `repoSyncStore.ts` defines the narrow async persistence boundary for repo sync. A store loads the current domain snapshot for a scan and persists only the applied repo, branch, and worktree records produced by `syncRepoFromScan`; it does not duplicate plan/apply logic. The included in-memory implementation is a test helper, and the concrete SQLite adapter lives under `src/infrastructure/sqlite/`.
- `eventStore.ts` defines the append/query boundary for durable Event records. Appends use injected ID
  and time providers, clone JSON payload objects at the boundary, and keep query behavior narrow:
  optional filtering by event kind and linked IDs plus chronological ordering with an ID tie-breaker.
  The included in-memory implementation is a test helper, and the concrete SQLite adapter lives
  under `src/infrastructure/sqlite/`.
- `taskRunStore.ts` defines the create/update/query boundary for durable TaskRun records. Creates
  require the owning task ID and execution state, use injected ID/time providers, and leave optional
  conversation, worktree, timing, and exit fields unset unless provided. Updates keep `taskId` and
  `createdAt` immutable, treat omitted fields as unchanged, and treat `null` as an explicit clear
  for optional fields. Queries filter by task, conversation, worktree, or execution state and order
  by `createdAt` plus a stable ID tie-breaker. The included in-memory implementation is a test
  helper, and the concrete SQLite adapter lives under `src/infrastructure/sqlite/`.
- `artifactStore.ts` defines the create/query boundary for durable Artifact records. Creates require
  an artifact kind and title, use injected ID/time providers, and leave optional task, task-run,
  conversation, URI, and content fields unset unless provided. Queries filter by kind or optional
  links and order by `createdAt` plus a stable ID tie-breaker. The included in-memory implementation
  is a test helper, and the concrete SQLite adapter lives under `src/infrastructure/sqlite/`.
- `validationRunStore.ts` defines the create/update/query boundary for durable ValidationRun
  records. Creates require a command and status, use injected ID/time providers, and leave optional
  task, task-run, timing, exit, and output-artifact fields unset unless provided. Updates keep
  `id`, `command`, and `createdAt` immutable, treat omitted fields as unchanged, and treat `null` as
  an explicit clear for optional fields. Queries filter by task, task run, status, or output
  artifact and order by `createdAt` plus a stable ID tie-breaker. The included in-memory
  implementation is a test helper, and the concrete SQLite adapter lives under
  `src/infrastructure/sqlite/`.
- `conversationStore.ts` defines the create/update/query boundary for durable Conversation records.
  Creates require a provider and title, use injected ID/time providers, and leave optional task,
  task-run, external-thread, and summary fields unset unless provided. Updates keep `id`,
  `provider`, and `createdAt` immutable, treat omitted fields as unchanged, and treat `null` as an
  explicit clear for optional fields. Queries filter by provider, task, task run, or external thread
  and order by `createdAt` plus a stable ID tie-breaker. The included in-memory implementation is a
  test helper, and the concrete SQLite adapter lives under `src/infrastructure/sqlite/`.
- `seedData.ts` provides demo records until SQLite persistence and repository APIs are introduced.

SQLite migrations, Rust database commands, Git scanning, and Codex runtime integration are intentionally outside this slice.

## Repo Sync SQLite Schema Foundation

The repo-sync persistence schema foundation lives under `src/infrastructure/sqlite/` as pure
TypeScript infrastructure. It defines ordered migration SQL for the minimal durable repo-sync
subset:

- `projects`: minimal project rows needed as the repo foreign-key parent.
- `repos`: repo identity and scan metadata, unique by `(project_id, root_path)`.
- `branches`: branch facts and app-owned annotations, unique by `(repo_id, name)`.
- `worktrees`: worktree facts, dirty/main flags, optional branch link, optional lock reason, and
  last scan timestamp, unique by `(repo_id, path)`.

The schema preserves repo-sync ownership boundaries: project or repo deletion cascades to owned
technical records, while deleting a branch uses `ON DELETE SET NULL` for `worktrees.branch_id` so a
worktree is not unexpectedly deleted merely because its branch row disappeared. Optional domain
fields persist as SQL `NULL`, booleans persist as checked `0`/`1` integers, and mappers convert
between SQLite row shapes and the existing `Project`, `Repo`, `Branch`, and `Worktree` domain
records.

This slice does not add a SQLite-backed `RepoSyncStore`; future store implementation should apply
these migrations, enable SQLite foreign keys on each connection, and keep scan reconciliation behind
the existing `RepoSyncStore` boundary.

## Repo Sync SQLite Store Adapter

The concrete repo-sync SQLite adapter lives in `src/infrastructure/sqlite/repoSyncStore.ts` and
implements the domain `RepoSyncStore` boundary without importing `node:sqlite` in production code.
It depends on a small injected database/statement interface with `prepare`, statement `get/all/run`,
and optional `exec` support.

`loadRepoSyncRecords({ projectId, rootPath })` normalizes the requested root path, loads the project
row if present, loads only the repo matching `(project_id, root_path)`, and includes only that repo's
branches and worktrees. Other domain record arrays are intentionally empty so repo sync receives the
minimal snapshot it needs and cannot accidentally reason over unrelated repos.

`persistRepoSyncRecords` upserts the applied repo, branch, and worktree records returned by the
domain sync flow. It does not delete stale worktrees; stale records remain durable while the domain
result reports them as missing from the current scan. Optional fields continue to round-trip through
SQL `NULL`, including explicit clears for `worktrees.branch_id`, `worktrees.lock_reason`, and repo or
branch optional columns. When the injected database supports `exec`, persistence runs inside
`BEGIN`/`COMMIT` and rolls back the whole upsert batch on failure.

Schema helpers now expose `enableRepoSyncSqliteForeignKeys` and `applyRepoSyncSqliteMigrations` so
tests and future runtime wiring can consistently enable foreign keys and apply the ordered
repo-sync migrations.

## Open Tasks SQLite Schema Foundation

The Open Tasks dashboard persistence schema foundation lives under `src/infrastructure/sqlite/` as
pure TypeScript infrastructure. It defines ordered migration SQL for the dashboard's task subset
without adding a CRUD/store implementation or runtime database wiring:

- `tasks`: durable task rows with separate execution state, attention state, priority, optional
  due/snooze timestamps, and optional technical anchors.
- `task_conversation_links`: ordered `Task.conversationIds` links using `(task_id,
conversation_id)` as the key plus a deterministic `position` column.

The task schema reuses the repo-sync `projects`, `repos`, `branches`, and `worktrees` tables as
foreign-key parents. Deleting a project cascades to its tasks. Deleting optional technical anchors
sets `tasks.repo_id`, `tasks.branch_id`, or `tasks.worktree_id` to `NULL` so task intent survives
technical cleanup. Deleting a task cascades to its conversation links.

`task_conversation_links.conversation_id` is still persisted as a stable text identifier without a
foreign key. The later TaskRun/Conversation schema introduces the `conversations` table for
provenance records, but this ordered dashboard link table intentionally remains text-only until a
future link-integrity/backfill migration decides how strictly task dashboard conversation IDs should
reference conversation rows. Mapper helpers convert between `Task` records and SQLite rows,
preserving optional fields as SQL `NULL` and preserving `conversationIds` by `position`.

## Open Tasks SQLite Read Store

The Open Tasks dashboard read store lives in `src/infrastructure/sqlite/openTaskDashboardStore.ts`.
It implements the domain `OpenTaskDashboardStore` boundary with a small injected
database/statement interface compatible with `node:sqlite` tests and does not import `node:sqlite`
from production infrastructure code.

The reader loads task rows, ordered `task_conversation_links`, and only the linked parent
`projects`, `repos`, `branches`, and `worktrees` referenced by those tasks. It intentionally loads
archived and abandoned task rows too, then relies on `projectOpenTaskDashboard` to omit closed tasks
so dashboard rules stay centralized in the domain projection instead of being duplicated in SQL.
Optional technical anchors may be `NULL`; missing repo, branch, or worktree rows simply produce
dashboard tasks without those optional labels.

## Open Tasks SQLite Write Store

The Open Tasks SQLite write adapter lives in `src/infrastructure/sqlite/openTaskWriteStore.ts`.
It implements the domain `OpenTaskWriteStore` boundary with an injected SQLite-like
database/statement interface and does not import `node:sqlite` in production infrastructure code.

The adapter creates tasks with deterministic `IdProvider` and `TimeProvider` inputs, persists
optional technical anchors and due/snooze timestamps as SQL `NULL`, and stores ordered
`Task.conversationIds` in `task_conversation_links`. Updates reuse the domain task-update helper so
omitted fields remain unchanged, `null` explicitly clears optional anchors or timestamps, and
conversation IDs replace the full ordered link list only when provided. `archiveTask` sets
`executionState` to `archived`; dashboard omission remains centralized in
`projectOpenTaskDashboard`.

When the injected database supports `exec`, create/update/archive writes run inside
`BEGIN`/`COMMIT` with rollback on failure. Missing update/archive targets throw the typed
`OpenTaskNotFoundError`.

## App SQLite Migration Coordinator

The app-level SQLite migration coordinator lives in
`src/infrastructure/sqlite/migrationCoordinator.ts` as pure TypeScript infrastructure. It composes
the current schema families in deterministic order: repo-sync migrations first, then Open Tasks
migrations, then TaskRun/Conversation migrations, then Artifact/ValidationRun migrations, then
Event migrations.

The coordinator depends on an injected SQLite-like interface with `exec` and `prepare`; it does not
open database files and does not import `node:sqlite` in production code. Runtime callers should use
`enableAppSqliteForeignKeys` on each connection, then `applyAppSqliteMigrations` before constructing
store adapters.

Applied migrations are tracked in `schema_migrations` with migration ID, applied timestamp, and
position. Duplicate migration IDs are rejected before any SQL is applied. Each unapplied migration
runs with its audit-row insert in a transaction, so failed migrations are not recorded and their DDL
is rolled back by SQLite. Tests inject deterministic applied timestamps for auditability while
future runtime wiring can provide its own clock.

## App SQLite Store Bundle

The app-level SQLite store bundle lives in `src/infrastructure/sqlite/appStore.ts` as a small
assembly boundary for runtime callers that already have a SQLite-like connection. It exports a
narrow `AppSqliteDatabase` interface, `initializeAppSqliteStoreDatabase` to enable foreign keys and
apply coordinated app migrations, and `createAppSqliteStoreBundle` to construct the existing
repo-sync, Open Tasks read/write, Event, TaskRun, Conversation, Artifact, and ValidationRun SQLite
adapters over the same injected connection.

The bundle does not open database files, choose paths, import `node:sqlite` in production code,
reach into Tauri/Rust, or add workflow services. Write-capable stores still receive explicit named
ID and time providers so runtime wiring remains deterministic and testable.

## TaskRun and Conversation SQLite Schema Foundation

The TaskRun and Conversation persistence schema foundation lives under
`src/infrastructure/sqlite/` as pure TypeScript infrastructure. It defines ordered migration SQL for
the provenance records that connect a task to a concrete execution attempt and its conversation
trace:

- `task_runs`: task-owned execution attempts with optional conversation/worktree links, execution
  state, optional start/completion timestamps, and optional exit code.
- `conversations`: Codex, ChatGPT-export, or manual conversation records with optional task and
  task-run links plus external thread metadata.

`task_runs.task_id` references `tasks(id)` with `ON DELETE CASCADE` so task-owned run history is
cleaned up with the task. Optional links use `ON DELETE SET NULL`: deleting a worktree does not
delete a run, deleting a conversation clears `task_runs.conversation_id`, deleting a task run clears
`conversations.task_run_id`, and deleting a task clears `conversations.task_id`. This keeps local
provenance records durable when related technical or workflow records are cleaned up.

The optional `TaskRun` to `Conversation` relationship is represented with nullable foreign keys on
both tables. This preserves referential integrity while keeping insertion practical: callers can
insert a task run without a conversation, insert the conversation linked to that run, then update the
task run with the conversation ID. Row mappers convert `TaskRun` and `Conversation` records to and
from SQLite rows, preserve optional fields as SQL `NULL`, and constrain execution/provider unions as
checked text values.

## TaskRun SQLite Store Boundary

The TaskRun store boundary lives in `src/domain/taskRunStore.ts`, with the SQLite adapter in
`src/infrastructure/sqlite/taskRunStore.ts`. It persists concrete task execution attempts without
starting runtime execution, appending events, managing conversations, or opening database files.

The SQLite adapter depends on an injected SQLite-like interface and does not import `node:sqlite` in
production code. It uses the Worker 016 `taskRunToRow` and `taskRunFromRow` mappers so SQL `NULL`
handling and domain row translation stay centralized in the schema layer. When the injected
database supports `exec`, create/update writes run inside `BEGIN`/`COMMIT` with rollback on failure;
missing updates throw the typed `TaskRunNotFoundError`.

## Conversation SQLite Store Boundary

The Conversation store boundary lives in `src/domain/conversationStore.ts`, with the SQLite adapter
in `src/infrastructure/sqlite/conversationStore.ts`. It persists conversation provenance records
without parsing transcripts, importing ChatGPT exports, appending events, managing task-run back
links, or opening database files.

The SQLite adapter depends on an injected SQLite-like interface and does not import `node:sqlite` in
production code. It uses the Worker 016 `conversationToRow` and `conversationFromRow` mappers so
optional SQL `NULL` handling and domain row translation stay centralized in the schema layer. When
the injected database supports `exec`, create and update writes run inside `BEGIN`/`COMMIT` with
rollback on failure; missing updates throw the typed `ConversationNotFoundError`.

## Artifact SQLite Store Boundary

The Artifact store boundary lives in `src/domain/artifactStore.ts`, with the SQLite adapter in
`src/infrastructure/sqlite/artifactStore.ts`. It persists durable local outputs without starting
validation storage, appending events, managing conversations, or opening database files.

The SQLite adapter depends on an injected SQLite-like interface and does not import `node:sqlite` in
production code. It uses the Worker 017 `artifactToRow` and `artifactFromRow` mappers so optional
SQL `NULL` handling and domain row translation stay centralized in the schema layer. When the
injected database supports `exec`, creates run inside `BEGIN`/`COMMIT` with rollback on failure.

## ValidationRun SQLite Store Boundary

The ValidationRun store boundary lives in `src/domain/validationRunStore.ts`, with the SQLite
adapter in `src/infrastructure/sqlite/validationRunStore.ts`. It persists validation attempt
metadata and output-artifact links without running validation commands, appending events, managing
artifacts, or opening database files.

The SQLite adapter depends on an injected SQLite-like interface and does not import `node:sqlite` in
production code. It uses the Worker 017 `validationRunToRow` and `validationRunFromRow` mappers so
optional SQL `NULL` handling and domain row translation stay centralized in the schema layer. When
the injected database supports `exec`, create and update writes run inside `BEGIN`/`COMMIT` with
rollback on failure; missing updates throw the typed `ValidationRunNotFoundError`.

## Artifact and ValidationRun SQLite Schema Foundation

The Artifact and ValidationRun persistence schema foundation lives under
`src/infrastructure/sqlite/` as pure TypeScript infrastructure. It defines ordered migration SQL for
durable local outputs and validation attempts:

- `artifacts`: optional links to tasks, task runs, and conversations plus checked artifact kind,
  title, optional URI/content, and creation timestamp.
- `validation_runs`: optional links to tasks and task runs, command/status details, optional timing
  and exit metadata, and an optional output artifact link.

All links in this schema are nullable and use `ON DELETE SET NULL` so artifacts and validation
history survive cleanup of related workflow/provenance rows. `validation_runs.output_artifact_id`
references `artifacts(id)` when present and is nullable so callers can insert a validation run
before its output artifact exists, then update the validation row after the artifact is recorded.
Row mappers convert `Artifact` and `ValidationRun` records to and from SQLite rows, preserve
optional fields as SQL `NULL`, and constrain artifact-kind/validation-status unions as checked text
values.

This slice did not add CRUD stores, runtime database file opening, Tauri/Rust commands, Codex
runtime integration, Git execution, or React/UI work.

## Event SQLite Schema Foundation

The Event persistence schema foundation lives under `src/infrastructure/sqlite/` as pure TypeScript
infrastructure. It defines ordered migration SQL for durable local audit/event records:

- `events`: checked event kind, occurrence timestamp, optional links to project, task, task run,
  conversation, artifact, and validation run rows, plus a deterministic JSON payload text column.

All event links are nullable and use `ON DELETE SET NULL` so event records survive cleanup of
related workflow, provenance, output, or validation rows. Row mappers convert between `Event`
records and SQLite rows, preserve optional links as SQL `NULL`, constrain `Event.kind` as checked
text, serialize payload objects with sorted keys for deterministic JSON text, and throw clear errors
when a persisted row contains invalid JSON or a non-object payload.

This slice does not add an event append/query store, runtime database file opening, Tauri/Rust
commands, Codex runtime integration, Git execution, React/UI work, or package dependencies.

## Event Store Boundary

The Event append/query boundary lives in `src/domain/eventStore.ts`, with the SQLite adapter in
`src/infrastructure/sqlite/eventStore.ts`. The boundary is intentionally append-only: callers
provide checked domain event kinds, optional links, and a JSON-object payload, while injected ID and
time providers create deterministic `Event.id` and `Event.occurredAt` values for adapters and
tests.

Queries stay narrow and useful for local audit reads. Callers can filter by event kind and optional
linked IDs (`projectId`, `taskId`, `taskRunId`, `conversationId`, `artifactId`, and
`validationRunId`), receive events ordered by `occurredAt` plus stable `id` tie-breaker, and apply a
simple non-negative `limit`. Payloads are cloned/serialized at the store boundary so caller-side
object mutation cannot change stored event payloads by reference.

The SQLite adapter depends on an injected SQLite-like interface and does not import `node:sqlite` in
production code. It uses the Worker 018 `eventToRow` and `eventFromRow` mappers, so payload
serialization, optional SQL `NULL` links, and persisted-row validation remain centralized in the
schema layer.

## Git Adapter Boundary

Git output parsing lives under `src/infrastructure/git/` as pure TypeScript infrastructure code. The current adapter foundation normalizes parseable command output from `git status --porcelain=v1 -z`, `git branch --format=...`, and `git worktree list --porcelain -z` into scan facts that can later feed the domain `Repo`, `Branch`, and `Worktree` records.

Repo scan assembly is also pure TypeScript: raw command outputs, a root path, optional default branch, and scan timestamp are composed into `GitRepoScanResult` without executing Git. A thin domain-facing mapping layer derives normalized repo, branch, and worktree facts from that scan while keeping non-root worktree dirtiness explicit as `unknown` until each worktree can be scanned directly.

React components should not invoke Git or parse Git output directly. Future Tauri/Rust or sidecar command execution should enter through the thin Git adapter boundary and hand raw command output to these parsers.

## Tooling Note

Rust/Cargo were not available on the worker PATH during bootstrap, so desktop verification could not run in this environment. Frontend build, lint, formatting, and tests remain independently verifiable through npm scripts.
