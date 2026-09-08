# Codebase projection

Part of [repair plan revision 1](README.md). Paths are relative to the repository. **Create** means proposed, not present. **Modify** means the path exists at `e77a725`. Test files listed as new are planned locations, not a requirement for a particular test-file split.

## Target ownership

| Owner                        | Owns                                                                                   | Public consumption route                                                                        | Does not own                                                     |
| ---------------------------- | -------------------------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------- | ---------------------------------------------------------------- |
| Execution Configuration      | Capability values, ceilings, defaults and profile resolution                           | Existing resolver/service facade plus creation-intent types                                     | Workflow identity, persistence of Sessions, provider login       |
| Agent Sessions               | Birth, pinned settings, working folder, identity assignment, invocation and transcript | Profiled creation/direct-message application commands; Session Event directory/dispatcher ports | Workflow graph or recipe selection                               |
| Workflow authoring           | Editable recipes and static validation                                                 | Authoring client/service; pure compiler                                                         | Session invocation or mutable runtime state                      |
| Workflow instances/execution | Recipe snapshot, target, occurrence routing and run read projection                    | Instance client/service and scoped event receiver                                               | Addressing algorithms, provider process launch, canonical run UI |
| Session Events               | Generic target selection, prompt assembly, groups/deliveries                           | Existing directory/dispatcher/store ports and query facade                                      | Git paths, Workflow imports or provider configuration            |
| Technical MCP binding        | Per-Session allowed tools, proxy/binding and invocation correlation                    | Technical binding input + existing launch extension                                             | Roles, prompts, identity selection or mixed Harness editing      |
| Feature controllers          | Editing lifetime and selected document/instance                                        | Small controlled view props and typed navigation                                                | Raw Tauri calls or backend policy                                |
| Shared UI                    | Disclosure, fields, markdown and edit-state mechanics                                  | Value/change/action props                                                                       | Workflow or provider rules                                       |

### Runtime sequence

```text
Saved active recipe + selected target
  -> Workflow instance (recipe snapshot + target)
  -> Workflow event definition + scoped occurrence
  -> Session Events: resolve target
       existing -> Agent Session's pinned profile -> invoke
       missing  -> resolve creation intent -> save Session/profile/address -> invoke
  -> group/delivery records -> instance and Session views

Successful invocation notification / managed MCP call / explicit application event
  -> Workflow event receiver -> next scoped occurrence
```

The canvas edits a recipe; it does not participate in execution. Session Events pass opaque creation configuration to the Agent Session adapter. The working directory belongs in the application creation envelope, not in a generic Workflow-aware field added to Session Events.

## Backend file map

### R1: Session birth and direct messages

| Action | Files                                                                                             | Responsibility                                                                                                                               |
| ------ | ------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------------------- |
| Create | `src-tauri/src/agent_sessions/application/profiled_creation.rs`                                   | Shared creation operation, optional address context, explicit standalone policy and pre-launch persistence.                                  |
| Modify | `src-tauri/src/agent_sessions/application/session_profile.rs`, `application/mod.rs`               | New standalone start surface and typed profile availability; preserve per-message resolution.                                                |
| Modify | `src-tauri/src/agent_sessions/application/lifecycle.rs`                                           | Minimum low-level creation/launch seam only; no Workflow policy.                                                                             |
| Modify | `src-tauri/src/agent_sessions/session_event_adapter.rs`                                           | Consume the common creation service, working directory and birth identity; preserve atomic addressed creation.                               |
| Modify | `src-tauri/src/agent_sessions/repository/mod.rs`, `repository/mapping.rs`, `repository/schema.rs` | Extend only where the profiled birth/status needs persistence support; existing Session/profile/address storage should remain the main path. |
| Modify | `src-tauri/src/agent_sessions/transport/profile.rs`, `transport/dto.rs`, `transport/mod.rs`       | New start input/result and pinned/unprofiled status; keep old data readable.                                                                 |
| Create | `src-tauri/src/agent_sessions/application/profiled_creation_tests.rs`                             | Real repository + recording runtime tests; register under the owning module.                                                                 |

