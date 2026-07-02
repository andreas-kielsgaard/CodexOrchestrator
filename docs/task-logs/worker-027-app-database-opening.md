# Worker 027 App Database Opening

Date: 2026-07-02

## Summary

Added the first runtime-facing SQLite database opening boundary around the existing pure TypeScript
app store bundle. This slice opens a local `node:sqlite` database path, initializes app schema and
foreign keys, returns the assembled store bundle, and exposes an explicit close/dispose path.

## Behavior

- Added `localAppDatabase.ts` with `openLocalAppSqliteDatabase(...)`.
- Default runtime providers use `crypto.randomUUID()` for entity IDs and `new Date().toISOString()`
  for timestamps.
- Tests can inject the SQLite connection opener, store providers, and migration timestamp provider.
- Initialization failures close the opened connection before rethrowing.
- The module stays under SQLite infrastructure and is not wired into React, Tauri commands, or
  runtime composition services.

## Changed Files

- `src/infrastructure/sqlite/localAppDatabase.ts`
- `src/infrastructure/sqlite/localAppDatabase.test.ts`
- `docs/architecture.md`
- `docs/task-logs/worker-027-app-database-opening.md`

## Verification

- `git diff --check main...worker/027-app-database-opening` -> pass
- `npm run test -- src/infrastructure/sqlite/localAppDatabase.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None known at implementation time. `npm run build:tauri` remains expected to be blocked unless
Rust/Cargo are installed or available on `PATH`.

## Review Notes

- Review whether the returned handle should expose the raw `db` long term. It is useful for runtime
  composition and deterministic tests now, but UI code should consume higher-level application
  services rather than the connection directly.
- The default providers intentionally live in the opener module, leaving `appStore.ts` deterministic
  and fully injected.
