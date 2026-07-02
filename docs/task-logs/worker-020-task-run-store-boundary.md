# Worker 020 TaskRun Store Boundary

Date: 2026-07-02

## Summary

Added a narrow pure TypeScript TaskRun create/update/query store boundary plus a SQLite adapter on
top of Worker 016's TaskRun/Conversation schema. This slice does not add Conversation CRUD/store
behavior, event emission, Codex runtime integration, runtime database file opening, Tauri/Rust
commands, Git execution, UI/React work, package/dependency changes, or broad stores for other record
families.

## Behavior

- Added `TaskRunStore` with deterministic `createTaskRun`, `updateTaskRun`, and `queryTaskRuns`
  behavior.
- Creates require `taskId` and `executionState`, use injected ID/time providers, and persist
  optional `conversationId`, `worktreeId`, `startedAt`, `completedAt`, and `exitCode` only when
  provided.
- Updates keep `taskId` and `createdAt` immutable, update `updatedAt` from the injected clock, leave
  omitted fields unchanged, and treat `null` as an explicit clear for optional fields.
- Missing updates throw the typed `TaskRunNotFoundError`.
- Queries support optional filters by `taskId`, `conversationId`, `worktreeId`, and
  `executionState`, order by `createdAt` plus stable `id` tie-breaker, and support a simple
  non-negative `limit`.
- Added an in-memory implementation for focused domain tests.
- Added a SQLite adapter behind an injected SQLite-like interface with no production import from
  `node:sqlite`.
- The SQLite adapter uses Worker 016's `taskRunToRow` and `taskRunFromRow` mappers and the app
  migration coordinator in tests.

## Changed Files

- `src/domain/taskRunStore.ts`: TaskRun store contract, create/update/query helpers, typed missing
  error, cloning, and in-memory implementation.
- `src/domain/taskRunStore.test.ts`: domain coverage for deterministic create, update semantics,
  explicit optional clears, missing-run errors, query filtering, ordering, limits, empty results, and
  clone/output isolation.
- `src/infrastructure/sqlite/taskRunStore.ts`: SQLite adapter using injected database interfaces,
  Worker 016 mappers, and optional transactions.
- `src/infrastructure/sqlite/taskRunStore.test.ts`: app-migrated SQLite coverage for create
  round-trip, SQL `NULL` persistence, update semantics, typed missing errors, query behavior, limit
  behavior, empty results, and transaction rollback.
- `docs/architecture.md`: documented the TaskRun store boundary.
- `docs/task-logs/worker-020-task-run-store-boundary.md`: recorded this worker result.

## Verification

- `git diff --check main...worker/020-task-run-store-boundary` -> pass
- `npm run test -- src/domain/taskRunStore.test.ts src/infrastructure/sqlite/taskRunStore.test.ts`
  -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None known.

## Review Notes

- Review whether `limit: 0` returning an empty list should remain the long-term query contract. It
  matches the current Event store boundary and is covered by tests.
- The SQLite adapter currently loads ordered task-run rows and applies the shared domain query
  helper in memory. This keeps filtering semantics centralized for the narrow boundary; SQL pushdown
  can be added later if volume demands it.
