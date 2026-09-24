# Codex parity, reversible navigation, managed devices, and MCP annotations

Status: implemented on `refinement/usability`, based on checkpoint `e74116f`.

## Target and boundaries

This pass should make four related corrections:

1. Codex-backed Sessions inherit the personality behavior of their selected Codex profile unless a Capability Profile route supplies a Codex-specific override.
2. Product navigation becomes one reusable, reversible model with Back and Forward across every top-level surface.
3. Remote devices gain product-owned lifecycle configuration so moving or dispatching work can start a device, wait for its Orchid Device Agent, and shut it down after one hour without Orchid-managed activity.
4. Every Orchid-owned MCP tool declares accurate MCP annotations, and Orchid preserves upstream annotations whenever it filters or proxies non-Orchid tools.

Capability Profiles remain design-time configuration. These changes do not add broad model/reasoning enforcement or new Agent Session controls. Personality is a Codex provider option, not a generic model setting, and does not appear in Agent Sessions. This pass does not add or edit developer instructions, collaboration instructions, or internal Codex prompts.

The first remote lifecycle controller is the local laptop. It may call generic start/stop programs configured for a device. No Android controller, cloud coordinator, Hetzner-specific API client, or controller-to-controller protocol is introduced. SSH remains developer-owned connection configuration and is not exposed as a Capability Profile or ordinary UI setting. The remote Orchid host is installed as an operating-system service and starts at boot; the start command powers on the machine, then Orchid waits for that service to become ready.

MCP annotations are advisory metadata, not authorization. Orchid should not use them to implement its own parallel scheduler. Codex remains responsible for batching and scheduling tool calls.

## 1. Keep personality inside the Codex provider boundary

### Persist a Codex-profile default

Add a provider-specific value type near the Codex adapter, for example:

```text
CodexPersonalitySelection = inherit | none | friendly | pragmatic
```

`inherit` is the default and means Orchid omits the app-server personality parameter. Codex then resolves the setting from the selected `CODEX_HOME` exactly as an ordinary Codex launch does. The other values are explicit Orchid overrides. `none` must remain distinct from `inherit`.

Adapt:

- `src-tauri/src/native_profiles.rs`
  - add `personality_selection` to `native_codex_profiles` and `NativeProfileDto`;
  - add one narrow update command that validates the four values;
  - migrate existing profiles to `inherit` when schema version 55 advances;
  - keep health/login/tool diagnostics independent of this preference.
- `src/infrastructure/nativeProfiles/nativeProfileClient.ts`
  - transport the value and expose the update operation.
- `src/features/nativeProfiles/CodexProfilesScreen.tsx`
  - add a **Personality** selector in the selected Codex profile details;
  - explain `inherit` as “Use this Codex profile’s configuration”; do not show it in Agent Sessions.

Do not parse and copy the Codex home’s effective personality into Orchid. Omission is the inheritance mechanism and avoids a stale duplicate of `config.toml`.

### Allow a Capability Profile route to override it

Add an optional `codexPersonality` to `ProfileRoutePolicy` in `src-tauri/src/execution_configuration/capability_profile.rs` and the corresponding TypeScript contracts/editor. The collapsed route summary need not display it; the expanded route’s Codex settings should offer:

- Use Codex profile default;
- None;
- Friendly;
- Pragmatic.

Resolve the route value at Session instantiation:

1. explicit route override;
2. selected native Codex profile preference;
3. `inherit`/no app-server parameter.

Store the resolved Codex selection in `SessionProfile` so an existing Session does not change when either profile is edited later. Put it in `RuntimeLaunchExtension` as an explicitly named Codex field, not `AgentRuntimeOptions`, model allowances, or a generic personality abstraction.

In `crates/orchid-engine/src/codex/app_server/mod.rs`, serialize an explicit value to the app-server `personality` field on both `thread/start` and `thread/resume`; omit it for `inherit`. Include the returned/effective personality in the internal effective-configuration event for diagnostics, but do not add a normal transcript field. Add adapter tests for all four selections, start/resume parity, and omission on inherit.

Delete no developer/collaboration prompt behavior because none should be introduced. Equal Codex behavior in this pass means letting the same `CODEX_HOME` and app-server defaults apply, plus the supported personality override.

## 2. Replace parallel screen state with one product navigation model

`src/app/App.tsx` currently owns both `surface` and `ProductNavigationState`; some tabs update only one. This is why the current location is not consistently restorable. Make `ProductNavigationState` the sole source of top-level location and derive the rendered surface from its typed destination.

### Extend the typed destinations

Adapt `src/application/productNavigation.ts`:

