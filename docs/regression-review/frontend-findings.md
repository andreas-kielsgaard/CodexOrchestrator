# Frontend regression review

Reviewed `9fc822f` → `4bded63` on 7 September 2026. This covers the new mounted editors and Session settings. The missing flow canvas and Workflow instance screens are covered by the main review.

The checks below used the checked-in React components with in-memory clients. No provider was called. Findings marked **reproduced** ran through React, Vite and JSDOM; findings marked **source trace** have a direct code path but were not replayed in the native app.

## F1. Activating the saved draft erases newer edits

**Reproduced. Priority: P1 — work is lost.**

1. Open a saved Workflow.
2. Change a node name or prompt without saving.
3. Press **Activate saved draft**.
4. The field goes back to its saved value. The newer edit is gone.

Activating the saved revision is consistent with the button label. Clearing the separate unsaved edits is the problem. There is no dirty warning or discard action.

Evidence: `src/features/workflowAuthoring/WorkflowAuthoringScreen.tsx:170` calls activation with only the recipe ID; line 172 replaces the local draft with the returned saved draft. `src-tauri/src/workflows/authoring_service.rs:106` loads that saved draft for activation.

Probe result: `ACTIVATE_DISCARDS_LOCAL_DRAFT: Node One` after the field had been changed to `Unsaved node text`.

## F2. A save response can erase text typed during the save

**Reproduced. Priority: P2.**

1. Change a Capability Profile name to `First edit` and save it.
2. While the save is pending, change it to `Second edit while saving`.
3. The save completes and the field becomes `First edit` again.

The form stays editable while its save button is disabled. The response then replaces all local fields. Changing the selected profile while a save is pending has the same ownership problem: completion selects the profile that was saved.

Evidence: `src/features/executionConfiguration/ExecutionConfigurationScreen.tsx:93`–112 saves then replaces the draft; line 191 keeps `onChange={setDraft}` active. `CapabilityProfileEditor.tsx:49`–53 leaves the name input enabled. The Workflow screen has the same response replacement at `WorkflowAuthoringScreen.tsx:154`–156 and keeps its editors enabled.

Probe output:

```text
FORM_ACCEPTS_EDIT_DURING_SAVE: Second edit while saving
SAVE_RESPONSE_REPLACES_NEWER_EDIT: First edit
```

## F3. A node can select capabilities its profile forbids

**Reproduced. Priority: P2 — the editor creates a draft that cannot activate.**

1. Give the runtime models A and B.
2. Create a Capability Profile that allows only A.
3. Select that profile on a node.
4. The node still offers an enabled checkbox for B. Checking it puts B in the node state.
5. Saving can succeed, but activation rejects the node as outside its Capability Profile.

Evidence: `src/features/workflowAuthoring/WorkflowNodeEditor.tsx:78` passes the whole runtime catalog to the node editor. `src/features/executionConfiguration/NodeProfileEditor.tsx:160`–163 uses it directly for the checkboxes. The backend check is in `src-tauri/src/workflows/authoring.rs:297`–305.

Probe result: B was enabled, and the node payload became `["model-a","model-b"]` despite the selected profile allowing only A.

## F4. Removing the default model leaves a hidden invalid default

**Reproduced. Priority: P2 — the visible field and saved value disagree.**

1. Let a node allow A and B, with A as its default model.
2. Uncheck A in **Exposed capabilities**.
3. **Default model** appears blank, but the node still stores A as the default.
4. Save and activate: activation reports an unavailable pinned model.

The same update pattern is used for reasoning and sandbox choices. The disabled sandbox field is especially awkward if its locked choice is unchecked above.

Evidence: `src/features/executionConfiguration/NodeProfileEditor.tsx:163` changes only the allowed set. Lines 175–177 filter the default dropdown but retain its old value. `src/components/CatalogSelect.tsx:47` binds that value without an option for it. The backend rejects the stale value at `src-tauri/src/workflows/authoring.rs:307`–329.

Probe output:

```text
REMOVED_DEFAULT_UI_VALUE: ""
REMOVED_DEFAULT_PAYLOAD_VALUE: model-a
```

## F5. Returning to the run page changes the instance but keeps old results

**Reproduced. Priority: P1 — the next action addresses a different instance.**

1. Open **Compile and run**, enter a known Instance ID, and compile.
2. Open a node editor, then return to **Compile and run**.
3. The Instance ID has changed to a new random ID. The previous compiled count is still shown.
4. A later dispatch uses the new ID, even though the page still shows the old result.

