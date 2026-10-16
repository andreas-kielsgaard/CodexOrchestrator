# Independent Orchid devices implementation shape

Enable phone-led development on a remote work host while retaining an independently usable laptop. The optional server synchronizes Orchid data and forwards explicitly addressed operations. Runtime execution belongs to the selected device; loss of the server must not stop local progress.

Status: implementation authorized and underway on `codex/independent-devices`. Source baseline: `4d947a3ba07a7ee7e32ddab5e28214d8deba73f7`, including the current working tree. Existing unrelated changes remain outside these assignments.

## Implementation record — 2026-10-16

The library extraction, independent server/host composition, addressed session/device API, durable session replication, definition replacement, owned skill packages, explicit ownership handoff, conversation forks and desktop synchronization controls are implemented. Setup and current API contracts are documented in [the service guide](../../crates/orchid-services/README.md).

Concrete resulting owners:

- `orchid-core/composition` composes the full local runtime and workflow services. `agent_sessions/queries.rs` can read persisted data without them. `agent_sessions/client.rs` owns addressed dispatch; the application retains preparation, queueing and execution.
- `orchid-services/server.rs` owns authenticated routing, stored queries and replica ingress. `host.rs` owns reconnecting transport and synchronization; accepted commands outlive a disconnected socket. Host/server instance locks prevent accidental data-root sharing.
- `synchronization` owns cursors, retention, owner-authored snapshots and definition-master replacement. Feature repositories retain export/import mappings. Runtime records never use definition conflict policy.
- `skills` owns source import, fingerprints, supporting files and host materialization. Provider discovery contributes origins; stable Orchid IDs are used across devices.
- Tauri command wrappers remain in `src-tauri`; generic control MCP moved into core with experimental voice supplied as a desktop extension. NativeProfile schema/repository/tests were separated from provider operations.
- The root Cargo workspace and lockfile own all three crates. Service packaging includes setup/verification scripts. Deployment has not been performed.

The original assignments below also include work that is not finished in this implementation:

- Per-category cache controls for definitions/skills: these are currently fully retained; configurable retained/on-demand-cache policy applies to copied session histories.
- Full remote branch-management UI and persisted branch catalogue observations: host Git/worktree operations are available, but desktop branch menus remain local-facing.
- Legacy SSH retirement and configuration import: retained explicitly for existing profiles. The legacy binary is named `orchid-ssh-host`; new service binaries install separately. No existing configuration is silently retargeted.
- Distributed workflow-instance commands and arbitrary Harness catalogue migration: workflow progression remains local, and host-local MCP/environment configuration is not exported. Portable profiles, identities, workflow designs and skill packages synchronize.

Validation completed during implementation: shared core and service suites on Windows; TypeScript and session UI/client tests; seven historical desktop NativeProfile migration tests; Linux service integration tests and a separate-process router-restart proof. The runtime outage test uses the actual Jev adapter with a controlled HTTP endpoint. Real paid-provider execution, packaged desktop acceptance and production deployment remain unverified. Test counts belong to command output; this document does not imply a live deployment.

## New implementation choices

1. Extract one headless `orchid-core` library, retaining the existing feature directories. Add `orchid-services` for the server and host binaries. Keep `orchid-engine` as the lower-level provider, process and Git library. Do not flatten feature ownership into generic `data` and `host` folders.
2. Route at the application-command boundary. The destination owns acceptance, preparation, queue advancement, native provider setup and MCP launch. Remote provider calls are no longer a substitute for a full application host.
3. Separate query services from execution services. A data-only server must be constructible without a dummy runtime, provider discovery, filesystem probing or recovery of copied executions.
4. Keep each session queue under one execution device. Existing sessions retain their target until an explicit online transition; unreachable-device continuation creates a new session. Replication imports records without acquiring queue authority.
5. Keep SQLite and the existing repository transactions. Add explicit feature export/import mappings and a durable change journal. Do not replicate the database file or arbitrary tables.
6. Consolidate provider composition around the desktop's `ProviderRegistrations`, which already pairs configuration discovery, runtime and launch preparation. The current engine host registry registers only Codex and is not the target composition.

These choices preserve the application's existing semantics while removing desktop and SSH lifetime assumptions.

## Assumptions for previously unsettled details

