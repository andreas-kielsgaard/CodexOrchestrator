# Worker 021 Artifact Store Boundary

Date: 2026-07-02

## Summary

Added a narrow pure TypeScript Artifact create/query store boundary plus a SQLite adapter on top of
Worker 017's Artifact/ValidationRun schema. This slice does not add ValidationRun store behavior,
Conversation store behavior, event emission, Codex runtime integration, runtime database file
opening, Tauri/Rust commands, Git execution, UI/React work, package/dependency changes, or broad
CRUD stores for other record families.

## Behavior

- Added `ArtifactStore` with deterministic `createArtifact` and narrow `queryArtifacts` behavior.
- Creates require `kind` and `title`, use injected ID/time providers, and leave optional `taskId`,
  `taskRunId`, `conversationId`, `uri`, and `content` unset unless provided.
- Queries support optional filters by `kind`, `taskId`, `taskRunId`, and `conversationId`.
- Query results are ordered by `createdAt` plus stable `id` tie-breaker and support a simple
  non-negative `limit`; `limit: 0` returns an empty list to match Event and TaskRun store behavior.
- Added an in-memory implementation for focused domain tests.
- Added a SQLite adapter behind an injected SQLite-like interface with no production import from
  `node:sqlite`.
- The SQLite adapter uses Worker 017's `artifactToRow` and `artifactFromRow` mappers and the app
  migration coordinator in tests.

## Changed Files

- `src/domain/artifactStore.ts`: Artifact store contract, create/query helpers, cloning, and
  in-memory implementation.
- `src/domain/artifactStore.test.ts`: domain coverage for deterministic create, no invented
  optionals, query filtering, ordering, limits, empty results, and clone/output isolation.
- `src/infrastructure/sqlite/artifactStore.ts`: SQLite adapter using injected database interfaces,
  Worker 017 mappers, and optional transactions.
- `src/infrastructure/sqlite/artifactStore.test.ts`: app-migrated SQLite coverage for create
  round-trip, SQL `NULL` persistence, query behavior, limit behavior, empty results, clone/output
  isolation, and transaction rollback.
- `docs/architecture.md`: documented the Artifact store boundary.
- `docs/task-logs/worker-021-artifact-store-boundary.md`: recorded this worker result.

## Verification

- `npm run test -- src/domain/artifactStore.test.ts src/infrastructure/sqlite/artifactStore.test.ts`
  -> pass
- Full required verification is recorded in the worker completion report after final run.

## Blockers

None known.

## Review Notes

- Review whether `limit: 0` returning an empty list should remain the long-term query contract. It
  matches Event and TaskRun stores and is covered by tests.
- The SQLite adapter currently loads ordered artifact rows and applies the shared domain query
  helper in memory. This keeps filtering semantics centralized for the narrow boundary; SQL pushdown
  can be added later if volume demands it.
- The SQLite tests use `node:sqlite`, which emits Node's experimental feature warning during test
  runs.
