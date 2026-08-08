# Research evidence universe

## Primary research checkout

- Path: `C:\Users\user\Documents\Code Projects\Codex Orchestrator Research`
- Branch: `codex/orchestration-engine-research`
- Initial commit: `b28137b66d79121d740267831fc22bf8cdbcbb40`
- Current inspected tip: `924036424969de293da17d0e29c67c34d1ec7c81`
- Initial status: clean

This is the primary place for research artifacts and the initial operational-engine sample. It is not treated as the complete product history.

## Material implementation lines

| Evidence line | Observed tip | Primary contribution | Relationship |
| --- | --- | --- | --- |
| Local `main` | `b86a8ac` | Latest canonical-branch integration | Shared ancestor of later lines |
| Operational/native-profile/MCP | `9240364` | Operational orchestration, native profiles, MCP reporting, shared Agent Session profile binding | Current inspection line; descendant of initial `b28137b` anchor |
| Product Decisions/navigation | `82d9351` | Product Decisions, navigation, inspection UX | Sibling after `e3bde2c` |
| Epic continuation/settlement | `8965191` | Sprint continuation and final Epic settlement | Sibling after `e3bde2c` |

The sibling lines contain substantial, materially different product work. None alone defines everything created since the Orchestration engine began.

## Moving and uncommitted evidence

Registered worktrees include dirty and detached working copies. During initial preparation, the active MCP correction was reconstructed on two semantically parallel lines:

- `dbe321d` is three commits after the initial research anchor and is held by `codex/slice/mcp-reporting-pending-receipt-bridge` plus detached worktrees;
- `385a4db` contains parallel cherry-picked equivalents on a sibling reconstruction held by a detached worktree.

Both end with `test: coordinate MCP selection race workers`. Relative to the initial anchor, the `dbe321d` line changes only `native_profiles.rs`, adding gated selection/probe serialization and its tests. `9240364` descends from `dbe321d` and additionally binds the shared Agent Session application to selected, ready Native Profiles. The research branch was safely fast-forwarded to that clean descendant after it was discovered.

Uncommitted work remains owned by its original worktree. Research must not normalize, stash, or commit it merely to simplify inspection.

See the [Inspection register](inspection-register.md) for exact moving worktrees and committed evidence lines.

## Historical sources

- Git commits, branches, worktrees, and final-tree comparisons
- Product and architecture decision records under `docs/`
- Test fixtures, recorded development compositions, and offline-review evidence
- Conversation and orchestration history when code and commits do not explain intent
- Controlled-live or packaged evidence where available

Historical statements are evidence of intent or observation at a point in time. They are not automatically evidence of present reachability.

## Evidence distinctions

The investigation should normally preserve these distinctions:

- intended
- implemented
- compiled or packaged
- reachable
- configured
- authorized
- persisted
- launch accepted
- runtime or provider observed
- deterministically tested
- reviewed
- integrated
- accepted
- historical or superseded

The list is a guide. A finding may use different distinctions when the subject requires them.