- add `capability_profiles` with selected saved/new profile identity;
- add `technical_settings` with the selected section and, where relevant, selected Codex profile/device identity;
- retain typed Agent Session, Workflow, Worktree Review, File Review, and contextual-origin locations;
- add a `future` stack and a `forward` action;
- add `canNavigateForward` beside `canNavigateBack`.

Use three distinct operations:

- **push** for a user moving to another product destination; append current to history and clear future;
- **update current** for selection changes inside the current surface; replace its typed location without creating a navigation entry;
- **back/forward** for moving entries between history/current/future.

Keep contextual Return separate from generic Back/Forward. A contextual return is still a typed product action and clears only the contextual origin it consumes. Reload restores only one validated current location; history, future, and contextual origin are not persisted.

### Centralize application consumption

Create `src/app/useProductNavigation.ts` to own dispatch, support checks, navigation-epoch invalidation, and destination-to-surface mapping. Remove `surface`, `setSurface`, and repeated `productNavigationEpoch` mutations from `App.tsx`. Top tabs, feature callbacks, contextual opens, and asynchronous File Review completion should all call this controller.

Make `ExecutionConfigurationScreen`, `TechnicalSettingsScreen`, and their selected-detail children controlled by typed location. Draft contents remain in their existing application-owned workspaces; navigation stores identity/section, not form data.

### Add buttons and mouse input through the same commands

Adapt `src/app/ProductCommandBar.tsx`:

- render icon-only Back and Forward buttons at all times;
- disable each independently;
- provide `aria-label`, title, and focus behavior;
- retain the separate contextual Return affordance.

Create a small `src/app/useHistoryMouseButtons.ts` adapter that maps mouse buttons 4/5 to the same Back/Forward callbacks. First verify `mouseup`/`auxclick` delivery in the packaged WebView2 application. If WebView2 consumes those buttons, add one narrow Tauri window-event bridge and keep the React adapter/controller unchanged. Do not register both paths simultaneously.

Tests should cover history/future ordering, a new push clearing future, typed equality, unsupported destinations, contextual return, delayed async callbacks, exact Agent Session restoration, Technical Settings subsections, Capability Profile selection, command-button disabled states, keyboard accessibility, and mouse input.

## 3. Give device lifecycle configuration its own ownership boundary

The current `ExecutionTargetService.devices()` derives devices from Capability Profiles, and `ExecutionBinding` embeds a transport connection in every route. That makes a configuration policy the accidental authority for device existence. Correct that ownership before adding lifecycle behavior.

Create `src-tauri/src/execution_devices/` with:

- `domain.rs` — `ExecutionDevice`, `DeviceConnection`, `DeviceLifecyclePolicy`, and lifecycle state/operation types;
- `repository.rs` — SQLite persistence and one-time migration;
- `service.rs` — device lookup, lifecycle transitions, readiness, and idempotency;
- `command_runner.rs` — bounded one-shot lifecycle program execution;
- `activity.rs` — durable Orchid activity leases and idle evaluation;
- `transport.rs` — Tauri query/update/start/stop operations.

### Separate route reference from resolved execution

Replace the overloaded `ExecutionBinding` with two explicit types:

- `ExecutionRouteRef`: stable device ID plus harness/provider/configuration reference, stored by a Capability Profile;
- `ResolvedExecutionBinding`: route fields plus the device name and developer-owned connection needed by a concrete Session/target.

Move `ExecutionConnection::{Local,Ssh}` to the device record. Capability Profile JSON should no longer embed SSH targets or host executable paths. Session profiles and execution targets retain a resolved snapshot so existing Sessions remain stable if device configuration later changes.

Advance the active schema and migrate existing route JSON by deduplicating embedded connections into device rows keyed by `device_id`, rewriting saved Capability Profiles to references, and preserving resolved connection snapshots in existing Session target JSON. Reject conflicting historical definitions for the same device instead of silently choosing one.

Adapt `src-tauri/src/execution_targets/inventory.rs`, `mod.rs`, `preparation.rs`, and `endpoints.rs` to join route references through `ExecutionDeviceService`. Capability Profile loading remains offline and validates references only; it must not probe or start a device.

### Configure generic lifecycle programs

`DeviceLifecyclePolicy` should contain optional structured start and stop commands and an optional idle timeout:

```text
program: absolute executable/script path
arguments: ordered string list
workingDirectory: optional absolute path
timeoutSeconds: bounded value
idleShutdownSeconds: optional; default 3600 when enabled
```

Do not store one interpolated shell command. Do not store Hetzner credentials in Orchid. The configured program obtains its credential from the laptop’s credential store/environment and may call any provider API. This makes the Hetzner setup one device configuration without introducing a Hetzner implementation into the product.

