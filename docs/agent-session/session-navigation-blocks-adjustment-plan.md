# Agent Sessions: compact blocks and ordering

Status: implemented and validated on the feature worktree; not merged or published.

Baseline: `codex/agent-sessions-navigation-refinement` at `99912ce`, worktree `C:/Users/user/.codex/worktrees/session-navigation`. This adjusts the implemented navigation described in `repository-session-navigation-plan.md`.

## Intended result

Use the third proposal, Reorderable trays, as the structural reference, with Sessions above Workflows. Keep repository headers as the outer organizing layer. Render Sessions, Workflows, and each expanded workflow instance as distinct collapsible blocks, with slight indentation from their parents. Added sessions and Workflow sessions each share a neutral group surface and a prominent small heading. Use the same neutral styling for both groups; their labels communicate ownership. The user's icon and color corrections override those details in the generated reference.

Selected reference: [Reorderable trays](C:/Users/user/.codex/generated_images/01a0812d-3599-73f0-b4bf-e62923dc0569/exec-165fb20a-1214-4092-9a51-6517c51bd82d.png).

- Remove conversation-bubble icons throughout the sidebar, including Pinned and Unfiled. Replace bubble-based creation icons with a small compose icon. Keep repository folder icons and disclosure chevrons.
- No drag-handle icons. Repository and block header surfaces remain draggable.
- Use compact rows around 30–32px, readable 14px text, modest section gaps, and roughly 8px indentation per structural layer. Group labels do not introduce another indentation step.
- Use a feature-local 24px action button with a 14–16px icon. The large global `.icon-button` appearance must not size sidebar rows. Keep the global action visible; reveal folder create actions on hover/focus.
- A pinned row shows a clickable Unpin control. An unpinned row shows a clickable Pin control on hover or keyboard focus, reserving its space to avoid text shifting. Apply this to every appearance of a conversation.
- Clicking a block header or its unused interior padding collapses that block. Clicking a session, pin, create action, context menu, or Show more performs only that action. For nested blocks, only the nearest owning block toggles.
- Collapsed sections read `Sessions…` and `Workflows…`; collapsed workflow instances retain their names. Empty blocks remain usable for creation and session placement.
- Keep five session rows per containing block, all pins visible, and Added before Workflow sessions within one shared five-row budget. Group headings do not consume rows. Show more exposes all remaining rows; collapsing and reopening does not silently reset its current-view disclosure state.

Pinning, placement, drafts, workflow ownership, main-working-tree creation, attention markers, and deeplinks retain the existing behavior. Neutral ownership surfaces do not remove meaningful Working/Needs attention indicators or the selected-row highlight.

## Current findings

`application/agentSessions/navigation.ts` models every parent as `SessionNavigationFolder`. `useSessionTree.ts` flattens them into rows, and `SessionTree.tsx` renders the same folder treatment at every depth. Added/Workflow headings are inferred by comparing adjacent rows. There is no group container to own their shared background or click boundary.

`SessionTree.tsx` also owns session rendering, drag/drop, keyboard navigation, and menus. Extending that single renderer with nested surfaces and a second drag behavior would make it harder to change independently.

The pin glyph is currently a passive `<Pin>` icon. `collection.pin`, the context-menu action, persistence, and the agent command already exist and should be reused. The oversized create action comes from `src/styles.css` assigning `.icon-button` a 36px minimum height, width, border, and fill; folder-row padding increases the resulting row height further.

Repository order is alphabetical and Workflows precedes Sessions in the projector. No navigation-order persistence exists. The repository catalog's disclosures record registration provenance, not UI expansion or order; Worktree Review settings belong to its retention policy.

## Model and rendering ownership

Adapt the current navigation model to distinguish a repository container from a section block and a workflow-instance block. Represent Added/Workflow grouping explicitly inside an instance, rather than detecting it from neighboring flattened rows. Keep typed placement and creation targets separate from visual container roles. Preserve stable container and session-row IDs.

Keep the pure fact-to-navigation projection in `application/agentSessions/navigation.ts`. Add `navigationView.ts` beside it for visibility, ancestor paths, and presentation grouping. Compute the shared session limit before forming visible ownership groups; place one Show more control at the workflow-instance level so it does not imply a separate limit for Workflow sessions.

Produce both the nested render structure and its ordered focusable-entry list from one projection. The nested structure provides real shared surfaces; the entry list supports keyboard navigation, deeplink reveal, and agent inspection. Decorative group labels are not independent focus/disclosure items. This also removes the current application-test import of visibility logic from a React feature hook.

Replace `SessionTree.tsx` with a small `SessionNavigation.tsx` composition that renders these containers and owns the active context menu. Extract two concrete pieces:

