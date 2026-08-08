# Research checkout and repository topology

## Research checkout

| Fact | Value |
| --- | --- |
| Path | `C:\Users\user\Documents\Code Projects\Codex Orchestrator Research` |
| Branch | `codex/orchestration-engine-research` |
| Initial anchor | `b28137b66d79121d740267831fc22bf8cdbcbb40` |
| Current inspected tip | `924036424969de293da17d0e29c67c34d1ec7c81` |
| Shared Git common directory | original `Codex Orchestrator/.git` object and ref store |
| Product-code mutations | none |
| Research state | untracked `research/` tree only |

The checkout is isolated for documentation while retaining read access to every local branch, commit and registered worktree through the shared Git store. It began at `b28137b` and was fast-forwarded to `9240364` after the evidence pass discovered that clean descendant in detached worktrees. The earlier observation passes retain their original snapshot labels.

## Why no aggregate merge was created

The repository does not currently contain one tip that includes every material capability. The latest operational, Product Decisions and final-settlement histories are sibling lines after a shared point. A synthetic merge before understanding them could:

- collapse alternatives and cumulative work without knowing which is which;
- create conflict resolutions that never existed or were validated;
- disturb active/dirty worktrees;
- make the research tree look more authoritative than the underlying evidence.

Centralization for inspection therefore means one stable checkout plus explicit access to all refs and worktrees, not prematurely combining their source trees.

## Topology snapshot

Read-only refresh on 2026-08-07 found:

- 97 local branch names;
- 72 local branches not merged into local `main`;
- 503 distinct commits reachable from all refs but not from `main`;
- 42 registered worktrees at the final refresh, including a new clean execution workspace under `2607` at `9240364`;
- eight dirty worktrees at the 2026-08-07 20:50 +02:00 refresh, including this research checkout and an active `af29` Worktree Review catalogue draft.

The counts are a moving operational snapshot, not a stable product metric. They establish that "current branch" and "everything created" are materially different evidence sets.

## Material lines

| Line | Tip used | Relationship to current inspected tip |
| --- | --- | --- |
| nearby Handler recovery correction | `63386e6` through `7fd921a`, `64162ca` and `550ddc5`, plus a current `af29` Worktree Review catalogue diff | direct descendant sequence; appeared after the stable evidence passes and remains outside the research branch |
| operational/native-profile/MCP inspection line | `9240364` | checked out; includes `b28137b`, the `dbe321d` race correction and global selected-profile binding |
| initial operational anchor | `b28137b` | ancestor retained as the snapshot for several observation passes |
| parallel MCP reconstruction | `385a4db` | equivalent correction sequence on different ancestry; not an ancestor |
| Product Decisions/navigation | `82d9351` | sibling from `e3bde2c` |
| Sprint continuation/final settlement | `8965191` | sibling from `e3bde2c` |
| local `main` | `b86a8ac` at preparation | older canonical integration line |

The exact moving-state inventory is maintained in [Inspection register](inspection-register.md). Uncommitted source is catalogued where it lives rather than copied into this checkout.

## Inspection rules used

- never reset, stash, switch, clean or merge another worktree;
- inspect branch-local files with Git object reads or from this checkout;
- record moving tips and snapshot labels whenever the research branch advances;
- classify branch-only functionality explicitly;
- treat uncommitted state as owned by its source worktree;
- avoid calling a branch "current product truth" merely because it is newest or named `main`.

## What remains centralized

The research repository centralizes:

- capability and artifact catalogues;
- branch and commit lineage;
- baseline versus sibling-line classifications;
- representative traces;
- unresolved integration questions;
- role-oriented interpretations.

It does not centralize product source by modifying or merging it. That remains a later architecture/integration decision.
