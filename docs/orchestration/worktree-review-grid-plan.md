# Worktree Review grid refinement: implementation shape

Status: implemented; validation results are recorded in [worktree-review-grid-report.md](worktree-review-grid-report.md). Inspected the dirty `codex/worktree-review-branch-navigation` implementation on 2026-09-09. This follows the branch-navigation implementation and its regression/usability report.

## Target

Make the branch modal a compact, readable grid for comparing current branches and worktrees. Give target cards stable rows and an aligned current-state column. Put historical junctions and clickable commit counts in reserved cells, joined by horizontal and vertical connections with small rounded corners. Columns express ancestry relationships, not elapsed time.

Keep the agreed sidebar ordering, eligible targets, hover/focus lineage preview, normal detail surface, exact commit selection, and Build-time checkout prompt. Existing source materialization, build execution, retention, and storage ownership carry forward.

The design assumption is that historical forks and merges may be folded into a count when that preserves relationships between displayed targets. Every eligible target stays represented. A folded section still opens its actual commits; it is not presented as a single commit or a linear Git path.

## What needs to change

The current layout already assigns regular coordinates, but it allocates a new lane for historical secondary parents without reclaiming lanes. Every retained historical junction increases the horizontal rank. Curves, count positions, and branch-label offsets are then calculated separately. `graphRangePositions` tries candidate points and eventually allows overlap. Global SVG scaling also shrinks controls and labels.

Native `branch_history.rs::compress` keeps every fork and merge, including retired internal paths that no longer distinguish displayed targets. Its `HistoryScope::Segment` describes one parent chain, verified again from Git on each page. A collapsed section containing merges cannot use that contract honestly.

`BranchFirstReviewService` currently owns an eight-entry cache of pinned target lists, while graph projection and history paging live in `branch_history.rs`. Move snapshot and range ownership together rather than adding a second cache to the frontend or screen.

A read-only feasibility pass over the current repository found 14 targets, 12 distinct tips, 836 loaded commits, and 144 existing anchors. Protecting target/boundary/signature-change junctions and folding closed regions identified 95 removable internal anchors, leaving approximately 49. This is an algorithm experiment, not a finished layout or a performance/visual acceptance result. Repository history can change before implementation.

## Native ownership and exact ranges

Create `src-tauri/src/worktree_review/branch_graph.rs` for pure graph projection. It consumes observed commit parents and pinned target heads and returns retained anchors, connections, target attachments, and ordered range-member definitions. It performs no Git, database, or drawing operations.

Use a conservative projection first:

1. Propagate the set of displayed target heads that can reach each loaded commit. Walk the existing multi-head DAG once; do not issue Git comparisons for every pair of branches.
2. Protect target heads, roots, unloaded boundaries, and junctions where those reachability sets change.
3. Fold an internal region only when it has one older boundary, one newer boundary, no interior target or unloaded boundary, and no external connections through its interior. Identical reachability sets provide a useful grouping criterion; the boundary checks establish whether contraction is safe.
4. Keep complex regions explicit when those conditions do not hold. Preserve separate components for unrelated roots, distinct targets sharing one commit, and distinct paths where they remain relevant.
5. Retain a range's exact ordered, deduplicated commit IDs. Include its newer endpoint and exclude its older endpoint. Derive its displayed count and pages from that same list. Counts on different connections may overlap at merge endpoints; they are not additive repository totals.

The projected graph must preserve reachability between retained anchors and displayed targets. This also preserves the ancestor/downstream-integration meaning used by lineage highlighting. Do not infer a branch's creation parent or replace Git ancestry with a guessed tree of branch names.

Adapt `branch_history.rs` into the application-owned `BranchHistoryService`: pinned graph snapshots, range lookup, cursor validation, and commit paging. It continues to use `RepositoryContext::commits()` for multi-head reads and batched commit facts, and delegates the pure projection to `branch_graph.rs`. Move the existing snapshot cache out of `branch_first.rs`; that service resolves the repository/inventory and delegates graph/history reads.

Replace public `HistoryScope::Segment` with `GraphRange { snapshotId, rangeId }`. Keep `Ancestry` for the existing build and association-baseline history pickers. Graph connections return their endpoint metadata, count, and eligible source-target identities separately from this opaque scope. The commit modal chooses the selected source when eligible, otherwise a stable eligible source; it never infers provenance from the nearest visual row.

