# Worktree Review grid refinement

Superseded by the [present-branch revision](worktree-review-present-branches-report.md). The results below describe the previous grid implementation.

2026-09-09. Implemented in the existing dirty `codex/worktree-review-branch-navigation` worktree, based on `deecb2f7`. No commit, merge, or publication was performed. The preceding branch-navigation implementation remains in place.

## Result

The branch selector uses deterministic grid rows, orthogonal history connections, reserved commit-count cells, and an aligned state column that remains visible during horizontal scrolling. Text stays at normal size; count buttons are 112 × 34 CSS pixels. Long names wrap, with measured card heights shared by the history rows. Dotted state attachments remain separate from counted Git history.

The native projector folds closed historical regions while retaining displayed targets, boundaries, and meaningful ancestry junctions. Linear and folded connections both use immutable native-owned ranges. The eight-snapshot history service pins heads and exact member lists; paging and earlier-history expansion do not substitute current refs. Invalid source identities and expired snapshots fail explicitly. Build and source materialization retain their existing owners.

The layout, HTML controls, and keyboard neighbors share `branchGraphLayout.ts`. The old candidate-point placement, Bézier calculations, SVG `foreignObject` controls, and scaled zoom path were removed. `BranchGraphDialog` still owns temporary selection and range opening; `branchLineage.ts` remains the local ancestry calculation. `ReviewDialog` supplies topmost modal boundaries, while refresh focus restoration stays beside the initiating screen control.

## Automated verification

| Check | Result |
| --- | --- |
| Full frontend suite, four workers | 962 passed across 169 files (`grid-frontend-verified.log`). |
| Final focused frontend checks | 20 passed after the last row-separator and paging-focus refinements (`grid-final-focused.log`). |
| Focused native Worktree Review/application suite | 64 passed, including real temporary Git repositories (`grid-native-tests.log`). |
| Full native library suite, four threads | 692 passed; one orchestration MCP response-body timeout, reproduced in isolation (`grid-native-full.log`, `grid-native-recheck.log`). |
| Native development build and `cargo check` | Passed; existing dead-code warnings remain. |
| Frontend production build | Passed; existing bundle-size warning remains. |
| ESLint over application source, changed-file Prettier, targeted rustfmt, `git diff --check` | Passed. |
| Repository-wide ESLint | 35 existing errors and one warning in documentation demo/probe scripts; application source is clean. |

Projection tests cover retired nested merges, surviving/shared heads, unrelated roots, missing boundaries, a series of 100 merge regions, and varied parallel DAGs. They compare retained reachability against the original graph and check membership, eligibility, deduplication, and endpoint inclusion. Real Git paging tests verify exact counts, pinned heads after ref movement, old ranges after expansion, source rejection, and snapshot expiry. Existing source/build regressions continue to pass.

The full frontend runs exposed a race in `EpicInitiationConfirmationModal.test.tsx`: after Cancel, `findByRole` could return the still-open old request's dialog. The assertion now waits for the next queued request's content. This changes only the test's synchronization; application behavior and timeouts are unchanged. The subsequent full run passed.

The remaining native failure is `orchestration::bootstrap_transition::tests::work_slice_planning_request_launches_one_prepared_planner_and_marks_readiness`. Its HTTP helper at `src-tauri/src/orchestration/bootstrap_transition.rs:6455` times out decoding an MCP response body. The full suite completed in 1049.45 seconds; an isolated replay failed the same way in 17.21 seconds. That orchestration code was not changed in this refinement. The native full-suite result is therefore not green.

## Native app usability evidence

Used the rebuilt Tauri app, isolated app/review data, and its owned loopback WebView debugger through `review-tools/app-inspector`. No Computer Use skill, desktop input, CUA, or Sky was used. Ownership receipts identify app PID 43772, its exact worktree executable, WebView debugger port 9347, and Vite URL `http://127.0.0.1:1459/`.

The real repository supplied 12 local branches, two physical detached worktrees, and 836 reachable commits. The conservative projection retained 68 anchors and 91 clickable connections. At 1500 × 880 CSS pixels the grid measured 6000 × 1536 pixels, including all target rows and a few reused historical lanes. Horizontal history scrolling remains intentional; the earlier 49-anchor estimate in the plan was a feasibility estimate, not an acceptance result.

- Inspected 1500 × 880 and 960 × 720 CSS viewports at device scale 1.25. The 304-pixel state column stays visible at both horizontal extremes, with shared vertical scrolling. A real small-window failure in the first sticky implementation was repaired by using a flex spacer instead of a large margin.
- All fully visible controls passed center hit checks in the recorded desktop, narrow, and oldest-history views. Geometry tests also check control/marker separation, clear routes, shared-tip cards, lane reuse, and stable positions under activity changes. Hover preserved every measured control rectangle and did not change selection.
- Arrow keys move focus without applying selection. Horizontal navigation prefers the same row. Tab leaves the graph for the footer, Shift+Tab returns to its last control, and nested forward/reverse Tab stays within the commit picker. Escape closes only the top modal and returns focus to its count.
- Opened the folded 302-commit range from `84d89958` to `4a3d5df1`. Native paging loaded 50 then 100 commit choices; Git independently confirmed the range count of 302. The paging button retains focus while loading.
- Applied `4a3d5df1c41909926ba74181c6342860d5add38e` to the ordinary review surface without a creation prompt. Build then prompted for that exact commit. Cancel preserved the selection and returned focus to Build. Refresh preserved the commit and returned focus to Refresh facts.
- Selected the detached `4d1a-scs02-frontend-validation-2` worktree on the ordinary surface without prompting. The isolated review database still has zero builds and zero retained outputs after the exercised flows.

Artifacts and command logs are under `.dev/branch-navigation/grid` and `.dev/branch-navigation/grid-*.log`. Representative captures:

![Desktop grid](<C:/Users/user/.codex/worktrees/d02e/Codex Orchestrator/.dev/branch-navigation/grid/36-final-grid.png>)

![Narrow grid with pinned states and folded history](<C:/Users/user/.codex/worktrees/d02e/Codex Orchestrator/.dev/branch-navigation/grid/31-small-final.png>)

![Exact commit on the normal review surface](<C:/Users/user/.codex/worktrees/d02e/Codex Orchestrator/.dev/branch-navigation/grid/15-normal-surface.png>)

## Limits

The usability run uses the development app and existing isolated test profile. It does not establish packaged/restart behavior or resolve the previously documented fresh-profile schema issue. Build confirmation was deliberately cancelled; execution of a retained build was not part of this refinement's live check. Earlier-history expansion was verified with pinned/truncated Git fixtures; the real 836-commit repository fits within the initial graph load. Repository-wide documentation-script lint errors are outside this change.