- Retention starts with retained and on-demand cache modes for sessions, definitions and skill content. Locally owned active work and unsynchronized runtime records are never cache eviction candidates.
- A conversation fork includes completed synchronized turns, with a source-session reference and cutoff. It does not clone an active queue, native provider handle or worktree.
- Session title and archive mutations use the current session owner; local pin, order and attention preferences stay local initially. This avoids adding an unrelated collaboration policy for navigation.
- Imported skill files stay externally owned at their origin. Orchid imports and refreshes its copy; cross-device edits do not write back into the external folder. Origin refresh happens when source bytes change, not merely whenever the source device reconnects.
- Initial network deployment uses authenticated private-network connections. Account management and public hosting are outside this shape.
- Host-process crashes retain explicit interrupted/uncertain outcomes. Surviving a router outage does not require surviving an execution-host crash without interruption.

## Prescribed product boundaries

- Server configuration and synchronization are optional for local operation.
- Commands target an explicit device and worktree. No scheduling or automatic destination selection.
- Worktrees and provider credentials remain device-local. Configuration identity and last-observed setup status are shareable, without guaranteeing readiness.
- Laptop and server retain conversations, attachments, skills and workflow definitions. Other devices can cache them.
- An unreachable execution device makes its conversation readable but requires a fork and a new target to proceed elsewhere. An uncertain last turn also blocks ordinary continuation.
- Definition edits remain locally possible. Non-master changes synchronize only through the simple online master/server path; incoming synchronized data may replace local non-master edits. Runtime history does not use master-wins.
- Workflow progression remains local to an assigned device. Workflow redesign, Android UI and exact experimental voice compatibility are lower priority.

## 1 Extract the actual dependency closure

### Library and executable placement

| Destination | Existing ownership to move or retain | Consumer change |
| --- | --- | --- |
| `crates/orchid-core/src/agent_sessions` | Move domain, application, ports and repositories; split queries and routing as described below | Desktop transport, host service, workflows and control tools use the same session services |
| Core `execution_configuration`, `execution_targets`, `execution_devices`, `identities` | Move non-Tauri services and contracts; split device descriptions from machine control | Server uses stored catalogues; host supplies live observations and operations |
| Core `runtime`, `harness_engine`, `agent_cli_updates` | Move application-owned provider setup and MCP mediation; retain engine adapters underneath | Both desktop-local and headless hosts compose the same provider registrations |
| Core `repository_catalog`, `repository_discovery`, `session_navigation` | Move shared data/services after separating local Git work and UI control | Server navigation reads stored facts; host handles discovery and mutation |
| Core `workflows`, `session_events`, `otp_api`, `otp_host`, `otp_packages` | Move the existing dependent feature group, excluding Tauri transport | Host can retain current workflow and OTP behaviour without inventing a replacement engine |
| Core `persistence`, shared schema initialization and explicit paths | Reuse `ActiveDatabase`; split historical desktop migration from current shared initialization | No reverse dependency from core into desktop storage or app-data resolution |
| `crates/orchid-services/src/bin/orchid-host.rs` | Replace the engine-owned stdio host executable with full host composition | Service starts independently of SSH/client connections |
| `crates/orchid-services/src/bin/orchid-server.rs` | New data/query/sync/router composition | No execution, CLI-update, idle-shutdown or workflow scheduler startup |
| `src-tauri` | Retain Tauri handlers, clipboard, notifications/sounds, deep links, window controls, Worktree Review/build opening, Product Decisions, legacy orchestration and experimental voice | These consume core public feature interfaces or attach optional observers |
| `crates/orchid-engine` | Retain contracts, provider process adapters, process supervisor, repository context and workspace snapshot implementation | Core depends on engine; engine never depends on core |

Keep feature modules discoverable. Expose the types and operations actually used by desktop consumers; do not make private repository helpers public just to preserve old import paths. Temporary re-exports may support a staged move, but duplicate implementations are removed when consumers switch.

### Concrete extraction blockers

