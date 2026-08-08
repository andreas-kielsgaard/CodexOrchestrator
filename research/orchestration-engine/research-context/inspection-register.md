# Source-state inspection register

This register centralizes where the investigation can inspect materially different source states. It does not merge them or claim they form one validated product.

## Primary inspection line

| Role | Location | Ref or state | Tip |
| --- | --- | --- | --- |
| research source and findings | `C:\Users\user\Documents\Code Projects\Codex Orchestrator Research` | `codex/orchestration-engine-research` plus untracked `research/` | `9240364` |
| canonical local branch | `C:\Users\user\Documents\Code Projects\Codex Orchestrator` | `main`, dirty | `b86a8ac` |

`9240364` is the newest clean descendant discovered on the operational/native-profile/MCP line. The research branch now names it, so the implementation is no longer protected only by detached worktrees. This is inspection centralization, not a decision that it replaces `main` as product integration authority.

## Material committed lines

| Tip | Relationship | Main research value |
| --- | --- | --- |
| `63386e6` | detached descendant of `9240364` through `7fd921a`, `64162ca` and `550ddc5`; source worktree currently dirty | Handler execution-support recovery plus durable failure projection; nearby implementation, not the current stable research snapshot |
| `9240364` | current research tip | operational spine, Native Profiles, MCP corrections, shared Agent Session profile binding |
| `b28137b` | ancestor of `9240364` | initial evidence anchor before MCP race serialization and profile binding |
| `385a4db` | parallel ancestry | reconstructed equivalent of the MCP race correction sequence |
| `82d9351` | sibling after `e3bde2c` | Product Decisions, correction authority, navigation and inspection |
| `8965191` | sibling after `e3bde2c` | Sprint continuation and strict final Epic settlement |
| `5ed3b49` | exploratory sibling | earlier Product Decisions presentation direction |
| `d2e50cd` | exploratory sibling | Worktree Review progress and evidence lineage |
| `ba130cf` | exploratory sibling | File Review application-source lineage |
| `06f04c4` | exploratory sibling | Agent identity and Session layout lineage |

The Git common directory contains the remaining archive, integration, correction and checkpoint refs. [Material implementation lines](../history/implementation-lines.md) and the capability-specific observations identify them when they explain behavior or intent.

## Uncommitted source worktrees

Snapshot taken 2026-08-07. These states remain owned by their source worktrees.

| Worktree | HEAD | Tracked changes | Untracked entries | Apparent evidence cluster |
| --- | --- | ---: | ---: | --- |
| `C:\Users\user\Documents\Code Projects\Codex Orchestrator` | `b86a8ac` | 67 | 37 | large skill-catalogue migration plus a small Plan Builder Harness/configuration correction |
| `C:\Users\user\.codex\worktrees\1919\Codex Orchestrator` | `249c899` | 2 | 1 | predecessor Codex-home profile persistence experiment |
| `C:\Users\user\.codex\worktrees\430c\Codex Orchestrator` | `b86a8ac` | 12 | 0 | developer/test-toolchain experiment on a stale base; partly parallel to committed acceleration tooling |
| `C:\Users\user\.codex\worktrees\af29\Codex Orchestrator` | `63386e6` | 1 | 0 | Worktree Review catalogue-identity resilience draft atop the Handler recovery descendant |
| `C:\Users\user\.codex\worktrees\demo-operational-spine-55cdd40` | `55cdd40` | 1 | 1 | normal-entry deterministic Work Unit checkpoint demonstration |
| `C:\Users\user\.codex\worktrees\human-review-integration-019fcb70\Codex Orchestrator` | `55cdd40` | 2 | 4 | development-only integration-settlement review Harness and retained proof residue |
| `C:\Users\user\.codex\worktrees\operational-spine-ps-r1\Codex Orchestrator` | `b964509` | 5 | 0 | early Work Slice Planner prelaunch precursor, superseded by committed descendants |

Dedicated observation passes cover the materially relevant diffs. The counts are orientation aids, not implementation-size measures:

- [Uncommitted Work Slice Planner transition](../evidence-passes/uncommitted-operational-transition-pass.md)
- [Uncommitted runtime toolchain](../evidence-passes/uncommitted-runtime-toolchain-pass.md)
- [Uncommitted presentation and Harness evidence](../evidence-passes/uncommitted-presentation-and-harness-pass.md)

## Clean registered evidence

The other registered worktrees are clean snapshots. Several duplicate these useful checkpoints:

- `9240364`: detached worktrees `1ff1` and `68dc`, in addition to the research branch;
- `dbe321d`: branch worktree `376c` and detached worktrees `9a49` and `a991`;
- `51d69e5`: branch worktree `cc50` and detached worktrees `3170`, `9510`, and `bc3e`;
- `b86a8ac`: detached worktrees `03b1`, `1b0b`, `1e37`, and `b3cc` beside the dirty `main` worktree;
- `6588275`: the `pip02-ui` root and one execution workspace;
- `5ed3b49`: the exploratory Product Decisions worktree and its review copy.

Other clean execution workspaces pin intermediate operational commits. They are useful for exact historical reads but do not each need to become a top-level research line.

The final refresh also found a newly registered clean execution workspace under `2607` at `9240364`, increasing the shared worktree count from 41 to 42 without adding another dirty source state.

A read-only recheck after the current evidence pass first found `af29` at partial commit `7fd921a` with an uncommitted test expansion. It subsequently settled through proof commit `64162ca`, cross-worktree recovery commit `550ddc5` and failure-projection commit `63386e6`. During final validation its uncommitted work changed again to a Worktree Review catalogue-identity resilience draft. The sequence is registered here rather than copied or merged into the stable research snapshot.

## Preservation rule

- Do not reset, stash, clean, commit, merge or switch any source worktree for research convenience.
- Use the research checkout for documentation and the shared object store for committed comparisons.
- Treat uncommitted diffs as moving evidence with an exact path and HEAD.
- Prefer a behavioral observation over copying an entire working tree.
- Re-run the register before any later integration or disposition phase.
