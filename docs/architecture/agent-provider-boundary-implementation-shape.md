# Agent provider integration boundary: implementation shape

Status: boundary foundation implemented on `feature/claude-support`, 2026-09-25. This document records the target shape, completed work, and remaining migration checkpoints.

Source baseline: local `main` at `8462dc3`. The existing optional-personality serialization behavior was preserved.

## Implementation status

Completed in this branch:

- provider-owned Codex directories in the shared engine, desktop runtime, and frontend;
- separate runtime, configuration, and continuation registries with explicit unsupported-provider behavior;
- provider-tagged native options and continuation payloads;
- generic interaction responses whose Codex wire encoding stays in the Codex adapter;
- provider-aware remote commands and a checked host protocol version;
- destination Agent Session creation when an already-bound ordinary session changes execution identity;
- provider-neutral UI wording on shared execution surfaces and an explicit Codex personality component;
- architecture and provider-registration documentation.

Completed by [the migration pass](agent-provider-boundary-migration-pass.md) (2026-09-25):

- semantic launch intent in place of orchestration-owned Codex configuration strings;
- typed configuration references and the provider options envelope in shared profile contracts;
- normalized transcript and observation consumers, with a raw-payload guard;
- model-independent skills;
- the remote `Capabilities` command using the typed reference.

Still required before the full target described below is complete:

- finish destination-instance ownership/navigation handling for every target-transition path and historical migration case;
- make the remote host construct providers through a provider factory rather than its Codex implementation directly;
- migrate live coverage off the test-only CLI runtime, then remove that competing path;
- perform the live desktop, restart, import, interaction, and SSH checks listed in the validation section.

These are not Claude-integration tasks. They are the remaining Codex boundary migration and verification work.

The migration pass supersedes this document's historical-compatibility choices: stored profiles and references are rewritten once in SQLite, no permanent Codex compatibility reader or legacy decoder is kept, and old event streams are left as recorded without a compatibility projection.

## Target

Make Codex a clearly bounded implementation of Orchid's existing agent capabilities. A later Claude Agent SDK bridge should have an obvious place to supply the same contracts. This task supplies no Claude implementation, dependency, placeholder settings screen, or new agent capability.

Orchid owns sessions, invocations, preparation, workflow semantics, capability policy, managed tools/skills, persistence, and presentation. An agent provider owns its native configuration, discovery, process/protocol interaction, native event translation, and native continuation/history operations. Device transport remains separate from provider behavior.

## Proposed choices, in review order

### Choices made in this plan

- Use **Agent provider** for the combined external harness and inference integration; **Provider configuration** for a registered instance; **Execution target** for its device/connection. Keep **Conversation Harness** for Orchid's existing prompt/tool/role configuration. No separate harness-versus-inference registry is needed now.
- Put shared Codex code under `crates/orchid-engine/src/providers/codex/` and desktop Codex code under `src-tauri/src/runtime/providers/codex/`. Each provider receives its own subdirectory.
- Reuse `AgentRuntime` and its prepare/deliver, start/resume, steering, interaction, cancellation, and shutdown operations. Add focused contracts only where an existing caller currently understands Codex. Use a small explicit provider registry with one production registration; no dynamic plugin loader or universal extension engine.
- Represent provider-native settings as a provider-tagged extension, decoded and validated by that provider. Generic profile/session code can retain the payload but must not interpret it. Preserve the existing Codex personality control as a Codex-owned component rather than inventing a generic personality setting.
- A new Orchid session instance does not inherently require throwing away existing Codex conversation history. Retain the existing, explicit Codex device-continuation feature through an optional provider continuation port, with a new destination Orchid session. A native context ID is scoped to provider/configuration/device; it is never a globally transferable identity.
- Preserve the existing invocation option behavior. In particular, do not turn capability-profile model/reasoning ranges into new enforcement gates. Profile edits affect newly created instances; existing instances retain their resolved revision.
- Keep historical data readable through a bounded Codex compatibility reader. Avoid rewriting old event streams or silently deriving missing historical configuration from today's selected profile. *Superseded for profiles, references and old events by the migration pass; event streams are still never rewritten.*

