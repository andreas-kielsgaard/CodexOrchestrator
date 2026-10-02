# Dark theme regression review

Reviewed on `feature/dark-theme`, 2026-10-01. Changes remain local and uncommitted.

## Design criteria

- Evaluate hue, lightness and saturation separately. Keep large surfaces neutral and reserve stronger colors for meaningful emphasis. [Adobe color theory](https://www.adobe.com/uk/creativecloud/design/discover/color-wheel.html), [Atlassian color](https://atlassian.design/foundations/color).
- Separate canvas, content and overlays by surface lightness; shadows alone provide weak depth cues in a dark UI. [Atlassian elevation](https://atlassian.design/foundations/elevation/).
- Require 4.5:1 contrast for ordinary text, including placeholders; 3:1 for large text and essential graphics/control indicators. Assess rendered colors after transparency and opacity. [W3C text contrast](https://www.w3.org/WAI/WCAG22/Understanding/contrast-minimum.html), [non-text contrast](https://www.w3.org/WAI/WCAG22/Understanding/non-text-contrast.html).
- Check hover, selection, focus and disabled states. Status meanings should have text or shape cues alongside color. [Atlassian interaction colors](https://atlassian.design/foundations/color), [W3C use of color](https://www.w3.org/WAI/WCAG22/Understanding/use-of-color.html).

## Findings and correction

The reviewed palette uses subdued charcoal surfaces, a consistent mint action/selection accent, pale text and muted semantic tints. Completed, active and pending work remain distinct through labels and icons. Unified diffs retain addition/removal signs. The warm keyboard focus outline is distinct from the mint hover treatment. The route overlay is lighter than its background, with a dark backdrop and readable controls. These are screenshot-based design judgments, not a universal aesthetic or accessibility certification.

Sprint connectors used `--border-strong`, producing **2.91:1** contrast against their canvas. Changed `.sprint-flow__connector` to `--graph-edge`, producing **5.54:1**. Added a test checking the actual workflow and Sprint connector stroke roles against their canvases. It failed before the correction and passes afterward. The live Sprint snapshot confirms the corrected stroke.

The branch-drop request was withdrawn. All changes made for that request, its tests and its temporary review fixture were rolled back. The drag hook and selector test file match HEAD; the existing theme changes remain.

## Regression results

| Check                                               | Result                                                                            |
| --------------------------------------------------- | --------------------------------------------------------------------------------- |
| Final frontend suite after rollback                 | 178 files, **868 tests passed**                                                   |
| Orchid CLI inspector suite                          | **39 tests passed**, no skips                                                     |
| Frontend production build                           | Passed; existing chunk-size/dynamic-import warnings                               |
| Changed TypeScript lint, formatting and diff checks | Passed                                                                            |
| Rendered text, input/placeholder and select checks  | **493 samples across 11 states**, no failures; lowest sampled contrast **5.27:1** |

Live review used the existing Orchid owned-WebView CLI, checking executable/PID and loopback debugger ownership. The native review instance used separate application data. Screenshots and audit JSON are retained under ignored `.dev/dark-theme/`.

| Reviewed state                                        | Samples |
| ----------------------------------------------------- | ------: |
| Epic overview                                         |      15 |
| Epic plan with completed, active and proposed Sprints |      46 |
| Sprint mixed-state flow                               |      51 |
| Unified diff                                          |      98 |
| Split diff                                            |     106 |
| Session transcript and composer                       |      24 |
| New Session composer and placeholder                  |      21 |
| Technical Settings including update-error state       |      37 |
| Capability Profile editor                             |      23 |
| Execution-route overlay                               |      11 |
| Registered repository Worktree Review                 |      61 |

The proposed-Sprint hover and keyboard focus were exercised separately. Selected and disabled states were inspected in the diff, composer and profile screens. Rendered audits composite ancestor background colors and opacity, filter clipped/occluded content, and exclude inactive controls. Decorative background gradients were inspected visually rather than treated as solid backgrounds.

Evidence examples: [Sprint](../../.dev/dark-theme/regression-sprint.png), [diff](../../.dev/dark-theme/regression-diff.png), [route overlay](../../.dev/dark-theme/regression-route-dialog.png). Logs: `.dev/dark-theme/regression-final-tests.log`, `regression-inspector-tests.log`, and `regression-build.log`.

External OS dialogs, all runtime states, color-vision simulations and arbitrary user identity/image colors were not covered by this review. Provider execution and full native accessibility are outside its evidence.