Standalone profile defaults belong to application policy in `profiled_creation.rs`, not to a new settings catalogue. If that helper grows, split it within Agent Sessions; do not make the pure configuration resolver depend on Session or Workflow types.

### R2: instances and their read model

| Action            | Files                                                                                                         | Responsibility                                                                                                                   |
| ----------------- | ------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------------------------------------------------------- |
| Create            | `src-tauri/src/workflows/instances/mod.rs`, `instances/domain.rs`                                             | Instance record, target and stable public facade.                                                                                |
| Create            | `src-tauri/src/workflows/instances/repository.rs`, `instances/service.rs`                                     | Atomic snapshot/target persistence, create/list/load; no invocation on create.                                                   |
| Create            | `src-tauri/src/workflows/instances/queries.rs`, `instances/transport.rs`                                      | Read projection of addressed Sessions and recorded attempts/groups; Tauri DTOs.                                                  |
| Create            | `src-tauri/src/workflows/instances/tests.rs`                                                                  | Create/reopen/revision/target tests.                                                                                             |
| Modify            | `src-tauri/src/workflows/mod.rs`, `src-tauri/src/storage.rs`                                                  | Register module and additive active-database schema.                                                                             |
| Reuse selectively | `src-tauri/src/workflows/instance_domain.rs`, `repository/instances.rs`; `src/application/worktreeTargets.ts` | Target shape, snapshot behavior and validation are sources. Do not bind new instances to old type/effective-recipe foreign keys. |

The instance service consumes authoring's saved recipe through its public route. Session membership comes from generic addresses; do not build another mutable association list that can disagree with them. Add a directory/read-query operation for a scope where needed, rather than teaching Workflow to update Agent Session address tables.

### R3: pinning and compilation

| Action                | Files                                                                                             | Responsibility                                                                                                               |
| --------------------- | ------------------------------------------------------------------------------------------------- | ---------------------------------------------------------------------------------------------------------------------------- |
| Create                | `src-tauri/src/execution_configuration/creation_intent.rs`                                        | Versioned requested creation input: Capability Profile reference + inline Node Profile. No Workflow IDs.                     |
| Modify                | `src-tauri/src/execution_configuration/resolution.rs`, `service.rs`, `mod.rs`                     | Resolve intent at birth, retain pure validation and existing Session Profile digest; allow a coherent runtime snapshot read. |
| Modify                | `src-tauri/src/workflows/authoring.rs`, `authoring_service.rs`, `compiled_plan.rs`, `compiler.rs` | Separate save/static activation validation from instance compilation; emit lazy creation configuration.                      |
| Modify                | `src-tauri/src/workflows/execution.rs`, `execution_transport.rs`                                  | Dispatch from a stored instance/definition, not an arbitrary recipe+ID pair; existing targets bypass current profile lookup. |
| Modify                | `src-tauri/src/agent_sessions/session_event_adapter.rs`                                           | Decode the application creation envelope, resolve intent only on birth, use stored profile on every later send.              |
| Modify / extend tests | `src-tauri/src/execution_configuration/tests.rs`; owning Workflow compiler/service test modules   | Pinning after edit/delete, default/lock validation, initial-prompt behavior.                                                 |

Do not change the shape or meaning of already pinned Session Profiles merely to fix creation. If creation payloads change version, update their producer and consumer together. Obsolete creation payloads should be explicitly rejected or migrated within a named package, not silently reinterpreted as new policy.

### R4: event routing and prompt input