### Discussed choices that remain provisional

- Claude Agent SDK bridge is the expected later implementation. Its packaging and exact API mapping remain outside this task and do not determine the shared contract.
- Broad access-mode equivalence across providers is not established. This task preserves actual Codex behavior and moves its translation into Codex ownership; a future provider should report an unsupported option instead of silently substituting another mode.
- The provider/configuration/inference split may grow later. The combined agent-provider model is the explicit working assumption for this implementation.

### User-prescribed decisions

- Clean the integration boundary before implementing Claude.
- Match the features Orchid currently supports through Codex; no expansion to additional Claude features.
- Prioritize full access and simple configuration. Preserve current authentication, installation, upgrade, and product-identity arrangements.
- Keep native escape hatches in the provider integration area. Introduce reusable Orchid interfaces only for concrete existing consumers.
- Lock each Agent Session instance to its provider, device, provider configuration, and capability-profile selection. Changing those creates another instance.
- Use current Codex validation practices as the baseline; do not introduce a new security or compliance program.

## Current facts that affect the shape

| Observed implementation | Consequence |
| --- | --- |
| `execution_targets/domain.rs` already has `ExecutionRouteRef` and `ResolvedExecutionBinding`. `SessionExecutionTarget` embeds a binding but also assumes a worktree. | Reuse those types; separate execution identity from optional workspace placement so auxiliary, imported, and orchestration sessions can be bound too. |
| `execution_targets/endpoints.rs` returns one local runtime and one local profile source regardless of provider. | Replace the singleton selection with registered provider resolution. Keep local/SSH transport selection here. |
| `execution_configuration/native_codex.rs::with_temporary_danger_full_access` forces full access for ordinary native Codex profiles. Retained orchestration roles also use read-only/workspace-write. | Preserve both paths. Do not add a sandbox editor or broaden all orchestration roles to full access. |
| `agent_sessions/interactions.rs` already expires unanswered requests after terminal state; attempted responses and steering can become uncertain. `application/lifecycle.rs` reconciles interrupted invocations. | Reuse these semantics. No durable callback resurrection or automatic response replay is needed. |
| `repository/preparation.rs::commit_prepared_binding_record` overwrites a session's target and current execution resolution. Preparation can transfer Codex history between devices. | Immutable instances require a real lifecycle change, not just adding a provider field. |
| Shared launch/profile contracts contain `CodexPersonality`, Codex TOML overrides, and Codex skill-catalogue types. Application callers parse `native-codex:`. | Move native types, encoding, and old reference decoding behind the provider boundary. |
| `orchestration/mcp.rs::CodexMcpInjection` builds MCP, approval, timeout, and network settings as native configuration strings. | Extract semantic managed-MCP launch intent and let Codex serialize it. Preserve actual current settings and limits. |
| Generic transcript/observation code reads raw item events, Codex transport markers, and import markers. | Normalize the facts those consumers use; retain raw data as evidence. |
| `native_profiles.rs` is a large Codex administration implementation with schema, process operations, domain types, service logic, and tests. | Give it an explicit Codex home and split those responsibilities without redesigning login/readiness behavior. |
| The older Tauri `CodexCliRuntime` is test-only, but several live-test consumers still use it. The engine JSONL normalizer also serves production history import. | Retire the test-only runtime after moving relevant coverage to app server; retain the production history normalizer. |

## Resulting ownership and paths

Paths below are proposed destinations. Existing entry points can be changed together in the same implementation stage; avoid permanent compatibility re-exports between old and new production modules.

