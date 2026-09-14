# Worktree Review branch navigation: implementation shape

Status: implemented on `codex/worktree-review-branch-navigation`; automated validation and the native usability follow-up are recorded below. The implemented grid refinement is described in [worktree-review-grid-plan.md](<C:/Users/user/.codex/worktrees/d02e/Codex Orchestrator/docs/orchestration/worktree-review-grid-plan.md>). Baseline inspected: `deecb2f7bb0c2678c8fbcef0e1184bf43e56d5f0` on 2026-09-09, with a clean working tree before this document.

## Functional target and resolved decisions

Preserve the current Worktree Review page. Order its list by available-worktree presence first, then recent activity descending, then name. Add **Select branch…** above the list. It opens a broad repository graph, and a graph range such as **12 commits** opens a second modal for selecting an actual commit.

Use the recovered design as a visual reference and implement through the current architecture. Import no historical feature code.

The user's clarified decisions supersede the earlier focused-graph proposal:

- Show a broad graph of the visible repository history. The selected branch's lineage remains highlighted; hovering another branch temporarily highlights its related lineage without changing selection. Keyboard focus provides the same preview. Keep other branches visible for comparison.
- Default visibility includes all live named local branches, whether or not they have worktrees, plus detached historical states that still have physical worktrees. In Git terms, HEAD/worktrees are detached rather than branch refs. Include detached worktrees without surviving refs, and retain known former branch names as provenance where available; do not manufacture refs or infer names from folders. Exclude historical-only entries with neither a live branch nor an instantiated worktree.
- Selecting a branch, worktree, or commit returns to the same Worktree Review detail surface. It changes the displayed review target, not just a hidden build-form option. Selection performs no checkout creation or compilation and opens no creation prompt.
- Clicking **Build** is the point at which the application prompts for a required worktree. On confirmation it uses the existing provisioning/build flow. An available checkout must actually represent the selected source; a worktree at the current tip is not a checkout of an older selected commit. Cancelling the prompt preserves the selected target and creates nothing.
- A target without a worktree uses its latest commit time for recency. A target with worktrees uses the approximate latest edit across those worktrees, discovered lazily. Exact filesystem activity tracking is not required.

## What the current implementation reveals

`WorktreeReviewScreen.tsx` combines repository selection, branch/detail loading, worktree selection, history paging, mutations, notices, and rendering. Repository changes and registration repeat much of the same selection sequence. Refresh reads only the selected branch, so the rest of the sidebar does not get a fresh inventory.

`BuildComposer.tsx` privately owns source mode and commit ID. A graph selection cannot currently set them through a normal data interface. It also builds application requests locally and has a second commit-deduplication implementation. Its collapsed commit summary always displays the branch tip, which would misrepresent a commit selected elsewhere.

The page's history state is shared by the build picker and the association-baseline picker. The backend pages first-parent history by continuing from the last commit. That cursor shape cannot simply be reused for all-parent graph history: following one commit's parents can skip another merged path.

`repository_context/refs.rs` already contains reference enumeration, commit facts, history traversal, and ancestry calculations. `RepositoryContext` exposes separate `references()` and `commits()` capabilities, so the file boundary should follow the existing object boundary.

`branch_first.rs` builds branch summaries using one Git inventory but queries stored associations per branch. Its count includes active retained associations without requiring a currently available checkout. Overview counting and detail availability use different logic.

Relevant dialog consumers are repository registration, the graph, commit selection, and the build-time checkout prompt. Existing dialogs implement focus behavior individually; there is no shared dialog primitive in the inspected frontend. Workflow's repository target selector instead returns an existing physical worktree through `RepoBranchWorktreeTargetSelectorProps`; it is not a consumer of this new branch/commit selection flow.

The existing review detail/build paths require a surviving local branch ref. Merely adding detached worktrees to the graph would therefore produce visible targets that cannot be selected or built. Supporting those targets needs an explicit source identity extension, described below.

