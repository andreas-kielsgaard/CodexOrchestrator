# Workflow usability correction pass

Status: implemented on `refinement/workflow_usability`, 2026-09-24. Automated validation and native release build complete; screenshot-level native review was unavailable because this session exposed browser surfaces only.

## Target and boundaries

Correct the reviewed Workflow designer without expanding Orchid's workflow policy. The pass fixes sidebar sizing and instance creation feedback, makes canvas placement visibly grid-based, consolidates OTP selection UI, clarifies prompt-source vocabulary, and gives the `workflow/prompt_agent` action a purpose-built session-selection editor.

The earlier usability decisions remain the baseline: designs and instances use generated identities; instances receive generated display names; nodes independently opt into entry status; connections between the same node pair share one bidirectional visual; node MCP and skill access use grouped toggles; and Orchid does not expose OTP grant-level configuration. This pass does not reopen those choices.

No graph or form dependency is warranted. The existing graph renderer already owns the required geometry and interaction surface, and the existing grouped capability picker contains the useful tree behavior. The correction should consolidate and extend those local components rather than introduce a second canvas or form system.

## Proposed ownership

### 1. Navigation owns navigation, not reload or eligibility policy

Remove the refresh action and `onReload` prop from `WorkflowNavigation.tsx`. Keep its header purely presentational and constrain every child to the sidebar content box. Narrow the instance-row CSS selector so it cannot restyle the shared create button. The create-design and create-instance actions should use the same full-width button component/style.

The instance-create button remains clickable whenever instance creation is available in the application composition. It no longer encodes the presence of an active revision. `RecipeInstanceCreationDialog.tsx` owns that state: with no active designs it renders the exact empty state **No active designs found**, plus the normal close affordance, and does not render a target selector or disabled submission form. With active designs it renders the existing creation flow.

Keep activation as the stable-revision boundary; do not silently activate a draft and do not instantiate an unsaved revision.

### 2. One authoring bootstrap read, then lazy secondary catalogues

The current first open performs `listRecipes`, waits, then performs `loadRecipe`, while instances and the complete execution-configuration catalogue also load immediately. Design count therefore does not determine the observed delay.

Add a bounded authoring bootstrap query to `WorkflowAuthoringClient`, for example `openWorkspace(preferredRecipeId?)`, returning recipe summaries and the selected recipe state in one application call. Implement it through `authoring_service.rs` and `authoring_transport.rs`; the repository keeps its existing list/load primitives. Register one Tauri command and replace the sequential frontend effect. Retain `listRecipes` and `loadRecipe` for later refreshes and explicit selection rather than turning every read into a large workspace payload.

Do not add a general cache. Load instances when the Instances tab is first opened and after an instance mutation. Enable `useExecutionConfigurationCatalog` only after a design needs the editor catalogue; the navigation shell and loaded graph should not wait for profiles, identities, or the OTP catalogue. Let `WorkflowNavigation` report tab changes to the screen so data ownership remains in `WorkflowAuthoringScreen.tsx`.

Measure the native development build before and after with per-command timings. The acceptance criterion is removal of the serialized list-then-load round trip and immediate shell/navigation rendering; do not claim that all cold-start latency is solved if Tauri or database initialization remains visible.

### 3. Graph geometry is one model, and the surface fills its viewport

Make `workflowGraphModel.ts` the single owner of a 20 px grid and fixed 220 × 100 px node geometry. Use those constants in node rendering, connection endpoints, midpoint placement, bounds, and drag clamping. Remove the unrelated 210 px CSS width, 92 px minimum height, and hand-maintained drag gap constants.

`WorkflowGraphSurface` should fill its scroll viewport while still growing when graph content exceeds it. Give the rendered canvas `min-width: 100%` and `min-height: 100%`, retain content bounds for overflow, and expose the actual surface element to `WorkflowCanvas`. Placement and drag projection use the larger of the content bounds and current surface client dimensions, so the visual canvas and interaction bounds cannot diverge.

While Add node or Copy node is armed, `WorkflowCanvas` tracks the pointer over the surface and renders a non-interactive translucent node-sized placement preview. The preview and committed node use the same projection function. Normal movement snaps to the 20 px grid; Alt bypasses snapping for both existing-node dragging and placement. Clear the preview when the pointer leaves, the brush changes, or placement completes. Grid lines remain visible only during placement/copy or an active drag.

Keep placement state local to `WorkflowCanvas`; persisted nodes and `WorkflowAuthoringScreen` should only receive the final coordinates.

### 4. One shared grouped element picker

