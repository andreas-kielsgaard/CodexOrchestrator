# Worker 024 App SQLite Store Bundle

Date: 2026-07-02

## Summary

Added a small pure TypeScript app-level SQLite store bundle/factory for assembling the existing
SQLite adapters over one injected database connection. This slice does not add runtime database
opening, path selection, Tauri/Rust commands, Codex runtime integration, workflow services, UI work,
or package/dependency changes.

## Behavior

- Added `AppSqliteDatabase`, a narrow SQLite-like connection type compatible with the existing
  adapter and migration coordinator interfaces.
- Added `initializeAppSqliteStoreDatabase`, which enables SQLite foreign keys and applies
  `applyAppSqliteMigrations` with optional deterministic migration timestamps.
- Added `createAppSqliteStoreBundle`, which returns the existing concrete SQLite adapters for repo
  sync, Open Tasks read/write, Event, TaskRun, Conversation, Artifact, and ValidationRun stores.
- Kept all ID and time providers explicit through named provider entries for each write-capable
  store.
- Added focused `node:sqlite` tests for idempotent initialization, concrete adapter assembly, and a
  shared-connection smoke path across task, task-run, conversation, artifact, validation, event, and
  dashboard reads.

## Changed Files

- `src/infrastructure/sqlite/appStore.ts`: app-level SQLite initialization and store bundle
  assembly boundary.
- `src/infrastructure/sqlite/appStore.test.ts`: focused in-memory SQLite coverage for the new
  module.
- `docs/architecture.md`: documented the app SQLite store bundle boundary.
- `docs/task-logs/worker-024-sqlite-store-bundle.md`: recorded this worker result.

## Verification

- `npm run test -- src/infrastructure/sqlite/appStore.test.ts` -> pass
- `git diff --check main...worker/024-sqlite-store-bundle` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None known.

## Review Notes

- Review the provider object shape in `src/infrastructure/sqlite/appStore.ts`; it intentionally uses
  named providers per write-capable store instead of hidden UUID/date defaults.
- Tests use `node:sqlite`, which emits Node's experimental feature warning during test runs.

## Orchestrator Review Addendum

The orchestrator made one small review correction before merge: `appStore.ts` now defines local
app-level ID/time provider interfaces instead of importing the Open Tasks write-store provider types
for every write-capable adapter. The provider shape remains explicit and deterministic, but the new
bundle boundary no longer semantically depends on the Open Tasks module for shared provider naming.