## Intended ownership

### Current branch inventory

Extract `worktree_review/branch_inventory.rs` from the inventory and summary portions of `branch_first.rs`. It combines live local refs, physical-worktree observations, and stored associations/branch provenance. Add a repository-scoped association query in `storage/associations.rs` so this is one grouped read rather than a database call per branch. Return live branch targets and detached-worktree targets; several worktrees at the same commit remain individually identifiable even when the graph shares one commit node.

Return both `associatedWorktreeCount` and `availableWorktreeCount`. The first preserves disclosure of retained associations; the second determines ordering and worktree-readiness presentation. Deduplicate by worktree identity. Extract one pure availability projection in `branch_presentation.rs`, used by overview and detail, so missing checkouts and branch mismatches are classified consistently. A missing observation should yield an unavailable row, not accidentally count as available or fail the whole detail response.

Keep ordering in one pure `orderReviewTargets` function in `src/application/worktreeReview/presentation.ts`. Sidebar rendering and initial/fallback selection consume it. Graph lane placement is stable and topology-based: activity updates must not rearrange the graph while the user is inspecting it. Refreshing facts preserves a valid selection even when its sidebar position changes.

`repository_context/refs.rs` supplies an accurately named committer timestamp from its existing batched ref read. Correct the corresponding commit-fact reads consistently; do not keep a field called `committedAt` populated by author date.

Create `worktree_review/worktree_activity.rs` for lazy activity estimates. Show the inventory immediately using cached activity or commit-time fallback, then inspect worktrees with bounded concurrency. Reuse Git status paths and inspect modification times of changed tracked and non-ignored untracked files; use cheap index/commit evidence where edits have no surviving file, such as deletions. Bound the scan and exclude ignored dependency/build trees. This estimates recent source edits; it does not promise the exact time of every edit or reverted change.

Return the estimate with its basis and observation time so pending/fallback data is not presented as exact. Aggregate the newest estimate across a branch's available worktrees. Cache by repository/worktree identity and observed HEAD for the current application session, reuse recent results, and refresh stale estimates lazily on reopening/refresh. Publish sidebar updates in small batches, preserving selection and scroll. Do not run recursive whole-checkout scans during overview or add filesystem watchers for this requirement.

### Git facts and history

Move `CommitReader`, commit facts, ancestry/divergence methods, commit parsers, and their tests from `repository_context/refs.rs` into `repository_context/commits.rs`. Preserve `RepositoryContext::commits()` and the existing public module re-exports. This is a file-ownership correction, not a new Git wrapper.

Extend that reader with exact history scopes and bounded page reads. A scope identifies an included tip and, optionally, a base whose reachable history is excluded. With a base, its meaning is `reachable(tip) minus reachable(base)`: the base is excluded, the tip is included, and merged-in commits are included. Without a base it covers the tip's reachable history.

Counting and pagination use that same scope and traversal policy. Return deterministic newest-first topological order, with each commit appearing once. Replace the last-commit cursor with a cursor tied to the exact scope and page position. It must continue the same traversal rather than start a new traversal from the previous page's last commit.

The graph pins all displayed branch/worktree heads to object IDs when loaded. Counts, commit pages, and selected-commit details retain those endpoints. Refresh obtains a new graph; moving refs never silently retarget an open range. The build service validates the selected target at submission and must never substitute the latest tip.

### Worktree Review history service

Create `worktree_review/branch_history.rs` for the application-owned graph and history reads. It resolves the registered repository and visible target heads, uses `RepositoryContext`, and projects typed results. `BranchFirstReviewService` delegates these reads; transport remains a thin adapter running Git work on blocking workers.

The graph response contains real parent edges, branch/worktree labels, exact anchor identities, and expandable commit ranges. Retain branch heads, worktree HEADs, fork points, and merge points; compress ordinary stretches between anchors. It contains no drawing coordinates or runtime/build authority. Branch relationships come from observed Git ancestry, not a guessed single-parent branch tree. Unrelated histories remain separate components.

