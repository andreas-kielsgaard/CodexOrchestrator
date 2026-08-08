# Observation pass: Native and managed runtime authority

## Evidence boundary

This pass follows concrete launch callers and effects rather than treating Native Profiles, Agent Sessions, Harnesses, MCP and CLI integration as separate catalog categories.

Two repository states matter:

- the research checkout is pinned at `b28137b`;
- a clean detached worktree at `C:/Users/user/.codex/worktrees/68dc/Codex Orchestrator` contains descendant commit `9240364` (`Bind managed sessions to ready native profiles`). It descends through `b583277`, which serializes MCP probe creation with profile selection.

No branch currently names `9240364`. It is therefore concrete implemented evidence, but not part of the checked-out research baseline or a named integration authority.

**Subsequent checkout note:** after this pass exposed the descendant, the dedicated research branch was safely fast-forwarded to `9240364`. The sentence above records the discovery-time topology; local `main` remains at the older `b86a8ac` line and the product integration authority is still unresolved.

Focused Rust validation was run against `9240364` with `cargo test -q managed_`: 4 passed, 1 ignored, 499 filtered out. This pass did not perform a live provider, packaged-application or cold-machine launch.

## Main finding

At `b28137b`, Native Profiles and managed Agent Sessions are parallel runtime systems. They resolve the same installed `codex` executable through shared discovery code, but selected-profile authority does not reach ordinary or managed Agent Session launches.

At `9240364`, selected Native Profile readiness becomes a pre-provider gate for the shared `AgentSessionApplication`. Every release-composed Agent Session launch receives an application-selected `CODEX_HOME` and a durable Session/profile binding. This includes standalone Agent Sessions and all orchestration-managed roles, not only Epic Plan Builder.

The continuation centralizes **Codex-home identity**, but not the Native Profile **launch policy**:

- the standard Agent Session runtime still owns process launch;
- it still inherits the parent environment;
- it still permits ambient Codex and project configuration unless individual launch arguments override it;
- Conversation Harness sandbox and approval settings still determine managed-role options;
- Native Profile `selected_mode`, Danger Full Access authorization, strict projection flags, environment clearing and application-owned target policy are not applied.

The systems therefore converge at one environment key, not at one complete runtime boundary.

## Concrete path A: Native Profile selection and readiness

### Product caller

`src/app/App.tsx` mounts `NativeProfileSettings` as the release Technical Settings surface when `nativeProfileClient` exists. `src/features/nativeProfiles/NativeProfileSettings.tsx` calls the typed methods in `src/infrastructure/nativeProfiles/nativeProfileClient.ts`, which invoke Tauri commands registered in `src-tauri/src/active_app.rs`.

The command-to-service path is direct:

| User operation | Tauri command | Service effect in `src-tauri/src/native_profiles.rs` |
| --- | --- | --- |
| register existing home | `register_native_profile` | canonicalize an existing directory, reject an application marker, record filesystem identity and user ownership |
| create dedicated home | `create_dedicated_native_profile` | create under app data `native-codex-homes/`, write the ownership marker, record filesystem identity |
| select | `select_native_profile` | validate continuity, clear every `selected_at`, set one selected profile |
| choose mode | `select_native_profile_execution_mode` | persist `workspace_write` or `danger_full_access` |
| authorize danger | `authorize_native_profile_danger_full_access` | bind versioned full-machine-filesystem and unrestricted-network authority to exact filesystem identity |
| login/status | `request_native_profile_login`, `refresh_native_profile_readiness` | launch `codex login`; separately inspect `codex login status` |
| sandbox/canaries | sandbox and canary commands | record request, launch, terminal, receipt and confirmation facts separately |
| MCP readiness | probe and reconcile commands | create and settle one correlated reporting request |

### Selection and continuity authority

`NativeProfileService::select` holds `operation_gate`, revalidates the candidate directory and its recorded filesystem identity, then moves the singleton selection. Losing continuity through `record_lifecycle_while_gated` has broad effects:

- the profile is unselected;
- pending login, setup, canary and MCP work is cancelled or terminalized;
- Danger Full Access authorization is revoked;
- readiness returns to unknown/not-run/not-assessed states;
- external sandbox adoption and its product confirmation are invalidated;
- an explicit continuity attention is recorded.

Changing selection does less. It invalidates external sandbox adoption for the profile being unselected, but a current Danger Full Access authorization remains stored and identity-bound. It can become usable again if that same profile is reselected and all other gates still hold.

`select_execution_mode` requires an active profile but not the selected profile. The UI also offers mode selection on every profile. Authorization itself is stricter and requires the currently selected active profile.

