# Workflow Designer correction pass

Status: proposed implementation shape, 2026-09-23. Planning only.

## Target and boundaries

Make Workflow authoring read as a direct graph editor: create an unnamed design, configure nodes and connections on a calm canvas, choose entry-capable nodes on the nodes themselves, and move between designs and their instances from one unclipped sidebar.

This pass does not add general connection conditions, node auto-layout, multi-node selection, canvas zoom/pan, instance renaming, or OTP MCP mutation grants. Existing Capability Profile and Agent Session correction work in the dirty checkout must be preserved.

## 1. Replace the single initial destination with entry nodes

Change the recipe contract from one `startingNodeId` plus configurable `entryAction`/`entryConfiguration` to `entryNodeIds`.

- Add an entry toggle to the top-right of each node card. Show it on hover and keyboard focus; keep a persistent entry marker while enabled.
- Remove **Initial destination** and **Set as start** from `WorkflowAuthoringScreen.tsx` and `WorkflowCanvas.tsx`.
- Delete `WorkflowDestinationActionPicker.tsx` and its initial-entry tests when it has no remaining consumer.
- Activation requires at least one entry node and rejects missing node references.
- A user request to an entry node always invokes `workflow/prompt_agent` in `new` mode. Session creation continues to use that node's identity, initial prompt, capabilities, default model, and default reasoning.
- Reject requests to non-entry nodes at the application boundary. If a caller omits the node, accept it only when the design has exactly one entry node; otherwise require an explicit node.
- Bump the recipe contract and migrate stored recipes by converting the old starting node to a one-element entry-node list, then discard the old configurable entry action.

Keep this change in the Workflow recipe/application contract rather than teaching the shared graph components about runtime entry semantics.

## 2. Make capability selection one reusable grouped experience

Replace the flat MCP and skill selection controls with one reusable grouped picker owned under `src/features/executionConfiguration/`.

- Extract the modal shell, hierarchical list, selection reducer, and parent/child checkbox behavior from the current OTP picker into a neutral grouped capability picker.
- Support checked, unchecked, and indeterminate states at source, service/package, capability-group, and leaf levels.
- Keep selection local until **Apply selection**. Preserve unavailable saved selections and allow their removal.
- Build MCP groups from Codex-profile servers and OTP package -> MCP service -> capability group -> endpoint metadata. Continue to render the existing OTP server/group/endpoint details in the right pane.
- Add a Skills picker using the same shell, grouped by Codex profile, Orchid, or OTP package/skill root. Extend catalog presentation with explicit group metadata instead of parsing display labels.
- Keep the compact field summary and **Set MCP tools** / **Set skills** launch buttons in `CapabilitySetFields.tsx`.

Remove `AgentMcpConfigurationEditor.tsx`, `OtpConfigurationEditor.tsx`, `agentMcpConfiguration` from Workflow node DTOs and Rust recipe/session-creation projection, and the Job Agent grant editor from Orchid. Endpoint toggles are the only Orchid MCP configuration in this pass. A contract migration should discard prior default-deny grant values rather than maintain a second configuration system.

## 3. Flatten node defaults

In `NodeProfileEditor.tsx`, replace the **Pinned defaults** disclosure with an always-visible two-field row:

- **Default model**;
- **Default reasoning**.

Remove the explanatory sentence and the repeated **Profile device runtime** source labels from the node editor. Runtime locks remain enforced by the existing catalog/default resolution; an actually locked field may communicate its disabled state locally.

Do not fold this into the currently dirty `CapabilityProfileEditor.tsx` route-card work. Reuse `RuntimeDefaultsFields`, but give it a presentation option or a small node-owned wrapper so Capability Profile consumers do not inherit Workflow-specific layout.

## 4. Give the canvas an interaction-only grid

Keep coordinate persistence unchanged. Add grid behavior to the shared graph geometry layer:

- Define one 20px grid constant and pure `snapWorkflowPoint`/drag projection helpers in `src/features/workflowGraph/`.
- Snap newly placed and copied nodes to the nearest grid point.
- Snap drag previews and the final committed position by default. Holding Alt during the pointer drag uses the unsnapped clamped position; releasing Alt resumes snapping before drop.
- Preserve one draft/history mutation on pointer-up rather than writing every preview.
- Keep keyboard movement grid-aligned. Alt remains the existing keyboard movement modifier; the snap bypass applies specifically to pointer dragging.
- Replace the permanently visible dotted background with line-grid styling enabled only while the Add/Copy placement brush is active or a node is being dragged.

`WorkflowCanvas.tsx` owns interaction state and CSS classes. `workflowNodeDrag.ts` owns geometry. `WorkflowGraphSurface.tsx` remains a reusable renderer and receives only presentation state needed to show the grid.

## 5. Project parallel connections as one visual edge

Keep every connection as an independent durable definition. Add a graph presentation projection that groups connections by ordered `source -> destination` node IDs.

- Render one line and one label per group. A multi-connection label shows the count rather than overlapping connection names.
- Activating a single-member group opens its existing connection editor directly.
- Activating a multi-member group opens an accessible modal listing each connection by name and trigger/action summary. Choosing an item closes the modal and opens the existing editor for that connection.
- Deleting or editing a connection continues to address its stable connection ID.
- Reuse the same grouped-edge projection in the instance graph. Its modal lists the member connections and their activity counts before opening the existing connection inspector.

