# Worker 004 Git Scan Mapping

Date: 2026-07-01

## Summary

Added pure TypeScript support for composing a Git repo scan from raw parser-boundary outputs and mapping scan results into domain-facing repo, branch, and worktree facts. This slice does not execute Git commands and does not add Tauri/Rust, SQLite, Codex, or UI integration.

## Changed Files

- `src/infrastructure/git/parsers.ts`: added `git remote -v` parsing into `GitRemoteSummary` records.
- `src/infrastructure/git/gitAdapter.ts`: added scan-result assembly and domain-facing mapping helpers.
- `src/infrastructure/git/types.ts`: added normalized domain-facing Git fact types.
- `src/infrastructure/git/parsers.test.ts`: added remote parsing fixtures for multiple remotes and fetch/push pairs.
- `src/infrastructure/git/gitAdapter.test.ts`: added scan assembly and mapping fixtures with missing upstreams, detached/prunable worktrees, and Windows paths.
- `docs/architecture.md`: noted the pure scan assembly and domain-facing mapping boundary.

## Verification

- `npm run test -- src/infrastructure/git` -> pass
- `npm run lint` -> pass
- `npm run format:check` -> pass
- `npm run test` -> pass
- `npm run build` -> pass

## Blockers

None.

## Review Notes

- Non-root worktree dirtiness is exposed as `dirtyState: "unknown"` because this scan only includes a status snapshot for the scanned root path.
- Domain-facing facts intentionally avoid persistence identifiers; repository sync can add IDs and merge behavior later.
- The scan builder accepts raw outputs and does not run Git.

## Orchestrator Review Addendum

The orchestrator accepted `dirtyState: "unknown"` for non-root worktrees because this scan only includes status output for the scanned root path.

The orchestrator changed the default-branch mapping before merge: domain-facing scan facts no longer invent `main` when neither an explicit default branch nor a current branch is known. `GitRepoDomainFacts.defaultBranch` is optional so persistence can decide how to handle unknown defaults later.
