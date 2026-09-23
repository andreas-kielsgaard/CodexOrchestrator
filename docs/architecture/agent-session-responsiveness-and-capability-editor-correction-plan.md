# Agent Session responsiveness and Capability Profile editor correction

Status: implemented on `refinement/usability` and validated 2026-09-23. See `docs/validation/agent-session-responsiveness/2026-09-23-correction-pass.md`.

## Target and boundaries

Finish the agreed Capability Profile presentation corrections and make a running Agent Session remain usable while runtime events are arriving. Capability Profiles remain design-time configuration until a Session is instantiated. Existing Sessions keep their pinned configuration; an unsent draft follows the current revision of its selected profile when the same execution route still exists.

This pass does not add model/reasoning enforcement, redesign workflow policy, change durable runtime-event retention, add remote-device behavior, or introduce a second transcript implementation. It does not switch Tauri events to channels without post-correction evidence that transport remains material.

The current performance defect is concrete: one observed invocation persisted 6,711 events over about 221 seconds, peaking at 80 events/second. Each correlated event currently causes `useAgentSession` to reload the complete history, while a separate 1.5-second poll does the same. A read-only lower-bound query and JSON decode of that history took about 31.6 ms before IPC, projection, Markdown, and layout work.

## 1. Complete the Capability Profile presentation corrections

### Remove misplaced and irrelevant content

- Remove **A selected worktree supplies its own Capability Profile** from `ExecutionConfigurationScreen.tsx`.
- Remove `NativeCapabilityInventory.tsx` from Capability Profiles. The selected Codex profile already owns the profile-scoped **Harness-provided tools** diagnostic in `CodexProfilesScreen.tsx`; keep that implementation as the one technical home for the probe.
- Delete the now-unused unscoped `load_native_capability_inventory` command and `ExecutionConfigurationClient.loadNativeCapabilityInventory` contract. Remove `nativeInventory` from `ExecutionTargetRuntime`/`ExecutionTargetRuntimeDto` and stop collecting it during ordinary target-runtime loading. Retain `load_native_profile_capability_inventory`, the native-profile client method, and the Codex profile diagnostic.

This is a deletion and consolidation, not another move or wrapper around duplicate inventory UI.

### Make each execution route a collapsed editor

Extract the route card from `CapabilityProfileEditor.tsx` into a focused `CapabilityRouteEditor.tsx`. Its collapsed row shows:

- device;
- harness;
- inference source;
- default-route state and the existing default/remove actions.

The detailed model, MCP, and skill controls mount only while that route is expanded. Routes start collapsed whenever a profile is opened. Keep `CollapsibleSection` for page-level sections; do not distort its heading semantics to represent nested route cards.

### Make the Add model dialog a reversible in-instance editor

Extract `CapabilityModelPickerDialog.tsx`. Keep it open after Add or Remove. Show the complete latest known model list, including already-added entries; each line displays **Add** or **Remove** according to the draft. A saved model missing from the latest observation remains visible and is labelled as saved but not currently observed.

Create a small `CapabilityProfileEditorMemory` owned beside the existing composition-root `DraftWorkspace`. Key removed allowances by draft/profile key, route ID, and model ID. Removing a model stores its full min/max reasoning allowance; re-adding restores it. The memory survives profile selection and top-level tab changes for the life of the application process, but is neither persisted nor included in dirty-state comparison. Re-key `$new` memory after first save and clear it when the profile is deleted. Do not add tombstones to the backend DTO.

## 2. Let unsent drafts follow harmless profile revisions

The current quick-feature path rejects any revision mismatch with `The Capability Profile changed. Select the target again`, so renaming a profile disables skill discovery even though the selected route is unchanged.

Add one application-layer draft-selection resolver used by quick-feature discovery, preparation, and direct-user Session creation:

- read the profile by stable profile ID;
- find the selected execution route in its current route policies;
- if that route still exists, use the current revision and current profile configuration;
- if the route was removed or materially rebound, return an actionable route-selection error.

Adapt `useSessionTarget.ts` to rebase a cached unsent selection when the profile catalogue loads. Preserve its workspace and composer draft while updating the revision. Existing Sessions do not rebase: their pinned Session profile and compiled skill/MCP exposure remain authoritative.

Test rename-only, capability-group changes, route removal, and the race in which the profile changes between draft discovery and Send. A rename must not interrupt `/skills`; a removed route must not silently switch to another route.

## 3. Give live Session history application-level ownership

Create `src/application/agentSessions/liveHistory.ts` with an `AgentSessionHistorySource` contract and pure update reducer. Create `src/infrastructure/agentSessions/tauriAgentSessionHistorySource.ts` as the application-scoped implementation composed next to `tauriAgentSessionClient`.

The source should:

- perform one complete load when a Session is first selected or explicitly refreshed;
- keep immutable, cached snapshots with stable identity between changes;
- apply `event_persisted`, `invocation_terminal`, and `diagnostic_recorded` directly from their existing update payloads;
- deduplicate events by ID/sequence and retain invocation ordering;
- coalesce subscriber notification to at most one animation frame while still ingesting every update;
- flush terminal state immediately;
- allow only one reconciliation load per Session at a time;
- perform a complete reconciliation only for an unknown invocation, a sequence gap, reconnect/resume, explicit refresh, or an update whose payload is insufficient;
- retain up to five recently viewed Sessions so returning to a tab or Session paints cached content immediately, while never evicting a currently subscribed Session.

For uncached Sessions, global updates should record only a dirty/status marker rather than hydrate their complete transcript. SQLite remains authoritative. A Session selected after time away loads a fresh snapshot.

Expose this source through `AppProps` and `EmbeddedAgentSessionComposition` so standalone Agent Sessions, Plan Builder, shared Session panels, Product Decisions, and development surfaces consume the same production read boundary. Recorded/test compositions receive an explicit in-memory source; avoid a module-global client-to-store registry.

Refactor `useAgentSession.ts` to retain composer, send, preparation, and interaction actions while delegating history loading/subscription to the source. Remove:

- per-update `loadSelected(..., true)` reconciliation;
- the 1.5-second active-invocation poll;
- generation guards whose only purpose was discarding overlapping full-history responses.

Retain a manual refresh operation, but make refresh stale-while-revalidate so cached conversation content does not disappear. Keep `useAgentSessionCollection` as the separately debounced lightweight navigation-summary path.

## 4. Project and render only the changed work

Split `transcriptProjector.ts` into reusable initial projection and incremental invocation/event projection. The incremental projection should consume the applied history change and preserve object identity for unchanged invocations. It must retain the existing durable ordering, imported-turn handling, lifecycle start/completion coalescing, final-response detection, diagnostics, anchors, and outcome semantics.

Do not sort and remap every persisted event after each append. Do not rebuild a transcript-wide revision string in `ConversationViewport.tsx`; publish a monotonic presentation revision from the projection source.

Adapt `ProcessingDisclosure.tsx` and remove `TechnicalDiagnosticDisclosure.tsx` from the normal Session presentation:

- running disclosures remain user-collapsible and start collapsed with a compact **Working · N updates** summary;
- collapsed disclosures do not mount their event list;
- an expanded processing list contains only meaningful agent progress and tool activity, not transport, process, configuration, usage, or raw protocol events;
- routine tool lifecycle pairs remain one logical activity and successful technical diagnostics are omitted;
- a failed turn shows its concise application/runtime failure and recovery guidance; a technical or tool event is promoted only when it is the clearest available explanation of the failure;
- raw payloads, sequence IDs, provider event names, and internal timestamps are never rendered in the ordinary transcript.