| Owner | Retain, adapt, or create | Responsibilities and consumers |
| --- | --- | --- |
| Shared provider contracts | Adapt `crates/orchid-engine/src/contracts/{domain,runtime}.rs`; create `contracts/{provider,interactions}.rs` | Provider/configuration identity, existing runtime operations, semantic launch material, normalized events and interaction responses. Used by desktop, remote host, and provider adapters. |
| Shared provider construction | Create `crates/orchid-engine/src/providers/`; adapt `lib.rs` | Explicit provider roots and shared contracts. Codex is the only production registration. |
| Shared Codex implementation | Move to `crates/orchid-engine/src/providers/codex/`; add `options.rs` | App-server RPC, native options, event translation, skill invocation syntax, history/continuation format. |
| Desktop provider composition | Create `src-tauri/src/runtime/providers/`; adapt `runtime/mod.rs`, `active_app/{sessions,execution_configuration}.rs` | Separate runtime, configuration, and continuation registries. Explicit concrete Codex imports are allowed at composition roots. |
| Desktop Codex configuration | Move under `src-tauri/src/runtime/providers/codex/configuration/` | Implement generic configuration/discovery ports. |
| Desktop Codex administration | Move under `src-tauri/src/runtime/providers/codex/profiles/` | Keep schema, process, discovery, readiness, and session-binding behavior inside the provider domain. |
| Product execution configuration | Adapt `execution_configuration/{ports,runtime_profile,capability_profile,session_profile,resolution,service,session_skills,quick_features,model_catalogue}.rs` | Registered route references, offline profile CRUD, pinned exposure, common selections, provider extension storage and delegated validation. No native home parsing or Codex types. |
| Device/workspace routing | Adapt `execution_targets/{domain,endpoints,preparation,remote_runtime}.rs` | Resolve device and provider configuration, materialize workspaces, route calls. Extract native continuation calls out of generic worktree preparation. |
| Session lifecycle and persistence | Adapt `agent_sessions/{domain,preparation,target_transition}.rs`, `application/`, `repository/`, and `transport/`; create `application/session_binding.rs` | Resolve/freeze binding once; reuse it for every invocation; create destination instances on execution changes; retain history and accepted-prompt ownership. |
| Native history | Adapt existing `agent_sessions/{imports,ports/import,repository/import}.rs` and `runtime/providers/codex/app_server/history.rs` | Generic import receipt/materialization remains session-owned. Codex URI parsing, native reads/forks, and historical payload interpretation stay Codex-owned. |
| Remote host | Adapt `crates/orchid-engine/src/{host,protocol}.rs` and host entry point | Provider-tagged configuration, factory dispatch, provider-scoped saved bindings, and opaque continuation transport. Git/snapshot commands stay provider-independent. |
| Frontend provider contracts | Create `src/application/agentProviders/{contracts,index}.ts`; adapt existing session/execution DTOs | Provider descriptors, registered configuration identities, option catalogues, and neutral request/response types. |
| Frontend provider implementation | Create `src/features/agentProviders/codex/`; move `features/nativeProfiles/` there; move `infrastructure/nativeProfiles/` to `infrastructure/agentProviders/codex/` | Codex administration and personality editor. Keep the existing Tauri command surface initially. Provider component selection is explicit frontend composition, not a dynamic plugin system. |
| Shared frontend consumers | Adapt `ExecutionSetupOverview`, capability/profile editors, `AgentSessionScreen`, `SessionInteractions`, `transcriptProjector`, `sessionAttention`, composer quick features, and Tauri clients | Render provider metadata and normalized state; submit product choices. No Codex wire response construction or native event parsing. |

## Identity and immutable instances

### One source of execution identity

Add a session execution binding containing the existing resolved provider/device/configuration binding plus an optional capability-profile ID and revision. The profile reference may be absent for retained unprofiled orchestration/import sessions; do not fabricate a profile solely to fill a field. Persist it on every newly created session before provider launch.

Reuse the existing `SessionCreationResolution` as the pinned exposure/defaults snapshot. `SessionExecutionTarget` becomes workspace placement associated with that binding rather than a second writable authority for profile/provider identity. Transport read models can combine the two for existing screens.

