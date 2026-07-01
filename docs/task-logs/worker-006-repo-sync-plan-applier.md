# Worker 006 Repo Sync Plan Applier

Date: 2026-07-02

## Summary

Added a pure TypeScript repo sync plan applier that applies Worker 005's persistence-neutral
`RepoSyncPlan` objects to in-memory `DomainRecords`. The applier resolves planned repo, branch,
and worktree references through an injected deterministic ID provider and does not add SQLite,
runtime integration, Git command execution, Tauri/Rust commands, or UI code.

## Behavior

- Repo, branch, and worktree insert plans create in-memory domain records with generated IDs.
- Update plans replace only repo/branch/worktree records and preserve unrelated domain record
  arrays.
- Planned worktrees can link to newly planned branches through resolved `branchId` values.
- Existing repo `defaultBranch` and `remoteUrl` are preserved when omitted by the plan.
- Missing default branch facts remain missing; the applier does not invent `main`.
- `lockReason: null` removes an existing worktree lock reason.
- `branchRef: null` removes an existing worktree `branchId`.
- Stale worktree plans are reported non-destructively as `reported_missing_from_scan` entries;
  existing worktree records are not deleted or mutated for stale handling.

## Changed Files

- `src/domain/model.ts`: made `Repo.defaultBranch` optional to match unknown default branch scan
  facts and plan inserts.
- `src/domain/repoSyncPlanApplier.ts`: added the pure applier, ID provider contract, and apply
  report types.
- `src/domain/repoSyncPlanApplier.test.ts`: covered inserts, updates, explicit clears, stale
  reporting, unrelated record preservation, planned ref resolution, and missing default branch
  behavior.
- `docs/architecture.md`: noted the persistence-neutral planning/applier boundary.

## Verification

- `npm run test -- src/domain/repoSyncPlanApplier.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

`npm run build:tauri` was not run; this slice is pure TypeScript and Rust/Cargo verification was
out of scope.

## Blockers

None.
