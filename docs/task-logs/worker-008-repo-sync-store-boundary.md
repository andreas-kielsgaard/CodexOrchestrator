# Worker 008 Repo Sync Store Boundary

Date: 2026-07-02

## Summary

Added a narrow pure TypeScript repo sync persistence boundary that composes through
`syncRepoFromScan`. This slice does not add SQLite migrations, runtime database dependencies,
Tauri/Rust commands, Git command execution, Codex runtime integration, or React/UI work.

## Behavior

- `RepoSyncStore` loads the current domain snapshot needed by repo sync and persists only the
  applied repo, branch, and worktree records.
- `syncRepoFromScanWithStore` loads records from the store, calls the existing
  `syncRepoFromScan` facade, persists the applied repo sync state, and returns the same plan and
  applied result for callers that need review/reporting details.
- Store load keys use the same normalized domain path format as repo sync planning, so
  Windows-style scan root paths are passed to persistence with forward slashes.
- `InMemoryRepoSyncStore` provides a dependency-free implementation for tests and future slice
  scaffolding.
- Unrelated domain arrays are preserved by the in-memory store when repo sync records are
  persisted.
- Existing repo/branch/worktree records are loaded and updated from scan facts through the
  established plan/apply path.
- Worktree lock and branch links are explicitly cleared when scan facts omit them.
- Stale worktrees remain non-destructive reports and are not deleted or mutated.
- Missing default branch facts do not synthesize `main`.

## Changed Files

- `src/domain/repoSyncStore.ts`: added the store boundary, store-backed use case, and in-memory
  implementation.
- `src/domain/repoSyncStore.test.ts`: covered normalized store load paths, loading existing
  records, persisted repo sync state, unrelated record preservation, explicit worktree clears,
  stale worktree reporting, and missing default branch behavior.
- `docs/architecture.md`: documented the new repo sync store boundary.
- `docs/task-logs/worker-008-repo-sync-store-boundary.md`: recorded this worker result.

## Verification

- `npm run test -- src/domain/repoSyncStore.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None.

## Review Notes

- The store boundary is intentionally async-capable so a future SQLite implementation can satisfy
  the same interface without changing the domain use case.
- The persistence input is intentionally limited to repo sync records. The store-backed use case
  still loads full `DomainRecords` because the existing pure planning/apply facade accepts that
  shape and preserves unrelated domain state.