Put grouping and aggregate-edge types in `src/features/workflowGraph/`; keep authoring/runtime modal content in their respective feature areas. Opposite directions remain separate groups because their arrows and routing semantics differ.

## 6. Rebuild the left column around Designs and Instances

Extract the sidebar from `WorkflowAuthoringScreen.tsx` into a focused Workflow navigation component.

- Add a two-option tablist at the top: **Designs** and **Instances**.
- Designs shows one **Create design** action and the design list. Creation sends no name, persists a generated recipe ID, opens the blank design, and focuses the existing name field in the main toolbar.
- Treat a design name as optional display metadata. Saving, caching, activation, instance creation, and routing use recipe IDs. Show **Untitled workflow** only as a UI fallback; do not persist that fallback as the name.
- Allow empty recipe names in the frontend and Rust storable/activatable contracts. Change `createRecipe(name)` and its Tauri command to `createRecipe()`.
- Instances shows **Create instance** and collapsible groups keyed by recipe ID. Prefer the current design name for the group label, fall back to the instance's pinned recipe name, and finally **Untitled workflow**. Keep instances for deleted/unavailable designs visible under their pinned recipe identity.
- Preserve the separate instance name field and generated instance ID; this pass removes the design-name creation dependency, not instance labels.

Replace the sidebar's fixed four-row grid with one header/tab/action/list composition. Give the active list the only scrolling region; apply `min-width: 0`, `max-width: 100%`, and border-box sizing to tab, header, action, group, and item content so controls and long labels cannot cross the divider. The narrow layout should deliberately restack rather than rely on clipped desktop columns.

## 7. Reduce `WorkflowAuthoringScreen` to orchestration

`WorkflowAuthoringScreen.tsx` currently owns loading, draft concurrency, creation, sidebar rendering, graph editing, instance navigation, and inspector composition. Keep request coordination and draft ownership there, but extract:

- `WorkflowNavigation.tsx` for tabs, design list, grouped instances, and create actions;
- `WorkflowConnectionGroupDialog.tsx` for authoring aggregate selection;
- shared grouped-edge and grid helpers under `src/features/workflowGraph/`;
- the reusable grouped capability picker under `src/features/executionConfiguration/`.

Do not introduce a new global store. Existing `DraftWorkspace`, stable recipe/instance IDs, and caller-owned route callbacks remain the state boundaries.

## Implementation sequence

1. Introduce recipe contract V3: optional design name, `entryNodeIds`, fixed fresh-session entry behavior, and removal of entry-action and agent-MCP-grant fields. Update migrations, transports, fixtures, and compilation first.
2. Extract the Workflow sidebar, switch creation to ID-only blank designs, add Designs/Instances tabs, and group instances by recipe ID. Correct clipping and responsive ownership with the new structure.
3. Replace start-node UI with accessible per-node entry toggles and remove the initial-destination picker and obsolete tests/components.
4. Add shared snap geometry and interaction-only line-grid presentation; adapt placement, copy, drag, and keyboard tests.
5. Add directional connection grouping and authoring/runtime group dialogs while retaining independent connection editors and records.
6. Add the reusable hierarchical capability picker, move MCP and skills onto it, then remove OTP MCP grant configuration end to end.
7. Flatten node defaults and remove the requested explanatory/source text without disturbing the concurrent Capability Profile route-editor correction.
8. Run focused contract/component tests, frontend and Rust builds, then launch the native app and exercise creation, save/reload, activation, instance grouping, entry dispatch, grid bypass, parallel connections, MCP/skill bulk toggles, keyboard focus, and narrow-window behavior.

## Acceptance

- Creating a design requires no name and immediately opens a blank, persisted design identified by generated ID. It can be saved, activated, and instantiated while unnamed.
- The sidebar never paints controls or text across its divider. Designs and grouped Instances are separate tabs, and collapsed instance groups retain their state while the screen remains mounted.
- Any number of nodes can be marked as entry nodes. Entry state is visible without hover, and the toggle is reachable by keyboard. Starting an entry always creates a fresh Session using node defaults.
- The canvas is visually quiet at rest. A 20px line grid appears during placement/drag, nodes snap by default, and Alt+pointer-drag produces the exact unclamped-to-grid position while remaining inside canvas bounds.
- Parallel connections in the same direction render as one edge. The aggregate modal exposes every underlying connection without merging their durable identity or behavior.
- MCP services/capability groups and skill sources support bulk, partial, and leaf selection with correct indeterminate states. No OTP grant configuration appears in Workflow authoring or its persisted node contract.
- Default model and reasoning are always visible; the removed explanatory and **Profile device runtime** text does not appear.
- Existing dirty Agent Session and Capability Profile work remains intact, and native screenshot/interaction review confirms the requested visible behavior rather than relying on component tests alone.

## Explicit non-goals

No auto-layout, zoom/pan, marquee or multi-node selection, reverse-direction edge merging, connection editing inside the aggregate modal, new MCP grant system, general permissions UI, instance renaming redesign, or compatibility UI for the removed initial-destination model.
