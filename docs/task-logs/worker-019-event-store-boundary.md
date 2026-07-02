# Worker 019 Event Store Boundary

Date: 2026-07-02

## Summary

Added a narrow pure TypeScript Event append/query store boundary on top of Worker 018's SQLite event
schema foundation. This slice does not add Codex runtime integration, event-sourced projections,
workflow engine behavior, UI/React work, Tauri/Rust database commands, Git execution, runtime DB
file opening, package/dependency changes, or broad CRUD stores for other domain records.

## Behavior

- Added `EventStore` with append-only `appendEvent` and narrow `queryEvents` operations.
- Appends use injected ID and time providers for deterministic `Event.id` and `Event.occurredAt`.
- Event payloads are cloned as JSON objects at the domain boundary and serialized through the
  existing SQLite mappers so callers cannot mutate stored payloads by reference.
- Queries filter by event kind and optional linked IDs: `projectId`, `taskId`, `taskRunId`,
  `conversationId`, `artifactId`, and `validationRunId`.
- Query results are ordered by `occurredAt` plus stable `id` tie-breaker, with optional
  non-negative `limit` support.
- Added an in-memory implementation for focused domain tests.
- Added a SQLite adapter behind an injected SQLite-like interface with no production
  `node:sqlite` import.

## Changed Files

- `src/domain/eventStore.ts`: Event store contract, deterministic append behavior, in-memory
  implementation, cloning, query filtering, ordering, and limit validation.
- `src/domain/eventStore.test.ts`: domain coverage for append cloning, filtering, ordering, empty
  results, limits, and cloned query results.
- `src/infrastructure/sqlite/eventStore.ts`: SQLite adapter using `eventToRow` and `eventFromRow`
  with optional transaction support.
- `src/infrastructure/sqlite/eventStore.test.ts`: app-migrated SQLite coverage for append
  round-trip, optional link persistence, deterministic payload serialization/cloning, query
  filtering, ordering, empty results, limits, and transaction rollback.
- `docs/architecture.md`: documented the Event store boundary.
- `docs/task-logs/worker-019-event-store-boundary.md`: recorded this worker result.

## Verification

- `npm run test -- src/domain/eventStore.test.ts src/infrastructure/sqlite/eventStore.test.ts` ->
  pass
- Full required verification is recorded in the worker completion report after final run.

## Blockers

None.

## Review Notes

- Review whether `limit: 0` returning an empty list is the preferred long-term query contract. It is
  simple, deterministic, and covered by tests.
- The SQLite adapter currently loads ordered event rows and applies the shared domain query helper
  in memory. This keeps filtering behavior centralized for the narrow boundary; SQL pushdown can be
  added later if event volume requires it.
