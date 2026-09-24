# Worktree Review build experience correction

Status: proposed 2026-09-24. No implementation has started.

## Target and boundaries

Make Worktree Review support several independent builds without trapping the user on the source screen, report completed work where the user can find it, and launch each retained build as a real isolated application instance. Keep the presentation narrow: show when a build was initiated, whether its runnable output still exists, and the useful actions.

This pass does not add source-freshness comparison, estimated completion times, persistent notification history across application restarts, automatic semantic review, or continuous synchronization between the controller and reviewed application data.

## 1. Make build creation a background Worktree Review operation

- Adapt `worktree_review::transport::create_worktree_review_build` and `ReviewBuildCoordinator` so the command returns after the build and its initial attempt are durably recorded, while the terminal build result is published independently.
- Resolve the repository from the submitted `repositoryId` through the registered repository catalogue. Remove the build path's dependency on the currently selected Worktree Review repository so later navigation cannot invalidate an already-started build.
- Keep build jobs keyed by build ID. Do not use the screen-wide `busy` state to disable repository, branch, worktree, or top-level navigation after the start receipt is returned. Only disable the modal's submit action while its own start request is pending.
- Emit one `worktree-review://build-terminal` event after the terminal attempt and output state are durably saved. Include build ID, repository ID, branch/commit target, originating worktree ID, completion time, outcome, and whether a runnable output exists.
- Correct same-source retention so a successful build may clean only strictly older terminal builds with the same retention key. A completion racing with a newer or still-running build must not delete the newer build's attempt data or output.

## 2. Give completion notifications application-level ownership

- Create an application-scoped `WorktreeReviewBuildActivitySource` and Tauri implementation, composed in `productApplicationComposition.ts` and supplied through `AppProps`. It listens once for terminal events and owns in-process unread build IDs plus their repository/branch/worktree targets.
- Show an unread dot on the **Worktree Review** surface button in `App.tsx`. Pass the target unread state into `WorktreeReviewScreen`, `BranchNavigator`, and `WorktreeSelector` so the relevant branch and worktree show the same signal.
- Clear a target's unread builds only after the user selects that branch/worktree and its detail response containing those builds has loaded. Merely opening Worktree Review or starting a selection must not clear it.
- Keep target-detail polling only for the visible build log/progress pane. Navigation and notifications must not depend on that screen being mounted.
- Keep notification state process-local in this pass. Durable build records remain the source of truth after restart, but previously completed builds do not reappear as unread notifications.

## 3. Replace the inline composer with one build modal

- Replace `BuildComposer.tsx` and `BuildConfirmationDialog.tsx` with `CreateBuildDialog.tsx`; delete the redundant second confirmation step.
- Put a **Create a build** button at the top of `BuildHistory.tsx`. It opens the modal for the currently selected branch/worktree and leaves the retained-build list directly below it.
- Use these source explanations exactly:
  - **Live worktree checkout:** “Build directly in the worktree.”
  - **Snapshot current work:** “Copy the selected worktree and build in the destination.”
  - **Specific commit:** “Create worktree from the selected commit and build there.”
- Reuse `BranchGraphBrowser`, the Worktree Review graph query, and the existing commit-range dialog inside the modal for specific-commit selection. Remove the bespoke select plus **Load more commits** path from `BuildComposer`/`useCommitHistory` where it is no longer used elsewhere.
- Keep build name, normal/debug profile, dirty-work warning, and workspace-creation disclosure in the modal. Closing or submitting it returns focus to **Create a build**.
- Add **Rebuild** only to live-worktree builds. It reopens the same modal prefilled with that build's worktree source, name, and build profile so the user can confirm the current live state before starting the new build.

## 4. Simplify retained-build presentation

- Add `initiatedAt` to `ReviewBuildView`, sourced from the build's durable `createdAt`, and show it on every build card. Do not calculate or display current/outdated source state.
- Make runnable existence the primary status:
  - **Available** when `output.state === 'available'`, with **Launch**;
  - **No longer available** for removed or missing output, with no Launch button;
  - **Building** or **Build failed** while/no output was produced.