| Action | Files                                                                                            | Responsibility                                                                                                          |
| ------ | ------------------------------------------------------------------------------------------------ | ----------------------------------------------------------------------------------------------------------------------- |
| Create | `src-tauri/src/workflows/event_sources.rs`                                                       | Successful-completion and explicit application-event adapters, scoped occurrence context and exact final-output lookup. |
| Create | `src-tauri/src/workflows/prompt_content.rs`                                                      | Supported worktree-relative file references and fixed recipe-field sources.                                             |
| Modify | `src-tauri/src/workflows/execution.rs`, `instances/repository.rs`, `instances/queries.rs`        | Small occurrence-attempt log, duplicate-source handling, source-to-group/failure read projection.                       |
| Modify | `src-tauri/src/workflows/compiler.rs`, `authoring.rs`                                            | Pairing validation, stable definition/source references, per-node user-entry mapping where required.                    |
| Modify | `src-tauri/src/session_events/domain.rs`, `materialization.rs`                                   | Shared definition validation and retained prompt references; keep Workflow out.                                         |
| Modify | `src-tauri/src/session_events/mod.rs`, `ports.rs`, `repository.rs`, `queries.rs`, `transport.rs` | Post-store change notice and query support. Keep generic target resolution and group/delivery ownership.                |
| Create | `src-tauri/src/workflows/event_sources_tests.rs`, `prompt_content_tests.rs`                      | Normal-notifier, scoped event and temporary-file tests.                                                                 |
| Modify | `src-tauri/src/active_app.rs`                                                                    | Wire the new receiver after durable Session notifications; no routing logic in the composition root.                    |

Only add an event-record notifier port for the real post-persistence notification boundary. No general event bus is needed. Preparation failures belong to the Workflow occurrence attempt when no generic materialized command/group exists yet; do not create fake delivered records.

### R5: technical MCP integration

| Action                                                      | Files                                                                                                       | Responsibility                                                                                                                                   |
| ----------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Create                                                      | `src-tauri/src/harness_engine/session_binding.rs`                                                           | New technical binding entry: pinned profile digest, allowed tools, Session ID and opaque origin.                                                 |
| Create if existing store cannot cleanly carry the new input | `src-tauri/src/harness_engine/session_binding_repository.rs`                                                | Versioned technical binding persistence without a fabricated mixed Harness revision.                                                             |
| Modify                                                      | `src-tauri/src/harness_engine/service.rs`, `domain.rs`, `repository.rs`, `sidecar.rs`, `proxy.rs`, `mod.rs` | Extract/share technical binding and proxy plumbing; remove old aggregate assumptions from the new path. Exact split follows the R5 source audit. |
| Modify                                                      | `src-tauri/src/workflows/mcp.rs`                                                                            | Replace old `WorkflowApplication` handoff calls with the scoped event receiver; preserve known MCP input/authentication behavior.                |
| Modify                                                      | `src-tauri/src/agent_sessions/application/profiled_creation.rs`, `session_event_adapter.rs`                 | Bind technical MCP exposure before launch; forward trusted invocation/source correlation.                                                        |
| Modify                                                      | `src-tauri/src/active_app.rs`                                                                               | Register the existing managed tool once, advertise it as application-owned and wire new binding.                                                 |
| Create                                                      | `src-tauri/src/harness_engine/session_binding_tests.rs`, `src-tauri/src/workflows/mcp_event_tests.rs`       | Binding/allowlist/launch/local handoff tests.                                                                                                    |

Keeping the technical code under `harness_engine/` for this repair avoids a broad directory rename. The replacement entry must not call `workflow_adapter.rs` or construct `WorkflowHarnessConfig`. Broad legacy renaming/removal stays later. Native-profile model/skill discovery remains mocked as previously agreed.

## Frontend contracts and clients

