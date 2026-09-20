# Device, Harness, Inference, and Composer Drafts

Status: implementation shape, 2026-09-20. The local usability slice is implemented: composer
draft recovery, Agent Sessions return location, local Codex harness/source projection, profile
route selection, and the app-server protocol fix. The Coordinator, Device Agent, and shared
catalogue migration remain planned work.

## Target

Make execution setup understandable and separately editable:

```text
Device
  -> Harness on that device
       -> connected Inference Source
            -> Capability Profile policy
                 -> Session and invocation snapshot
```

Technical Settings is the controller surface for devices, their harnesses, and inference-source connections. After Orchid Network enrollment, the Coordinator is the canonical non-secret catalogue for Devices, Harnesses, Sources, bindings, Profiles, and product session identity; controller applications cache that catalogue. Capability Profiles remains a top-level surface and owns only reusable route policy, allowed capabilities, and defaults. Agent Sessions owns unsent composer drafts locally. Existing Session/Profile snapshots remain historical execution evidence.

Top-level tab switches restore the last semantic location for that surface in the running application. They are not generic Back navigation and do not need to preserve viewport scroll in this slice.

## Decisions

- A Device owns a stable Orchid identity and device-local repository locations. It does not own a provider account.
- SSH is developer-operated provisioning/recovery infrastructure outside Orchid product data. It is not a Device field, profile route, or UI configuration.
- A Harness belongs to one Device and owns how that device runs an agent. Initial supported kind: Codex CLI, including its configuration/home reference.
- An Inference Source is a separately named logical source. Credentials and provider configuration remain on the harness's device; Orchid stores no Codex auth file or API key.
- `HarnessInferenceBinding` is the only allowed connection between a Harness and an Inference Source. A Capability Profile selects one binding, so its discovered model catalogue and allowed-capability ceiling are unambiguous.
- A Codex CLI binding exposes a managed OpenAI/Codex account observed from that harness configuration. A second account requires a separately authenticated harness configuration; it is not inferred to coexist in one configuration reference.
- A Capability Profile revision pins its binding reference, allowed capabilities, and defaults. A Session continues to snapshot its resolved execution facts; later setup edits never retarget it.
- Composer drafts are durable local product data. A successful accepted send clears the unsent draft; failure, cancellation, and uncertain acknowledgement retain it.
- Default profile/model values seed a new empty draft only. Restored choices win; unavailable restored choices remain visible as needing attention.

## Networked device control

Treat a running Orchid UI and an execution Device Agent as separate roles. A laptop or Android UI is a controller: it can create, observe, steer, and cancel work. A Device Agent runs on a device that can actually perform work and owns that device's repositories, harness configurations, provider processes, and credentials. The Android application is not yet a Device Agent; its first useful network role is a controller for an already-enrolled remote device.

The current SSH path remains useful only to developers for installing, configuring, verifying, or recovering the remote `orchid-host`/Device Agent outside Orchid. It deliberately has no Technical Settings, Device, Harness, Profile, or Android UI representation. It is not a mechanism by which one Orchid controller reaches another; do not copy SSH private keys, aliases, or a laptop-local connection assumption into product state.

For the laptop-off/Android-to-remote case, add an always-on Orchid Coordinator. The remote Device Agent establishes an outbound authenticated connection to the Coordinator, registers its Device identity and live availability, and accepts typed execution commands over that connection. The Android controller also connects to the Coordinator and addresses the remote Device by its Orchid ID. This permits the server to remain reachable without the laptop being online or exposing an inbound SSH endpoint.

```text
Laptop controller ─┐
                   ├── authenticated control connection ── Orchid Coordinator
Android controller ┘                                      (identity, presence, queue, relay)
                                                                  ▲
                                                                  │ outbound authenticated connection
                                                          Remote Device Agent
                                                                  │
                                                   repository / Codex CLI / credentials
```

The Coordinator is a control-plane and optional encrypted relay, not a provider runner and not a repository or credential mirror. It stores device membership, the non-secret setup catalogue, availability, command receipt and routing state, and product-owned session/configuration identity. The target Device Agent performs preflight, worktree preparation, provider launch, approvals, and cancellation against its own filesystem. A request is therefore distinct at least as: controller requested, Coordinator accepted, target Device Agent accepted, and target processed. An offline target can receive a queued request only if the product explicitly supports that queue; it must never appear to have run it.

