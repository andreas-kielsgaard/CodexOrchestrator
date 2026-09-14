# Worktree Review regression and usability review

The subsequent grid refinement and its validation are recorded in [worktree-review-grid-report.md](worktree-review-grid-report.md). This report preserves the earlier layout findings.

2026-09-09. Tested the current dirty implementation on `codex/worktree-review-branch-navigation`, based on `deecb2f7`. The real repository contained 12 named local branches and two physical detached worktrees. Main had advanced to `ac22f015`.

The main selection flow works. Testing found and repaired a commit-count button obstructed by a merge marker. The graph still needs a layout refinement to make comparison practical across this repository.

## Verification

- Frontend regression: **958 tests passed across 169 files** after the repair (`npm test -- --maxWorkers=4`, exit 0).
- Focused native Worktree Review/application tests: **60 passed**, using real temporary Git repositories where applicable.
- Broader native regression: **689 distinct tests passed across two runs** of the compiled library test binary. The first serial run supplied 237 passing tests; a continuation excluded those exact names and passed the remaining 452 with four threads (exit 0). The redundant serial process was stopped after the excluded long-running test passed. `native-coverage.json` verifies every name against the complete test inventory. This is combined coverage, not a successful exit from the original interrupted Cargo invocation.
- Production frontend build: passed after the final repair; existing bundle-size warning remains.
- Targeted ESLint and Prettier: passed after the repair.

Logs and exact screenshots are in `.dev/branch-navigation/audit`. The initial and final frontend logs are separate. The new geometry regression failed before the repair; its failure is recorded in `overlap-regression-before.log`. The first repair exposed an existing branch-card overlap test; the final placement preserves interactive controls when marker clearance is constrained. Both geometry cases pass. A concurrent full frontend rerun also produced timeouts in other UI suites, so final verification uses four workers and unchanged test timeouts.

## Exercised flow

1. **Register and inspect the repository.** Used an isolated app-data directory and the real Git repository. The list showed all 14 eligible targets, with ten available-worktree targets before the four branches without worktrees. Lazy activity updates reordered rows while preserving the selected source.
2. **Open Select branch and preview lineage.** The graph opened as a native modal. Hovering `codex/workflow-engine-v1` changed highlighted connections from 211 to 194 while `main` remained selected; leaving the card restored main's preview. The footer supplied the full branch name and commit subject.
3. **Open a count and choose a historical commit.** The two-commit connection from `deecb2f7` to `ac22f015` listed exactly `ac22f015` and `e914a9e1`. Selecting the older commit opened the normal review surface, with no creation prompt. Focus returned to Select branch.
4. **Click Build and cancel.** Build displayed a checkout-creation prompt naming `e914a9e1`. Cancel preserved that commit and returned focus to Build. Git branch refs and physical worktree inventory matched the pre-navigation snapshots; the isolated database still contained zero builds and outputs.
5. **Check cancellation, refresh, and other targets.** Nested Escape closed only the commit modal and restored its range-button focus. Outer Escape restored Select branch. Refresh preserved the historical commit. A branch without a worktree and a physical detached worktree both opened the normal detail surface without prompting.
6. **Check zoom and a narrower viewport.** Inspected the graph at 85% and 40%, and the modal at a 960 × 720 CSS viewport. The narrower modal retained its close, zoom, and selection controls. The large graph still required extensive panning.
7. **Reproduce and retest the obstructed range.** A click at the center of the `f4e4c12d` → `deecb2f7` count hit a merge circle and opened no dialog. After repair, the same count passed the app-inspector click check and opened the expected one-commit dialog, containing `deecb2f7`.

## Findings

**P1 — Count obstructed by a merge marker: repaired.** Range placement avoided branch cards but ignored commit markers. SVG decoration could also intercept input. Placement now reserves marker/label space, and decorative paths and markers ignore pointer events. A regression test covers a marker crossing another connection. Native WebView hit checks found no obstructed count centers in the final visible graph.

Before the repair:

![Merge marker covering a commit count](<C:/Users/user/.codex/worktrees/d02e/Codex Orchestrator/.dev/branch-navigation/audit/05-graph-loading.png>)

After the repair, that count opens its actual commit:

![Formerly blocked range now opens its commit dialog](<C:/Users/user/.codex/worktrees/d02e/Codex Orchestrator/.dev/branch-navigation/audit/36-final-range.png>)

**P2 — Broad branch comparison remains difficult: open.** At 85%, the graph occupies approximately 32,487 × 7,475 CSS pixels inside a 1,360 × 536 viewport. Only three of fourteen branch cards are fully visible around main. At the minimum 40% zoom, ten cards become visible, but count buttons shrink to about 44 × 11 pixels and branch text becomes very small. All targets exist; the problem is navigating and reading their relationships. Reuse inactive lanes and compact historical topology while retaining legible controls; a fit-to-branches action alone would make the current canvas too small to read.

![Minimum zoom shows more branches but makes labels and controls very small](<C:/Users/user/.codex/worktrees/d02e/Codex Orchestrator/.dev/branch-navigation/audit/19-zoom40.png>)

## Accessibility and limits

Forward Tab navigation and nested Escape were exercised through WebView key events. Reverse Tab from the initial commit-dialog control briefly left `document.activeElement` on the body; the next Tab re-entered the dialog. Refresh also lost the initiating button's focus. These are follow-up keyboard observations, not a claim of complete focus containment or accessibility compliance. Physical keyboard, screen-reader behavior, contrast compliance, and touch use were not validated.

After the user's steering, all inspection and input used the repository's app-inspector transport and direct CDP commands scoped to the isolated app's owned WebView. No further Computer Use skill or desktop-input tool was used. The screenshots above are actual captures from this run, not fixture renderings. Normal captures used a 1500 × 880 CSS viewport; the narrower capture used device-metrics emulation.

The existing isolated database needed the previously documented empty legacy Sprint tables to start. Fresh-profile startup remains a separate limitation. This review did not confirm an actual build through the new UI, launch a retained build, or validate packaged/restart behavior. Automated native tests cover exact historical materialization. Production app data was not changed.

The repair is limited to `branchGraphLayout.ts`, `branchSelection.css`, and `branchGraph.test.ts`. The repair and report are local and uncommitted. No merge or publication was performed.