- [Preparation](../../crates/orchid-core/src/agent_sessions/application/preparation.rs), [queue](../../crates/orchid-core/src/agent_sessions/application/queue.rs), [steering queue](../../crates/orchid-core/src/agent_sessions/application/steering_queue.rs) and [target execution](../../crates/orchid-core/src/agent_sessions/application/target_operations/execution.rs) call `tauri::async_runtime::spawn_blocking`. Give host composition an owned Tokio runtime handle and use it at these four worker sites. Preserve their existing cancellation and per-session gates.
- `execution_devices.rs`, Codex `profiles/mod.rs`, Claude `setups.rs`, CLI updates and OTP installations mix services with Tauri handlers. Extract those handlers into desktop transport modules. Move the service code once.
- `HarnessEngineTauriState` belongs with desktop transport, not `harness_engine/service.rs`.
- `session_navigation/agent_access.rs` intentionally round-trips through the mounted UI. Keep it desktop-side; it is not the future server API.
- `orchid_control/service.rs` currently advertises voice tools in its generic tool map. Split generic Orchid tools from the voice contribution; register the latter only when voice is composed.
- `product_home.rs` and `SessionWorkspaces` use user-global directories. Introduce explicit instance paths supplied by composition; retain existing desktop locations through migration. Server and host on one machine receive different data roots.
- `harness_engine/sidecar.rs` launches `current_exe()` in a special child mode. Both the desktop and new host entry points must recognize that mode before ordinary startup. The server never creates the sidecar.

### Composition and migration

Extract `active_app/sessions.rs` and `active_app/execution_configuration.rs` into core host composition. `active_app.rs` retains shell setup and attaches legacy/voice observers. Keep the ordering that installs local MCP upstreams and terminal observers before session recovery.

Do not move `storage.rs` wholesale: its supposedly navigation-related initializer also creates voice, preparation, queue and provider-binding tables; its workflow initializer includes provider setup and repository-device tables. Extract a current shared-schema initializer with feature-owned initialization calls. Desktop runs its existing historical migrations first, then the new core migrations; fresh headless stores initialize only the shared current schema. New core migrations use their own migration ledger rather than competing with the historical desktop `user_version`.

Remove core calls back to `crate::storage::open_active_database` from repository/service constructors. Production construction uses `from_database`; isolated tests use the core schema initializer. Keep desktop legacy orchestration migrations in the desktop package.

## 2 Separate session reads from session execution

Current [creation.rs](../../crates/orchid-core/src/agent_sessions/application/creation.rs) puts list/history reads on `AgentSessionApplication`, whose constructor requires an `AgentRuntime`. Transport queries also reach through navigation for event details.

Create `agent_sessions/queries.rs` with `AgentSessionQueries`: summaries, full/display history, event details, interaction projections, stored queue/preparation views and continuation availability. Reuse `repository/display_history.rs`, `summaries.rs`, `history.rs` and `project_interactions`; do not create a second history model. Replication exports full persisted events, not compact display DTOs that omit deferred tool details.

Keep `AgentSessionApplication` as the executor. Its `InvocationDeliveryService`, prepared submission flow, direct/product delivery semantics, worker gates and runtime update sink remain the implementation of host commands. Move only read-only entry points out; internal execution reads can call the same query service/repository.

Create `agent_sessions/client.rs` with `AgentSessionCommandClient`, selecting local dispatch or a configured device route. Cover prepared send, product delivery, queue edits, steering, approval responses, cancellation and target operations. Resolve the target from the explicit first-message selection, existing session owner, or invocation execution record as appropriate. Never route a cancellation using only a mutable current session target.

### Callers that must change

| Caller | Assignment |
| --- | --- |
| `agent_sessions/transport/{commands,preparation,selections,queue,interactions}.rs` | Invoke the command client; queries use `AgentSessionQueries` |
| `send_prepared_agent_session_message` | Move folder-to-working-directory resolution out of the Tauri handler and onto destination-side preparation; do not resolve a remote folder against the client's filesystem |
| `AgentSessionEventAdapter` | Keep logical addressing and deterministic delivery identity; dispatch through the command client instead of retaining a concrete executor for every destination |
| `otp_host/session_control.rs` | Cancel through invocation-owner routing |
| `orchid_control/service.rs` | Use shared queries and command client; add explicit target information to start/list-workspace operations |
| `voice_sessions/pipeline.rs` and other direct product-session callers | Adapt to the same product-delivery client for compatibility; no voice redesign |
| Legacy orchestration and Product Decisions | Keep local semantics, using local client adapters and optional observers; do not export their private execution models as part of this work |

