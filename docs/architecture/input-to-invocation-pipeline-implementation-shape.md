# Input-to-invocation pipeline implementation shape

Implementation is now represented by
[`input-to-invocation-pipeline.md`](./input-to-invocation-pipeline.md). This file remains the change
shape and verification checklist rather than the description of current ownership.

## Objective

Give every prompt source one durable Orchid-owned path from semantic input to one immutable provider invocation. Keep provider-native request formats, process behavior, discovery, and interaction handles inside provider adapters. Keep Workflow and OTP responsible for workflow semantics, not provider execution.

The change should correct the current prepared-path omissions, remove the competing legacy launch path, and leave clear extension points for provider, Workflow, and OTP features without implementing unrequested features.

## Behavioral rules

- An Orchid Session is the canonical conversation. Provider-native sessions are caches of parts of it.
- A running invocation keeps one provider, configuration, device, workspace, and native interaction target until terminal.
- Steering and interaction responses always return to that invocation's runtime. They never trigger route selection.
- Provider or device changes occur only when preparing a later invocation.
- Native provider sessions are not transferred between devices or configurations.
- A destination provider instance resumes only its own stored handle. A new instance starts from Orchid context.
- “Previously used” is determined by a stored provider-session binding and context cursor, never by a Capability Profile or local execution profile.
- Workflow and OTP do not receive provider runtimes, provider handles, repositories, or Tauri state.

## Target flow

1. A caller submits an `InvocationDeliveryIntent` with a stable delivery ID.
2. Agent Sessions durably accepts the Session and pending invocation.
3. The delivery planner resolves the Session policy, the route that exposes the user's selected model, its provider configuration, the device, workspace, and invocation selections.
4. The continuity planner finds the destination provider-session binding and computes the canonical context it has missed.
5. The context assembler combines caller input, initial instructions, missed conversation context, and pinned skill guidance without flattening their provenance.
6. The capability preparer resolves the invocation snapshot and asks the Harness Engine for authorized managed MCP and skill-reader exposure.
7. The selected provider prepares its local or remote native launch data.
8. The runtime prepares one immutable invocation and returns its native context identity.
9. Agent Sessions atomically commits the invocation binding, provider-session binding, execution target, and invocation snapshot.
10. Agent Sessions records delivery intent, releases the prompt once, and records launch acceptance.
11. Runtime events and control records update the canonical Session log and the provider-session context cursor.

Preparation remains asynchronous for the composer and recoverable across restart. Application callers may await launch acceptance through the same coordinator; they must not use another launch implementation.

## Core contracts

### Delivery intent

Create `agent_sessions/application/delivery/contracts.rs` with:

- `InvocationDeliveryIntent`
  - stable delivery/invocation ID;
  - user or application provenance;
  - new or existing Session target;
  - `InvocationContent`;
  - route/workspace intent;
  - model, reasoning, sandbox, and approval selections;
  - optional Session creation policy and identity.
- `InvocationContent`
  - one primary query;
  - ordered `ContextPart` values with typed provenance.
- `ContextPart`
  - initial instructions;
  - referenced product content;
  - missed conversation turn;
  - skill guidance.
- `InvocationDeliveryReceipt`
  - Session and invocation IDs;
  - durable preparation phase;
  - launch-acceptance evidence when requested synchronously.

Do not add speculative media variants. Preserve enough structure to add them without another string-prefix channel.

### Resolved invocation

Add a provider-neutral engine contract in `crates/orchid-engine/src/contracts/invocation.rs`:

- `ResolvedRuntimeInvocation`
  - immutable Session and invocation IDs;
  - structured `InvocationContent`;
  - working directory;
  - resolved model/reasoning/sandbox/approval intent;
  - authorized managed MCP servers;
  - pinned and explicitly invoked skills;
  - provider-scoped options;
  - provider-scoped launch payload.

Replace `submitted_text` plus `initial_prompt_prefix`. Split the current `RuntimeLaunchExtension` responsibilities between resolved product intent and an opaque provider launch payload. Environment variables and executable paths remain provider-owned data rather than general product fields.

