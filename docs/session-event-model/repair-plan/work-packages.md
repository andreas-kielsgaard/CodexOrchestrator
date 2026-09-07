# Work packages

These packages belong to [plan revision 1](README.md). File names are mapped in [codebase projection](codebase-projection.md); proof IDs are in [validation](validation.md). Each package produces code, focused tests and a short evidence note. No package may report a backend happy flow from a compiler-only fixture.

## R1 — Complete Session creation

**Outcome:** a new standalone Session and a Workflow-created Session both have a valid pinned profile before launch. Addresses remain optional for standalone Sessions.

**Addresses:** C1/B1; D1/D2. Depends on G0; can overlap R2/R6/R9.

Tasks:

1. Add an Agent Session-owned creation service around profile resolution and Session persistence. Keep the existing pure resolver and runtime launch code.
2. Supply standalone defaults from the selected runtime as an explicitly application-owned input. Use the Node Profile value shape as needed, without creating a Workflow node/catalogue entity. Read the runtime snapshot coherently within creation.
3. Route the first standalone message through this service, then through direct-user invocation resolution. A message override, if supplied, must not become the pinned default.
4. Make the addressed adapter use the same profile construction. Preserve its existing atomic Session/address persistence; never launch while required birth records are missing.
5. Return a clear pinned/unprofiled read status. Historical unprofiled Sessions remain readable and cannot accidentally take the old send path. A normal new Session must never end up in that state.
6. Surface missing runtime/profile errors before invocation. Ensure a failed resolution creates no orphan Session or address.

**Deliverables/boundary:** creation application service, standalone entry contract/client, addressed adapter update and tests. No legacy profile fabrication, conversation migration, provider setup or new Session override object.

**Local acceptance:** V1/V2. Real repository + fake runtime: first and second sends, pinned digest unchanged, direct model/reasoning override limited to one turn, working directory retained, resolution failure launches nothing. Existing profile resolver tests remain valid. The earliest UI consumer is the mounted standalone screen with the new client.

**Risk:** changing the generic low-level `create_session` for every caller would pull adjacent orchestration into scope. Change the product entry path and common profiled creation operation explicitly; keep remaining low-level callers visible for later retirement.

## R2 — Restore stored Workflow instances

**Outcome:** an instance has a durable ID, name, recipe snapshot and resolved worktree target. Creating it does not start a Session.

**Addresses:** C2/B3/U2/F5; D3. Depends on G0; can overlap R1/R6/R9.

Tasks:

1. Add create/list/load instance contracts separate from recipe authoring commands.
2. Capture the active recipe's exact revision and contents when creating an instance. Reject a changed activation if the submitted revision no longer matches. Store the instance and snapshot together.
3. Reuse the existing repository/branch/worktree target shape and validation. Store the resolved path; do not create, move or delete Git worktrees.
4. Supply the target path in the application-owned Session creation envelope. The generic Session Event layer passes opaque creation configuration; it does not learn Git/Workflow rules.
5. Add an instance read projection of Sessions and recorded event attempts/groups using canonical addresses and records. Do not add a second authority for Session membership.
6. Make instance execution commands address a stored instance, with an optional explicit node where the UI opens a non-starting node. Do not accept an arbitrary fresh instance string as a substitute for creation.

**Deliverables/boundary:** new instance module/schema, transport/client and query projection. No direct Agent Session creation inside the Workflow instance service; no canonical run graph; no migration of the old Role-based instance schema.

**Local acceptance:** V3. Real SQLite create/list/load/reopen, exact target/recipe equality, zero runtime launches on instance creation, new instances use newly activated recipes while old ones retain theirs. Fake runtime captures the selected directory on first Session launch once R3 connects it.

**Risk:** the existing `workflow_instances` table references old `workflow_types` and `workflow_effective_recipes`. Reuse behavior and target types, not those foreign-key assumptions. Use an additive new-model schema in the active database.

## R3 — Separate targeting from creation-time resolution

**Outcome:** existing Sessions remain usable after shared profiles change; new Sessions resolve the configured inputs at their own birth.

**Addresses:** C3/B4 and backend side of C8/F3/F4. Depends on R1/R2 contracts and services for final proof.

Tasks:

1. Compile from the instance's stored recipe, not the current active recipe. Keep preview/activation validation separate from execution compilation.
2. Replace eager embedding of every live Capability Profile during dispatch with a versioned creation intent: Capability Profile reference, inline Node Profile and creation context. Only the creation consumer resolves that reference.
3. Preserve `SessionDirectory` target lookup before creation. A matching existing target must not require any Capability Profile catalogue read.
4. On missing-target/create, resolve the named profile and selected runtime, enforce ceilings/locks/defaults, then pin through R1. A deleted profile may prevent a new birth; it must not prevent addressing an existing Session.
5. Preserve workflow defaults on managed sends and full attached-runtime model/reasoning choice on direct user sends. A changed/missing attached runtime must be reported, never silently substituted.
6. Keep static activation checks for capability/default validity. Recheck actual creation inputs at birth; activation is not a promise that a provider remains available forever.