- `SessionBlock.tsx`: repository/block header, disclosure, nested body, create action, and the nearest-block blank-space click boundary. Repository presentation uses a folder glyph; section/workflow presentation uses a surface. Keep controls as sibling buttons, not nested buttons.
- `SessionEntry.tsx`: title, ownership label, selected/attention state, clickable pin, session drag source, and context-menu trigger. Pinned, grouped, repository, and Unfiled rows all consume this component.

Retain tree keyboard behavior and proper nested tree/group semantics. Keep disclosure buttons accessible and focus visible. Prevent session/action events from reaching a block's collapse handler. Suppress the click generated by a completed header drag so reordering does not also collapse a block.

Rename/adapt `useSessionTree.ts` to `useSessionNavigation.ts` for disclosure, focus, and reveal state. The screen continues to own this controller so the rendered view and agent commands share it. Do not move conversation or selection ownership into the new components.

## Reordering

Recommendation: persist sibling order per application database. Support only these scopes:

1. Repositories within the Repositories section.
2. Sessions and Workflows within one repository.
3. Workflow instances within one repository's Workflows block.

Dragging headers does not reparent repositories or workflow instances. Session dragging retains its existing placement meaning. Added/Workflow group order stays fixed, conversation rows retain activity ordering, and Pinned retains pin-time ordering. Without a saved order, show Sessions before Workflows as in the selected proposal; repository and instance ordering retain their current defaults. Newly registered siblings append in the existing deterministic order.

Create `application/agentSessions/navigationOrder.ts` for typed scopes and the pure sibling insertion/order helpers. Create `useSessionNavigationDrag.ts` for gesture state, payload dispatch, and drop indication. Use separate payload kinds for moving a session and reordering a container. A session drop highlights its placement destination; a container drop shows a thin insertion line before/after a valid sibling. This distinction prevents a header reorder from invoking session movement. Start header dragging outside its buttons; show a grab cursor without adding an icon. Focused headers can use Alt+ArrowUp/Down for the same sibling reorder operation, leaving ordinary arrows for tree navigation.

Extend the existing `SessionNavigationClient` with `reorder(scope, orderedIds)` and include saved orders in its load result. `useAgentSessionCollection` remains the only owner of query refresh and navigation writes. Do not add a parallel order store in React or browser localStorage.

On the backend, add `session_navigation/order.rs` for typed scopes and `session_navigation/order_repository.rs` for storage through the existing `ActiveDatabase`. One narrow table stores a scope key and its ordered-ID array. The service derives valid siblings from current catalog/instance facts and validates the submitted order; SQL stays in the repository module. Each update replaces one scope's order atomically. No rank allocation, generic preferences framework, or ordering fields on workflow/domain records.

Register the additive schema through the current `storage.rs` initialization path, advancing schema 49 to the next available version after checking for intervening work. Handle fresh databases and existing schema-49 databases. `active_app.rs` only wires the repository/service and command. No new behavior in quarantined `lib.rs`.

## Agent command access

Retain the existing local bridge, CLI, pin/move/new/deeplink commands, and disclosure command IDs. The word `folder` in the existing disclosure command remains a logical target name; it does not prescribe a folder glyph.

Add one typed `reorder_navigation` command using the same scope/ordered-ID input as pointer and keyboard reordering. Extend inspection with container role, parent ID, and ordered sibling IDs so agents can understand the block hierarchy without parsing row-ID strings. Adapt `useSessionNavigationCommands.ts` to consume the shared view projection, replacing its separate recursive folder collection. Commands must report the applied ordering and preserve the selected session or draft.

## File changes

Paths below are relative to the feature worktree.

| Action | Files | Responsibility |
| --- | --- | --- |
| Adapt | `src/application/agentSessions/navigation.ts`, `organization.ts` | Explicit container/group model and saved ordering facts/client operation. |
| Create | `src/application/agentSessions/navigationView.ts`, `navigationOrder.ts` | Shared visible structure/focus list and sibling-order rules. |
| Replace/remove | `src/features/agentSessions/SessionTree.tsx` → `SessionNavigation.tsx` | Block composition; remove universal-folder rendering and adjacent-row group detection. |
| Extract | `src/features/agentSessions/SessionBlock.tsx`, `SessionEntry.tsx`, `useSessionNavigationDrag.ts` | Container interaction, shared conversation row, and two distinct pointer-drag behaviors. |
| Rename/adapt | `src/features/agentSessions/useSessionTree.ts` → `useSessionNavigation.ts` | Shared disclosure, focus, and reveal controller; pure traversal moves out. |
| Adapt | `SessionSelector.tsx`, `AgentSessionScreen.tsx`, `sessionNavigation.css` in the same feature | Compact shell, new composition, neutral surfaces, scoped controls, slight indentation. |
| Retain/adapt | `SessionContextMenu.tsx`, `useAgentSessionCollection.ts` | Existing menu/pin/placement actions; collection adds reorder through its current client. |
| Create/adapt | `src-tauri/src/session_navigation/{order.rs,order_repository.rs,application.rs,transport.rs,mod.rs}` | Navigation-owned order storage and service/transport operation. |
| Adapt | `src-tauri/src/storage.rs`, `src-tauri/src/active_app.rs` | Additive schema and composition only. |
| Adapt | `src/application/agentSessions/agentAccess.ts`, `src/features/agentSessions/useSessionNavigationCommands.ts`, `src-tauri/src/session_navigation/agent_access.rs`, `src/infrastructure/agentSessions/tauriSessionNavigationClient.ts` | Same block/controller state and reorder action for agents. |
| Update | Existing navigation/selector/App command tests and `docs/agent-session/navigation-commands.md` | Verify new behavior at existing boundaries; document one new command. |