Direct peer-to-peer connections may later optimize local-network traffic, but they are not the initial contract. NAT, changing mobile networks, background restrictions, and target discovery make a pure peer mesh unsuitable for the stated "any device" guarantee. The Coordinator can run on the always-on remote server in the first slice. It needs device identity keys, scoped controller/device authorization, encrypted transport, and target-side user approval for newly privileged operations; it must expose typed protocol commands rather than a generic remote shell.

## Existing seams to retain

| Concern | Retain | Change |
| --- | --- | --- |
| Device worktree inspection, preparation, remote host connection, and frozen execution facts | `src-tauri/src/execution_targets/` | Resolve its `ExecutionBinding` from a saved Harness binding rather than from a Capability Profile field. Retain the existing SSH adapter as developer infrastructure only; networked target delivery resolves by stable Device identity. |
| Local Codex-home continuity/readiness | `src-tauri/src/native_profiles/` | Surface it as a local Codex Harness configuration, not as a competing Capability Profile editor. |
| Runtime observation and launch | `src-tauri/src/execution_configuration/native_codex.rs`, `src-tauri/src/execution_targets/endpoints.rs` | Make observation provider/harness owned and return a concise health result. |
| Profile narrowing, revision, Session Profile resolution, and node restrictions | `src-tauri/src/execution_configuration/` | Remove embedded device/connection fields; resolve the selected binding before validation. |
| Conversation/session lifecycle | `src-tauri/src/agent_sessions/` | Add only a draft command/query boundary; do not turn drafts into Sessions or invocations. |
| Generic Back and contextual origins | `src/application/productNavigation.ts` | Extend it with per-surface recency rather than create another router. |

`harness_engine/` remains the product MCP exposure/delivery mechanism. It is not renamed into a device Harness and retains its existing session binding authority.

## Backend shape

Create `src-tauri/src/execution_setup/` with `domain.rs`, `repository.rs`, `service.rs`, `health.rs`, and `transport.rs`. Its repository is a Coordinator-backed catalogue client with a local read cache, not another independently editable source of Device/Harness/Profile truth.

- `Device`: ID, display name, stable public identity, and device-local repository locations.
- `Harness`: ID, device ID, name, kind, and device-local configuration reference.
- `InferenceSource`: ID, name, source kind, and non-secret display metadata.
- `HarnessInferenceBinding`: ID, harness ID, inference-source ID, status, and discovered-runtime receipt/reference.
- The service owns CRUD, connection compatibility, non-secret inventory/health reads, and delete restrictions. The remote host and local runtime retain provider execution and secret authority.

Add a single active-database migration that derives an importable setup catalogue from unique legacy `CapabilityProfile.execution` values, creates a bounded imported Codex inference source/binding, and rewrites reusable profiles to reference that binding. The first controller submits this non-secret catalogue during Coordinator enrollment and records the accepted revision before another controller can edit it. Preserve existing profile ID, revision, capability set, and defaults. Existing Session Profile and execution-target snapshots stay readable as historical records; new writes use the new profile contract only.

Move `CapabilityProfile` to a route/binding reference plus its existing capability/default fields. `ExecutionTargetService` obtains a frozen `ExecutionBinding` from the resolved profile binding before any repository/worktree or runtime action. It must never reconstruct a mutable route from a historical Session snapshot.

Keep SSH `ExecutionConnection` readable only for historical compatibility inside the existing implementation, but do not create it from new product setup or expose it in a product DTO. New networked routes freeze the Device, Harness, binding, and provider facts; endpoint resolution looks up the target Device Agent's current authenticated connection by Device ID. The active socket, address, and availability receipt are runtime facts, not a Capability Profile field or historical session route.

Create an `orchid_network` boundary with a small Coordinator client on controllers and a Device Agent protocol on targets. It owns enrollment, device presence, command delivery receipts, reconnect behavior, and typed update/approval/cancellation multiplexing. Reuse the existing `HostCommand` semantics behind that boundary where they fit, but do not expose the JSON-lines SSH process as the network protocol. The Coordinator is the one durable owner for product/session/configuration identity when multiple controllers participate; a UI's local database remains a cache and draft store, not a competing source of truth.