### Native CLI environment and capability evidence

`SystemNativeCliPort` and `spawn_system_native_cli_child` form a private process adapter inside `native_profiles.rs`.

For login, setup, canary and MCP work it:

- resolves `codex` through `runtime::codex::resolve_program`;
- clears the process environment;
- supplies selected `CODEX_HOME`;
- on Windows, restores only `APPDATA`, `COMSPEC`, `HOMEDRIVE`, `HOMEPATH`, `LOCALAPPDATA`, `PATH`, `PATHEXT`, `SYSTEMROOT`, `TEMP`, `TMP`, `USERPROFILE` and `WINDIR` when present;
- discards native CLI stdout/stderr rather than retaining provider or profile content.

Its surface probe also clears the environment and checks:

- `codex --version`;
- `codex exec --help`;
- `codex sandbox --help`;
- `codex sandbox setup --help`.

Workspace project-config isolation and Danger Full Access semantics are accepted only for exact `codex-cli 0.144.0` plus the expected help tokens. This is stronger than generic Agent Session capability discovery, which interprets version/help into flag support and caches it but does not require that exact version.

### MCP readiness is an isolated proof route

The Native Profile MCP route is a dedicated one-tool server, not the orchestration MCP host:

- capability: `native-codex-profile-reporting/v1`;
- server: `codex-orchestrator-reporting`;
- tool: `report_native_profile_readiness`;
- authority: profile id, correlation, exact probe root, server and tool, deadline, and dispatch claim;
- transport: loopback with a fresh bearer passed through a fresh environment-variable name.

`begin_mcp_reporting_probe` persists a `pending` request. `reconcile_pending_mcp_reporting` claims it as `dispatching`, starts the server and launches strict Codex in the application-owned probe root with:

- `--strict-config`;
- `--ignore-user-config`;
- `--ignore-rules`;
- Workspace Write sandbox;
- only the injected reporting server/tool requested by the application prompt.

Process exit does not establish readiness. Only the correlated tool receipt changes MCP readiness to `ready`; a completed process without a receipt becomes an explicit failed exchange.

`b583277` adds `operation_gate` around probe creation, closing the race in which selection could move after the selected-profile check but before the durable request was inserted.

One frontend contradiction remains in both inspected states: `NativeProfileSettings` labels its action “Start MCP/reporting probe”, but `NativeProfileClient.probeMcp` invokes only `reconcile_native_profile_mcp_reporting`. It never invokes `probe_native_profile_mcp_reporting`, so a user with no existing pending request cannot create the request from this button.

### Three different “consumer” boundaries

The code currently contains three differently strict concepts that can be mistaken for one:

1. `src/infrastructure/nativeProfiles/nativeProfileConsumer.ts` accepts a decoded profile when its caller-supplied id exists exactly once, is selected and has lifecycle `active`. It returns readiness but does not require any readiness value.
2. `NativeProfileService::resolve_selected_home` requires selected active continuity plus `authenticated`, `initialized`, Workspace Write canary `passed`, and MCP reporting `ready`.
3. `NativeProfileService::project_launch` requires selected active continuity and mode-specific launch authority, but does not call `resolve_selected_home`; authentication and MCP readiness are not part of that projection gate.

The TypeScript consumer is constructed in `productApplicationComposition.ts`, passed as an `AppProps` field, and never destructured or called by `App`. At `b28137b`, Rust `resolve_selected_home` likewise has no non-test launch caller. `project_launch` is used productively only by the bounded Danger Full Access canary.

No Tauri command exposes `resolve_selected_home` or `project_launch` to the frontend. The visible settings surface manages evidence and authority; it is not itself an Agent Session launch surface.

## Concrete path B: managed Epic Plan Builder before native binding

Epic Plan Builder is the closest fully composed managed-agent comparison because its frontend, Tauri operation, role Harness, invocation-scoped MCP server and shared runtime are all connected.

The call chain at `b28137b` is:

1. `EpicPlanBuilder` uses `createTauriManagedPlanBuilderSessionClient`.
2. Ordinary discussion invokes `send_managed_plan_builder_message`; the application-authored build action invokes `request_managed_plan_builder_action`.
3. `orchestration/transport.rs` validates DTO identities and calls `ManagedPlanBuilderService::send` or `request_plan`.
4. `ManagedPlanBuilderService::send_with_provenance` loads the `epic_plan_builder` Conversation Harness, fixes the working directory, Session options, prompt prefix and MCP tool list, then bootstraps durable draft/profile/association authority.
5. It starts an invocation-scoped MCP listener and builds a `RuntimeLaunchExtension` containing Harness configuration plus MCP endpoint configuration and one bearer environment pair.
6. It asks `AgentSessionApplication` to persist and launch the invocation.
7. `AgentSessionApplication::send_message_with_provenance` runs generic runtime preflight, marks the invocation running and constructs `RuntimeInvocationRequest`.
8. `CodexCliRuntime` builds `codex exec --json ...` or `codex exec resume --json ...` and `SystemProcessFactory` starts the child.

