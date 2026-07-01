# Worker 007 Repo Sync Service

Date: 2026-07-02

## Summary

Added a pure TypeScript repo sync service facade that composes Worker 005's `planRepoSync` with
Worker 006's `applyRepoSyncPlan`. The facade keeps scan reconciliation synchronous and
persistence-neutral while returning both the generated plan and the applied in-memory result.

## Behavior

- Accepts `DomainRecords`, a target `projectId`, `GitRepoScanDomainFacts`, `plannedAt`, and a
  deterministic repo sync ID provider.
- Generates a `RepoSyncPlan` through the planning layer, then applies it to in-memory domain records
  through the applier layer.
- Returns both `plan` and `applied` so future persistence/UI callers can inspect or persist the plan
  separately from the materialized record result.
- Preserves existing plan/apply semantics: no invented `main`, explicit `null` clears for worktree
  lock and branch links, non-destructive missing-worktree reporting, and app-owned branch fields
  preserved by the underlying layers.

## Changed Files

- `src/domain/repoSyncService.ts`: added the pure sync facade and result/input types.
- `src/domain/repoSyncService.test.ts`: covered inserted records, existing-record updates,
  explicit worktree clears, stale worktree reporting, missing default branch behavior, and returning
  the plan alongside applied records.
- `docs/architecture.md`: documented the facade as part of the persistence-neutral repo sync
  boundary.
- `docs/task-logs/worker-007-repo-sync-service.md`: recorded this worker result.

## Verification

- `npm run test -- src/domain/repoSyncService.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

`npm run build:tauri` was not run; this slice is pure TypeScript and Rust/Cargo verification was
out of scope.

## Blockers

None.