Create `src-tauri/src/agent_sessions/composer_drafts.rs` and its repository/transport seam. Store a typed, versioned draft payload under one semantic key:

- `session:<session-id>`
- `new:repository:<repository-id>`
- `new:workflow:<workflow-instance-id>`
- `new:unattached`

Payload: text, optional working directory, Capability Profile/binding selection, model/reasoning/sandbox selection, workspace destination, and update timestamp. Route selections use Coordinator-stable IDs. Use one upsert/read/delete contract; there is no hidden expiry, background send, cross-controller draft sync, or provider transmission.

Add the migration and commands to `src-tauri/src/storage.rs` and register them in the existing Agent Session Tauri state. Test reopening the active database and the migration independently from runtime launch.

## Codex protocol health

The current adapter calls `skills/extraRoots/set`, while the installed 0.154 CLI reports that method as unsupported. Before changing setup UI, update the app-server contract capture and adapter against the actual supported 0.154 request/response schema in a disposable Codex home.

`execution_setup::health` reports structured states such as `ready`, `authentication_required`, `protocol_mismatch`, and `observation_failed`. It keeps raw protocol details diagnostic-only. Do not guess that `skills/config/write` has compatible parameters, parse an error string, or make a capability editor unusable when discovery fails.

## Frontend shape

Create `src/features/executionSetup/`:

- `DeviceSetupScreen` lists Devices and selects one.
- `HarnessList` and `HarnessEditor` configure the selected Device's harnesses.
- `InferenceSourceScreen` manages sources and `HarnessInferenceBindingEditor` connects them to harnesses.
- `HarnessHealthSummary` is the compact, actionable result of runtime discovery. Its expandable technical details hold raw diagnostics.

Adapt `TechnicalSettingsScreen` to select **Devices**, **Inference Sources**, and its existing native/OTP sections. Move local Codex homes under the relevant local Codex Harness flow. Remove its “Devices and Capability Profiles” section and its `ExecutionConfigurationScreen` dependency.

Rename/reframe `ExecutionConfigurationScreen` as the Capability Profiles surface. Retain the profile catalogue/editor split, but remove `ExecutionConnectionFields`, inline remote-host setup, and repository-device-location editing. Add a route selector that chooses an existing Harness–Inference binding, then reads that binding's catalogue. Keep defaults and allowed-capability controls here.

The profile catalogue becomes navigation only: profile list, New, and filtering. Move the default-profile control into the main surface. Give field-bearing side panels a practical minimum width/resizer; the current 230–280px catalogue must not contain a select plus explanatory copy.

Adapt `SessionComposerToolbar`, `useSessionTarget`, and target-selection DTOs to select a Capability Profile route first and a worktree destination second. Device is derived from the profile's bound Harness. Preserve the existing Session's frozen route and target unless the user explicitly prepares a transition.

## Draft and tab-return behavior

Add `composerDrafts` to the Agent Session application contract and a feature-local `useComposerDraft` hook. It loads before seeding defaults, debounces edits, flushes on semantic-context change/unmount, and guards late reads/writes with the draft key/revision.

`useAgentSession` remains the controlled conversation hook. It receives a hydrated draft and reports changes through the draft hook rather than keeping the only copy in `useState`. Replace random draft IDs as persistence keys with the semantic scope above; retain the ephemeral ID only where React needs it to distinguish an explicit new-draft action.

Extend `ProductNavigationState` with typed last destinations per top-level surface and a `select_surface` reducer action. The action snapshots the leaving surface, restores the selected surface's last valid destination or its default, and does not add generic Back history. Continue to clear unsafe draft/evidence destinations on reload; tab recency is in-memory only.

Use this one owner for Agent Sessions, Workflow, Capability Profiles, Technical Settings, Worktree Review, and other top-level tabs. Adapt Capability Profiles and Technical Settings to accept controlled location/selection props. Agent Sessions already accepts controlled selection; update its selection callback to refresh the stored Agent Sessions location. The initial acceptance scope is semantic location (for example, the same Session or profile), not exact scroll offset or every disclosure state.