Use the current default branch as an orientation cue when available, not as the only graph root or a prerequisite for displaying other branches. Highlight the hovered/selected head's ancestor paths, shared fork/merge anchors, and observed downstream integration paths. Sibling-only paths stay subdued. Highlighting means graph reachability, not inferred semantic equivalence or proof that work was deliberately discarded.

Read the multi-head commit graph in batches when the modal opens. Traverse shared history once rather than issuing a comparison for every pair of branches. Preserve every eligible target label even when deeper history is loaded incrementally; mark unloaded boundaries explicitly. Hover uses the loaded adjacency structure locally, with no Git request per pointer movement. Load descriptions and detailed commit pages only on demand.

The range contract must describe exactly the commits represented by each compressed section. Keep a linear segment's count tied to its member commits, retaining merge/fork anchors rather than silently including off-segment histories. The commit reader can support a whole ancestry difference as well, but the graph must not label that broader set as one linear segment. Tests must prove both count/page agreement and segment membership across merges.

### One history contract for all consumers

Replace `branchHistory` / `BranchHistoryPage` with a typed `commitHistory(query, cursor?)` contract and `CommitHistoryPage`. The query carries repository and review-target identity plus the exact history scope. The page echoes that scope, its total count, commits, and continuation cursor. Add `branchGraph(repositoryId)` for the broad graph and a selected-commit detail read if descriptions are loaded separately. Represent linear-segment and ancestry scopes explicitly, using the same reader machinery rather than parallel history implementations.

The graph's range modal, the existing build picker, and the existing association-baseline picker all use this history contract. Full-branch consumers omit the excluded base. This deliberately makes merged-in commits available to the existing exact-commit pickers too; first-parent filtering is not an established user requirement here.

Create `useCommitHistory.ts` in the feature for query identity, paging, loading/errors, stale-response rejection, and deduplication. Instances can have independent selection while sharing the same loading implementation. Keep retained pages scoped to the current screen/dialog lifetime; no persistent history database or background refresh service is needed.

Remove the old command, DTO, frontend adapter path, and first-parent cursor implementation after migrating their callers. The traced callers are internal to Worktree Review and its tests; no compatibility adapter is warranted.

### Review target, page selection, and build draft

Introduce a `ReviewTarget` discriminated union in current frontend contracts and Rust domain source types: live branch, physical worktree, or exact commit with its source context. Repository identity is always required; a surviving branch ref is not. Branch and former-branch labels are provenance, while the selected worktree/commit determines the actual source. A detached target is not automatically associated with an unrelated live branch just to pass validation.

Adapt detail projection to render these targets in the same page sections: selected source, available matching worktrees and their states, build controls, and relevant retained builds. A selected historical commit must be shown as such throughout the detail surface. Worktree markers expose HEAD, attachment state, and staged/unstaged/untracked summaries lazily; uncommitted state remains distinct from committed graph nodes.

Extend source validation/materialization to accept an exact worktree or commit target without a live branch. Reuse current physical checkout/build operations and ownership policies. Update source-binding persistence and build lookup only where required to distinguish these targets; plan a focused schema migration if current mandatory branch fields cannot represent them honestly. Do not use fabricated branch refs or overload branch labels as source identity. Preserve existing branch-based consumers through the explicit live-branch variant, not a second execution path.

Extract `useReviewSelection.ts` from `WorktreeReviewScreen.tsx`. It owns repository/target/detail loading, selected worktree, ordered inventory, and refresh. Repository registration returns a repository ID to this same selection path. Errors and request identities belong to this owner rather than being copied into each dialog.

Refresh reloads overview facts and the relevant branch detail, retaining valid selections. Successful worktree creation, association, and builds that materialize a checkout use that refresh path. Remove manual `count + 1` patches; the backend inventory remains the source of truth for sorting and counts.