| Action | Files                                                                                                                                                     | Contract change / owner                                                                                                                                 |
| ------ | --------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Modify | `src/application/agentSessionProfiles/contracts.ts`, `index.ts`; `src/infrastructure/agentSessionProfiles/tauriAgentSessionProfileClient.ts` and its test | Start standalone Session, read pinned/unprofiled state, send one direct message. R1/R8.                                                                 |
| Create | `src/application/workflowInstances/contracts.ts`, `index.ts`; `src/infrastructure/workflowInstances/tauriWorkflowInstanceClient.ts`, `index.ts`, test     | Create/list/load stored instances, address a node within one, query run evidence. R2/R7.                                                                |
| Modify | `src/application/workflowAuthoring/contracts.ts`, `index.ts`; `src/infrastructure/workflowAuthoring/tauriWorkflowAuthoringClient.ts` and test             | Authoring-only save/activate/preview; expected saved revision on activation. Remove arbitrary-instance dispatch methods after consumers move. R2/R3/R6. |
| Modify | `src/application/executionConfiguration/contracts.ts`; relevant transport mapping                                                                         | Creation intent/default validation if surfaced to clients; avoid exposing internal binding data. R1/R3.                                                 |
| Modify | `src/application/sessionEvents/contracts.ts`; `src/infrastructure/sessionEvents/tauriSessionEventQueryClient.ts` and test                                 | Source references, delivery-change subscription and explicit query errors. R4/R8.                                                                       |
| Modify | `src/application/productNavigation.ts` and its tests                                                                                                      | New recipe/instance destinations with stable IDs. Do not treat the old Workflow type ID as the new recipe ID without an explicit mapping. R7.           |

Use module facades and typed clients as the frontend consumption surface. A view never invokes a raw Tauri command or reads an SQLite table.

### Small contract shapes to settle at G0

These are responsibilities, not final syntax:

- **Standalone start:** text, optional title/working folder and any already-supported one-message choices → Session ID, invocation acknowledgement, pinned profile.
- **Instance creation:** active recipe ID + expected revision, name, resolved target → stored instance. No launch effect.
- **Instance send:** stored instance ID, explicit node/start-entry identity, text → generic event result. No caller-selected worktree override or unverified instance string.
- **Creation envelope:** versioned capability/node intent, working directory and optional assigned identity. Workflow builds it; Agent Sessions consume it; Session Events pass it opaquely.
- **Workflow occurrence:** instance/source-node scope, source Session/invocation where applicable, stable source occurrence identity, trigger payload. MCP caller context comes from the managed binding.
- **Delivery change notice:** recorded group identity and affected Session references → consumers re-query. The notice is not a command or completion claim.
- **Draft state:** document key, saved revision/value, working value, edit version, pending save identity. Remote replies only update the matching document/version.

## UI file map

### R6: draft state and editor validity

| Action | Files                                                                                                                                                                                    | Responsibility                                                                                                      |
| ------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------- |
| Create | `src/components/editing/draftState.ts`, `draftState.test.ts`                                                                                                                             | Small domain-free saved/working/request state transitions.                                                          |
| Create | `src/features/workflowAuthoring/workflowAuthoringController.ts`; `src/features/executionConfiguration/executionConfigurationController.ts` and tests                                     | Feature-owned draft caches, queued saves, error state and stable selection; lifetime supplied above screen unmount. |
| Modify | `src/features/workflowAuthoring/WorkflowAuthoringScreen.tsx`, `WorkflowNodeEditor.tsx`, `WorkflowConnectionEditor.tsx`, `workflowAuthoringPresentation.ts`                               | Consume controllers and selected-profile constraints instead of owning unsafe state replacement.                    |
| Modify | `src/features/executionConfiguration/ExecutionConfigurationScreen.tsx`, `NodeProfileEditor.tsx`, `CapabilityProfileEditor.tsx`, `CapabilitySetFields.tsx`, `types.ts`, `presentation.ts` | Shared validation presentation, locked exposures and visible unavailable defaults.                                  |
| Modify | `src/features/sessionEvents/TriggerBindingEditor.tsx`, `PromptSourceListEditor.tsx`, `defaults.ts`, `types.ts`                                                                           | Supported variants and compatible source choices. Generic target editor retains its existing selector variants.     |
| Create | Mounted screen tests beside the two screens and controller consumers                                                                                                                     | Real save/activate/navigation behavior, not only controlled field tests.                                            |

### R7: flow and instances