## Concrete file changes

| Action | Files | Purpose |
| --- | --- | --- |
| Create | `src-tauri/src/execution_setup/{mod,domain,repository,service,health,transport}.rs` | Coordinator-backed Device/Harness/Inference catalogue client, bindings, health, legacy import, and commands. |
| Adapt | `src-tauri/src/execution_configuration/{capability_profile,repository,service,transport}.rs` | Binding-backed profile contract and migration-safe profile CRUD. |
| Adapt | `src-tauri/src/execution_targets/{domain,endpoints,inventory,transport,preparation}.rs` | Resolve and freeze a Harness-derived execution binding; retain target mechanics. |
| Create | `src-tauri/src/orchid_network/` and a target-side Device Agent service in `crates/orchid-engine/` | Enroll Devices, keep outbound authenticated connections, route typed execution commands, and report receipt/presence. |
| Adapt | `src-tauri/src/native_profiles.rs`, `src-tauri/src/storage.rs`, `src-tauri/src/lib.rs` | Setup integration, schema migration, and command registration. |
| Create | `src-tauri/src/agent_sessions/{composer_drafts.rs,repository/composer_drafts.rs,transport/composer_drafts.rs}` | Durable composer-draft store and Tauri boundary. |
| Adapt | `src-tauri/src/agent_sessions/{mod,ports,application,transport}.rs` | Typed draft client and send-acceptance cleanup. |
| Create | `src/application/{executionSetup,agentSessions/composerDrafts}.ts` and matching `src/infrastructure/` Tauri clients | Browser-safe contracts and transport adapters. |
| Create | `src/features/executionSetup/` | Device, Harness, Inference Source, binding, and health UI. |
| Adapt/remove | `src/features/{technicalSettings,executionConfiguration}/` | Remove setup duplication; delete `ExecutionConnectionFields.tsx`; retain Capability Profile policy editor. |
| Adapt | `src/features/agentSessions/{useAgentSession,useSessionTarget,SessionComposerToolbar,AgentSessionScreen}.tsx` | Hydrated semantic drafts and binding-derived target controls. |
| Adapt | `src/application/productNavigation.ts`, `src/app/App.tsx` | One typed owner for per-tab return locations. |
| Revise | `docs/execution-configuration.md` | Replace the current profile-contains-device description and document ownership boundaries. |

## Validation

- Migration: imported profiles retain ID/revision/policy, route to a generated binding, existing Session snapshots remain readable, and deletion is restricted while referenced.
- Setup: a Device can own multiple Codex Harnesses; a Harness can connect only to compatible Sources; profile save rejects a non-ready/mismatched binding without deleting its previous revision.
- Protocol: fixture/contract test against the installed CLI version; protocol mismatch yields structured health plus diagnostics, not a raw editor-blocking error.
- Drafts: text and every listed choice survive tab switches and app restart per semantic key; repository, workflow, and unattached new drafts never overwrite one another; accepted send clears only its draft; failed/uncertain send retains it.
- Navigation: Agent Sessions -> Capability Profiles -> Agent Sessions restores the same session. The same holds across another top-level surface. Tab selection does not add a generic Back entry or weaken contextual-return behavior.
- UX: the profile list stays readable at normal desktop width, field controls do not live in a forced 230–280px rail, and keyboard focus returns to the restored semantic selection.
- Network control: no Device, Harness, Profile, or Technical Settings UI exposes SSH. With the laptop offline, an authenticated Android controller can obtain Coordinator receipt for a request to an online remote Device; target acceptance and processing are reported separately. An offline Device never reports the request as executed.

## Out of scope

- Credential copying, account switching inside one Codex configuration, or secret editing in Orchid.
- Provider execution beyond the currently supported Codex Harness.
- An Android Device Agent, peer-to-peer mesh, credential synchronization, generic remote shell, or automatic execution of queued work. The first network slice makes Android a controller for an already-enrolled target Device.
- Persisting tab recency, scroll position, or every disclosure state across application restart.
- Workflow-engine or product Conversation Harness redesign beyond adapting their existing profile references.