Reuse `PreparedMessageInput.submission_id` and its existing matching/retry logic. Reuse steering input IDs and workflow delivery identities. For operations without a durable identity, add one at the host command boundary and persist the receipt with the mutation. This fills the remaining gaps rather than placing a second queue in the server.

## 3 Move remote execution ownership above the provider boundary

[ExecutionEndpoints](../../crates/orchid-core/src/execution_targets/endpoints.rs) currently combines a local provider registry, cached SSH runtimes, remote discovery and worktree operations. Split it by responsibility:

- Local provider runtime/configuration/launch lookup remains with `ProviderRegistrations`.
- A `DeviceDirectory` owns stable device identities, route availability and advertised provider configuration descriptions.
- A `DeviceOperationsClient` owns addressed worktree inspection/materialization/snapshot operations, repository discovery/Git actions and live provider capability queries.
- `ExecutionTargetService` keeps selection validation, inventory composition and target planning. It consumes the directory and operations client; it no longer manufactures a `RemoteRuntime`.

Change `ResolvedExecutionBinding` and provider-binding cache keys to contain stable device/provider/configuration identity, not SSH command details. Resolve local versus remote from the receiving instance's identity. Existing `ExecutionRouteRef` is the starting vocabulary. Remove serialized connection assumptions from session/profile DTOs and UI icons.

The new service connection is an authenticated outbound host connection to the configured server. Server routing correlates responses and forwards commands to the addressed device. It does not prepare invocations, keep provider sinks alive or interpret transport loss as an execution outcome. Local dispatch bypasses the server entirely.

### Provider and MCP reuse

The current [engine host registry](../../crates/orchid-engine/src/host/providers.rs) registers only Codex. Desktop [provider registration](../../src-tauri/src/active_app/execution_configuration.rs) registers Codex, Claude and Jev with configuration/launch collaborators. Use the latter composition for both host forms.

Retain Codex native-home discovery/readiness, Claude setups, Jev credential resolution, `ProviderLaunchPreparation`, `SessionProfileResolver`, `SessionProfileHarnessAuthority`, `HarnessEngineService` and the MCP proxy/sidecar. Execute them on the selected device. Remove the preparation branches that skip harness setup, provider preparation or skill handling merely because the caller is remote.

Split Codex `profiles/mod.rs` at the touched responsibilities: Tauri handlers, schema/repository helpers, setup/probe processes and service facade. This large mixed module otherwise obscures what headless composition needs. Preserve probe behaviour; it is not a provider redesign. Jev already exposes credential-presence flags and an environment fallback; advertise observations, not credential values.

Keep `session_harness_bindings`, native profile bindings, login/setup attempts, upstream tokens and executable/home paths local. They are not replication payloads. Replicate portable policy separately; each host resolves its own current runtime configuration.

### Lifetime and notification changes

`PersistedRuntimeUpdateSink` currently returns notifier failures, and `QueueDispatchRuntimeUpdateSink` advances the queue only after a successful inner update. Split local execution observers from presentation delivery. Network publication is driven from committed changes, not the notifier. An absent UI or failed network notification cannot fail a persisted runtime event or suppress local queue advancement.

Retain local workflow callbacks and their persisted attempt handling. Replication import sends only a data-refresh notification; it never invokes `InvocationTerminal` execution observers.

Move active invocation accounting to the executing host. Separate `execution_devices.rs` into advertised metadata and local lifecycle control. Existing power-command configuration remains on its controlling device. A server replica neither runs copied shell commands nor evaluates copied leases for idle shutdown.

Change `agent_sessions/application/lifecycle.rs::reconcile_startup`: it currently scans every stored session, interrupts active invocations and resumes queue processing. Restrict recovery and worker wakeups to executions owned by this device and ownership generation. Replicated running sessions remain observations; restarting a replica must not mark another host's work interrupted. The data-only server does not run this recovery routine.

## 4 Make repositories participate in synchronization

Use `ActiveDatabase` as the transaction boundary. Feature repositories record a portable change or dirty aggregate within the same transaction as the local mutation. A separate synchronizer exports committed data and records delivery cursors. A crash between local commit and network send therefore leaves work available to synchronize later.

Add `synchronization/{contracts,repository,service,retention}.rs`; keep entity serialization and import validation with each owning feature. Use source device plus source sequence for transport deduplication, preserving existing session, invocation and runtime-event IDs. Transport sequence is not conversation order.