Both linear connections and collapsed sections use the same native-owned graph-range mechanism. Validate repository, source identity, and snapshot/range membership when resolving it. Preserve the existing eight-snapshot and 20,000-commit bounds. Loading earlier history retains pinned heads and previously issued range memberships; new range IDs must not retarget old pages. An expired range produces a clear reopen/refresh action. It never silently changes to current refs. Commit selection continues to return an exact object ID independent of the range cache.

This replaces the caller-supplied member array and its repeated full-history validation. No new database schema, persistent graph store, Git runner, or build lifecycle is needed. Keep existing transport command names and migrate the graph DTO, adapter, fixtures, and tests together; no legacy graph mode or compatibility adapter.

## Grid layout and rendering

Replace `branchGraphLayout.ts` in place with one pure layout function returning a `BranchGraphLayout`: grid tracks, anchor positions, range/card rectangles, routed connection polylines, target attachments, and deterministic keyboard-neighbor identities. Keep sizing constants here so SVG geometry and HTML controls share one coordinate model.

- Give each branch target a stable display row; detached worktree targets remain independently selectable. Targets sharing a tip attach to one real commit anchor but retain separate cards. A branch with several associated worktrees keeps its existing grouped state summary.
- Orient the graph around the repository's default branch when available through the current reference reader. Pass its target identity as presentation metadata; otherwise use a deterministic fallback. Place related target families together with stable identity/name tie-breaks. Activity, selection, and hover do not change row order.
- Keep current-state cards aligned at the right, with branch name, commit, worktree count, and change summary. Use a shared vertical scroller for cards and history, so rows cannot drift apart. Keep the state column visible while history scrolls horizontally. Its attachment lines identify refs at their real tip; they do not count as additional history or imply equal commit dates.
- Allocate historical lanes by occupied intervals and release them after paths end. Compact unused tracks. Route forks/merges through reserved gutters. Reserve count cells before routing connections; a crowded route receives another gutter or cell, never an overlapping fallback.
- Draw dots only at real retained commit junctions. Use a visual break at a crossing that is not a junction. Ordinary state-card attachments are visually distinct from ancestry connections.
- Use roughly 64–80 px target rows, 120–160 px history cells, and a 280–320 px state column as starting dimensions, then tune with the real repository. Keep text at normal product sizes and interactive controls at least 32 px high. Long target names can wrap within the card; the layout receives the resulting row height.

Adapt `BranchGraph.tsx` to render that plan: a decorative SVG layer for routed connections/markers, and HTML buttons for ranges/cards in their assigned cells. Paths cannot intercept input. Remove Bézier calculations, `foreignObject` controls, branch-label offset arithmetic, `graphRangePositions`, and global SVG zoom scaling. These are replaced rather than retained as fallback behavior.

Keep `BranchGraphDialog.tsx` as the owner of loading, temporary selection/preview, range opening, and apply/cancel. Replace the current zoom percentage controls with the readable compact view and the existing Center selection action. Scroll or reveal overflow instead of shrinking labels. A viewport resize may change track sizing; hover and lazy activity must not. Earlier-history loading preserves the viewed target and an anchored scroll position.

Retain `branchLineage.ts` as a pure highlight calculation over the projected relationships. Geometry consumes highlight sets but does not discover ancestry, invoke Git, or choose source identities.

## Keyboard and focus

Give the graph a single entry in the Tab order, restoring its last focused control or the selected target. Arrow keys follow the layout's deterministic neighboring controls; Enter/Space selects a target or opens a count. Tab leaves the graph for the dialog footer rather than traversing hundreds of historical controls. Move focus without applying selection, and reveal focused items without recentering unrelated content. Announce enough context to distinguish counts with the same number.

Keep this behavior local to `BranchGraph.tsx` and its pure layout/navigation data. Do not introduce a general navigation framework. Use `ReviewDialog.tsx` for the tested modal boundary behavior: initial focus, topmost-only Escape, forward/reverse Tab containment, and focus return to the count/trigger. Verify the WebView behavior before adding a focused boundary handler; do not make a second independent trap in each modal.

Keep refresh focus restoration in `WorktreeReviewScreen.tsx`, alongside the button that initiates refresh. Restore it only when the initiating control lost focus because it became disabled, the same screen remains mounted, and the user has not moved focus elsewhere. Preserve the current selection and draft ownership in `useReviewSelection.ts` and `buildDraft.ts`.

## File changes

