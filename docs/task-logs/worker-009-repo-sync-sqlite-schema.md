# Worker 009 Repo Sync SQLite Schema Foundation

Date: 2026-07-02

## Summary

Added a pure TypeScript SQLite schema foundation for the repo-sync persistence subset. This slice
does not implement a concrete SQLite `RepoSyncStore`, add runtime database wiring, execute Git
commands as product behavior, touch Tauri/Rust, or add package dependencies.

## Behavior

- Added ordered migration SQL for `projects`, `repos`, `branches`, and `worktrees`.
- `repos` are unique by `(project_id, root_path)`.
- `branches` are unique by `(repo_id, name)`.
- `worktrees` are unique by `(repo_id, path)`.
- Project and repo deletes cascade to owned repo-sync records.
- Branch deletes set `worktrees.branch_id` to `NULL` instead of deleting worktrees.
- Optional domain fields map to SQL `NULL` and back to omitted optional properties.
- `Worktree.isMain` and `Worktree.isDirty` map to checked SQLite `0`/`1` integers.
- Row types and mappers cover `Project`, `Repo`, `Branch`, and `Worktree`.
- Tests apply migrations to an in-memory `node:sqlite` database with foreign keys enabled.

## Changed Files

- `src/infrastructure/sqlite/repoSyncSchema.ts`: migration SQL, row types, and domain row mappers.
- `src/infrastructure/sqlite/repoSyncSchema.test.ts`: executable SQLite constraint and round-trip
  tests.
- `docs/architecture.md`: documented the repo-sync SQLite schema foundation.
- `docs/task-logs/worker-009-repo-sync-sqlite-schema.md`: recorded this worker result.

## Verification

- `npm run test -- src/infrastructure/sqlite/repoSyncSchema.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None.

## Review Notes

- `node:sqlite` emits Node's experimental feature warning during tests; it is used only for
  executable no-dependency schema verification.
- Future SQLite connections must enable `PRAGMA foreign_keys = ON` before relying on schema
  constraints.
- `worktrees.branch_id` uses `ON DELETE SET NULL` to match repo-sync's non-destructive stale or
  missing-branch behavior.