### Session write coverage

| Existing owner | Required journal coverage and reuse |
| --- | --- |
| `repository/queue.rs::accept_queued_preparation_record` | Record session creation, invocation, preparation, optional target operation and queue change in the existing atomic acceptance transaction |
| Queue edit/state/retry methods | Record the affected queue snapshot and preparation changes; retain revision validation and known-delivery retry checks |
| `repository/preparation.rs` | Export portable preparation progress, current resolution and execution provenance; retain local-only resolved launch secrets outside export |
| Event append, runtime-binding update and finish methods in `repository/mod.rs` | Export appended events and terminal/binding changes; keep invocation event sequence and idempotent finish rules |
| `repository/target_operations.rs` and `history.rs` | Export target-transition receipts and existing timeline positions; import positions rather than reallocating them on arrival |
| `native_conversations.rs` | Export device-scoped provider conversation references using the new stable key; do not import them as a local native-home binding |
| `addressing.rs`, `file_history.rs` and initial prompt prefixes | Include causal/workflow references, recorded file observations and initial context required by history and continuation |
| `repository/import.rs` | Separate generic historical materialization from its Codex native-profile write; reuse the former for portable forks and replication mappings |

Add explicit replica import transactions. They bypass local runtime-transition validation where replaying already-established facts requires it, while validating identity, source ownership and references. They do not call public send/finish/queue methods, emit local execution events or re-export incoming changes as new local changes.

### Data boundaries

| Data | Storage and synchronization treatment |
| --- | --- |
| Conversations, events, attachments, provenance and completed target receipts | Retain or cache as selected; attachment bytes already exist in `RuntimeImageAttachment` and invocation storage |
| Queue/preparation status | Replicated for inspection; only the session owner executes or changes it. Full queue state transfers during an explicit online ownership handoff |
| Skill content, workflow draft/active definitions, portable profiles and identities | Definition synchronization, with their dependencies included |
| Repository/device names and locations, capability observations | Replicate descriptions with source and observation time; they are not proof of current availability |
| Native credentials/homes, MCP tokens, local setup attempts, running processes, physical worktrees, build products | Local only |
| Navigation attention/order, review-build associations and experimental voice state | Remain local for this scope |

Retained mode includes the referenced skill content, profile/identity data and full conversation events needed for independent reading and forking. Cache mode fetches missing content on demand. Track completeness so a summary-only cache is never presented as a complete retained copy. Cache eviction removes local copies, not shared objects; unsynchronized local changes remain durable.

Definition synchronization uses its own simple replacement path. Non-master offline edits remain local but are not queued for later automatic publication. Online edits are accepted through the master/server path. Incoming synchronized data replaces non-master content; a master sync supplies the master's content. Keep acknowledgements and internal change counters, without exposing version histories or implementing merges. Accepted non-master edits update the master's Orchid copy so its next sync does not revert them.

## 5 Separate repository identity from local checkout identity

`RegisteredRepository` currently combines label with `anchor_root` and `git_common_directory`; engine repository identity hashes the canonical common-directory path. Preserve that physical identity in the engine. Introduce a portable logical repository ID in the catalog and map device-local repository instances to it.

Adapt `repository_device_locations` into that mapping, including the local device. Keep existing configured mappings when migrating. Do not merge independently registered repositories automatically just because a remote URL matches.

Changes to existing owners:

- `repository_catalog/repository.rs`: split logical registration metadata from per-device physical observations; remove the assumption that one common directory is the global identity.
- `execution_targets/inventory.rs`: resolve every root through device locations instead of using `anchor_root` for a special local branch.
- `session_navigation/branch_inventory.rs`: retain grouping/projection code; replace literal `local` identities and direct filesystem reads with host observations. Persist last-known observations for remote queries.
- `session_navigation/git_actions.rs`: move planning/execution to host-side repository operations; add explicit device identity to the input. Preserve existing branch validation and previews.
- `session_navigation/application.rs`: split the navigation read model and organization writes from working-directory resolution and capability discovery. Server reads do not scan local Git directories.
- `transport/preparation.rs`: remove folder-path injection from transport; destination preparation resolves it through these host operations.

Reuse `orchid-engine/repository_context` and `workspaces` for Git inspection, snapshot capture/apply and checkout creation. Worktree Review remains a desktop feature using these shared physical operations; it does not become the server's artifact store.