Create feature-local `buildDraft.ts` containing the draft value, pure transitions, defaults, and conversion to `CreateBuildRequest`. The screen owns the draft and passes it to a controlled `BuildComposer`. Keep source mode, exact selected commit, and build name together. Move `createRequest` out of the JSX file and render the chosen commit's summary from the draft, including when history is collapsed.

The graph returns a typed review target. After the matching detail loads, the screen applies that target to both the normal detail surface and the draft. Applying a commit within the current branch must work as well as changing targets. Obsolete requests must not overwrite a later selection. No imperative child ref, remount-key trick, or effect watching an external “requested commit” is needed.

Keep the build-time decision in a small `BuildCheckoutDialog.tsx` and a pure checkout-requirement projection. **Build** checks whether the selected execution mode/source requires provisioning, displays the proposed checkout when needed, and proceeds through the current coordinator only after confirmation. Selection and graph browsing never invoke this decision. Existing direct/snapshot source semantics continue to determine whether an existing checkout can be used.

### Graph and dialog presentation

Create a `branchSelection/` area under the feature:

- `BranchGraphDialog.tsx`: repository graph loading, temporary hover/focus preview, persistent selection, and apply/cancel actions.
- `BranchGraph.tsx`: a pure HTML/SVG rendering of the broad graph, target/worktree markers, and range buttons.
- `branchGraphLayout.ts`: stable lanes and positions for anchors, merge/fork edges, and compressed sections. `branchLineage.ts` derives highlight sets from adjacency. Neither module invokes Git or the client.
- `CommitRangeDialog.tsx`: the selected range, paged commit list, selected-commit details, and **Use this commit**. It uses `useCommitHistory`.
- `branchSelection.css`: graph geometry, modal content layout, and responsive rules, using current product tokens.

The graph has actual endpoint/relationship meanings and compressed ranges. It does not render pretend commit dots. The commit modal expands a range into the real commits. A zero-length range is displayed as zero; unrelated history is not represented by a made-up count.

Add feature-local `ReviewDialog.tsx` for the modal shell: accessible naming, focus entry/return, background inertness, scroll containment, and topmost-only Escape/backdrop handling. Use it for the graph, commit range, build checkout prompt, and existing registration modal. Preserve each caller's own pending-action dismissal policy. Verify nested behavior in the application's WebView; do not duplicate independent focus traps or create an application-wide modal manager.

The main page's existing build and baseline pickers keep their visible placement and purposes. Their data comes from the shared history loader. Other feature dialogs and the Workflow worktree target selector are not migrated as part of this task.

## Concrete file changes