Each provider adapter renders `InvocationContent` into its native input representation. Codex may emit text and native skill items; Claude may render the same semantic parts into its stream-json input. Orchid should not construct provider-flavored XML wrappers.

### Provider-session bindings

Replace current/parked provider conversations with a collection of `ProviderSessionBinding` records:

- Orchid Session ID;
- provider;
- concrete provider configuration;
- device/execution endpoint identity;
- native external context ID;
- runtime version;
- last context cursor known to be present in that native session;
- created and last-used timestamps.

Use the provider configuration and execution endpoint as part of the identity. Do not key only by provider.

The Session may identify the binding used by its most recent terminal invocation for presentation, but an invocation stores its own immutable binding snapshot. Update a binding's cursor only from durable delivery/terminal evidence. Failed preparation must not advance it.

Remove native continuation export/install. Moving to another device selects or creates that device's provider-session binding and supplies the missing canonical context.

### Interaction handles

Remove `RuntimeTurnTarget { thread_id, turn_id }` from the provider-neutral public shape. Prefer runtime-owned lookup by invocation ID. If a target must cross the remote-host protocol, use a provider-scoped opaque envelope validated by the selected provider.

Persist neutral steering/request/response evidence. Do not persist a fabricated common provider handle.

### Session policy and invocation snapshot

Split the current overloaded Session Profile concept:

- `SessionPolicySnapshot`: immutable constraints attached at Session creation, primarily for Workflow-addressed Sessions.
- `InvocationExecutionSnapshot`: exact provider route, runtime capabilities, selections, skills, MCP exposure, provider options, and policy revision used by one invocation.

Ordinary Sessions resolve a fresh invocation snapshot from the selected Capability Profile. Workflow Sessions additionally narrow it through their immutable Session policy. Preserve the creation snapshot for provenance; do not use it as the current provider configuration after later route changes.

## Application ownership and files

### Agent Sessions

Create `src-tauri/src/agent_sessions/application/delivery/`:

- `mod.rs`: `InvocationDeliveryService` facade and public use cases.
- `contracts.rs`: intent, content, receipt, and resolved plan types.
- `acceptance.rs`: idempotent durable acceptance and worker scheduling.
- `planning.rs`: route, workspace, Session policy, and invocation snapshot resolution.
- `continuity.rs`: provider-session lookup and missed-context cursor planning.
- `context.rs`: canonical context extraction and ordered content assembly.
- `capabilities.rs`: skill selection and Harness preparation.
- `execution.rs`: provider preparation, runtime prepare, atomic commit, and one-time delivery.
- `recovery.rs`: startup classification, retry boundaries, cancellation, and delivery uncertainty.

Keep `AgentSessionApplication` as the Tauri/application facade, but have it delegate all prompt delivery to this service.

Adapt:

- `application/preparation.rs` into `delivery/acceptance.rs` and `delivery/recovery.rs`.
- `application/preparation/execution.rs` into the shared planner/executor.
- `application/history_handoff.rs` into `delivery/context.rs` using canonical typed context.
- `application/interactions.rs` to resolve only the invocation's stored runtime binding.
- repository ports and SQLite storage for provider-session bindings and invocation snapshots.

Remove after migration:

- `application/invocation.rs` launch sequencing;
- `application/direct_user.rs` launch sequencing;
- `application/addressed.rs::deliver_profiled_message`;
- `application/preparation/conversation.rs` current/parked-provider model;
- `agent_session_parked_conversations` and first-prefix persistence after migration;
- `add_workspace_capabilities` if it remains a no-op.

Retain split prepare/deliver semantics, launch-acceptance evidence, update gates, durable diagnostics, and invocation update lanes.

### Execution Configuration

Keep Capability Profiles as design-time route policy. Make route selection strict:

- remove the unmatched-configuration fallback to the default route;
- expose one model catalogue per Capability Profile and device while keeping route composition internal;
- resolve each selected model to exactly one route and derive the provider from that route;
- reject missing or ambiguous model-to-route resolution instead of falling back to another provider;
- represent the selected route explicitly in the resolved delivery plan;
- enforce model allowances as route policy, or rename them before migration if they are intended only as suggestions;
- remove legacy flat `execution`, `defaults`, and `allowed_capabilities` fields after persisted-profile migration;
- compile product skills and provider-native skills into exact identities before invocation planning.

Provider setups remain provider-owned. Execution Configuration consumes their neutral catalogue; it does not implement provider readiness or authentication.

### Execution Targets and devices

Execution Targets own:

- device readiness;
- workspace/worktree materialization;
- target transition and sister-worktree state;
- local versus remote endpoint routing.

They do not own provider-session continuation. Delete `transfer_continuation`, continuation registrations, and host export/install commands. A target transition may queue a later delivery intent, but it must not mutate a running invocation.

### Harness Engine

Make Harness preparation a required delivery stage after the invocation snapshot is resolved and before provider preparation.

The Harness Engine continues to own:

- Session-policy binding;
- provisioner invocation;
- managed MCP selection and proxying;
- per-invocation proxy preparation;
- Job Agent and skill-reader upstream mediation.

Change its input from re-reading an ambiguously current `session_profile` to the resolved Session policy and invocation snapshot passed by the delivery coordinator. Remove the orphan `otp_host/agent_mcp.rs` trait.

### Workflow, Session Events, and OTP

Keep Workflow prompt-input resolution and durable references in Workflow. Keep target selection logic in OTP packages. Convert `SessionRequest` and user entry requests into `InvocationDeliveryIntent` without flattening referenced prompt parts.

Session Events continue to own:

- logical/exact addressing;
- create-on-missing policy;
- fan-out and deterministic delivery IDs;
- source and contribution persistence.

Replace their invocation dispatcher with a delivery-coordinator port. Remove direct use of `deliver_profiled_message`.

OTP retains only `node`, `sessions`, and `emit` host handles. New OTP abilities should be added as explicit curated outputs/handles when a real feature requires them, not as access to the delivery service.

### Existing orchestration

Migrate `orchestration/application.rs`, bootstrap and sprint transitions, conversation harnesses, and `product_decisions.rs` to construct delivery intents. Their role-specific instructions, durable attempt IDs, and managed action grants remain feature-owned context/capability inputs.

Do not retain a compatibility launch path after all callers migrate. Temporary adapters may exist only within the migration and should be deleted in the final slice.

## Provider ownership and files

Replace the four independent provider maps with one `ProviderRegistration` per provider. It should declare:

- configuration/setup source;
- runtime factory;
- local launch preparer;
- supported interaction kinds;
- local and remote availability;
- native skill/model discovery;
- provider option codec.

Absence of a capability is explicit. Native session transfer is not a capability.

Keep product-side provider code under `src-tauri/src/runtime/providers/<provider>/` for setup persistence, discovery, and local composition. Keep native protocol/process code under `crates/orchid-engine/src/providers/<provider>/`.

Change remote host configuration from Codex-shaped `executable` and `home` fields to a provider-scoped configuration envelope. Register Claude on the host only when its remote setup and tests are implemented; until then return an explicit unsupported-provider result. Continue rejecting remote managed MCP until the Harness proxy transport is deliberately supported.

In the frontend, create one explicit provider composition registry for descriptors, setup clients, and settings destinations. Generic screens should render registry entries. Provider-specific setup forms remain in provider folders. The composer exposes the Capability Profile's models; the selected model implicitly identifies its provider route, while provider composition, setup routing, and adapter mechanics remain hidden. Remove hard-coded Codex/Claude navigation branching and wire or remove `ExecutionConnectionFields`.

No external library is needed. Existing serde, SQLite, Tauri, and runtime process abstractions cover the required contracts and persistence.

## Canonical context rules

Build missed context from durable Orchid records, not provider transcripts. Include only evidence relevant to conversational continuity:

- initial Session instructions;
- accepted primary messages;
- accepted steering inputs in order;
- final agent replies;
- interaction answers when they materially changed execution;
- terminal outcome when a turn failed or was canceled.