## 6 Adapt explicit target transitions and unavailable-session forks

Retain `plan_target_switch`, migration resolutions, source/destination inspection, snapshot verification, sister-worktree relationships and `WorkspaceTransitionReceipt`. Replace their direct SSH calls with `DeviceOperationsClient` calls.

Today `activate_target_operation` updates one application's database. With device-owned queues, activation must also transfer execution ownership:

1. The source owner reaches the target-operation queue barrier and stages/verifies the destination using existing operations.
2. The destination durably receives the required session context and remaining queue in an inactive state.
3. The source durably relinquishes execution for a new ownership generation and sends activation to the destination.
4. The destination activates that generation and advances its queue. Repeated messages acknowledge the same handoff. An ambiguous transfer stays blocked; it never restores source execution automatically.

This is a bounded extension to the existing target operation, not a new scheduler. It requires both devices to be reachable. Preserve owner generations in replicated session-control records so a late source snapshot cannot restore an old target.

Create a separate `agent_sessions/application/fork.rs` operation for unreachable/uncertain sessions. It uses locally retained or fetched complete turns and existing handoff-context construction. Store lineage and copied conversational context as history without inventing launch evidence. It creates a fresh session ID and requires a destination target; provider handles, old queue and source worktree state are not copied.

Add a continuation-availability projection to queries and enforce it at the command boundary: reachable clean sessions continue; running sessions expose steering/cancel according to current behaviour; interrupted/uncertain sessions require a fork. Known failures before delivery retain existing safe preparation retry behaviour. Source unreachability always requires a fork for work elsewhere.

## 7 Make skills content owned by Orchid

Create `skills/{domain,repository,import,materialization}.rs` in core. A skill has stable identity, name/description, origin device and source location, master device and the current package content. Supporting files are part of the package. Record the last imported source fingerprint to distinguish a changed origin from an unchanged source that has simply reconnected.

Adapt these existing consumers:

- `ProductSkillRoots`: become discovery/import sources; stop acting as portable skill identity.
- `compile_session_skill_inputs` and `pin_discovered_skill`: resolve selected catalogue entries to Orchid content, not external absolute-path IDs.
- `SessionProfile` and `SessionCreationResolution`: store portable skill selection; keep existing policy validation/digests without adding skill revision history.
- `validate_session_skill_inputs`: validate locally materialized launch content. Refresh current selected skill content when preparing a turn rather than requiring a permanent historical source file.
- `skill_reader.rs`: read the host's materialized Orchid copy and supporting resources; retain session exposure restrictions.
- Composer quick features and native skill mention handling: combine the shared catalogue with destination capability observations, retaining duplicate-name diagnostics and explicit selection.

The server can serve descriptions and files without provider processes. The host resolves paths for its OS and provider before invocation. A copied package does not imply its external programs or source-machine paths are executable elsewhere.

## 8 Keep workflows local while sharing their definitions

Reuse `WorkflowAuthoringService`, compiler, instance store, `SessionEventApplication`, OTP package contracts and deterministic attempt IDs in `otp_host/workflow.rs`. Add definition export/import at the authoring repository. Preserve existing draft/active semantics and revision checks; no new versioning UI.

Add an executor device to workflow instances. Instantiate progression and file-input resolution only on that host. `prompt_content.rs` and `file_inputs.rs` currently read its worktree directly; retain this co-location assumption initially instead of building a distributed workflow engine.

Adapt `SessionDirectory`, `SessionInvocationDispatcher` and `SessionControl` implementations to shared queries plus addressed commands. This permits explicitly targeted actions without putting workflow interpretation in the router. Copying definitions or instance observations never invokes `on_agent_notification`.

Do not claim full workflow resilience from the existing attempt IDs: `SessionEventApplication::dispatch` currently records its group result after dispatch. Routed deliveries must retain their deterministic identity at the destination acceptance boundary. Preserve existing recorded failures rather than adding general automatic workflow replay.

## 9 Client contracts and server interfaces

Move reusable Rust DTO mapping out of `agent_sessions/transport/dto.rs` into a transport-neutral feature API module. Keep Tauri `State` and command attributes in desktop adapters. The network service uses the same mapping and queries.