- Do not show a green success badge on a card whose output no longer exists. Preserve a concise compilation outcome/failure and the optional build log as secondary information, but remove cleanup-policy and AppData-retention prose from the ordinary card.
- Keep the backend's existing filesystem verification of retained output. Collapse its `removed` and `unavailable` distinctions into the same user-facing **No longer available** state rather than creating a larger status taxonomy.

## 5. Give each reviewed build a truthful application identity

- Extend `PhysicalWorktreeBuildRequest`, `worktree_application::build`, and `scripts/build/application.mjs` with a validated review application identity supplied by Worktree Review.
- Generate an identifier containing the sanitized branch (or detached commit) label, a short stable worktree ID, and a short build ID, for example `dev.codex-orchestrator.review.refinement-usability.wt-03274180.bld-a1b2c3d4`. Store the full branch/worktree/build labels in the build receipt and expose the human-readable identity in the build detail/window title.
- Pass the generated identifier through Tauri's existing `--config` merge. This is preferable to a second lock implementation: on Windows the existing single-instance plugin derives its mutex from the compiled Tauri identifier, so the build-ID suffix makes the lock build-scoped. Two different reviewed builds can run together; launching the same build again focuses its existing instance.
- Keep the ordinary controller identifier unchanged. Validate and hash/truncate unsafe or overlong label components deterministically; never accept a caller-supplied raw Tauri identifier.
- Adapt `worktree_application::open` to observe a bounded launch result: either focus an exact existing executable window, observe a visible window for the spawned executable, or report early exit/timeout as an error. Return that `OpenOutcome` through `ReviewBuildCoordinator::open_build` and the Tauri/client contract. Report **Focused existing build** or **Launched build** only after the corresponding outcome; process creation alone is not success.

## 6. Provision isolated reviewed-application data on first launch

- Create `worktree_review::review_runtime` to own the per-build runtime root, initial data seed, launch environment, and seed manifest. Keep this out of generic `worktree_application`, whose responsibility remains build/open mechanics.
- Use `worktree-review/review-runtimes/<build-id>/app-data` and `.../webview-data`. Pass them as `CODEX_ORCHESTRATOR_APP_DATA_DIR` and `WEBVIEW2_USER_DATA_FOLDER`, while continuing to pass the shared `CODEX_ORCHESTRATOR_WORKTREE_REVIEW_DATA_DIR` plus active build/worktree IDs.
- On the first launch of a build, mirror the controller's durable AppData into the private app-data directory so profiles, repositories, capability configuration, and related setup are available. Preserve that private copy on later launches of the same build; do not overwrite changes made by the reviewed instance.
- Mirror regular durable files and directories by default, but exclude the shared `worktree-review` tree, `EBWebView`, live navigation/IPC token files, probe/temp/cache/log/lock data, and SQLite `-wal`/`-shm` sidecars. Create consistent copies of SQLite databases with SQLite snapshot/VACUUM semantics instead of copying live database files byte-for-byte. Start WebView2 with a new private profile rather than copying an in-use browser profile.
- Write a seed manifest containing build ID, branch/worktree identity, source app-data path, seed time, and schema version. If seeding fails, do not start the executable; surface the failed item and leave the controller data unchanged.
- Include each review-runtime root in the existing Worktree Review retention/cleanup resources. Removing a retained build may remove its private app-data and WebView data only through the current contained-path cleanup service.

## File and ownership shape

### Retain and adapt