**Deliverables/boundary:** compiler/service split, versioned creation-intent consumer and resolver tests. Existing profile digest/storage stays the Session truth. No whole-run capability freeze or new generic configuration evaluator.

**Local acceptance:** V4. With an existing Session, edit/delete the shared profile and still send using the old digest/defaults. Attempt a new Session and observe current creation rules. Re-activate the recipe and prove an existing instance's connections/initial prompts remain unchanged. Test direct override followed by a managed message.

**Risk:** the current compiler builds full creation requests even for messages that will not create anything. The producer and consumer must change together; do not add a UI fallback for this backend dependency.

## R4 — Connect completion/application events and real prompt inputs

**Outcome:** normal successful completion of node A can drive a configured delivery to B through Session Events. Failures are visible without changing A's terminal result.

**Addresses:** C4/B2 and C6/B5. Depends on R2/R3. R5 may overlap after the event envelope is agreed.

Tasks:

1. Add a Workflow event receiver called from the normal Agent Session notifier after terminal state is durable. Resolve source Session address to the owning instance/node and inspect only that node's outgoing connections.
2. Preserve source Session, invocation, connection definition and occurrence identity. Reuse the same occurrence identity for a duplicate notification and avoid a second dispatch. No general retry scheduler is required.
3. Read the exact completed invocation's final output. Failed/canceled/interrupted invocations do not masquerade as successful-completion triggers.
4. Add an explicit application-event entry taking a scoped source identity, event kind and named fields. Prove this real service entry with an application-owned caller fixture; do not invent a background event producer or script language.
5. Add a small prompt-content reader for supported file references. Resolve relative files inside the stored instance worktree, retain the reference and delivered text, and distinguish path text from file contents. Reject path escape, missing files and unsupported reference kinds visibly. Reuse existing path-validation logic where it can be separated cleanly.
6. Keep configured prompt order, include node initial text only for a created Session, and retain source field/revision references for fixed node/connection text. Do not make a full source-navigation UI part of this package.
7. Validate trigger/source compatibility, including creation-only sources. Literal and supported referenced content are general; invocation output, MCP arguments and application fields require their matching source. Disable/reject group-completion definitions for now.
8. Record a small Workflow occurrence attempt before resolving content/dispatch, linked to the resulting event group or a preparation failure. Existing group/delivery records remain the authority for dispatch outcomes. Expose pre-dispatch failures rather than dropping them from a best-effort callback.
9. Emit a delivery-record change notice after persistence for R8. Release registry/database locks before callbacks can launch another Session.

**Deliverables/boundary:** scoped event adapters, prompt reader, attempt records, validation and normal-notifier tests. The generic Session Event layer still knows neither Workflow nor Agent Sessions. No arbitrary code injection, plugin registry, filesystem-wide reader or reliable queue.

**Local acceptance:** V5/V6/V7. Real repositories and fake runtime through the normal notifier: A → B, two instances isolated, first/all/newest/last-addressed/running/missing-target variants retained, exact ordered prompt provenance, no repeated initial text, duplicate notification no extra launch, file failure leaves A completed and records the failed handoff. Application-event entry and unsupported group trigger are exercised separately.

**Risk:** runtime notification can be synchronous/re-entrant. Do not hold a lock while dispatching; record correlation before launch. Pending attempts after restart must remain visible, not be treated as completed or automatically retried.

## R5 — Restore the existing managed MCP handoff

**Outcome:** the current application-provided handoff tool is exposed and routed using the new pinned capability model.

**Addresses:** C5/B2. Depends on R1/R3 and R4's event envelope; no new CLI control is introduced.

Tasks:

1. Extract the technical exposure/binding path from its dependency on `WorkflowHarnessConfig`, mixed Harness revisions and Role adapters. Consume the pinned Session's MCP allowlist, Session ID and opaque origin/correlation instead.
2. Reuse managed-upstream registration, proxy filtering, invocation correlation and launch-extension plumbing. Advertise the application-provided tool from that existing registry; do not claim it was discovered from Codex.
3. Bind the allowed tool before the first invocation; preserve the binding for later turns. No binding means no claim of managed MCP exposure.
4. Route the existing `workflow_handoff/handoff_to_agent` source into R4's receiver. Obtain instance/node/Session/invocation context from the managed binding, not model-supplied routing fields.
5. Keep the existing file-path/text interface as the first supported contract. Materialize its named inputs and references without adding arbitrary MCP schema support.
6. Replace the old Workflow MCP registration for this product path; do not run two handlers for the same tool. Keep sender/target authorization and tool filtering checks.

