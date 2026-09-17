# OTP catalogue master-detail interaction

Planned against the dirty `codex/OTP-Job-Agent` worktree after the OTP catalogue/detail slice. Preserve all existing uncommitted work. This plan does not change OTP contracts, package imports, Workflow routing, or MCP behavior.

## Objective

Replace package-page element detail dialogs with a persistent two-column view:

- The left column is a navigable list of the selected OTP's offered elements.
- The right column shows the OTP overview while no element is selected.
- Selecting an element replaces that overview with its existing typed details.
- Selecting that same element again clears the selection and restores the OTP overview.

The package directory continues to use its existing **View details** control because it opens a package page. The replacement applies to controls that inspect offerings within that page.

## Shape

- Keep `src/features/otp/otpElements.tsx` as the shared renderer for the selected-element details. Its typed contracts and existing picker consumers remain unchanged.
- Keep `src/features/otpCatalogue/otpCataloguePresentation.ts` as the descriptor-to-presentation boundary. Extend it with one ordered section/list model for a package: Workflow source and destination sections, then MCP service, capability group, and endpoint entries. It owns stable keys and nesting depth; components do not rebuild that hierarchy independently.
- Create `src/features/otpCatalogue/OtpPackageElementList.tsx`. It renders semantic section labels and native button rows for the ordered model. Button rows use a selected `aria-pressed` state, support normal keyboard activation, and expose nesting through presentation data. Hover and keyboard focus visibly highlight rows without presenting separate detail controls.
- Create `src/features/otpCatalogue/OtpPackageDetailPanel.tsx`. With no selected element, it renders the package's name, description, ID/version, and any package-specific configuration content. With an element selected, it renders the shared `OtpElementDetails` component for that element. Selection therefore replaces the right-panel content rather than appending an extra panel beneath it.
- Refactor `src/features/otpCatalogue/OtpPackageDetailPage.tsx` into the coordinator. It owns `selectedElementKey`, resolves the selected element from the presentation model, and toggles selection when the same key is clicked. It composes the left list and right panel; it keeps the existing back-to-directory behavior.
- Delete `src/features/otpCatalogue/OtpElementDetailsDialog.tsx`. The OTP package page no longer owns native-dialog focus or Escape handling. Existing picker dialogs retain their own dialog behavior.

## Layout and styling

- Replace the current stacked Workflow sections and embedded Job Agent service/group cards with `.otp-catalogue__master-detail`: a narrow, independently scrollable element list and a flexible detail pane. The list gets a viewport-relative maximum height and `overflow-y: auto`, so its own scrollbar handles long endpoint inventories without moving the selected detail out of view.
- Give selectable rows a neutral resting state, clear pointer affordance, hover/focus highlight, and persistent selected treatment. Group and endpoint depth is visible through indentation and subdued labels, without duplicating descriptions in the list.
- Move package overview content into the detail pane. The Job Agent installation form is package-level content, so it appears in the overview and returns when selection is cleared.
- Remove the package-page dialog selectors from `src/features/technicalSettings/technicalSettings.css`; keep shared `OtpElementDetails` styles. Add a responsive breakpoint that stacks the list above the pane on narrow windows.
- Preserve the established Technical Settings palette, borders, and typography.

## Tests and validation

- Update `src/features/technicalSettings/TechnicalSettingsScreen.test.tsx` to cover opening a package, its overview, selecting a Workflow source, selected state, selecting it again to restore the overview, and the absence of a dialog.
- Cover a Job Agent capability group and endpoint in the same master-detail flow, including endpoint schema/grant text in the right pane.
- Add a focused `OtpPackageDetailPage` test only if these cases become awkward at the settings-screen level; otherwise keep the behavior test in its current owner.
- Run the affected Vitest tests, `npm run build`, and `git diff --check`. Launch the existing isolated Orchid runtime and inspect the settings page at normal and narrow widths.

## Deliberate limits

No new element type, catalogue data field, selector behavior, directory search, expansion state, route, or separate Workflow-entry category. Package cards are not changed into selectable catalogue elements in this slice.
