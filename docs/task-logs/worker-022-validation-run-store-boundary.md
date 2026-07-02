# Worker 022 ValidationRun Store Boundary

Date: 2026-07-02

## Summary

Added a narrow pure TypeScript ValidationRun create/update/query store boundary plus a SQLite
adapter on top of Worker 017's Artifact/ValidationRun schema. This slice does not add runtime
validation command execution, event emission, Codex runtime integration, runtime database file
opening, Tauri/Rust commands, Git execution, UI/React work, package/dependency changes, or broad
stores for other record families.

## Behavior

- Added `ValidationRunStore` with deterministic `createValidationRun`, `updateValidationRun`, and
  `queryValidationRuns` behavior.
- Creates require `command` and `status`, use injected ID/time providers, and leave optional
  `taskId`, `taskRunId`, `startedAt`, `completedAt`, `exitCode`, and `outputArtifactId` unset unless
  provided.
- Updates keep `id`, `command`, and `createdAt` immutable, update `updatedAt` from the injected
  clock, leave omitted fields unchanged, and treat `null` as an explicit clear for optional fields.
- Missing updates throw the typed `ValidationRunNotFoundError`.
- Queries support optional filters by `taskId`, `taskRunId`, `status`, and `outputArtifactId`, order
  by `createdAt` plus stable `id` tie-breaker, and support a non-negative integer `limit`.
- Added an in-memory implementation for focused domain tests.
- Added a SQLite adapter behind an injected SQLite-like interface with no production import from
  `node:sqlite`.
- The SQLite adapter uses Worker 017's `validationRunToRow` and `validationRunFromRow` mappers and
  the app migration coordinator in tests.

## Changed Files

- `src/domain/validationRunStore.ts`: ValidationRun store contract, create/update/query helpers,
  typed missing error, cloning, and in-memory implementation.
- `src/domain/validationRunStore.test.ts`: domain coverage for deterministic create, update
  semantics, explicit optional clears, missing-run errors, query filtering, ordering, limits, empty
  results, non-integer limits, and clone/output isolation.
- `src/infrastructure/sqlite/validationRunStore.ts`: SQLite adapter using injected database
  interfaces, Worker 017 mappers, and optional transactions.
- `src/infrastructure/sqlite/validationRunStore.test.ts`: app-migrated SQLite coverage for create
  round-trip, SQL `NULL` persistence, update semantics, typed missing errors, query behavior, limit
  behavior, clone/output isolation, and transaction rollback.
- `docs/architecture.md`: documented the ValidationRun store boundary.
- `docs/task-logs/worker-022-validation-run-store-boundary.md`: recorded this worker result.

## Verification

- `npm run test -- src/domain/validationRunStore.test.ts src/infrastructure/sqlite/validationRunStore.test.ts`
  -> pass
- Full required verification is recorded in the worker completion report after final run.

## Blockers

None known.

## Review Notes

- Review whether mutable `taskId` and `taskRunId` links are desirable long term. They follow the
  recent optional-field clear/update contract and the schema's `ON DELETE SET NULL` durability
  choice.
- The SQLite tests use `node:sqlite`, which emits Node's experimental feature warning during test
  runs.