| Action | Files / objects | Resulting responsibility |
| --- | --- | --- |
| Retain | `repository_catalog/*`, `worktree_application/*`, build execution, retention and cleanup policies | Existing registration and physical checkout/build effects remain authoritative. |
| Adapt | `src-tauri/src/repository_context/refs.rs`, `mod.rs`; create `commits.rs` | Ref enumeration separated from reusable commit reading. |
| Extract | `src-tauri/src/worktree_review/branch_inventory.rs` from `branch_first.rs` | One current branch/worktree inventory for overview and graph choices. |
| Adapt | `storage/associations.rs`, `branch_presentation.rs` | Batch association reads; consistent availability and summary facts. |
| Create | `src-tauri/src/worktree_review/worktree_activity.rs` | Bounded lazy edit-time estimation and session cache. |
| Create | `src-tauri/src/worktree_review/branch_history.rs` | Product graph/history scope and projection. |
| Adapt | `domain/source.rs`, source materialization/validation, build projections and relevant storage | Explicit targets for detached worktrees and exact commits without fabricated live refs. |
| Adapt | `worktree_review/mod.rs`, `transport.rs`, `src-tauri/src/active_app.rs` | Register and delegate current product read capabilities. |
| Adapt | `src/application/worktreeReview/{contracts,client,presentation,index}.ts`, `src/infrastructure/tauriWorktreeReview.ts` | Exact graph/history contracts and one branch ordering function. |
| Extract | `src/features/worktreeReview/useReviewSelection.ts`, `useCommitHistory.ts`, `buildDraft.ts` | Explicit ownership of page selection, history requests, and form intent. |
| Adapt | `WorktreeReviewScreen.tsx`, `BranchNavigator.tsx`, `BuildComposer.tsx`, `WorktreeSelector.tsx` | Compose those owners; add the entry point; consume the selected commit and shared history. |
| Create | `branchSelection/{BranchGraphDialog,BranchGraph,CommitRangeDialog}.tsx`, `branchGraphLayout.ts`, `branchLineage.ts`, `branchSelection.css` | Broad graph layout, lineage highlighting, and commit-range presentation. |
| Create | `BuildCheckoutDialog.tsx` | Checkout creation prompt invoked by Build only. |
| Extract/adapt | `ReviewDialog.tsx`, `RepositoryRegistrationModal.tsx`, dialog rules in `worktreeReview.css` | One modal shell for this feature's consumers. |
| Remove | Old first-parent history command/DTO/cursor path; page-local history bookkeeping; duplicate commit deduplication; private composer draft/request construction; optimistic count increments | Eliminate competing implementations rather than wrap them. |

When implementation lands, update the current durable architecture document's historical-search exclusion to reference this bounded capability. Keep it describing the implemented architecture, not this proposal.

## Delivery and verification

1. **Inventory and history foundation.** Extract readers and inventory; represent all eligible branch/detached-worktree targets; migrate history callers; prove graph ranges and paging with temporary repositories. Add lazy activity estimation without delaying initial display.
2. **Target and selection ownership.** Extend source identity for detached/exact targets; extract page selection and build draft. Verify that every target opens the normal detail surface, no selection creates a checkout, and only Build prompts for required creation.
3. **Graph flow.** Add broad layout, local hover/focus lineage highlighting, worktree state markers, and the range dialog. Test cancel/apply, nested focus, incremental graph loading, and request races. Compare the rendered flow with the chosen design and the current app's visual language.
4. **Run the current app.** Exercise list → graph → hover lineage → count → commit → normal details → Build → checkout prompt. Cover a live branch without a worktree, an older commit, and a detached worktree without a live ref. Verify exact materialization and inventory updates. Use a disposable fixture for mutation checks and the current app for visual/native behavior.

Focused Git fixtures cover multiple branch families, nested branches, merges, shared tips, unrelated roots, detached worktrees, and deleted refs with retained worktrees. Tests must prove correct default visibility, highlight reachability, compressed-section membership, and count/page agreement. A missing physical checkout or mismatched association must not receive worktree-first priority. Activity tests cover edits, staged/deleted files, ignored build output, multiple worktrees, and fallback behavior; do not demand exact edit timestamps.

Frontend checks cover both existing history consumers, lazy activity updates, target detail consistency, and preservation on cancel. Hover must neither change selection nor fetch Git facts; leaving hover restores the selected lineage. Build-prompt cancellation creates nothing. Inspect desktop and narrow layouts, including keyboard focus after each modal closes. Run the repository's focused Worktree Review frontend/Rust suites, build, lint, formatting, and development/release checks appropriate to the changed transport and modules.

## Scope boundary

No old runtime restoration, graph persistence, reflog/archive recovery without a surviving worktree, GitHub discovery changes, branch restoration, merge/rebase controls, diff viewer, automatic build-on-selection, precise filesystem event tracking, or graph editing. The broad graph, detached target support, build-time provisioning prompt, and lazy activity estimates are now included. Any storage change is limited to representing those explicit targets in the current source/build model.

## Execution record — 2026-09-09

