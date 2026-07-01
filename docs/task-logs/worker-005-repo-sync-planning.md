# Worker 005 Repo Sync Planning

Date: 2026-07-02

## Summary

Added a pure TypeScript repo sync planning layer that maps existing `DomainRecords` plus
`GitRepoScanDomainFacts` into persistence-neutral upsert plans for repos, branches, and worktrees.
This slice does not execute Git, add SQLite, add Tauri/Rust commands, integrate Codex runtime, or
touch React UI.

## Correction Note

The first implementation pass was accidentally created as untracked files in the main checkout at
`C:\Users\user\Documents\Code Projects\Codex Orchestrator`. After orchestration correction, the
implementation was recreated in the assigned worker worktree
`C:\Users\user\.codex\worktrees\e56f\Codex Orchestrator` on branch
`worker/005-repo-sync-planning`. The accidental untracked implementation files were removed from
the main checkout and were not committed there.

## Review Correction

Updated worktree upsert plans to make optional field clearing explicit for future persistence
appliers. `WorktreeUpsertPlan.values.lockReason` is now `string | null`, and
`WorktreeUpsertPlan.values.branchRef` plus the top-level plan `branchRef` are now
`BranchPlanRef | null`. Current Git scan facts therefore distinguish "set this value" from
"clear the existing value" when a worktree becomes unlocked or detached.

## Changed Files

- `src/domain/repoSyncPlanning.ts`: added persistence-neutral planning types and `planRepoSync`.
- `src/domain/repoSyncPlanning.test.ts`: covered new repo discovery, existing Git-owned updates,
  branch intent/base preservation, worktree branch linking, stale worktree marking, and unknown
  default branch behavior.

## Behavior

- New repo scans plan a repo insert plus branch and worktree inserts using planned refs.
- Existing repo scans plan updates for Git-owned fields such as default branch, remote URL, branch
  head SHA, worktree dirty state, lock reason, and scan time.
- Existing branch `intent` and `baseBranch` are preserved from domain records.
- Worktrees link to existing or planned branch refs by branch name when available.
- Worktree plans explicitly clear stale `lockReason` and branch association with `null` values
  when current scan facts no longer provide those optional fields.
- Domain worktrees absent from the current scan are represented with the non-destructive
  `mark_missing_from_scan` action.
- Missing default branch facts do not synthesize `main`; existing repo defaults are preserved on
  update, and new repo plans leave `defaultBranch` unset.

## Verification

- `npm run test -- src/domain/repoSyncPlanning.test.ts` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

`npm run build:tauri` was not run; this slice is pure TypeScript and Rust/Cargo verification was
out of scope.

## Blockers

None.

## Review Notes

- Plan refs intentionally carry either existing IDs or natural keys for planned records so a future
  persistence layer can resolve IDs while applying the plan.
- Stale worktree handling is deliberately non-destructive and does not mutate the existing
  worktree model because no durable missing/stale field exists yet.