Retain the shared conversation/profile hooks, main-tree resolver, profiled first-send path, ownership projection, session placement/pin table, deep-link adapter, and the existing Tauri window settings. The transcript and composer are outside this sidebar adjustment. Avoid changing global button styles or creating one React component per decorative label.

## Demo and verification

During implementation, expand the separate demo database rather than the user's application data. Provide three repositories, two workflow instances, at least eight repo conversations, three Added and five workflow-owned conversations in the main instance, and enough pins to show the unlimited list. Populate valid ownership facts for the workflow-owned rows. Keep demo creation reproducible in a small `scripts/seed-session-navigation-demo.py` accepting the explicit demo directory; seed while that instance is stopped. Extend `navigationTestFixtures.ts` with comparable populated scenarios.

The opening demo frame should show multiple rows in both ownership groups. Show more then exposes the remaining owned rows. Demonstrate moving a repo, swapping Sessions/Workflows, and reordering the two instances without drag icons; clicking pin/unpin from both a shortcut and its original row; and collapsing via block padding without collapsing when a child action is clicked.

Validation belongs at the behavior's owner:

- Pure projection: neutral group structure, correct shared five-row limit, unlimited pins, saved sibling order, and deeplink ancestor/reveal paths after reorder.
- Component interaction: inline pin/unpin, hover/focus availability, nested event boundaries, header dragging versus session placement, collapse labels, Show more, focus after unpin/collapse, and unchanged active draft.
- Persistence: additive migration, scope order surviving reopen, and ordering writes leaving session timestamps, workspace, and workflow records untouched.
- Mounted agent-command integration: inspect the real block hierarchy, reorder it, and compare resulting UI/command state while preserving selection.
- Visible Windows demo: compact spacing, neutral group surfaces, tiny compose/pin targets, no bubble/drag icons, pointer reordering, session placement, inline pin clicks, and restart persistence. Pointer behavior requires this check; agent commands alone do not validate dragging.

Run focused frontend/backend tests and the production build, then the frontend regression suite after the integrated changes. Update the execution record with what was actually verified. No custom folders, arbitrary session sorting, drag animation framework, search, or workflow lifecycle changes.

## Implementation adjustment from native validation

The native pointer pass delivered HTML drag-start and drag-over events, but no drop event. The sidebar now handles both session placement and container ordering through pointer capture with a small movement threshold. It keeps the two typed operations and distinct drop indicators, excludes action buttons, and suppresses click after dragging. Hit testing uses the rendered navigation targets. This replaces the HTML drag-transfer handlers rather than retaining two gesture implementations.

## Execution evidence

- Production frontend build and focused lint passed. Frontend regression suite: 175 files, 985 tests passed after the pointer-gesture replacement.
- Native validation: 3 navigation/ownership tests, 41 storage tests (including schema 49 to 50 migration), and 70 Agent Session tests passed. The native test-fast application build passed.
- Visible Windows checks passed for repository order, Sessions/Workflows order, workflow instance order, session placement into a workflow, inline pin/unpin from both appearances, padding collapse, and Show more from five to eight workflow rows. No drag grips or conversation bubbles remain in the sidebar. Added and owned groups use the same neutral surface.
- Agent commands inspected the mounted hierarchy and applied reorder operations. All three orders, pins, and placements survived a native restart. Comparing the database before and after pointer reordering/movement found session records, logical addresses, and workflow records unchanged.
- The isolated demo contains 22 sessions, three repositories, two workflow instances, and six pins. It uses seeded conversations without provider profiles; no provider messages were sent. Existing Vite chunk-size and Rust unused-code warnings remain.

To refresh a stopped, initialized demo: `python scripts/seed-session-navigation-demo.py <demo-directory>`. The directory contains `app-data/codex-orchestrator-active-v3.sqlite`. Run the feature application with `CODEX_ORCHESTRATOR_APP_DATA_DIR` pointing at that `app-data` directory. Navigation commands are documented in `navigation-commands.md`.