**Deliverables/boundary:** technical binding input/service, local MCP adapter and launch tests. No legacy Harness conversion in the new path, external MCP discovery, new provider settings or blanket filesystem/skill enforcement claims.

**Local acceptance:** V8. Real binding/proxy code with a fake sidecar or local test server: exact allowed tool appears in the launch configuration, disallowed tools are rejected, a local authenticated call reaches the new event receiver and records the correct source/target. A fake provider must not bypass binding construction. This is not a live Codex MCP proof.

**Risk:** largest extraction. Existing binding persistence/proxy DTOs contain old Harness provenance. Keep only the technical/common part and version its stored input where needed; do not create a replacement mixed aggregate to fit the old schema.

## R6 — Protect edits and constrain valid choices

**Outcome:** saves, activation and navigation cannot quietly replace working edits; node/connection choices match the rules.

**Addresses:** C7/F1/F2/F6, C8/F3/F4, UI side of B5. Can start after G0; validation consumes R3/R4 rules.

Tasks:

1. Add a small shared draft-state helper: document key, working value, saved baseline/revision, local edit version and pending request identity. Feature owners keep draft state above route unmounts, keyed by recipe/profile ID.
2. Save a snapshot. On return, update the saved baseline/revision; replace working values only if their document and edit version still match. Preserve newer typing. Serialize writes for one document without copying the old Role-aware persistence client.
3. Activation must name the expected saved revision and only update active/saved metadata. It must not overwrite working edits. Render clearly when unsaved edits are not being activated.
4. Switching screens/recipes retains edits and selection. Explicit reload/discard/window close uses a dirty guard. A save failure retains the draft. Do not add autosave, multi-user merging or crash recovery.
5. Restrict node catalogues to runtime ∩ selected Capability Profile. Do not apply this restriction to the direct-message model/reasoning controls.
6. Preserve valid defaults. A removed/unavailable default is visible with an error until corrected; do not silently choose the first model. Runtime-locked values remain selected, inherited and disabled, including their exposure checkboxes.
7. Apply the same rule when copying a node or switching its Capability Profile. A loaded older invalid draft stays editable and explains why activation is blocked.
8. Filter/disable unsupported trigger variants and show source-pairing errors beside the field. Keep server-side validation authoritative.

**Deliverables/boundary:** draft helper plus feature-owned controllers, input validation/presentation and screen tests. No schema-generated editor or global universal store.

**Local acceptance:** V9/V10. Mounted screens with deferred promises prove typing-during-save, old replies after selection changes, activation while dirty, navigation/reopen, save failures and new-recipe state. Assert submitted payloads as well as visible selections.

**Risk:** a component-local helper alone does not survive `App.tsx` unmounting the screen. The feature controller must have an explicit composition-owned lifetime; navigation must not bypass its dirty guard.

## R7 — Restore flow authoring and instance UI

**Outcome:** the accepted flow editing interaction and instance lifecycle are available through the new contracts.

**Addresses:** C2/F5/U2 and C9/U1/U3. Depends on R2 contracts and R6; final proof uses R2/R3 services.

Tasks:

1. Extract a controlled Workflow canvas: node IDs/positions, connection endpoints, selection, drag/place/connect/copy/delete actions. No capability resolution, repository access or Role imports inside the canvas.
2. Adapt the existing pointer/geometry and editor-history behavior. Host new Node Profile and connection editors in the selected-node/connection panel or popup. Keep name, position and identity of the destination when copying configurable state.
3. Restore a landing view with recipes and instances. Adapt the creation dialog to active recipe IDs/revisions and reuse the existing worktree selector.
4. Add an instance view showing its name, target, pinned recipe, Sessions and event attempts/deliveries. Use the existing shared Session workspace for conversation; do not prescribe a future run graph.
5. Keep recipe/instance selection in typed product navigation and Back/reopen behavior. Replace editable random Instance ID state. Scope all results, loading state and pending responses to the selected instance.
6. Allow the starting node and explicitly selected nodes to use their intended user-entry creation/addressing route. Once a Session is displayed, its direct messages use R1/R8's Session surface.
7. Keep canvas actions and Workflow/instance selection reachable in narrow windows with a compact picker/drawer. Preserve keyboard/focus behavior, including dialogs and node editors.
8. Remove the obsolete mounted `Compile and run` experiment once its useful inspection is available under a real instance. An optional read-only compile preview must not create an instance.