Resolve aliases such as `selected` at session creation. Store the actual registered configuration reference, never a moving default. Keep `external_context_id` and observed runtime version in `AgentRuntimeBinding`; the session binding scopes their meaning. Record observed runtime version per invocation where available, since upgrading a provider executable should not rewrite the version of historical executions or force a new session.

The lock applies to selected configuration identity, not a hash of every native configuration file. Continue existing native-home continuity checks. Changing a model/reasoning override within an instance follows today's invocation semantics; no new policy enforcement is added.

### Destination-instance flow

Adapt `application/{direct_user,addressed,preparation,targets,target_transition}.rs`, `application/preparation/execution.rs`, and `repository/{preparation,addressing,target_transition}.rs` together:

1. Resolve the requested provider/device/configuration/profile revision before choosing an invocation owner.
2. If it matches the current session binding, prepare the invocation in that instance.
3. If it differs, create one destination session and record its relationship to the source on the transition. Repeated preparation of the same transition reuses that destination.
4. Keep the source session and its execution binding intact. A queued or newly accepted prompt belongs to the destination; do not move already-delivered invocations or existing transcript events between sessions.
5. Reuse current worktree snapshot/migration services and sister-group locking. Transfer active ownership to the destination when preparation commits; failed preparation must leave the source recoverable. Audit every lock consumer for the new owner session ID.
6. Commit the destination binding, optional native continuation receipt, profile snapshot, and preparation result without updating the source. Return the destination session ID so frontend navigation follows the right instance.

Update `useSessionTarget.ts`, target/preparation DTOs, `AgentSessionScreen`, navigation/placement, and `workflows/{session_navigation,address_references}.rs` consumers. A workflow logical address may select a new current instance only through its existing application routing authority; an address is not permission to mutate an old instance. Historical workflow deliveries retain their original session IDs.

Preserve the explicit Codex continuation action where it already exists. Its provider implementation decides whether to fork/copy native context and returns a destination-scoped result. Reusing one live native context for independently writable source/destination sessions is not acceptable; if a same-store transition cannot fork, create a fresh context. A changed provider starts fresh; no automatic summaries or cross-provider transcript translation are introduced. The previous suggestion that every binding change must discard native history is not adopted here.

### Historical binding migration

Use existing SQLite migration machinery. For old sessions, prefer persisted current execution/preparation evidence, then a concrete execution target, the native session-to-home binding, and the pinned profile. Classify existing provider-less records as legacy Codex because Codex was the only production implementation; that does not establish their exact configuration/device.

If those records conflict, preserve the history and surface an unresolved legacy binding instead of selecting today's default. Such records can be inspected and a new explicitly configured instance can be created. Do not rewrite old invocation history to claim one immutable device where historical transitions actually occurred. The invariant governs migrated effective bindings and new execution, while old per-invocation evidence remains intact.

`agent_session_current_execution` currently permits a second changing profile. Consolidate new writes into the pinned session binding/resolution; retain old rows only for migration/history until their information is accounted for. Session-profile digest migration must validate the old stored representation before changing it; preserve the concurrent optional-personality serialization fix. *Superseded: the migration pass reseals stored snapshots without verifying the old digest, because the data is experimental.*

## Provider contract and configuration

Use existing ports as the starting point. Rename/adapt `ProviderConfigurationSource` into a configuration-addressed provider source: registered configurations, resolve reference, observe catalogue, inventory, and quick features. Replace its `CodexSkillCatalogue` return type with a neutral skill descriptor. Remove product callers' need for `configuration_home`; providers resolve their native homes themselves. Product auxiliary workspaces should use the existing Orchid app-data workspace owner, with old paths retained for existing sessions.

The registry composes the runtime and configuration source for the chosen provider. It should have explicit unsupported-provider behavior and no fallback to Codex for an existing session. Desktop configuration registration remains backed by the current Codex profile service; no second generic configuration database is needed. Host configuration maps its registered entries to the same engine factory.

