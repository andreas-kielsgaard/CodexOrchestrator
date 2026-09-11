# Present-branch graph revision

2026-09-09. Authorized to plan and implement in the existing branch-navigation worktree. This supersedes the conservative projection and large state cards in the previous grid plan.

## Design target

Show the relationships between current local branch heads and instantiated detached worktrees. Historical forks matter only when they distinguish those current states. Remove the old shared history before their earliest relevant branch point. Preserve the sidebar, exact commit selection, and Build-time checkout prompt.

Use GitKraken's recent-at-top graph direction, narrow coloured lanes, small reference labels, and fading of unrelated ancestry as visual references: [official interface guide](https://help.gitkraken.com/gitkraken-desktop/interface/). This view remains an overview of current states, with whole commit ranges collapsed between them.

Each label contains a branch/worktree icon and its name. Worktree count, changed/new file counts, and commit identity appear in a hover/focus tooltip. Remove the pinned state-card column, permanently displayed metadata, and card-height measurement. Keep normal-size text and 32-pixel controls. Range badges share their connection's colour and open the existing commit picker. Shared heads retain individually selectable labels.

## Projection

Replace the closed-region heuristic in `branch_graph.rs` with a graph of current heads and their best common ancestors. A non-head point survives only when it is a nearest common ancestor of a pair of distinct current heads. This removes the screenshots' historical diamonds even when they have extra connections into other old history.

Compute target reachability over the observed DAG once, find those shared branch points, then connect each retained point to its nearest retained ancestors. Remove transitive connections. This preserves reachability between current heads without reconstructing extinct branch lanes or guessing branch creation names.

Every displayed connection's members are the commits reachable from its newer endpoint that are not reachable from its older endpoint, including merged side history. Deduplicate and retain their native order. Keep the existing snapshot/range cache, source validation, and exact paging. An incomplete loaded span explicitly shows a plus count; its picker identifies the commits as loaded history. Preserve necessary unloaded boundaries until earlier relationships can be resolved. Never expand the old common tail merely because more history was fetched.

A read-only prototype over this repository's 836 commits and 14 labels reduced the graph from 68 points/91 connections to 17 points/19 connections. Its oldest retained point was `b86a8ac8`; the older 302/47/5-commit stretches from the screenshots disappear. Final runtime figures must be verified after implementation.

## Ownership and implementation

- Replace projection in `src-tauri/src/worktree_review/branch_graph.rs`; retain its pure ownership. Adapt `branch_history.rs` only to expose incomplete-range metadata and meaningful earlier-history availability.
- Update the typed graph contract in `src/application/worktreeReview/contracts.ts`. Retain existing transport commands and snapshot paging.
- Replace horizontal tracks and state attachments in `branchGraphLayout.ts` with compact vertical rows and reusable coloured lanes. Return all control positions and arrow-key neighbors from this owner. Rows represent retained states or additional range/alias controls, never individual historical commits.
- Adapt `BranchGraph.tsx` and its CSS to render thin routes, compact icon/name labels, range badges, and hover/focus details. Remove card measurement, sticky-column geometry, and large card styling.
- Narrowly adapt `BranchGraphDialog.tsx` and `CommitRangeDialog.tsx` for revised copy and incomplete spans. Retain shared modal/focus behavior, selection ownership, build drafts, and source materialization.
- Revise projector/geometry fixtures and add regressions for the screenshots' obsolete diamonds, omitted shared prefixes, retained live forks, shared heads, and incomplete history. Preserve range paging and Build-cancel tests.

## Acceptance

Verify that every non-head loaded point is relevant to a pair of current heads and that reachability is preserved. Check counts against Git ancestry differences, with pinned paging still stable after ref movement/expansion. Verify that hidden old history cannot reappear as extra lanes.

Inspect the real native app through its owned CDP endpoint at 1500 × 880 and 960 × 720, without Computer Use. Confirm a compact coloured graph with no horizontal scrolling at those sizes, small labels, readable hover/focus metadata, and local lineage preview without layout changes. Exercise counts, exact commit application, nested cancellation, and Build cancellation. Run focused native/frontend tests, production build, source lint, and format checks. The previously recorded unrelated full-native MCP timeout and documentation-script lint failures remain separate from this revision.

## Completion

Implemented and validated on 2026-09-09. See [implementation and usability evidence](worktree-review-present-branches-report.md).