Retain `src/application/agentSessions` and the existing React view contracts. Update them for device identity, data completeness, execution availability, fork lineage and fork commands. Adapt `tauriAgentSessionClient` to the shared command/query facade; no separate frontend execution state machine.

Affected UI owners include `AgentSessionComposer`, `useSessionQueue`, `useSessionTarget`, `DeviceContinuationDialog`, target selection and branch context menus. Show readable-but-unavailable sessions, require a new target for forks, and distinguish cached configuration from a successful live check. Remove UI inference from `connection.kind === 'ssh'`.

Add server query endpoints for sessions/history/event details, shared definitions/skills, devices and repository observations; add addressed host commands and a change subscription. Hosts initiate the configured server connection. A server-relayed acknowledgement means the host accepted the command, not that execution finished. Lost replies can be checked/retried against the same host request identity. Offline remote commands are not silently queued by the server in this scope.

Reuse Axum/Tokio/Serde/UUID/SHA facilities already in the repository. Use maintained WebSocket client/server support rather than custom framing. SQLite changesets were considered, but selected typed exports better fit the existing mixed local/portable schemas and different definition/runtime policies. No CRDT framework or universal event bus.

## 10 Cleanup and delivery assignments

Remove after callers migrate:

- `RemoteRuntime` as a provider adapter for remote execution and SSH disconnect-generated terminal outcomes.
- `Host::SessionBinding` JSON files and the second provider registry, after importing existing configuration/bindings into the full host.
- Engine-owned host executable composition. Keep reusable engine workspace operations; relocate application-level network contracts above the engine.
- Runtime preparation exceptions based on the caller being remote.
- Duplicated DTO conversions, local-path fallback logic in transport, and literal `local` identities in shared records.

Retain the old SSH transport only during migration/testing. Do not delete its worktree operations until the new host operations cover every existing caller. Update `remote-host-smoke.mjs` and `remote-host-interaction-smoke.mjs` to exercise persistent host commands rather than stdio process ownership.

`scripts/deploy-orchid-host.ps1` currently archives only `orchid-engine`. Change packaging to include the new service/core crates and their dependencies. Add host/server build and service-install targets using the existing build tooling conventions; preserve isolated data roots and lockfiles. Keep deployment scripts and documentation separate from executing a deployment during implementation.

## Implementation sequence and validation

| Assignment | Required result before proceeding |
| --- | --- |
| Extract feature library, query service, handlers and owned runtime handle | Desktop behaviour retained; headless data queries construct without provider/runtime startup; core has no Tauri dependency |
| Consolidate providers and create persistent host entry point | Existing preparation, queue, steering, MCP and cancellation tests run against shared composition; host builds on Linux and desktop on Windows |
| Add device/repository identity and addressed operations | Same selected target resolves correctly from laptop and server; folder and Git operations run only on the addressed host |
| Add repository journal/import and retention | Local commit survives failed sync; replay preserves IDs/order and does not launch work; complete retained histories include attachments and deferred event details |
| Add online target ownership transfer and portable forks | Queue has one owner after migration; lost activation reply does not duplicate execution; unreachable-device forks need no source process/filesystem |
| Add shared skills/definitions and client controls | Server offers imported skills with laptop off; both hosts use local materializations; simple replacement policy behaves as prescribed |
| Update build/deploy and run integrated proof | Separate router and work-host processes; client/server disconnects do not stop host work; laptop-only operation and later synchronization both work |

Extend existing preparation persistence, target-switch, interaction, queue and navigation suites instead of replacing them. Add focused integration cases for dropped acknowledgements, replay without execution callbacks, notification failure during completion, copied remote-running state at startup, wrong-device binding migration, ownership handoff interruption and definition replacement. Test backup/retention using a fresh isolated replica, not the user's production database.

The final runtime proof starts work on the remote host, closes its client, restarts the server while work progresses, reconnects and verifies complete history with no duplicate invocation. A separate laptop run works with no configured server. Loss of the remote host leaves its synchronized conversation readable and forkable to an explicitly chosen local target.

## Source and library references

The linked source owners and their callers define these assignments. External building blocks checked for this shape: [Axum WebSocket support](https://docs.rs/axum/latest/axum/extract/ws/index.html), [Tokio Tungstenite client](https://docs.rs/tokio-tungstenite/latest/tokio_tungstenite/) and [SQLite session changesets](https://sqlite.org/sessionintro.html).