### Harness-owned conditions

`conversation_harness_catalog.json` defines Epic Plan Builder version 4 with:

- no fixed model or reasoning effort;
- `read_only` sandbox;
- approval policy `never`;
- required tools `submit_epic_plan_proposal` and `request_epic_initiation`;
- first-query prompt context and `epic-plan-builder` skill guidance.

`ManagedPlanBuilderService` rejects a caller-selected model and any sandbox that differs from the Harness. It ignores the caller's working directory and uses `epic_plan_builder_discovery_root`, which validates the canonical skill artifact but returns the whole Codex Orchestrator repository root. A newly created Session stores that root; an existing Session receives it as a per-invocation working-directory override.

Only the first query receives the static Harness prefix unless a pending button-initiation context replaces it. Persisted submitted text remains separate from the rendered provider prompt.

### Managed MCP authority and lifecycle

`orchestration/mcp.rs` starts a fresh loopback listener for each invocation. `CodexMcpInjection` creates:

- a random server name;
- a random bearer environment-variable name and value;
- `mcp_servers.<name>.url`;
- the exact enabled-tool list;
- `required=true` for Plan Builder;
- automatic approval for those tools;
- bounded startup and tool timeouts.

The HTTP guard checks bearer, exact Host and an allowlist containing `tauri://localhost` when Origin is supplied. Absence of an Origin is permitted.

The server starts before the Agent Invocation is bound. Tool handling waits up to five seconds for that binding, then derives durable commands from server-side authority:

- draft id;
- capability profile id;
- draft/Session association id;
- Agent Session id;
- Agent Invocation id;
- captured revision precondition;
- application-owned actor and deterministic idempotency identity.

The model cannot supply those identities. Proposal submission writes the typed proposal through `OrchestrationApplication`. Initiation requests enter a separate confirmation coordinator and may wait up to 240 seconds for explicit user resolution.

The bearer value is passed only in the child launch environment and is not stored in Agent Session runtime events; launch provenance records environment key names. `ManagedPlanBuilderRegistry` owns the server until the exact invocation becomes terminal. Send failure, synchronous terminal completion, registry failure and application shutdown also stop it.

### Standard runtime configuration posture

Before `9240364`, the managed launch extension contains the MCP bearer but no `CODEX_HOME`. `SystemProcessFactory` calls `.envs(...)` without `.env_clear()`, so the child inherits the full desktop-process environment and overlays the supplied pair.

The constructed Plan Builder arguments add:

- `--json`;
- `--sandbox read-only` for a fresh invocation, or the equivalent resume config;
- `-c approval_policy="never"`;
- the dynamically injected `mcp_servers.*` values.

They do not add `--strict-config`, `--ignore-user-config`, `--ignore-rules`, `mcp_servers={}`, project-root suppression, hook/plugin/app disabling or a parent-environment allowlist. From the constructed launch alone, other user/project configuration and MCP definitions remain eligible to affect Codex. This pass did not perform a live CLI experiment to enumerate the resulting effective configuration.

## Descendant path: `9240364` binds Agent Sessions to Native Profiles

### Composition change

`active_app.rs` changes `NativeProfileService` to `Arc<...>` and installs it on the shared `AgentSessionApplication` through `with_native_profile_launch_authority`. The same service is also placed in Tauri state for Technical Settings.

This is a backend composition seam. It does not use the unused TypeScript `NativeProfileApplicationConsumer`.

Because `AgentSessionApplication` is also used by standalone Agent Sessions, Plan Builder, bootstrap/runner transitions and Work Unit execution services, the authority is global within the release composition. The code and focused tests exercise generic `send_message`, not only a managed-role method.

### Pre-provider binding sequence

`AgentSessionApplication::send_message_with_provenance` now performs this sequence after persisting a pending invocation but before generic Codex capability preflight:

1. ask `NativeProfileLaunchAuthority::prepare_launch` for an effective launch extension;
2. on rejection, finish the invocation as a durable runtime preflight failure and return `launch_accepted=false`;
3. on success, perform generic runtime option preflight;
4. mark running, construct `RuntimeInvocationRequest`, call start/resume, then separately persist launch acceptance.

`NativeProfileService::prepare_managed_agent_session_launch`:

- rejects a role extension that already supplies any case-insensitive `CODEX_HOME` key;
- calls the strict Rust `resolve_selected_home` gate;
- creates or checks `agent_session_native_profile_bindings` for Session id, profile id and filesystem identity;
- requires a pre-existing binding on resume;
- creates or checks `agent_session_native_profile_launch_provenance` for Invocation id, Session id, profile id, filesystem identity, the key name `CODEX_HOME`, and start/resume mode;
- commits those facts;
- appends the selected home as `CODEX_HOME` while preserving role MCP environment and arguments.

The raw home path is not stored in either new table. It exists in the existing profile table and the ephemeral process environment. Schema version advances from 35 to 36.

### Continuity effects

The first start binds a Session to the selected ready profile. Later starts or resumes must see that same profile id and filesystem identity as the current selected ready profile. Consequences:

- a cold reopen can resume when durable Session binding, filesystem identity and selection still match;
- switching the selected profile prevents an established Session from continuing;
- returning selection to the originally bound, still-valid profile can restore eligibility;
- replacing the directory at the same path fails through filesystem-identity continuity;
- a caller cannot override `CODEX_HOME` through a role-specific extension.

This is durable identity continuity, not process reattachment or proof of provider activity.

## Runtime policy comparison

| Concern | Native readiness/projection route | Managed Plan Builder at `b28137b` | Shared Agent Session path at `9240364` |
| --- | --- | --- | --- |
| selected profile required | yes | no | yes, for every composed Agent Session send |
| readiness required | operation-specific; `resolve_selected_home` requires auth, sandbox, Workspace Write canary and MCP | no Native Profile gate | full `resolve_selected_home` gate |
| durable Session/profile binding | no | no | yes, profile id plus filesystem identity |
| `CODEX_HOME` | selected explicitly | inherited if present in parent | selected explicitly as child overlay |
| parent environment | cleared and allowlisted | fully inherited | fully inherited, then selected home and role bearer overlay it |
| user config/rules | ignored on strict probe/projected routes | not ignored | not ignored |
| project config/discovery | suppressed in `project_launch`; bounded probe root for MCP | repository root deliberately used for skill discovery | same managed working root and ambient project behavior |
| execution mode | selected Native Profile mode plus mode-specific authority | Harness read-only | still Harness/session requested sandbox; Native Profile mode is not consulted |
| Danger Full Access authority | exact identity, versioned scope, explicit authorization | not relevant to Plan Builder read-only | not enforced by the binding seam for any Agent Session sandbox request |
| MCP | separate one-tool readiness server; projected general launch clears MCP | invocation-scoped Plan Builder server is added | Plan Builder server is still added; selected profile only supplies home identity |
| launch adapter | private `SystemNativeCliPort` | shared `CodexCliRuntime` | shared `CodexCliRuntime` |
| launch acceptance | separate from projection/request/receipt | persisted after runtime accepts start | still separate; profile provenance is prepared before runtime preflight |

## Surprising relationships and unresolved contradictions

### 1. The new “managed” binding is product-wide

The method is named `prepare_managed_agent_session_launch` and the commit title says “managed sessions”, but the authority is invoked inside the generic `AgentSessionApplication` send path. Standalone Agent Sessions are gated and bound too. The intended scope needs an explicit product decision and caller inventory.

### 2. Home identity is centralized; execution authority is not

`9240364` does not route Agent Sessions through `project_launch`. Selecting `danger_full_access` and authorizing it does not make Plan Builder dangerous because its Harness remains read-only. Conversely, an Agent Session request for a dangerous sandbox is not checked against Native Profile Danger Full Access authorization by this binding seam. Native Profile mode selection and authorization are therefore not the authority for shared Agent Session sandbox selection.

### 3. Readiness is proved under one environment and consumed under another

Native login, canary and MCP evidence is gathered using a cleared/allowlisted environment and strict probe commands. The bound Agent Session uses an inherited parent environment, normal configuration loading and different role arguments. The shared executable discovery makes these related, but readiness does not prove the exact effective managed-launch environment.

### 4. CLI drift is not re-established at the consumer gate

`resolve_selected_home` revalidates profile filesystem continuity and the stored readiness values. It does not itself call `NativeCliPort::surface` or compare the current executable/version with the provenance that produced all readiness facts. `project_launch` does recheck the CLI surface, but the bound Agent Session path does not call it. An installed CLI change can therefore sit between recorded readiness and a later managed launch without an explicit shared-version comparison.

### 5. “Launch provenance” precedes launch preflight