Keep common option keys limited to current consumers:

| Common intent | Ownership and behavior |
| --- | --- |
| Model | Provider-scoped ID plus display metadata; default inheritance and invocation overrides retain current behavior. |
| Reasoning | Shared label, provider/model-supplied supported IDs and ordering. Remove hard-coded global rankings from generic UI; do not equate equal labels with equal compute across providers. |
| Access mode | `read_only`, `workspace_write`, `full_access` intent. Codex translates to its native sandbox names. Decode existing `danger_full_access` records without changing effective behavior. |
| Approval behavior | `inherit` or `unattended` for the current launch-policy need. This is separate from access mode: unattended means do not prompt, not permission to bypass a restricted mode. |
| Managed MCP exposure | Existing server identity/URL, bearer reference, selected tools, required flag, and timeouts. Values come from Orchid's current tool owners. |
| Native MCP exposure | Inherit, configured exposure, or suppress as required by current profile/Harness consumers. Native server-level behavior remains provider-owned; do not claim new native per-tool enforcement. |
| Application guidance and skills | Existing initial context, pinned skill manifest, source identity/fingerprint, and explicit invocation intent. Product-owned skills remain served through the existing reader/broker. |
| User-rule inheritance | Preserve the existing managed-Harness request to ignore native user rules; provider translates or reports unsupported. Do not add a general settings-precedence policy. |

Move `CodexPersonality` and native override validation into Codex options. A small `{ provider, settings }` persisted envelope is sufficient for current native settings; only provider code decodes its settings. Keep public native settings limited to existing controls. Raw command flags and native TOML are not accepted from generic UI or synthesized by workflow services.

Models/catalogues/quick-feature caches must be keyed by provider, device, and concrete configuration, including working context where existing discovery depends on it. Preserve offline profile saving and last-successful catalogue behavior. Do not call live discovery merely to save a profile.

## Launch material and orchestration

Extract a shared session launch-material builder from the duplicated direct/addressed/prepared paths. It composes the pinned profile exposure, Conversation Harness contributions, managed MCP endpoints, skill guidance, and invocation selections. Its result is semantic launch material, not a command line or environment containing a native home.

Adapt `orchestration/{mcp,conversation_harness,work_unit_execution_harness}.rs` and their transition callers. Replace `CodexMcpInjection` with structured managed-MCP material; keep the actual listener, `rmcp` tools, bearer generation, and business authorization in orchestration/Harness Engine. Codex owns native server naming/configuration syntax, bearer environment injection, approval settings, and the existing reporting-transport network workaround. Replace comparisons of exact TOML arrays with assertions about the requested semantic contract and separate Codex serialization tests.

Use the existing `SessionHarnessLaunchAuthority` and profile resolution order. Replace the native-profile launch authority with a provider-addressed preparation hook implemented by desktop Codex profiles; do not add a general hook pipeline. Provider homes, executable resolution, process environment, and native options stay within that implementation.

Do not blindly run local launch preparation for a remote target. Local paths and loopback MCP URLs are not valid remote capabilities. Share semantic intent and translate it on the execution device; retain current availability limits where the remote path cannot provide a local Orchid service. No new tunnel, remote skill distribution, or remote feature parity work is included. A required unavailable capability must be reported explicitly rather than dropped or redirected locally.

## Events, interactions, and history

Extend the existing normalized event/update contracts only with facts already consumed by the product: stable item identity and phase, message role/finality, request opened/result state, imported-history provenance, transport/process terminal evidence, and existing usage counters. Preserve the distinction between launch acceptance, provider completion, process exit, and workflow semantic completion.

Codex `app_server/{items,notifications,requests,configuration}.rs` produces these facts. `transcriptProjector.ts`, `sessionAttention.ts`, `application/observation.rs`, and `interactions.rs` consume them without examining native payloads. Raw provider payloads remain attached for diagnostics.