- `src-tauri/src/worktree_review/build_service.rs`, `build_presentation.rs`, `branch_first.rs`, `transport.rs`: background terminal notification, repository-by-ID execution, initiated time, build identity, truthful open result, and older-only retention.
- `src-tauri/src/worktree_review/state.rs`, `cleanup_service.rs`, and domain/storage records: controller AppData authority, review-runtime resource ownership, and contained cleanup.
- `src-tauri/src/worktree_application/domain.rs`, `build.rs`, `open.rs`, plus `scripts/build/application.mjs`: validated build identity/config input and observable open outcome.
- `src/application/worktreeReview/{contracts,client,presentation}.ts` and `src/infrastructure/tauriWorktreeReview.ts`: terminal event, initiated/existence presentation, rebuild request, and open outcome contracts.
- `src/features/worktreeReview/{WorktreeReviewScreen,BuildHistory,BranchNavigator,WorktreeSelector}.tsx` and `worktreeReview.css`: non-blocking navigation, modal trigger, notification dots, simple status, and rebuild.
- `src/app/App.tsx`, `src/bootstrap/productApplicationComposition.ts`: application-level build activity composition and top-level unread dot.

### Create

- `src-tauri/src/worktree_review/review_runtime.rs`: identity generation, one-time safe AppData seed, launch environment, manifest, and runtime cleanup descriptors.
- `src/application/worktreeReview/buildActivity.ts` and `src/infrastructure/tauriWorktreeReviewBuildActivity.ts`: stable terminal-event subscription and in-process unread state.
- `src/features/worktreeReview/CreateBuildDialog.tsx`: the single build-creation and rebuild surface using the shared graph browser.

### Delete after callers move

- `src/features/worktreeReview/BuildComposer.tsx`.
- `src/features/worktreeReview/BuildConfirmationDialog.tsx`.
- Worktree Review-only commit-dropdown/loading code that becomes unreachable after graph selection moves into `CreateBuildDialog`.
- Card-level cleanup/AppData-retention presentation that no longer helps decide whether a build can launch.

## Implementation sequence

1. Add backend regressions for two concurrent builds on different repositories/branches, same-source completion ordering, terminal event timing, generated identity, per-build lock separation, seed exclusions, SQLite snapshot consistency, and contained cleanup.
2. Make build start independent of current selection, return the durable start receipt, emit terminal results, and correct older-only retention.
3. Add review identity to the build tool and build receipt; provision and seed the per-build runtime on first launch; return and display the real open outcome.
4. Add the application-level activity source and unread dots, then remove screen-owned job tracking from navigation state.
5. Replace the inline composer/confirmation pair with the graph-backed modal and add prefilled live-worktree rebuild.
6. Simplify build cards and remove superseded presentation/code.
7. Run focused tests, broad Worktree Review validation, then a packaged native smoke flow.

## Verification and acceptance

- Start a build, navigate immediately to another branch/worktree, and start a second build. Both finish against their submitted source and neither deletes or blocks the other.
- Completion produces dots on the Worktree Review tab and exact branch/worktree. Selecting and successfully loading that target clears its dot only.
- The build list starts with **Create a build**. Its modal uses the required descriptions and graph-based commit selection. A live build has **Rebuild**, prefilled from that build.
- Every card shows when it was initiated. An absent output says **No longer available**, never looks successful, and has no Launch action.
- Two builds from the same branch/worktree have identifiers containing that branch/worktree plus different build IDs and can run simultaneously. Re-launching one build focuses that exact instance.
- First launch creates isolated AppData and WebView roots populated with usable controller configuration without copying live SQLite sidecars, IPC tokens, Worktree Review recursively, or the controller WebView profile. Relaunch preserves reviewed-instance changes.
- Launch failure or immediate exit does not show a false success notice; the UI reports the observed open outcome or error.
- Validate with focused frontend and Rust tests, `npm run test:worktree-review`, `npm run test:build-tools`, `npm run build:frontend`, `npm run test:rust:worktree-review`, and a native release-controller build -> concurrent builds -> launch/focus -> cleanup flow. Capture screenshots and process/AppData evidence under `docs/validation/worktree-review-build-experience/`.

## Explicit non-goals

No source-current/outdated badge, progress percentage or remaining-time estimate, persisted unread state, copying of an active WebView2 profile, continuous configuration sync into reviewed instances, remote build execution, installer identity changes, automated application review, or general replacement of Tauri process management.