**Deliverables/boundary:** controlled canvas, new-model landing/instance/dialog views, stable navigation and mounted tests. Do not remount the old Workflow screen as a shortcut or bring back Role editing.

**Local acceptance:** V11/V12. Draw A → B, drag/copy/edit/save/reopen; create an instance without launching, open its Session and return with the same instance ID/result set. Check two instances, pending replies after navigation, browser reload of a saved destination and narrow-window access.

**Risk:** the old editor controller and instance dialog types are not drop-in compatible. Preserve behavior through new contracts, and keep old view code only as an extraction source until retirement.

## R8 — Restore Session identity and current delivery details

**Outcome:** Session settings show the actual pinned state and assigned identity; delivery history updates after records are saved.

**Addresses:** C10/F7/F8 plus the visible R1/R3 behavior. Depends on R1 status and R4 notice/query contracts.

Tasks:

1. Read identity from `session.assignedIdentity`; reuse initials/color/shape presentation and the Session-local picker/update command. Do not reconstruct it from the current node or identity catalogue after assignment.
2. Keep Session Profile inspection read-only. Keep model/reasoning controls per message, from attached runtime exposure, and clear successful-send choices as today.
3. Distinguish loading, unprofiled historical state, provider/profile error and ready state. Do not show a broken send form as if it were ready.
4. Add a delivery query owner with load/error/reload/subscription behavior. Refresh it on the post-persistence notice and explicit Refresh; ignore replies for a different selected Session.
5. Show delivery/attempt sources and linked group outcomes honestly. Keep `dispatched` separate from an invocation's terminal outcome. Preserve source references without building arbitrary custom views now.

**Deliverables/boundary:** Session screen/settings/query owner and reuse of existing identity components. No Session capability editor, Role picker or automatic identity-to-policy coupling.

**Local acceptance:** V13. Assigned identity appears, color/shape edit persists on that Session only, reopening retains it, new deliveries update without switching away, errors are visible/retryable, and stale replies do not replace another Session's data.

## R9 — Repair shared control layout

**Outcome:** sections really collapse and form controls remain readable at supported window sizes.

**Addresses:** C11/U4/U5, layout aspects of U3. Independent of backend work.

Tasks:

1. Keep the shared collapsed-content rule effective when feature styles add grid layout. Do not rely only on the `hidden` attribute test.
2. Scope text-input sizing away from checkbox/radio inputs, or reset these in the shared catalogue control. Audit other consumers before changing the global selector.
3. Verify all new editors, Session settings and instance controls at normal and narrow sizes, with long labels and scrolling. Toolbar controls must not be clipped or covered.
4. Verify expand/collapse by keyboard, focus visibility and access to all controls. Keep the current visual direction; no restyling project.

**Deliverables/boundary:** CSS/control corrections and real-browser assertions/screenshots. No new UI framework or component catalogue.

**Local acceptance:** V14. Collapsed content has no visible layout footprint; hidden inputs cannot receive keyboard focus; checkboxes do not crowd text; the workflow picker/create action and toolbar remain reachable.

## R10 — Compose, prove and update status

**Outcome:** the mounted product consumes the repaired paths, and the evidence describes what actually works.

**Addresses:** C12 and cross-package integration. Depends on locally accepted R1–R9.

Tasks:

1. Wire creation, instance, event/MCP, identity and delivery-query clients through the native and frontend composition roots. Keep business logic in the owning modules, not `active_app.rs` or `App.tsx`.
2. Remove obsolete new-screen runtime methods/controls and duplicate registrations only after replacement callers pass. Mark the remaining old Workflow/Harness code as unmounted extraction/retirement sources; broad deletion is deferred.
3. Prove the combined flow with actual application services, SQLite and fake runtime through normal notifications. Reopen repositories between operations. Do not use seeded Sessions as proof of creation.
4. Exercise the actual mounted UI composition with fake clients and run browser journeys. Promote the saved observation probes into checks that assert correct behavior; keep original baseline evidence intact.
5. Run full frontend/build and focused Rust/transport/binding tests, then a native build and isolated no-provider smoke. A live provider run is an additional gate requiring an explicit test scope.
6. Update conceptual/status/UI-map documents and the repair checklist with exact evidence. Refresh only changed demo fixtures/screenshots needed for review; a new slide deck is not required here.

**Deliverables/boundary:** composition, integration tests, current docs and a review-ready build. No merge with parallel branches, release publication, old database reset or broad legacy retirement.

**Acceptance:** V15 plus all package proof IDs. Final evidence separates requested, stored, dispatched, completed, visually checked and user-accepted. Failing local Rust/native checks remain explicit blockers for those claims even if frontend tests pass.