Reuse `ProcessLaunchSpec` and `ChildProcessFactory` from `crates/orchid-engine/src/processes`, extracting a bounded one-shot runner there if necessary. Keep lifecycle policy, persistence, and retries in `execution_devices`; do not force a device operation into `ProcessSupervisor`’s agent-invocation identity. On Windows the existing factory supplies hidden-window and process-tree cleanup behavior.

The start flow is:

1. probe the existing Orchid host connection;
2. if ready, return success without running the command;
3. run the configured start command once;
4. poll the existing authenticated Orchid host/SSH readiness seam until ready or timed out;
5. record request, command result, readiness result, and terminal outcome separately.

The stop flow refuses while an Orchid activity lease exists, runs the stop command, and verifies that the host becomes unavailable. A missing command is a clear “manual lifecycle” state, not a runtime crash.

### Make Technical Settings the device surface

Replace the derived `DeviceSetupOverview` with a device list backed by `ExecutionDeviceService`. A device detail/modal edits display name, start/stop command specifications, and idle shutdown. Connection details may be summarized as developer configured but are not editable or secret-bearing in this UI. Local Codex profile/harness configuration remains in Codex profiles.

Capability Profile route pickers read this registered device/harness catalogue. Saving a profile references configured entities and never tests whether a remote machine is currently on.

## 4. Integrate lifecycle with remote Agent Session operations

### Make offline devices selectable

`listDevices` should return configured devices regardless of reachability. Worktree inventory should report an unavailable/offline status rather than deleting the device. In `DeviceContinuationDialog`, selecting an offline managed device starts an explicit **Prepare device** phase, shows start-command and readiness progress, and then loads its worktrees. The transition start repeats the readiness operation idempotently so a stale dialog cannot bypass it.

Add `EnsureDestinationDeviceReady` as the first task in `SessionTargetTransition`. Move the first destination inspection in `request_target_transition` until after readiness; the present eager inspection prevents even planning against a powered-off destination. Keep source inspection and migration checks separate from device activation.

The ordinary remote runtime preparation path should also call `ensure_ready` before opening an endpoint, because remote work can begin without a device-move dialog. Local targets take the no-op ready path.

### Track only Orchid-managed activity

Persist `execution_device_activity_leases` keyed by device, owner kind, and durable owner ID. Acquire a lease for:

- an active remote Agent Session invocation;
- an active target transition or remote preparation/transfer.

Release it only when the corresponding durable operation reaches a terminal state. On application startup, reconcile leases against durable invocation/transition state before evaluating shutdown.

A local scheduler checks managed remote devices at a modest interval. When the last lease has ended and `last_orchid_activity_at + idleShutdownSeconds` has passed, it requests the same idempotent stop flow used by the UI. The one-hour timer therefore measures Orchid-managed work, as requested; the UI must warn that unrelated manual server work is not activity and offer a simple temporary keep-awake hold before enabling automatic shutdown.

The laptop application must be running to start a powered-off device or perform an idle shutdown. Keep the controller contract behind `DeviceLifecycleController` so a future always-on Orchid node can implement it, but do not build remote controller discovery, credential transfer, Android control, or coordinator networking in this pass.

## 5. Publish and preserve MCP ToolAnnotations

Use `rmcp::model::ToolAnnotations` and the existing `#[tool(annotations(...))]` support rather than defining a competing wire model.

### Annotate every Orchid-owned producer

Audit every `tools/list` producer and give each tool all applicable fields (`title`, `readOnlyHint`, `destructiveHint`, `idempotentHint`, `openWorldHint`):

- rmcp macro tools in `src-tauri/src/orchestration/mcp.rs`, `bootstrap_transition.rs`, `sprint_runner_transition.rs`, and `native_profiles.rs`;
- the session skill reader in `src-tauri/src/execution_configuration/skill_reader.rs` (`read_skill` is read-only, non-destructive, closed-world);
- Workflow OTP tools in `src-tauri/src/otp_host/mcp.rs`;
- Agent MCP descriptors and the Job Agent facade in `src-tauri/src/otp_api/contract.rs`, `src-tauri/src/otp_packages/**`, and `src-tauri/src/otp_host/job_agent.rs`.

Add an annotation field to both `ToolDescriptor` and `AgentMcpToolDescriptor`, declare it beside each tool definition, and serialize it unchanged into `tools/list`.

Classify from actual behavior, not names:

- pure reads of Orchid state/files use `readOnlyHint: true`;
- commands that record, create, update, enable/disable, hand off, launch, or request work use `readOnlyHint: false`;
- `destructiveHint` is true only when an operation can remove, archive, reject, replace, or disable existing state;
- `idempotentHint` is true only when repeating the same arguments is guaranteed to add no effect, not merely because duplicate calls return an error;
- `openWorldHint` is true for tools that contact provider/external sources or affect actors outside Orchid’s closed state; local database/file reads remain false.

