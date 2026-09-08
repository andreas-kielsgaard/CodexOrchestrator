# UI and layout findings

These findings cover the missing features already raised by the user and layout failures found during this review. Screenshots use real feature components with fake data, not the native application database.

## U1. The flow canvas was replaced, not adapted

**Source comparison. Priority: P1 for restoring the accepted editing flow.**

The mounted Workflow screen is now `WorkflowAuthoringScreen`. It renders a node/connection outline and one editor. It does not render a canvas or use the saved node positions. The old canvas, drag behavior, connection drawing and node popup code still exist, but the product no longer routes to them.

- Product routing: `src/app/App.tsx:1099-1110`.
- New client mounted by `src/bootstrap/productApplicationComposition.ts:38-39`.
- Existing canvas and drag handlers: `src/features/workflows/WorkflowScreen.tsx:1073-1243`.
- Earlier reuse guidance: `docs/session-event-model/codebase-map.md` and `ui-mapping.md`.

The changed domain model did not require a list-only editor. A likely repair is to keep the new Node Profile and Session Event controls but host them from the existing flow layout, adjusting its data adapter as needed. This does not require choosing a graph for the future run UI.

## U2. Workflow instance creation and selection are missing

**Source comparison. Priority: P1.**

The previous Workflow landing page listed instances and offered **Create Workflow instance**. Creation selected a Workflow and a repository/worktree target. The instance page showed that target and its Sessions.

The new page has **Compile and run**, a free-text Instance ID and a message box. Its client has compile/dispatch calls, but no instance creation or listing contract. Users cannot find or reopen their instances there. A manually entered string scopes addressing; it does not restore instance storage, target selection or a pinned recipe.

- Previous creation/listing: `src/features/workflows/WorkflowScreen.tsx:155-342`.
- Previous instance target display: that file at `1577-1594`.
- New ID and run form: `src/features/workflowAuthoring/WorkflowRunPanel.tsx:28-106`.
- New contract: `src/application/workflowAuthoring/contracts.ts`.
- Backend consequence: [B3 and B4](backend-findings.md).

Restore the instance lifecycle as well as its entry points. Merely adding a **Create** button around the current ID would leave the worktree and pinning gaps.

## U3. A narrow window hides Workflow selection and creation

**Browser reproduction. Priority: P2.**

At 850 px wide, the recipe sidebar is hidden. It contains both the recipe list and **New Workflow**. There is no replacement menu or button, so a user can only edit the already selected recipe.

Evidence: `src/features/workflowAuthoring/workflowAuthoring.css:313-319` hides `.workflow-authoring-screen__recipes` below 900 px. The browser probe confirms it is not visible. [Screenshot](evidence/workflow-narrow.png).

Keep these actions available through a compact picker, drawer or a stacked layout.

## U4. Sections marked collapsed still show their content

**Browser reproduction. Priority: P2.**

The **Runtime profile** section starts collapsed: its button says **Expand** and its content has `hidden=true`. Yet all its fields are visible. The same CSS is used by other new profile sections.

The shared component sets `[hidden]` to `display: none` at `src/components/collapsibleSection.css:43`. Later feature CSS sets that same content to `display: grid` at `src/features/executionConfiguration/executionConfiguration.css:84`. Both selectors have the same specificity, so the later display rule wins.

Measured content: `hidden=true`, `display=grid`, height about 296 px. [Screenshot](evidence/collapsed-runtime-still-visible.png). This is a CSS integration failure, not missing collapse state in the React component.

Ensure feature layout styles only apply to open content, or otherwise preserve the shared hidden rule. Verify by measuring visible layout, not only the HTML attribute.

## U5. Checkbox sizing squeezes the labels

**Browser reproduction. Priority: P2.**

The global form rule gives all inputs `width: 100%` and a minimum height of 36 px (`src/styles.css:296-300`). The new catalog checkbox style does not reset that sizing. In the tested reasoning row, the input takes about 155 px of a 212 px label. The word **high** gets only 22 px and wraps.

[Screenshot](evidence/checkbox-label-wrap.png). The checkbox itself should have a small fixed footprint, leaving room for its text. Check the other new checkbox and radio controls against the same global rule.

## Status wording that needs updating

`docs/session-event-model/README.md`, under **Current checkpoint**, says the new controls are intentionally not mounted. They were mounted by `aaca806`. That paragraph is stale and should not be used as current implementation evidence. This review leaves historical guidance untouched; the review README records what is actually mounted at `4bded63`.