Implemented the agreed surface on the current architecture. No historical feature code was imported. Inventory, activity, graph/history, selection, dialog, and build-draft responsibilities were extracted into the owners above. The previous branch-only detail/history transport was replaced, and refresh now reloads inventory instead of patching counts.

The graph retains forks, merges, current local branch heads, and physical detached worktrees. It centers the selected lineage, offers zoom and earlier-history loading, and places clickable commit counts clear of branch cards. Distinct paths between the same merge anchors retain distinct identities. Open graph snapshots and history cursors remain pinned to object IDs. The traversal limit is 20,000 commits, with explicit earlier-history boundaries.

Branchless physical-worktree and exact-commit sources use the existing materialization and build coordinator. The nullable branch provenance migration preserves existing columns and rows; borrowed physical worktrees keep the existing no-removal ownership policy. Selecting a target creates no checkout. Build opens the required-checkout prompt, and Cancel preserves the selection.

Validation:

- `npm run build`: passed (TypeScript and Vite; existing bundle-size warning).
- `npm run test:worktree-review`: 39 passed, including graph geometry, lineage, selection, cancellation, exact build requests, and existing application surfaces.
- Native `worktree_` suite: 60 passed. Real temporary Git repositories cover merged history, pagination/count agreement, moving refs, parallel compressed merge paths, detached worktrees after branch deletion, ignored-output activity, exact historical checkout creation, and durable branchless source records.
- Native `repository_context::` suite: 6 passed.
- Native development binary build: passed. Targeted ESLint and `git diff --check`: passed.
- Browser visual checks used the real React feature with isolated fixture data at 1280 × 720. Verified graph and range layout, selecting a commit into the normal review surface without prompting, Build opening the checkout dialog, Cancel retaining the exact commit, nested Escape returning to the graph and its range button, and branch selection returning focus to Select branch. This is frontend evidence, not a live Tauri invocation or compilation run.

Screenshots (fixture data):

- [Branch graph](<../../.dev/branch-navigation/graph-fixture.png>)
- [Commit selection](<../../.dev/branch-navigation/commit-fixture.png>)
- [Build checkout prompt](<../../.dev/branch-navigation/build-prompt-fixture.png>)

Remaining verification: the isolated desktop launch registered its main Tauri window and finished application setup, but exposed no visible accessible window or WebView debugging endpoint. A fresh isolated database also required empty tables from the existing Sprint schemas before setup; this workaround was confined to `.dev/branch-navigation/app-data`. Production app data and the existing running app were untouched. Native keyboard behavior, a build launched through the new UI, and packaged/restart behavior have not been validated. Temporary startup traces were removed from source and the final binary was rebuilt.

Logs and the disposable frontend preview are retained under `.dev/branch-navigation`. Changes are local and uncommitted; no merge or publication was performed.

### Regression and native usability follow-up — 2026-09-09

The isolated app subsequently opened successfully and exposed its owned WebView debugging endpoint. The current repository flow was exercised through the app-inspector/CDP transport after the user excluded Computer Use. The prior native-visibility limitation is superseded by the [regression and usability report](<C:/Users/user/.codex/worktrees/d02e/Codex Orchestrator/docs/orchestration/worktree-review-regression-usability-report.md>).

The follow-up passed 958 frontend tests, all 689 native test names across partitioned runs, the production frontend build, and targeted lint/format checks. It verified historical selection, Build-only prompting, cancellation without checkout/ref changes, detached-worktree detail, hover lineage, refresh preservation, and nested Escape focus restoration. A real-repository merge marker obstructed a count button; marker-aware placement and non-interactive SVG decoration repaired it, with a failing-before/passing-after regression and a successful native click afterward.

Graph readability remains an open usability finding: the default viewport shows only three of fourteen target cards, and zooming out makes labels and count controls very small. Reverse-Tab and refresh focus observations also remain. Actual build confirmation/execution through this UI, fresh-profile startup without the existing schema workaround, physical keyboard/screen-reader behavior, and packaged/restart validation remain unproven.