Keep every event and diagnostic in durable history for correctness and troubleshooting. A failed turn may expose one secondary **Copy diagnostic details** action that packages relevant data without mounting it. Do not add a general technical-details browser, paging control, or virtualization dependency in this pass.

Replace the transcript-wide `useLayoutEffect` auto-scroll loop with one coalesced `requestAnimationFrame` follow operation per published presentation revision. Preserve the current rule that scrolling away from the bottom disables follow until the user requests it or changes Session.

Separate Session navigation and conversation subscriptions so a transcript update cannot rerender the navigation tree. User input, Session selection, scrolling, and disclosure state remain ordinary immediate React state. Do not rely on `startTransition` for the external store: React applies external-store mutations as blocking updates, so bounded work and subtree isolation are the responsiveness mechanism.

## 5. Transport and backend boundary

Keep the current persist-before-notify ordering and the existing update DTOs; they already contain enough data for the high-frequency incremental path. Keep full-history loading as initial/recovery behavior.

Do not add a delta endpoint, Web Worker, or Tauri channel in the first correction pass. After the frontend no longer reloads and rebuilds complete history, measure event bridge cost separately. If the global Tauri event bridge remains material, replace only the live history stream with an ordered channel in a follow-up; collection/status notifications can remain ordinary events.

Likewise, do not change SQLite schema, event retention, or the synchronous history command merely on suspicion. Re-measure them after eliminating the confirmed amplification.

## Implementation sequence

1. Add regression tests that reproduce rename invalidation and complete-history reload amplification; capture baseline load counts and rendering/projection measurements.
2. Apply the Capability Profile UI deletions, route disclosure extraction, modal behavior, and editor-memory ownership.
3. Add the draft-selection resolver and remove rename-only invalidation across discovery, preparation, and first Send.
4. Introduce and compose the shared live-history source; adapt `useAgentSession` consumers and remove polling/full reloads from contiguous updates.
5. Add incremental transcript projection, lazy processing/raw-event presentation, batched follow scrolling, and navigation/render isolation.
6. Run focused and broad automated validation, then exercise the real native application with the existing large Session and a live event-producing turn.

## Verification and acceptance

- Opening or renaming a Capability Profile no longer exposes native inventory in that screen; the selected Codex profile remains able to run the profile-scoped tool diagnostic.
- Execution routes open collapsed with the required summary. Add/Remove in the model dialog does not close it, and removing then re-adding restores the exact reasoning range until application exit.
- Renaming a Capability Profile does not break `/skills`, clear its draft, or require reselecting an unchanged route. Removing that route produces an explicit selection error.
- Selecting a Session performs one complete history load. A contiguous burst of persisted-event, terminal, and diagnostic updates performs zero additional complete loads. A sequence gap produces one coalesced recovery load with no overlapping request.
- A synthetic 10,000-event history and an 80-event/second burst preserve order, lifecycle coalescing, final response, diagnostics, and terminal outcome. Restarting and loading from SQLite produces the same visible transcript.
- While the burst runs, the user can type, select another Session, scroll, expand/collapse details, and switch top-level tabs. Returning shows the cached Session immediately and then reconciles without blanking it.
- Ordinary processing never formats or mounts raw payloads or routine technical events. A failed turn remains understandable from its concise failure and guidance, with diagnostic copying available only when useful.
- Record before/after history-load counts, projection/commit measurements, and native-window interaction evidence under `docs/validation/agent-session-responsiveness/`. Treat timing as measured evidence, not a brittle unit-test threshold.
- Run focused Agent Session, execution-configuration, native-profile, composition, and Rust application tests; then run `npm run build:frontend`, the broad frontend suite, relevant Rust suites, and a packaged/native application smoke flow.

## Explicit non-goals

No broad Capability Profile enforcement, no model/reasoning restriction for direct-user Sessions, no new remote connection behavior, no event deletion or compaction in storage, no persisted model-removal tombstones, no child sidecars, and no speculative transport/database rewrite.