The new Invocation provenance row is committed before generic Codex capability preflight and before process start. A later runtime preflight rejection leaves a durable row named `agent_session_native_profile_launch_provenance` even though launch was not accepted. Its `prepared_at` column is truthful, but consumers must not interpret table presence as launch evidence.

### 6. Session binding can be established without a successful launch

The Session/profile binding is committed in the same pre-preflight transaction. A first Invocation can bind a Session and then fail generic runtime preflight or spawn. This may be a valid continuity choice, but it is not “profile used by a provider process” evidence.

### 7. Selection and binding are not one serialized operation

`prepare_managed_agent_session_launch` calls `resolve_selected_home`, then opens and commits a separate binding transaction without holding `NativeProfileService::operation_gate`. Selection can move between those steps. The MCP creation race was explicitly closed by `b583277`; the analogous Session-binding window remains in `9240364`.

### 8. Native MCP readiness does not constrain managed MCP exposure

Passing the one-tool native reporting probe proves that exact reporting transport. Plan Builder then injects a different random MCP server and two semantic tools. Because the shared runtime does not clear inherited/user/project configuration, the launch construction also does not establish that these are the only MCP tools visible to Codex.

### 9. The frontend and Rust “application consumer” contracts disagree

The unused TypeScript consumer accepts any selected active profile and merely returns readiness. The productive Rust consumer rejects until four readiness dimensions pass. Keeping both names without one canonical contract invites future callers to select the weaker gate accidentally.

### 10. The visible MCP action still cannot initiate its backend lifecycle

The backend now has stronger dispatch, terminal-history and selection-race handling. The release frontend still calls reconciliation without request creation. This is a reachability break, not a missing server implementation.

### 11. The most current implementation is detached from named integration state

At discovery time, the research checkout and its catalogs described the `b28137b` disconnect while `9240364` was held only by detached worktrees. The dedicated research branch now names that descendant and the shared orientation pages have been updated. Current-state synthesis must still distinguish the inspected line from older local `main` and divergent product lines; otherwise it will either miss implemented work or overstate one line as the whole product.

## Artifact and symbol index

| Responsibility | Artifact / symbol |
| --- | --- |
| release composition | `src-tauri/src/active_app.rs::run` |
| Technical Settings UI | `src/features/nativeProfiles/NativeProfileSettings.tsx` |
| native Tauri adapter | `src/infrastructure/nativeProfiles/nativeProfileClient.ts` |
| unused frontend consumer | `src/infrastructure/nativeProfiles/nativeProfileConsumer.ts` |
| native profile service, schema, CLI, Tauri commands | `src-tauri/src/native_profiles.rs` |
| strict readiness gate | `NativeProfileService::resolve_selected_home` |
| isolated native command projection | `NativeProfileService::project_launch` |
| descendant Session binding | `NativeProfileService::prepare_managed_agent_session_launch` at `9240364` |
| descendant launch authority interface | `agent_sessions/application/lifecycle.rs::NativeProfileLaunchAuthority` at `9240364` |
| managed Plan Builder frontend seam | `src/infrastructure/orchestrations/tauriManagedPlanBuilderSessionClient.ts` |
| Plan Builder Tauri commands | `src-tauri/src/orchestration/transport.rs` |
| managed send and authority assembly | `src-tauri/src/orchestration/application.rs::ManagedPlanBuilderService::send_with_provenance` |
| static role policy | `src-tauri/src/orchestration/conversation_harness_catalog.json` and `conversation_harness.rs` |
| Plan Builder MCP host/tools/injection | `src-tauri/src/orchestration/mcp.rs` |
| generic invocation persistence/preflight | `src-tauri/src/agent_sessions/application/lifecycle.rs::send_message_with_provenance` |
| generic Codex args and launch | `src-tauri/src/runtime/codex/arguments.rs`, `runtime.rs` |
| inherited environment process adapter | `src-tauri/src/runtime/processes/system.rs::SystemProcessFactory` |

## Next evidence questions

- Was `9240364` intended to gate every Agent Session or only application-managed orchestration roles?
- Should the canonical runtime boundary apply Native Profile mode/authorization and strict projection, or only selected-home identity?
- Should a profile/Session bind hold the same selection gate through durable commit?
- Which CLI provenance facts must be revalidated immediately before a shared runtime launch?
- Should Session binding occur at Session creation, first accepted launch, or the current pre-preflight point?
- Should managed roles explicitly clear or allowlist parent environment and unrelated MCP/config layers?
- Should the TypeScript consumer be removed, strengthened, or become the frontend projection of the Rust contract?
- Should “Start MCP/reporting probe” create and reconcile atomically, or expose the two durable stages deliberately?
