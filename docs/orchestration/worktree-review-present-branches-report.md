# Present-branch graph revision

2026-09-09. Implemented in `codex/worktree-review-branch-navigation`, based on `deecb2f7`, preserving the existing dirty branch-navigation work. This report supersedes the previous grid design and its screenshots. No commit, merge, or publication was performed.

## Result

The selector now shows current local branch heads, physical detached worktrees, and only the shared branch points needed to connect them. Historical merge diamonds and the common prefix before the earliest relevant shared point are omitted. In the real repository, 14 labels / 12 distinct heads now use **17 points and 19 connections**, down from **68 points and 91 connections**. The oldest displayed point is `b86a8ac8`; the screenshots' old 302/47/5-commit prefix is gone.

[GitKraken's interface](https://help.gitkraken.com/gitkraken-desktop/interface/) informed the recent-at-top direction, narrow coloured lanes, compact reference labels, and fading of unrelated ancestry. Each branch label contains just its icon and name. Hover or keyboard focus reveals worktree-instance count, changed/new file counts, and commit identity. Shared heads retain separate selectable labels. Thin rounded routes have clearance at crossings so crossings do not imply extra junctions.

The normal sidebar, selection surface, and Build-time checkout prompt retain their existing behavior. Clicking a range opens its exact commit picker. Selecting a branch or commit does not provision a worktree.

## Implementation

`branch_graph.rs` replaces closed-region contraction with current heads plus pairwise best common ancestors, then removes transitive connections between retained points. Every range is the newer endpoint's ancestors minus the older endpoint's ancestors, preserving native order and merged side history. Counts are comparisons between endpoints, not disjoint partitions of the repository. The existing snapshot service keeps immutable ranges valid across paging, ref movement, and graph expansion. Incomplete spans show a plus count and a loaded-history explanation; resolved old common tails do not invite unnecessary expansion.

`branchGraphLayout.ts` now owns vertical rows, reusable lanes, compact control positions, colours, and arrow navigation. `BranchGraph.tsx` renders those positions and local hover/focus details. The pinned state-card column, horizontal timeline, state-attachment routes, and card-height observer were removed. `BranchGraphDialog`, `CommitRangeDialog`, the graph contract, and native history owner received the corresponding narrow changes. No new layout framework, persistence schema, or Git wrapper was introduced.

## Verification

| Check | Current result |
| --- | --- |
| Full frontend suite | 964 tests passed across 169 files; four workers. |
| Final focused frontend checks | 22 passed after the final tooltip/heading refinements. |
| Focused native suite (`--lib worktree_`) | 67 passed, including temporary Git repositories and source/build regressions. |
| Native development build | Passed; launched the rebuilt executable. |
| Final frontend production build | Passed; existing bundle-size warning remains. |
| Source ESLint; final changed-source lint and formatting; targeted rustfmt; `git diff --check` | Passed. |
| Real repository range counts | All 19 displayed counts independently matched `git rev-list --count NEWER ^OLDER`. |

Projection regressions cover obsolete merge diamonds, omitted common prefixes, live/shared heads, multiple best bases in criss-cross history, unresolved boundaries, partial side history, 100 retired merge regions, and retained reachability. Range tests cover merged-side membership, exact Git ancestry differences, deduplication, paging, pinned snapshots, ref movement, source rejection, and expiry. UI regressions cover compact labels, hover/focus without selection or requests, partial-range disclosure, nested modal focus, and checkout creation deferred until Build.

The full native suite was not rerun in this revision. The previous full run's unrelated orchestration MCP response-body timeout remains documented in the [previous report](worktree-review-grid-report.md). Likewise, previous repository-wide lint errors in documentation probe scripts were outside this change; application source lint is clean. No actual review compilation or newly provisioned checkout was launched during the usability checks.

## Native usability evidence

Used the owned app-inspector/CDP endpoint, not Computer Use, CUA, or desktop input. Receipts identify rebuilt app PID `35168`, its exact worktree executable, WebView debugger port `9347`, and Vite URL `http://127.0.0.1:1459/`. Isolated app and review data remain under `.dev/branch-navigation`.

- At 1500 × 880, the graph viewport is 1070 CSS pixels wide, with 1054 pixels of content plus its vertical scrollbar. At 960 × 720, it is 853 pixels wide, with 836 pixels of content. Neither size has horizontal overflow. Labels and counts remain 32 pixels high, with 13-pixel text.
- Hovering the workflow-continuation branch changed highlighted connections from 16 to 8, with identical control rectangles and the selected branch still `main`. Its tooltip showed one worktree instance and `0 changed · 1 new`. The review branch's tooltip showed its separate dirty state.
- Top and bottom screenshots confirm that the graph ends at the earliest relevant shared point. Every fully visible control at both inspected positions passed a centre hit test. Off-screen rows remain available through vertical scrolling and arrow navigation.
- Arrow navigation reveals focused off-screen labels below the sticky heading, keeping hover-equivalent details visible. Left moves to the corresponding count, Enter opens its picker, Shift+Tab wraps inside that picker, Escape returns focus to the count, and Right returns to the branch label. Evidence: `18-key-events.json` and `18-focused-final.png`.
- Opened the 348-commit range and paged from 50 to 100 unique commits. All 100 displayed identities belong to the corresponding Git ancestry difference. Load more retained keyboard focus.
- Chose `ca8f672` in that range. Both modals closed and the normal review surface displayed the exact commit, with focus restored to Select branch. Build then opened the checkout prompt. Cancel preserved the exact selection, identical physical worktree inventory, zero review builds, and zero retained outputs.

Evidence lives in `.dev/branch-navigation/present/`: screenshots, ownership/action receipts, graph metrics, Git-count verification, range membership, Build-cancel before/after facts, and test/build logs. Primary screenshots are `05-compact.png`, `06-lineage.png`, `07-small.png`, `08-small-bottom.png`, `09-range.png`, and `18-focused-final.png`.

The rebuilt app is left open on the revised selector for `codex/worktree-review-branch-navigation`. Final screenshot: `.dev/branch-navigation/present/22-final.png`; final geometry and hit tests: `22-final-metrics.json`. Applying that branch returned to the normal review surface without a checkout prompt.