Use generic interaction responses such as `Choose { choice_id }` and `Answer { question_id -> text[] }`. A displayed approval choice has an opaque ID, label, description, and optional readable scope. The Codex adapter retains the mapping from choice ID to exact native response; the frontend must not echo native permission JSON. Preserve all currently supported approval scopes, free-text/secret questions, URL elicitation, validation, and unsupported-request behavior.

Keep restart behavior: unanswered requests expire; an attempted response whose delivery is unknown stays uncertain; no automatic replay. Reuse `application/lifecycle.rs` and current repository reconciliation rather than introducing a second restart subsystem.

For historical records, a Codex compatibility projection can read known older payloads into a display representation. *Superseded: the migration pass adds no compatibility projection; older tool rows may display unpaired.* Generic UI must not retain a fallback raw-Codex parser. Do not fabricate process evidence or rewrite old events simply because the new contract has more fields; unknown remains unknown.

Keep import preview, fork, receipt, and materialization behavior. Move `codex://` parsing and native history source operations into the Codex integration, expose them through a small optional history port, and retain the Codex-specific import entry point. No generic provider history browser is needed now.

For composer skills, separate product-owned references from native invocation syntax. The shared picker retains source/configuration identity and a minimal delivery kind; Codex owns `$name` translation, while Orchid-owned skills use the existing manifest reader guidance. Avoid adding a universal command/skill/plugin execution framework.

## Remote host and compatibility

Tag configured providers and runtime requests with provider/configuration identity. Move `CodexConfiguration` out of `host.rs` into the Codex implementation. The host owns saved session binding, dispatch, and transport; the adapter owns executable/home/settings interpretation.

Replace protocol references to concrete `CodexContinuation` with a provider-tagged opaque continuation payload decoded by the selected provider. Generic code validates source/destination provider and delegates compatibility to that provider. Git/worktree snapshot payloads remain independent.

Provide a narrow reader for existing Codex-only host configuration and session records. If the wire contract changes, add a contract version to the existing `Describe` response and check it before provider commands; the current SSH connection has no protocol-version handshake. Reject incompatible desktop/host builds clearly and update both ends together, without a version-negotiation framework. Existing Codex development scripts/configuration samples need updating with the new explicit provider identity.

## Frontend shape

Provide one registered-provider/configuration projection to Technical Settings and Capability Profiles, replacing `localCodexRoutes` use in generic screens. It can derive from the existing Codex registrations without creating independent inference-source entities. Show device, agent provider, and configuration; keep Codex labels/details in provider descriptors/components.

Move the Codex personality fragment out of `CapabilityRouteEditor.tsx` into the Codex feature area and render it through explicit provider component composition. Keep existing Codex administration screens and controls inside that same feature area. Shared profile contracts retain only common options and the provider extension envelope.

Update `src/application/{agentSessions,executionConfiguration,executionTargets}/contracts.ts` and matching Tauri DTOs together. Use typed normalized interaction/events in the shared UI. Keep existing composer layout, draft persistence, profile editing, and model picker behavior; no broad screen redesign is implied.

## Removals and reuse

- Remove generic `native-codex:` parsing; keep one legacy decoder inside Codex migration/compatibility code. *In the migration pass, that decoder exists only in the one-time schema v59 migration.*
- Remove concrete Codex skill/personality/continuation types from generic ports and host protocol.
- Remove raw native response construction and native event interpretation from shared application/UI code.
- Remove duplicated launch assembly after current direct, addressed, prepared, and remote callers use the common product material builder plus provider preparation.
- Remove same-instance provider/device/profile retarget writes after destination-instance transitions own the flow.
- Retire the Tauri test-only `CodexCliRuntime`, its arguments module, and wrapper tests after adapting `agent_sessions/live_smoke.rs` and affected orchestration/product-decision live tests to production app-server behavior. Retain shared JSONL parsing and executable discovery that production import/runtime paths still use.
- Keep the current process supervisor, Windows job handling, SSH transport, repositories, Harness Engine broker, and workflow control authority.