| Action | File or owner | Result |
| --- | --- | --- |
| Create | `src-tauri/src/worktree_review/branch_graph.rs` | Pure compact projection and member definitions; extract graph DTOs/projection from history paging. |
| Adapt | `src-tauri/src/worktree_review/branch_history.rs` | One history service owns pinned snapshots, graph ranges, and shared paging. Remove `compress` and the public linear-segment request path. |
| Simplify | `src-tauri/src/worktree_review/branch_first.rs` | Delegate to history service; remove the private snapshot deque and graph-cache management. |
| Adapt | `src-tauri/src/worktree_review/{mod,transport}.rs` | Compose/import the focused graph/history owners through existing commands. |
| Adapt | `src/application/worktreeReview/{contracts,client}.ts`, `src/infrastructure/tauriWorktreeReview.ts` | Typed graph/range contract, reference-target metadata, unchanged application operations. |
| Replace in place | `src/features/worktreeReview/branchSelection/branchGraphLayout.ts` | Deterministic cells, compact lanes, reserved controls, routes, and navigation neighbors. Delete candidate-position and curve-bend heuristics. |
| Adapt | `branchSelection/BranchGraph.tsx`, `branchSelection/branchSelection.css` | HTML controls, decorative SVG routes, aligned state column, scrolling, and roving focus. Delete SVG-scale zoom and duplicate geometry. |
| Adapt | `branchSelection/{BranchGraphDialog,CommitRangeDialog}.tsx` | Existing modal flow consumes native range identity and endpoint metadata. Remove segment-shape assumptions and obsolete zoom state. |
| Retain/adapt narrowly | `branchSelection/branchLineage.ts`, `useCommitHistory.ts` | Existing local highlighting and shared history-loading behavior consume the revised contracts. |
| Adapt narrowly | `ReviewDialog.tsx`, `WorktreeReviewScreen.tsx` | Verified modal boundary behavior and refresh focus restoration. |
| Update | `navigation_tests.rs`, `branchGraph.test.ts`, `WorktreeReviewScreen.test.tsx`, fixtures, adapter tests | Preserve current regressions and add projection, range, grid, and keyboard coverage. |

Paths in the final five rows are within `src/features/worktreeReview/` unless otherwise stated. Keep the grid algorithm together until its actual size warrants extraction; do not create one file per layout phase in advance.

## Delivery and acceptance

1. **Projection and exact-range foundation.** Extract the pure projector and consolidate history/snapshot ownership. Prove contraction preserves reachability and range membership before changing the rendered graph. Migrate all graph-range consumers while retaining ancestry-picker behavior.
2. **Grid replacement.** Implement lanes, track allocation, reserved buttons, right-angle routes, and the aligned state column. Remove the old placement and scaling path. Check representative fixture geometry and the real repository at normal text size.
3. **Interaction completion.** Carry over target/commit selection and local preview, add graph arrow navigation, fix modal boundaries and refresh focus, and preserve scroll through history expansion.
4. **Regression and usability verification.** Run focused frontend/native suites, build, lint/format, then full suites with bounded concurrency. Complete the real app flow through app-inspector/CDP; honor the user's exclusion of Computer Use. Record screenshots and limits separately from automated results.

Git fixtures must cover a long series of retired merge paths, surviving side branches, nested/parallel merges, shared tips, unrelated roots, detached worktrees, and truncated history. Verify every displayed range's count equals its paged unique members; tip/base inclusion is exact; retained-target reachability and sibling-only highlighting survive contraction; moving refs and loading earlier history do not retarget an open range. Failed or expired range reads must not change the selected source.

Geometry tests assert non-overlapping interactive rectangles, aligned tracks/cards, routes clear of control interiors, meaningful junction/crossing distinctions, lane reuse, and unchanged positions after hover/activity updates. Keep the previously discovered merge-marker and branch-card collision cases as regressions against the new layout rather than deleting their intent.

On the real repository at 1500 × 880 and 960 × 720 CSS viewports, the default view must offer readable current-state cards without 40% scaling. Vertical scrolling through fourteen target rows is expected; thousands of empty historical pixels are not. The state column stays available while examining history. Review default framing, long names, forks/merges, count clicks, detached targets, keyboard focus, nested cancellation, and refresh. Recheck that selection creates nothing and only Build opens the required-checkout prompt.

This slice adds no branch editing, graph persistence, drag positioning, search/filter mode, minimap, alternative layout engine, or new build/source semantics. It does not resolve the separately documented fresh-profile schema or packaged-build validation gaps.