Do not replay hidden reasoning or arbitrary raw provider events. Keep product references and statuses structured until the provider adapter renders them.

The context cursor should identify the last canonical record represented in a native provider session. Tests must distinguish accepted, delivery-uncertain, failed-before-launch, running, completed, and canceled invocations.

## Migration sequence

1. **Characterization and immediate defects**
   - Add failing coverage proving prepared delivery includes initial/history context and Harness MCP exposure for Codex and Claude.
   - Correct those omissions without creating another permanent path.
   - Add a strict route-resolution test and remove the unmatched default fallback.

2. **Contracts and provider rendering**
   - Add structured invocation content and resolved invocation contracts.
   - Adapt Codex and Claude to render them.
   - Split product intent from provider launch payload.

3. **Provider-session persistence**
   - Add provider-session binding and context-cursor storage.
   - Migrate current and parked handles where their route is known.
   - Stop exporting/installing native sessions across routes.

4. **Unified delivery coordinator**
   - Move prepared-session acceptance, planning, Harness preparation, provider preparation, commit, delivery, and recovery into `application/delivery/`.
   - Switch the ordinary Agent Session composer to the coordinator.

5. **Workflow and OTP migration**
   - Preserve prompt parts through Session Events.
   - Route new/exact Session requests through the coordinator.
   - Verify Workflow node policy, tools, identity, and launch evidence.

6. **Orchestration migration**
   - Convert each application-owned launch family to delivery intents while retaining its durable attempt semantics.
   - Delete the legacy invocation sequencing after the last caller moves.

7. **Provider and setup composition**
   - Introduce provider bundles in the app and remote host.
   - Consolidate frontend provider composition.
   - Make unsupported local/remote capabilities explicit.

8. **Cleanup and migration retirement**
   - Remove old columns/tables only after data migration and rollback-safe verification.
   - Delete continuation transfer, duplicate traits, no-op helpers, old commands, DTOs, clients, and tests tied only to removed paths.
   - Update architecture documentation to describe the resulting contracts, not the migration.

## Verification matrix

Exercise each source through the same coordinator:

- new direct-user Session;
- later direct-user message on the same provider instance;
- never-used provider;
- previously used provider with missed turns;
- same provider on a different configuration/device, with and without an existing destination binding;
- active-turn steering while later route selections differ;
- Workflow user entry to a new node Session;
- Workflow output to one and multiple existing node Sessions;
- OTP MCP call from an authorized Session;
- Job Agent and skill-reader MCP exposure;
- orchestration application-owned launch and recovery;
- local Codex and Claude;
- remote Codex and explicit unsupported remote Claude;
- cancellation during preparation, delivery uncertainty, restart recovery, and idempotent retry.

For every scenario verify the persisted intent, resolved route, invocation snapshot, provider-session cursor, Harness exposure, native request, normalized events, launch evidence, and canonical Session history separately.

Run focused contract/provider tests first, then Agent Session, Workflow/OTP, orchestration, frontend, full Rust, and live provider smoke checks. A mocked native request proves translation, not account-level execution; report those separately.

## Explicitly out of scope

- Moving a running invocation between providers or devices.
- Transplanting native provider session data between devices.
- Dynamic third-party provider plugin loading.
- New Workflow or OTP capabilities without a current feature consumer.
- Remote managed MCP transport.
- Multimodal input implementation.
- General synchronization of provider homes, credentials, or device files.

## Completion criteria

- Every production prompt launch enters `InvocationDeliveryService`.
- Codex and Claude receive identical Orchid semantic context through provider-owned rendering.
- Every eligible invocation receives its resolved Harness exposure.
- Provider-cache reuse is based on the exact provider-session instance and durable context cursor.
- Active interactions cannot change provider/device/runtime.
- Workflow, OTP, and orchestration no longer call a legacy launch path.
- Provider registration and setup UI have one discoverable composition point per layer.
- Removed implementations, tables, commands, and dead files have no remaining callers.
- The scenario matrix passes with local live evidence clearly separated from mocked and remote evidence.