| Action                            | Files                                                                                                                                                 | Responsibility                                                                                                                                   |
| --------------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------ |
| Create                            | `src/features/workflowAuthoring/WorkflowCanvas.tsx`, `workflowCanvas.css`, `WorkflowCanvas.test.tsx`                                                  | Controlled layout, node/connection interaction and selection.                                                                                    |
| Reuse / adapt                     | `src/features/workflows/editor/workflowNodeDrag.ts`; old `workflowEditorController.ts` and canvas parts in `WorkflowScreen.tsx`                       | Drag math is reusable. Extract history/edit behavior into the new controller without old config unions or Role commands.                         |
| Create                            | `src/features/workflowInstances/WorkflowInstanceScreen.tsx`, `WorkflowInstanceCreationDialog.tsx`, `workflowInstances.css`, `index.ts`, tests         | New-contract instance creation, target, Session/evidence projection and shared conversation host. Reuse the old dialog's focus/pending behavior. |
| Create                            | `src/features/workflowAuthoring/WorkflowLanding.tsx`, test                                                                                            | Recipe/instance entry points.                                                                                                                    |
| Modify                            | `src/features/workflowAuthoring/WorkflowAuthoringScreen.tsx`, `WorkflowOutline.tsx`, `workflowAuthoringTypes.ts`, `workflowAuthoring.css`, `index.ts` | Canvas composition, optional outline navigation, compact picker and inspector host.                                                              |
| Reuse unchanged where possible    | `src/features/worktreeTargetsTemp/DiscoveredWorktreeTargetSelector.tsx`; shared `AgentSessionWorkspace`                                               | Existing target choice and conversation rendering, through their current value/action contracts.                                                 |
| Remove after replacement is wired | `src/features/workflowAuthoring/WorkflowRunPanel.tsx`                                                                                                 | Random-ID run experiment. Move useful read-only inspection under the stored instance first.                                                      |

Do not delete the old `src/features/workflows/` tree wholesale in this repair. It contains tests and extraction sources; the new mounted route should no longer depend on its old domain surface.

### R8/R9: Session details and shared controls

| Action                  | Files                                                                                                                                                        | Responsibility                                                                                                      |
| ----------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------- |
| Create                  | `src/features/agentSessions/useSessionEventDeliveries.ts`, test                                                                                              | Load/error/reload/post-store invalidation with selected-Session correlation.                                        |
| Modify                  | `src/features/agentSessions/AgentSessionScreen.tsx`, `AgentSessionExecutionSettings.tsx`, `useAgentSessionController.ts`, `agentSession.css`, affected tests | New creation route, profile readiness, Session identity display, working direct-message controls and event refresh. |
| Reuse / minimally adapt | `src/features/identities/AgentIdentityBadge.tsx`, `IdentityPickerDialog.tsx`, `identityPresentation.ts`; identity and Session update clients                 | Show/edit the copied Session assignment, never its capability policy.                                               |
| Modify                  | `src/features/sessionEvents/EventDeliveryList.tsx`, `EventGroupInspector.tsx`, tests                                                                         | Honest dispatch status, source references, error/empty states.                                                      |
| Modify                  | `src/components/CollapsibleSection.tsx` only if needed; `collapsibleSection.css`, `CatalogSelect.tsx`, `catalogSelect.css`; `src/styles.css`                 | Effective hidden layout and correct checkbox/radio sizing without unrelated global restyling.                       |
| Modify                  | Execution Configuration and Workflow feature CSS                                                                                                             | Do not override hidden; maintain toolbar/field access and narrow-window selection.                                  |
| Create                  | Browser checks under `tests/browser/session-event-repair/`                                                                                                   | Actual layout/focus/drag tests, using package-local fake clients and no provider.                                   |

## Composition, cleanup and documentation