Add catalogue tests that fail when an Orchid-owned tool omits annotations and focused assertions for every read-only tool. These hints must remain honest even if a more permissive value would allow more parallel calls.

### Preserve non-Orchid annotations exactly

`src-tauri/src/harness_engine/proxy.rs` currently filters `result.tools` by retaining whole JSON objects. Keep that representation and add regression tests proving that annotation objects and unknown extension fields survive:

- unfiltered JSON pass-through;
- selected-tool JSON filtering;
- complete and chunk-split SSE `tools/list` events;
- Workflow-warning response rewriting;
- OTP/managed MCP routing.

Do not deserialize an upstream tool into Orchid’s narrower descriptor simply to filter it. If an upstream server supplies annotations, forward them exactly. If it omits them, preserve that omission; Orchid cannot accurately infer the behavior of an external tool.

Codex-profile-native MCP servers are not proxied when `native_mcp_enabled` allows Codex to load them. Keep that direct Codex ownership so their `tools/list` annotations reach the runtime without Orchid normalization. Add an adapter/integration fixture that exposes annotated native tools and verifies Orchid’s launch configuration does not replace or shadow their server definition.

No Orchid batching executor, call reordering, or annotation-based authorization belongs in this pass. Once accurate metadata reaches Codex, the Codex runtime handles parallel eligibility and scheduling.

## Implementation sequence

1. Add failing contract tests for personality inheritance/override, history/forward semantics, offline device registration, idle leases, and MCP annotation preservation.
2. Add Codex-profile and route personality persistence, resolve it into immutable Session launch data, and serialize it through the Codex app-server adapter.
3. Extend typed product destinations and history/future, make the controller the single App owner, then add command-bar and mouse adapters.
4. Add the device repository/service and migrate connection ownership out of Capability Profiles while retaining resolved Session snapshots.
5. Add structured lifecycle command execution, readiness, durable activity leases, Technical Settings controls, and idle shutdown.
6. Integrate `ensure_ready` with device selection, target transition, preparation, and remote runtime launch.
7. Add annotations to Orchid/OTP tool catalogues and macro tools, then lock proxy/native preservation with JSON/SSE integration tests.
8. Run focused tests after each boundary, then broad frontend/Rust builds and packaged native flows.

## Verification and acceptance

- With a Codex profile set to inherit, Orchid omits personality and a test `CODEX_HOME` default is observed. Explicit profile and route overrides produce the expected `thread/start` and `thread/resume` parameter. No Agent Session personality control exists.
- No developer/collaboration instruction editor or prompt-copying behavior is introduced.
- Every top-level tab participates in typed Back/Forward. The buttons are always present, icon-only, and correctly disabled. Mouse buttons 4/5 invoke the same commands. Returning to Agent Sessions, Capability Profiles, or a Technical Settings subsection restores the exact typed selection.
- Devices exist independently of Capability Profiles. A profile can be created while a registered device is offline. No Capability Profile query/save starts a device or probes runtime promises.
- A powered-off fake remote device is started during device preparation/remote dispatch, becomes ready through the normal host probe, and proceeds through worktree selection/transition. Concurrent requests cause one start operation.
- Remote invocations/transitions hold activity leases. One hour after the final durable Orchid activity, the configured stop command runs once; active work and a keep-awake hold prevent it. Restart reconciliation neither strands nor prematurely clears a lease.
- The Hetzner demonstration uses local start/stop adapter scripts and an OS-service-hosted Orchid Device Agent; no Hetzner API secret appears in Orchid storage or logs.
- Every Orchid-owned tool in every `tools/list` response has reviewed annotations. Read-only tools are visible as read-only to Codex; mutating tools are never mislabeled merely to increase concurrency.
- Annotated third-party/OTP tools retain the exact annotation object through unfiltered, filtered, and SSE proxy paths. Codex-native MCPs retain their own annotations because Orchid does not rewrite their server protocol.
- Tests verify call results and ordering are unchanged; Orchid adds no parallel scheduler and relies on Codex for batching.
- Run focused navigation, native-profile, execution-configuration, target-transition, device-lifecycle, OTP, skill-reader, and harness-proxy tests; then run the broad frontend suite, `npm run build:frontend`, Rust tests/checks, a packaged mouse-navigation smoke flow, and a fake-provider remote lifecycle flow.

## Explicit non-goals

No Agent Session personality chooser; no broad model/reasoning enforcement; no additional developer or collaboration instructions; no Android controller; no always-on coordinator; no Hetzner-specific product integration; no SSH credential UI; no general secret store; no external-work activity detection; no multiple per-session MCP sidecars; no synthesized annotations for unknown third-party tools; and no Orchid-owned MCP batching or scheduling.
