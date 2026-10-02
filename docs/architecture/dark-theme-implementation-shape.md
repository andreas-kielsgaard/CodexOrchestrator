# Dark theme implementation shape

Branch: `feature/dark-theme`. Discovery baseline: `a1b3c70d`, clean checkout.

## Choices made during discovery

- Use a dark default with charcoal surfaces, soft white text and a restrained green interaction accent. Keep distinct success, warning, danger, information and concern colors for meaningful states.
- Own the palette in `src/theme/tokens.css`. Consumers use semantic CSS custom properties; feature files retain layout and state selectors. Native CSS variables and `color-scheme` meet the requirement without a theme library or provider.
- Extract shared modal styling from Worktree Review into `src/components/modalDialog.css`. The shared `ModalDialog` currently depends on a feature stylesheet and carries a Worktree Review class, even when used by Agent Sessions and other features.
- Replace the branch graph's literal palette and Session navigation's arbitrary HSL hues with references to one curated, dark-readable lane palette. Keep existing lane assignment and branch hashing.
- Keep user identity colors and the contrast calculation for identity initials as data. The two apparent inline UI colors are identity defaults, not presentation overrides. Images and attachment content also retain their original colors.

No discussed decisions remain inconclusive.

## Prescribed outcome

Create a dark theme without stray unthemed views or clashing colors. Examine the whole change surface, clean up styling ownership where needed, and apply `plan-implementation-shape` before implementation. Work on `feature/dark-theme`.

## Discovery and coverage

The source audit parsed all 54 CSS files: 1,782 literal-color occurrences and 869 distinct values. Colors are embedded in backgrounds, text, borders, outlines, shadows, gradients, SVG fill/stroke, checkbox accents and variable fallbacks. There is no shared color palette. Most existing custom properties describe geometry or user identity.

| Surface                             | Styles to migrate and states to cover                                                                                                                       |
| ----------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Application shell                   | `styles.css`: navigation, command bar, notifications, initiation confirmation, errors and loading                                                           |
| Shared controls                     | Catalog selectors, Markdown editor/content, collapsible sections, resolved fields, validation and OTP picker                                                |
| Agent Sessions                      | Workspace/transcript, navigation, composer and hover menus, preparation, runtime controls, queue, import, device activity, target selection and transitions |
| Identity                            | Picker, previews and badges; preserve user-selected marker backgrounds and computed foregrounds                                                             |
| Harness and execution configuration | Harness editor/inspector/history, inherited/resolved fields, profile/capability editors, route/model/skill pickers and dialogs                              |
| Technical Settings                  | Devices, inference, Codex/Claude setup and OTP catalogue/package views                                                                                      |
| Workflows                           | Authoring, recipes, instance creation/runtime, node/connection selection, grid, SVG edges/arrowheads/labels and dangling states                             |
| Repository and Worktree Review      | Navigation, registration, detached worktrees, build history/logs, readiness, source selection, commit ranges and branch graphs/tooltips                     |
| Files and diffs                     | File list, modes, unified/split changes, additions/removals, line numbers, Markdown, raw content and errors                                                 |
| Retained orchestration              | Epic list/detail/plan builder, Sprint workspace/dialogs, plans/flow maps, work-unit/planning detail, activity, continuation, concerns and documents         |
| Product Decisions                   | Hierarchy, evidence/citations, dialogs, publish placeholder and contextual navigation                                                                       |
| Native/loading surfaces             | HTML color-scheme metadata; Tauri window theme and initial background                                                                                       |

## Ownership and edits

1. **Create** `src/theme/tokens.css` for canvas, surface levels, controls, borders, text levels, selection, focus, semantic states, shadows/backdrops and graph lanes. Import it through `styles.css` so every route and body-mounted dialog inherits it.
2. **Adapt** all audited feature/component styles to consume those roles. Collapse near-identical old green/blue/beige neutrals into the same neutral surface and text families. Review accent treatment by purpose: primary action, selected item, lifecycle status and diff content. Remove competing literal palettes rather than adding an override stylesheet.
3. **Extract** shared dialog foundation into `components/modalDialog.css`, imported by `ModalDialog.tsx`. Rename its foundation class to `modal-dialog`; update the Session configuration selector. Keep Worktree Review sizing/actions beside that feature. Preserve existing native modal behavior.
4. **Extract** branch color hashing from application navigation into `src/theme/graphColors.ts`, alongside the shared lane references. Both Session navigation and branch graph consumers use it without changing graph geometry or navigation ownership. Preserve identity presentation and data contracts.
5. **Adapt** global browser controls: themed default button background/border and inherited text, checkbox/radio accent, placeholder/caret, selection and focus defaults. Use `color-scheme: dark` for browser-owned controls and scrollbars.
6. **Adapt** `index.html` and `src-tauri/tauri.conf.json` so document startup, window chrome and the WebView background agree with the dark canvas.
7. **Create** focused theme checks for full-source literal-color coverage, defined token references, readable text/status contrast and native background agreement. Keep visual evidence in ignored `.dev/dark-theme/` and record verified coverage/limitations in this document.

## Execution and validation