Dependency review: the existing `orchid-engine` crate, `serde`/`serde_json`, `rusqlite`, `uuid`, Tauri, React, and `rmcp` already cover this work. A provider-framework dependency would overlap the existing runtime/process ports. No new library is proposed; Anthropic SDK/bridge runtime selection belongs to the later integration.

## Implementation order and validation

Each stage should leave Codex runnable; the boundary is complete only after all consumers have adopted it.

1. **Shared contracts and provider dispatch.** Introduce provider identity, registered factory/composition, common options, and provider extension types. Adapt desktop and host configuration lookup. Validate distinct fake provider registrations dispatch correctly and unknown providers never fall back to Codex. Preserve offline profile CRUD and existing Codex option serialization.
2. **Codex ownership and launch extraction.** Move native configuration/administration responsibilities, extract semantic launch material, migrate orchestration callers, and update frontend provider components. Test direct, addressed, workflow, and retained orchestration launches against the production app-server fixture, including managed MCP credentials/tool sets, selected skills, reasoning, personality inheritance/override, and current access modes. Ensure native secret values do not enter persisted public configuration.
3. **Immutable instances and migration.** Freeze complete bindings at every creation/import path. Replace retargeting with destination-instance creation and adapt workflow/navigation/sister ownership. Test same-binding reuse, changed binding creation, repeated preparation without duplicate sessions, preserved source history, pending prompt delivery once to the destination, failure recovery, profile revision pinning, and migration from existing records. Preserve Codex continuation where supported and reject cross-provider native continuation.
4. **Events, interactions, and historical reads.** Adopt typed normalized events and generic responses. Test approval choice scopes, questions, invalid/stale responses, uncertain writes, cancellation, interruption/restart, imported history, usage display, transcript coalescing, and provider-versus-process completion. Use old stored fixtures to prove readable history without invented evidence.
5. **Complete remote dispatch and retire competing paths.** Exercise provider/configuration identity across host requests, destination session binding, and continuation encoding. Move live coverage off the old CLI runtime before deleting it. Audit remaining Codex references: valid locations are provider implementations, composition, migration/legacy fixtures, and explicitly Codex-named UI/tests; generic execution code should have none.

Use a minimal test-only second provider to prove routing and native-payload independence, implementing only the operations used by those tests. It must not become a simulated Claude implementation or a shipped provider.

Run focused Rust/Vitest suites during each stage, then `npm run build:frontend`, `npm run check:rust`, relevant `npm run test:rust:fast` coverage, engine tests through `cargo test --manifest-path crates/orchid-engine/Cargo.toml`, and `npm run test:codex-app-server`. Follow repository build tooling for the Tauri target and retain useful caches.

Final live Codex checks should cover ordinary first send/resume, current full-access selection, one interactive question/approval under an appropriate existing configuration, steering/cancel, restart with a pending request, Codex import, and a representative managed workflow/MCP/skill flow. Inspect the desktop selection/navigation after a destination-instance transition. Exercise the existing local/SSH Codex continuation path when its configured host is available; report source/fixture proof separately if that host cannot be used.

Completion means a future provider can implement the documented runtime/configuration/normalization ports and register its descriptor without editing shared session lifecycle, transcript parsing, interaction response encoding, or orchestration-native settings. Adding genuinely unsupported functionality would still require a deliberate contract change.

## Documentation handoff

After implementation, update `docs/architecture.md`, `docs/execution-configuration.md`, `docs/agent-session/README.md`, and the remote/sister transition guides with actual ownership and instance semantics. Add a short provider integration guide listing the registration points, existing required operations, optional history/continuation ports, and test entry points. Revise the Rust boundary note only where module placement changed. Do not rewrite unrelated historical documents as current implementation evidence.