Move `GroupedCapabilityPicker.tsx` out of `features/executionConfiguration` into `components/otp` and rename it for its actual scope, for example `GroupedElementPicker`. Extend it with explicit single- and multiple-selection modes and an empty-node presentation. A node with no selectable descendants renders a static label and status, without a checkbox, caret, or expand button.

Adopt the shared picker in:

- `OtpMcpToolsPicker.tsx` and `SkillsPicker.tsx` for multiple selection and group toggles;
- `WorkflowTriggerPicker.tsx` for single selection;
- `WorkflowDestinationActionPicker.tsx` for single selection.

The trigger picker supplies every OTP package as a top-level row. Packages with triggers remain expandable; packages without triggers render **No triggers** and do not appear collapsible. Keep details preview separate from committed selection and preserve unavailable saved selections.

Delete `components/otp/OtpElementPicker.tsx` and its competing group implementation after both workflow consumers migrate. Consolidate its tests into the shared picker tests so preview, cancellation, single selection, multiple group toggling, unavailable values, and static empty groups have one behavioral specification.

### 5. Prompt-source presentation maps cleanly onto the existing contract

Keep the persisted `node_files` source and its `created | edited | either` association; no backend or recipe-contract migration is needed. Add a small presentation mapping in `WorkflowPromptInputsEditor.tsx` (or an adjacent pure module) with these authoring choices:

- **Source Node Output** → `output_field`;
- **List of files edited by node** → `node_files/edited`;
- **List of files created by node** → `node_files/created`;
- **List of files created or edited by node** → `node_files/either`;
- **File content** → `file_content`.

Because three visible choices map to one persisted kind, the source-type control should use a presentation key and pure conversion functions rather than overloading `Input['kind']`.

Remove the generated per-entry headline. Retain move/delete actions with ordinal-only accessible labels. Rename the subordinate output selector to **Field** so it does not repeat the source choice. For node-file sources, the association is selected by the source type, so remove the second File association dropdown and retain only the node selector.

### 6. Destination action has a stable chooser and an action-owned editor

Keep `WorkflowDestinationActionPicker.tsx` responsible for the top action chooser. Beneath it, render a bordered configuration card using the same `session-event-editor` visual language as Prompt logic. The hierarchy and spacing communicate ownership; do not add prose explaining that the lower fields configure the selected action.

Keep `OtpConfigurationEditor.tsx` as the generic fallback for actions whose scalar catalogue fields are sufficient. Add `WorkflowPromptAgentConfigurationEditor.tsx` for `workflow/prompt_agent`; do not force list filters and conditional session targeting into the generic scalar renderer.

Present the prompt-agent values with capitalized product labels while retaining stable internal identifiers where the semantics have not changed:

- Session mode: **Continue a session** (`select`) or **Start a new session** (`new`).
- **Sessions to prompt**: **One session** (`first`) or **All sessions** (`all`).
- **Session selection logic**: **Newest session** (`newest`) or **Most recently addressed session** (`last_addressed`).
- If no session matches: **Create a new session** (`create`), **Send failure event** (`fail`), or **Do nothing** (`noop`).

Show Session filters before Session selection logic. Render filters as an addable/removable list styled like prompt sources. The available filter types are **Idle sessions**, **Created by event**, and **Created by session**. Each filter has an **Include / Exclude** segmented toggle; provenance filters additionally require an Event ID or Session ID. Permit each filter type at most once and combine configured filters with AND semantics. Filter order has no runtime meaning, so do not add reorder controls.

The existing backend configuration cannot express exclusion for creator provenance. Replace `running`, `createdByEvent`, and `createdBySession` in `otp_packages/workflow/prompt_agent.rs` with a typed `filters` list shared in shape with the frontend editor. `Idle sessions / Include` accepts only non-running sessions; Exclude accepts only running sessions. Creator Include requires equality and Exclude requires inequality. Validate unique filter kinds and non-empty provenance IDs. Keep the remaining wire values stable, update the serialized OTP catalogue fixture, and cover the new predicate behavior in the package tests.

This is pre-release correction work, so do not add a permanent compatibility layer for the superseded scalar filter keys. Unknown old keys continue to fail validation rather than being silently reinterpreted. Existing empty/default configurations remain valid.

Short consequence text may explain the selected cardinality, ordering, or no-match outcome where labels alone remain ambiguous. It belongs with that control; do not add a generic section description.

## Concrete change surface

### Adapt

