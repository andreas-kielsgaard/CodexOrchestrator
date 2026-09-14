# Worktree Review

Worktree Review selects an exact source, compiles it, retains the resulting application, and opens it for inspection. Git supplies current repository facts; durable review records explain which source and checkout produced each build. This guide describes main `60c3798`, checked on 2026-09-14.

## Review flow

1. Select a registered repository. **Add repository…** opens the separate registration dialog, where a local directory or a repository discovered through Codex can be registered. GitHub discovery can also show repositories without a local checkout.
2. Choose a branch from the sidebar, or open **Select branch…** for the graph. The graph includes current local branch heads and existing detached worktrees. Select a head directly, or select a commit count to inspect that range and choose an exact commit.
3. Review the ordinary detail panel and choose the physical checkout and compilation source. Selecting a branch, worktree, or commit does not create a checkout. **Build** prompts when the selected source requires one; the detail panel also has an explicit **Create worktree** action.
4. Build and inspect its recorded result. A successful retained output exposes **Open**, which requests focus for an existing window using that executable when supported, or launches it independently.

Repository browsing, build prerequisites, and output storage have separate readiness states. A repository can remain inspectable when compilation is unavailable. The current builder uses this project's Node/npm and Tauri recipe, including `package.json`, `package-lock.json`, and `src-tauri/Cargo.toml`; registration alone does not establish build support for an arbitrary repository.

### Reading the graph

The graph shows relationships relevant to present heads and their nearest common ancestors. It compresses intervening history and omits the common prefix before the relevant branching point. Compact labels carry branch/worktree identity; hover and focus reveal additional worktree and change facts.

Each connection's commit count represents its own ancestry difference. Counts can overlap across connections and cannot be added together as a repository total. Native code owns the exact members of each selectable range. Paging keeps that graph snapshot's scope; an expired snapshot asks the user to reopen the selector instead of silently substituting a different range. Incomplete history is represented explicitly.

Worktree edit recency is an approximate, lazily collected filesystem observation. Where no worktree activity is available, the UI shows commit time. Neither timestamp establishes agent activity or acceptance.

## Source and build identity

| Compilation source         | Behavior                                                                                                                                                                                                                                                        |
| -------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| **Live Worktree checkout** | Records the selected source, then compiles the existing checkout with its existing dependencies. Dirty content is captured as a virtual commit for the receipt, but the build still reads the live checkout. Later edits do not invalidate the retained result. |
| **Snapshot current work**  | Captures dirty work as a stable virtual commit, or uses the clean current commit, then creates a retained build checkout from that exact object.                                                                                                                |
| **Specific commit**        | Uses the selected Git object in a retained checkout. The request records whether this creates a managed branch checkout or a build-owned checkout.                                                                                                              |

Virtual-commit capture preserves the source branch, index, and working tree. A source receipt identifies what triggered the operation; it is not a guarantee of application quality or protection against concurrent edits to a live source. Snapshot mode provides the stable checkout when that is needed.

The checkout running the current review build is unavailable for live-checkout and snapshot compilation. An exact-commit build can create another checkout. This restriction concerns the exact active source checkout, not a hierarchy of parent and child applications.

Created build checkouts remain available after compilation. Application output and attempt logs live in Worktree Review AppData, rather than being copied back into the original source checkout. Borrowed checkouts use their installed dependencies; dependency preparation for a new checkout is a separate build policy.

Opening a build does not create a managed runtime session. Closing it belongs to that application instance. Each opened application retains Worktree Review capability, and its launch context explicitly carries the shared review data root even though the reviewed application receives private AppData.

## Repository and storage ownership

The shared repository catalog records registration and disclosure provenance in [ActiveDatabase](architecture/active-database.md). Worktree Review and Workflow target selection consume that same catalog. Codex and GitHub help discover candidates; Git remains the authority for repository identity, common-directory continuity, refs, commits, worktrees, and working changes.

Worktree Review has a separate `worktree-review.sqlite` operational database and output root. It records selections, observations, source bindings, checkout associations, build attempts, retained outputs, and cleanup receipts. A past observation does not override a fresh Git read.