Migrate the foundation and shared controls, then all feature styles and graph colors. Review migrated rules for role mismatches rather than treating matching old hex values as equivalent purposes.

Run frontend compilation, focused theme/branch/modal tests, the frontend suite and diff checks. Validate Tauri configuration through the normal native build/check path. Use the existing recorded development routes and isolated browser review tooling for screenshots and interactions; inspect populated transcripts, retained orchestration detail, Harness, file diffs and dialogs. Check other screens plus hover, selected, disabled, error and focus styling. A browser preview does not establish native title-bar/dialog behavior or provider execution.

## Reference checks

- [CSS color-scheme](https://developer.mozilla.org/en-US/docs/Web/CSS/Reference/Properties/color-scheme): browser-owned controls/scrollbars and early HTML metadata.
- [Tauri WindowConfig](https://v2.tauri.app/reference/config/#windowconfig): `theme` and `backgroundColor`.

## Implementation and validation record

Implemented the shared dark palette, migrated every audited stylesheet, removed obsolete fallback token names, extracted the shared modal foundation, and moved presentation color hashing out of application navigation. Identity data and its foreground contrast calculation remain intact. Branch graph clearances match the canvas; inactive graph connections retain at least 3:1 contrast after opacity. Status text and numbered marker fills use separate roles.

- Final full frontend suite: 178 files, 867 tests passed, including 7 palette, source-coverage and contrast checks.
- Final frontend production build, Rust/Tauri check and isolated native development build passed. Rust reported existing unused/dead-code warnings; the frontend reported existing chunk-size/dynamic-import warnings.
- Changed TypeScript passed ESLint. Changed files passed formatting and diff checks.
- Source checks cover all stylesheets, token definitions, foreground/background pairs, semantic contrast, graph visibility and document/native startup configuration.
- Live review used Orchid's app-inspector and owned-WebView CLI controls, with exact executable/PID/debugger ownership validation. The review instance used a separate app identifier and `.dev/dark-theme/native-data`; the normal app remained running. Native titlebar and canvas were dark. Screenshots and rendered style reports are retained in `.dev/dark-theme/`.
- Populated review: Epic list/plan, Sprint flow and documents, Work Unit detail, Product Decisions, Agent Session transcript/composer/new-session surface, Harness conversation/editor/skill picker, unified/split diffs, and registered-repository Worktree Review with branch-history graph and registration/selection dialogs.
- Native setup review: Technical Settings including its existing update-error state, Capability Profile creation and execution-route form, and empty/draft Workflow canvas. Focus outline was visible on the workflow title input; selected/disabled and modal surfaces were inspected.
- The proposed-Sprint review found a browser-default gray button background: the neutral state selector covered only noninteractive rows. Both variants now use the same themed surface, and global button defaults provide a palette-backed fallback.
- Every reviewed final rendered style report found no small-text contrast failures or large bright presentation surfaces. These checks sample visible text and exclude disabled/faded elements; they do not establish every possible CSS cascade, keyboard interaction or content/identity color contrast.
- External OS file/attachment dialogs and every populated runtime/catalog state were not visually exercised. Source coverage and automated theme checks cover their application styles; no provider execution or user acceptance is claimed.

The initial implementation was completed locally on `feature/dark-theme`. The main integration is recorded below.

Follow-up [design-guided regression review](dark-theme-regression-review.md) corrected Sprint connector contrast, added connector coverage, and passed 868 frontend tests plus 39 CLI inspector tests. The separately requested branch-drop change was withdrawn and fully rolled back.

## Main integration, 2026-10-02

Committed the theme as `f4e71b43`, integrated main `98796107` as `2f7c755a`, and aligned the new overlay controls as `61325cc5`. Resolved the two CSS conflicts by retaining the new operation-group selector and queue geometry alongside the theme tokens. Migrated main's new completion indicators, branch hover, worktree cards, Git dialogs and quote controls. The source was validated before fast-forwarding local main; the rebuilt release executable is included in the subsequent rebuild commit. Nothing was pushed.

Final combined validation: 180 frontend files / 882 tests, 39 CLI inspector tests, frontend production build, Rust check, native release build, changed TypeScript lint, CSS formatting and diff checks passed. Existing React test warnings, Rust unused-code warnings and frontend chunk warnings remain.

The additional review used real overlay components with disposable fixture clients in an isolated native WebView: branch menu, worktree card, Git preview, steering editor, quote menu and optional-comment form. All 78 rendered text/control samples passed contrast checks; minimum sampled contrast was 5.72:1. Branch hover, filled/hollow completion markers and keyboard focus were also inspected. The review caught browser-default bevelled buttons in the new Git/worktree controls, which now use explicit shared-palette styling.

A bundled debug review variant of the final source, with a separate application identifier and data directory, passed startup and Agent Sessions navigation checks. Its native titlebar, document scheme and canvas were dark. The production release binary was built separately with the normal identifier. Provider execution and real Git mutations were not part of this review.

Integration evidence and logs are retained under `.dev/dark-theme/` in `C:/Users/user/.codex/worktrees/dark-theme-main/Codex Orchestrator`. The production build is retained at `C:/Users/user/AppData/Local/CodexOrchestrator/dark-theme-main-20261002-final/output`.