Evidence: `src/features/workflowAuthoring/WorkflowRunPanel.tsx:28` creates the ID on mount. `WorkflowAuthoringScreen.tsx:336` only mounts that panel while selected, but lines 56–57 keep its results in the parent. Dispatch uses the current local ID at `WorkflowRunPanel.tsx:99`.

There is a second scope leak: `WorkflowAuthoringScreen.tsx:131`–141 creates a new recipe without clearing those results. `openRecipe` does clear them at lines 104–105. Creating a new recipe after a run can therefore show results from the previous recipe.

Probe result: leaving and returning produced a different ID while `1 Session Event definition compiled.` stayed visible.

## F6. Changing screens or recipes drops unsaved drafts without notice

**Recipe switching reproduced in the saved browser probe; cross-screen navigation traced in source. Priority: P2.**

Edit a Workflow, open **Capability Profiles** to adjust its available tools, then return to **Workflow**. The editor has been unmounted and reloads the first saved recipe; the unsaved edit and previous recipe selection are gone. Clicking another recipe, including the currently selected recipe, also reloads over the local draft. Capability Profile selection has the same discard behavior.

Evidence: `src/app/App.tsx:1096`–1106 mounts one screen at a time. `WorkflowAuthoringScreen.tsx:42` holds the draft only in component state; lines 99 and 121 replace it and choose the first recipe; line 224 makes every recipe button reload it. `ExecutionConfigurationScreen.tsx:87`–90 replaces the draft on selection.

The prior editor saved node edits on close at `src/features/workflows/WorkflowScreen.tsx:1270`–1273. The design notes also proposed retaining its draft/save controller: `docs/session-event-model/codebase-map.md:33`. The new manual-save approach needs an explicit decision about how it protects unfinished edits.

## F7. Session identity display and editing lost their entry point

**Source trace. Priority: P2.**

Previously, **Manage harness** showed the Session's assigned name, color and shape and opened the identity picker. That entry point is gone from the product composition. The replacement Session settings show no assigned identity. The node editor can choose a catalog identity, but cannot inspect or change the Session's copied identity.

Evidence: the baseline-to-HEAD diff of `src/bootstrap/productApplicationComposition.ts:35` removes `agentSessionHarnessManagementSource`. The former display read `session.assignedIdentity` in `src/infrastructure/conversationHarnesses/canonicalConversationHarnessManagementSource.ts:277`; its edit dialog remains in `src/features/conversationHarnesses/HarnessEditor.tsx:779`. The new `AgentSessionExecutionSettings.tsx:58`–99 only renders message options, profile fields and deliveries. `AgentSessionScreen.tsx:181` only gets header identity from an optional callback, which product composition does not supply; the plain branch at line 298 does not pass an identity header anyway.

This is a loss of the identity inspection/editing surface, not a claim that the old product always showed a colored Session header. Whether Session identity should remain editable after creation can be decided separately from making its assigned value visible.

## F8. Delivery history stays stale while a Session is open

**Source trace. Priority: P2.**

Open a Session and its delivery list, then have a managed event deliver another message to that same Session. Its conversation can update, but the delivery list stays at the data loaded when the Session was selected. The visible **Refresh** button does not reload this list. Switching away and back is needed.

Evidence: `src/features/agentSessions/AgentSessionScreen.tsx:108`–122 loads deliveries only when the selected ID or client objects change. Errors are silently ignored at line 116. The sidebar refresh at line 237 only calls `collection.reload()`; there is no delivery reload action or update subscription.

## Checks and gaps

- Passed: `ExecutionConfigurationEditors.test.tsx` (4), `SessionEventEditors.test.tsx` (3), `EventGroupInspector.test.tsx` (1), and `PerMessageRuntimeControls.test.tsx` (1). Total: 9 tests across 4 files.
- The actual Workflow and Capability Profile screen state is not covered by those tests. No test imports `WorkflowAuthoringScreen`, `WorkflowRunPanel`, `WorkflowNodeEditor` or `ExecutionConfigurationScreen` at this checkpoint.
- The probes above reproduced five failures using real components and fake clients. They are component checks, not native screenshots or provider runs.
- The existing `08-session-profile-and-message-controls.jpg` was inspected. It shows the mounted Session settings, but its seeded demo Session cannot prove a live Workflow created a Session or that a saved identity was shown.
- Keep the accepted changes: Role removal, pinned Session configuration and per-message model/reasoning choices from the full attached runtime. These are not regressions. The per-message control uses that full runtime list and clears its selection after a successful send.
- No product code was changed by this review.