Build-output retention is separate from checkout retention. The retention evaluator protects the configured newest successful builds per source, leaves running work alone, and keeps unverified cases distinct. Cleanup uses durable jobs and receipts and confines effects to owned output/attempt material under the canonical review root. It does not remove physical worktrees or manage opened application processes.

## Implementation map

| Concern                                                                     | Owner                                                                                                                                                                                                                                                                                                                |
| --------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Product composition and shared catalog injection                            | [productApplicationComposition.ts](../src/bootstrap/productApplicationComposition.ts)                                                                                                                                                                                                                                |
| Typed targets, build requests, presentation facts, and application port     | [application/worktreeReview](../src/application/worktreeReview/)                                                                                                                                                                                                                                                     |
| Selection, details, checkout prompt, and build draft                        | [WorktreeReviewScreen.tsx](../src/features/worktreeReview/WorktreeReviewScreen.tsx), [useReviewSelection.ts](../src/features/worktreeReview/useReviewSelection.ts), [buildDraft.ts](../src/features/worktreeReview/buildDraft.ts), [BuildCheckoutDialog.tsx](../src/features/worktreeReview/BuildCheckoutDialog.tsx) |
| Graph and commit-picker presentation                                        | [branchSelection](../src/features/worktreeReview/branchSelection/)                                                                                                                                                                                                                                                   |
| Frontend native adapter                                                     | [tauriWorktreeReview.ts](../src/infrastructure/tauriWorktreeReview.ts)                                                                                                                                                                                                                                               |
| Registration and live Git reads                                             | [repository_catalog](../src-tauri/src/repository_catalog/), [repository_context](../src-tauri/src/repository_context/)                                                                                                                                                                                               |
| Current-head projection, exact ranges, and recency                          | [branch_graph.rs](../src-tauri/src/worktree_review/branch_graph.rs), [branch_history.rs](../src-tauri/src/worktree_review/branch_history.rs), [worktree_activity.rs](../src-tauri/src/worktree_review/worktree_activity.rs)                                                                                          |
| Product records, source materialization, build orchestration, and retention | [worktree_review](../src-tauri/src/worktree_review/)                                                                                                                                                                                                                                                                 |
| Reusable physical capture, checkout, compile, and open operations           | [worktree_application](../src-tauri/src/worktree_application/)                                                                                                                                                                                                                                                       |

The physical-worktree component has no authority over product review retention or runtime lifecycle. Worktree Review owns those product records while delegating concrete Git, filesystem, and process effects through the smaller component.

## Decisions and proof

The final design resulted from explicit user corrections. **Worktree Stabilization** (`01a0395c-f32c-78c0-8da1-521197812474`, original messages at raw rollout lines 460, 3281, 5136, 8522, 8578, and 9802) established retained checkouts, compile/open without runtime management, independent application instances, Git authority, separate registration, and the shared catalog/database integration.

**Worktree Refinement Review** (`01a0853c-31a2-77e3-81af-4aa1b086ff7d`, raw lines 139, 284, and 2869) added branch/commit navigation through the current architecture and then narrowed the graph to present branching with compact labels. The intermediate large-card grid and the earlier branch-only scope are superseded designs.

The September 14 integration record covers native startup and registration, migration of a copied review database, exact checkout creation, actual compilation, retained output, open/restart, and existing-window focus reuse. It includes the Windows long-path correction at `057c4c7`; completion was published at `e2bfc6c`. These are dated executed results, not tests rerun for this documentation change. See [validation evidence](validation-evidence.md) for checkpoints, proof boundaries, and remaining observations.

Historical source records are recoverable at `e2bfc6c:docs/orchestration/worktree-review-durable-architecture-plan.md`, `e2bfc6c:docs/orchestration/worktree-review-present-branches-report.md`, and `e2bfc6c:docs/orchestration/worktree-review-main-integration-report.md`. The earlier [offline runtime package](../offline-review/README.md#worktree-runtime-exploration) records the replaced exploration, not this feature's operating model.
