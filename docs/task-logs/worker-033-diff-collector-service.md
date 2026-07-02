# Worker 033 - Diff Collector Service Boundary

Date: 2026-07-02

## Summary

Added a focused application-layer diff collection boundary in `src/application/diffCollection.ts`.
The service validates the task, optional task run, and worktree path through
`OpenTaskDashboardStore` records, calls an injected `GitDiffProvider`, stores the collected output
as a `diff` artifact, and appends an `artifact_created` event with compact diff metadata.

## Decisions

- Kept Git execution behind a tiny injected `GitDiffProvider`; no concrete Git command runner or UI
  wiring was added in this slice.
- Preserved empty diffs by storing `content: ''` on the artifact and recording
  `isEmptyDiff: true` plus `diffLength: 0` in the event payload.
- Resolved worktree paths from explicit input first, then from the task run worktree link, then from
  the task worktree link.
- Allowed explicit worktree paths that are not in the loaded records; matching records are included
  only when a normalized path match is available.
- Let provider failures propagate without creating an artifact or event, keeping failed collection
  distinct from a clean empty diff.

## Changed Files

- `src/application/diffCollection.ts`
- `src/application/diffCollection.test.ts`
- `docs/architecture.md`
- `docs/task-logs/worker-033-diff-collector-service.md`

## Verification

- `npm ci` - passed, installed lockfile dependencies in the worker worktree.
- `npm run test -- src/application/diffCollection.test.ts` - passed, 4 tests.
- `npm run lint` - passed.
- `npm run format:check` - passed.
- `npm run test` - passed, 39 test files / 233 tests.
- `npm run build` - passed.
- `git diff --check main...worker/033-diff-collector-service` - passed.

`npm run build:tauri` was not run per worker instructions; docs still note the known Rust/Cargo
environment blocker.

## Blockers

None.

## Review Notes

- Orchestrator review correction: added focused coverage for rejecting a `taskRunId` that exists
  but belongs to a different task before diff provider execution.
- The event payload intentionally stays compact: artifact kind/id, diff length, empty flag,
  normalized worktree path, and worktree id when known.
- This service does not mutate task/run lifecycle state; callers should invoke it after an
  appropriate run boundary.
- Runtime wiring still needs a concrete Git diff provider.

## Final Git Status

Expected after committing this slice:

```text
## worker/033-diff-collector-service
```