- **Modify:** `src-tauri/src/active_app.rs` and relevant module facades for native wiring. `src-tauri/src/lib.rs` may receive module declarations only if required by Rust; no new business logic there.
- **Modify:** `src/app/App.tsx`, `src/bootstrap/productApplicationComposition.ts`, `src/bootstrap/productApplicationComposition.test.ts` and mounted App tests for new instance/creation/query clients and feature-controller lifetime.
- **Create:** `src/app/App.sessionEventModel.test.tsx` and `src-tauri/src/workflows/runtime_flow_tests.rs` for the joined path.
- **Modify if required:** `package.json`/lockfile only for a deliberate browser-test dependency/script. Current review runner uses bundled Playwright. Prefer an explicit reproducible dev dependency if CI must run it; no runtime dependency change is otherwise planned.
- **Modify:** `docs/session-event-model/README.md`, `conceptual-model.md`, `functional-happy-flow.md`, `contract-rules.md`, `codebase-map.md`, `ui-mapping.md`, `delivery-sequence.md` to match delivered ownership and status. Do not turn the plan into a claim of completion.
- **Preserve:** `docs/regression-review/` findings, screenshots and observation JSON. New passing evidence goes beside them or in the normal tests, labelled with its commit.
- **Delete scope:** only the obsolete new run panel and dead methods/imports/duplicate registrations made unreachable by the repairs. Any additional file deletion needs a consumer check. No branch/worktree/data cleanup is included.

## Storage and contract changes

| Data                                  | Planned treatment                                                                                                                                                     | Why                                                                                     |
| ------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------- |
| Existing Session/profile/address rows | Retain; use the existing atomic addressed-create operation and profile digest                                                                                         | They already encode Session truth. Do not re-pin historical rows.                       |
| New-model instances                   | Add a `workflow_recipe_instances` table (proposed name) containing ID/name, recipe ID/revision/snapshot, resolved target and timestamps                               | Old instance tables reference retired authoring concepts. No old-data reset is needed.  |
| Workflow occurrences                  | Add a small `workflow_event_occurrences` table (proposed name), keyed by instance + source occurrence + connection/entry, with group reference or preparation failure | Visible pre-dispatch failure and duplicate notification correlation; not a retry queue. |
| Session Event groups/deliveries       | Retain generic records; extend source references/query indexes only where needed                                                                                      | UI projections should consume these records, not invent run outcomes.                   |
| MCP bindings                          | New technical input/version and, if needed, a separate technical binding table                                                                                        | Avoid fabricating old Harness revisions to use proxy machinery.                         |
| Creation payload                      | Version the intent/envelope together at compiler and adapter                                                                                                          | Existing-target sends no longer need eager current profile reads.                       |
| Saved/active authoring                | Keep saved draft and active copy; add expected-revision activation check                                                                                              | Avoid activating a different saved revision than the one the UI identifies.             |

Use additive migrations against an isolated database. Verify an old database opens without rewriting historical data. No migration promises to make old Role-based instances runnable in the new model.

## Producer–consumer rules worth testing

| Boundary                       | Allowed variants / correlation                                                                 | Negative boundary                                                                | Earliest real consumer proof             |
| ------------------------------ | ---------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------------- | ---------------------------------------- |
| Creation → Session repository  | Standalone without logical address; addressed birth with profile/address together              | No launch before storage, no fake Workflow identity for standalone               | R1 real repository + recording runtime   |
| Recipe → compiler → occurrence | User entry; successful invocation; supported MCP/application source; ordered prompt references | No group-completion activation; no live capability lookup for an existing target | R3/R4 compiler plus actual adapter       |
| Completion → Workflow receiver | Exact Session/invocation/instance/node/connection                                              | No failed-turn success trigger, cross-instance routing or duplicate send         | R4 normal notifier, two instances        |
| MCP binding → handoff          | Existing managed server/tool and trusted invocation context                                    | No model-supplied authority, disallowed tool or extra CLI controls               | R5 proxy/local MCP + recording runtime   |
| File reference → reader        | Supported relative worktree file and explicit path/text/content meaning                        | No path escape, credential scraping or arbitrary external reader                 | R4 temporary files and captured prompt   |
| Save response → draft          | Document key + saved revision + local edit version                                             | No overwrite of newer typing or other document                                   | R6 mounted screen with deferred promises |
| Record store → UI              | Notice after group/delivery persistence, exact affected Session                                | No dispatch-as-completion claim or stale Session reply                           | R8 query subscription + mounted screen   |

No generic extension registry is required. Keep the known variants explicit, with small ports at real external boundaries. A later source can be added when its concrete caller and data contract exist.