- `src/features/workflowAuthoring/{WorkflowAuthoringScreen,WorkflowNavigation,RecipeInstanceCreationDialog,WorkflowCanvas,WorkflowTriggerPicker,WorkflowDestinationActionPicker,WorkflowConnectionEditor,WorkflowPromptInputsEditor,OtpConfigurationEditor}.tsx`
- `src/features/workflowAuthoring/{workflowAuthoring,workflowCanvas,recipeInstanceCreationDialog}.css`
- `src/features/workflowGraph/{workflowGraphModel,workflowNodeDrag,WorkflowGraphSurface}.ts*` and `workflowGraph.css`
- `src/features/executionConfiguration/{OtpMcpToolsPicker,SkillsPicker}.tsx`
- `src/application/workflowAuthoring/contracts.ts`
- `src/infrastructure/workflowAuthoring/tauriWorkflowAuthoringClient.ts`
- `src-tauri/src/workflows/{authoring_service,authoring_transport}.rs` and command registration in `active_app.rs`
- `src-tauri/src/otp_packages/workflow/prompt_agent.rs`
- `src/features/workflowAuthoring/otpCatalogue.fixture.json`

### Create or extract

- `src/components/otp/GroupedElementPicker.tsx`, extracted from the execution-configuration feature and extended for single selection and empty groups.
- `src/features/workflowAuthoring/WorkflowPromptAgentConfigurationEditor.tsx` plus a small typed presentation/configuration module if keeping conversion logic out of the component materially improves its tests.
- An authoring bootstrap DTO at the existing application contract boundary; no new repository abstraction.

### Remove

- `src/components/otp/OtpElementPicker.tsx` and its superseded tests after all consumers move.
- `src/features/executionConfiguration/GroupedCapabilityPicker.tsx` after extraction.
- Refresh-button imports, props, styling, and callback plumbing.
- The broad `.workflow-instance-list button` rule, generated prompt-source headlines, node-file association dropdown, and old prompt-agent scalar filter fields.

## Implementation sequence

1. Extract and test the shared grouped picker, then migrate MCP tools, skills, triggers, and actions and delete the two old picker implementations.
2. Correct navigation sizing/button selectors and the instance-dialog empty state.
3. Add the bootstrap query and lazy secondary loads; record before/after native timings.
4. Unify graph constants, make the surface viewport-aware, then add the shared projection and placement preview.
5. Simplify prompt-source presentation without changing persisted recipe contracts.
6. Add the prompt-agent typed filter contract and specialized editor, update the OTP fixture, and restructure Destination action.
7. Run focused suites throughout, then broad frontend/Rust validation, build, launch, and review the real native workflow at the widths and flows that exposed the defects.

## Verification

- Sidebar: no refresh control, no horizontal clipping at normal and narrow desktop widths, and matching create-button presentation on both tabs.
- Instances: clicking Create instance with no active design opens a modal containing **No active designs found**; an active design still creates an instance against its active revision.
- Startup: one authoring bootstrap command supplies summaries and the selected draft; instances and configuration catalogues do not gate first navigation/graph paint. Record command timings and visible ready time in the native development app.
- Canvas: the grid covers the whole visible surface, nodes are exactly 11 × 5 cells, placement preview and committed position agree, Alt bypasses snapping, and content beyond the viewport remains scrollable.
- Pickers: MCP and skill group toggles retain behavior; triggers/actions are single-select; all OTP packages appear in the trigger modal; empty packages have no collapse affordance and say **No triggers**.
- Prompt sources: all five product choices round-trip to the existing DTO shapes, have no generated headline, and created/edited/either each remain independently authorable.
- Destination action: action control is above its configuration card; every visible option is capitalized and uses the approved wording; Continue hides/shows the correct fields; Start a new session hides selection-only fields.
- Session filters: include/exclude semantics for idle, creator event, and creator session work independently and in combination; duplicate kinds and blank IDs are rejected; One/All, ordering, and all three no-match outcomes retain runtime behavior.
- Automated coverage: update Workflow authoring composition tests, add navigation/dialog empty-state tests, extend workflow graph and drag tests, consolidate grouped-picker tests, add prompt-source mapping tests, and extend Rust `workflow::prompt_agent` tests.
- Final checks: focused Vitest and Rust package tests, full frontend type/build, relevant Rust suite, native build/launch, and screenshot-led review of sidebar, placement, trigger picker, prompt sources, and destination action.

## Explicit non-goals

No draft instantiation, automatic activation, new OTP grant controls, dynamic node sizing, free-form node resizing, graph-library migration, general form-schema framework, workflow execution-policy change, or speculative caching layer.
