# Claude provider: implementation shape

Status: plan, 2026-09-25. Nothing in this document is implemented yet. It builds on the provider boundary described in [agent-provider-integration.md](agent-provider-integration.md) and the parked items in [agent-provider-boundary-implementation-shape.md](agent-provider-boundary-implementation-shape.md).

## Functional target

- A Capability Profile can route to Codex and Claude. On each device it uses at most one Codex setup and at most one Claude setup. The provider for a message follows from the device and the selected model.
- Technical Settings shows a per-device list of provider setups: the native folder (Codex home, Claude config folder) and the CLI executable with its login state. Setup stays minimal and mostly automatic; Codex keeps its current automation.
- The Agent Session stays Orchid's shell around lifecycle and control. Each provider registers its own handlers: runtime, configuration discovery, launch preparation and, optionally, conversation transfer.
- A session keeps one native conversation per provider. Switching to a provider without a conversation starts one with context rebuilt from the session log. Switching back resumes that provider's conversation with the turns it missed placed before the new prompt.
- One neutral interaction request shape covers Codex's narrower and Claude's broader question and approval features.
- The remote host becomes provider-neutral, with Codex as its only registered host provider. Remote Claude comes later.
- Access modes are provider-reported options on a profile route. Claude starts with full access only.
- Claude runs locally by driving the installed `claude` CLI. The target is the happy path under assumed circumstances; problems are fixed when seen.

## What discovery found

| Observation                                                                                                                                                                                                                                                                                                | Consequence for the shape                                                                                                   |
| ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------- |
| `AgentSessionApplication` holds one configuration source (`application/configuration.rs:350`), one fallback runtime (`application/targets.rs:42`) and one launch hook (`application/mod.rs:55`). All are Codex's.                                                                                          | Consumers ask for the provider of the route they already hold instead of a single slot.                                     |
| `CapabilityProfileService` holds one configuration source and passes bare configuration IDs (`execution_configuration/service.rs:16-136`).                                                                                                                                                                 | Profile discovery calls take a route and go through the router.                                                             |
| `ExecutionEndpoints` is the app-wide router from an instance's binding to its runtime, and it already holds the provider registries. Its constructor registers exactly one provider (`execution_targets/endpoints.rs:40`).                                                                                 | Keep it as the one place consumers ask; let it take every provider's registrations.                                         |
| The runtime, configuration and continuation registries are three copies of the same keyed map (`runtime/providers/{runtime_registry,configuration_registry,continuation}.rs`). A launch registry and a host provider map would add two more.                                                               | One generic provider map; each provider registers its parts through one `register` function.                                |
| The launch hook (`NativeProfileLaunchAuthority`, `application/dependencies.rs:47`) is implemented only by Codex's profile service and rejects other providers (`codex/profiles/mod.rs:3831`).                                                                                                              | A per-provider launch-preparation registration.                                                                             |
| A session stores one native conversation ID that can never change (`agent_sessions/domain.rs:247-257`, column `agent_sessions.external_context_id`). Device moves transfer it; a provider change is rejected (`execution_targets/preparation.rs:100`).                                                     | Native conversations become per provider, each with its location and the last invocation it saw.                            |
| The final agent reply of an invocation is found by scanning for `details.role == "final"` in four places: `product_decisions.rs:1929`, `workflows/event_sources.rs:54`, `orchestration/bootstrap_transition.rs:2258` and `transcriptProjector.ts:513`. The convention exists only in the Codex normalizer. | A second normalizer and the new history handoff would add more copies. Name the convention once and add one backend reader. |
| Interaction requests are untyped JSON built in the Codex adapter (`codex/app_server/requests.rs`) and typed only in the frontend DTO (`application/agentSessions/contracts.ts`). The question UI supports single choice only (`SessionInteractions.tsx:93`).                                               | A typed neutral request in the engine contracts; the UI gains multi-select and "other" answers.                             |
| The profile editor's route choices and the Technical Settings harness cards are built from local Codex homes (`ExecutionConfigurationScreen.tsx:175`, `ExecutionSetupOverview.tsx:26`, `application/agentProviders/codex/localRoutes.ts`).                                                                 | A backend list of provider setups per device replaces the frontend Codex projection.                                        |
| The composer's target selection is a whole route (`useSessionTarget.ts:53`), and per-message models come from the pinned route only (`AgentSessionExecutionSettings.tsx:86`).                                                                                                                              | Model choice spans the profile's routes on the session's device; the backend derives the route.                             |
| The model catalogue is keyed by configuration ID only (`execution_configuration/repository.rs:29`).                                                                                                                                                                                                        | Key it by route.                                                                                                            |
| The remote host accepts only Codex: configuration type, runtime construction, continuation and capabilities are hard-coded (`orchid-engine/src/host.rs:105,234-265,305-318` and the `CODEX_HOME` launch environment).                                                                                      | A host provider trait with a Codex implementation in `providers/codex/host.rs`.                                             |
| The engine depends only on `chrono`, `serde`, `serde_json`, `sha2` and `uuid`, and owns child processes through its `ProcessSupervisor` (with Windows job handling). Codex's line framing sits inside its JSON-RPC connection (`codex/app_server/connection.rs:71-117`).                                   | Claude reuses the supervisor. The JSON-lines framing is extracted and shared.                                               |
| Workflow Harness roles fix their access mode (`orchestration/conversation_harness_catalog.json`: read-only for most roles, workspace-write for the Implementer).                                                                                                                                           | See the assumptions: a Claude route that does not offer the mode fails with the existing explicit error.                    |

## Resulting ownership

- **Agent Session (Orchid):** session, invocation and instance lifecycle; steering, requests and cancellation routing; persistence; the per-provider conversation records and the history handoff text.
- **Execution endpoints (Orchid):** given a binding or provider, return the provider's runtime, configuration source, launch preparation and transfer port; route local versus SSH.
- **Provider modules:** everything native: process and protocol, launch translation, event and request translation, setup storage and detection, discovery, launch environment, and optional conversation transfer.
- **Capability Profiles (Orchid):** routes per device and provider, allowed models and reasoning, access mode, skill and MCP groups; deriving a route from device and model.

## Shape by area

### 1. Provider registration and routing

| Change                                                                                                                                                                            | Files                                                                                                                                                                                                              |
| --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Create one generic keyed map and a `ProviderRegistrations` value holding runtimes, configuration sources, launch preparations and continuation ports.                             | Create `src-tauri/src/runtime/providers/registrations.rs`. Remove `runtime_registry.rs` and `configuration_registry.rs`. Keep the trait in `continuation.rs` and drop its registry half.                           |
| Each provider exposes one `register(&mut ProviderRegistrations, …)` that composes its parts.                                                                                      | `runtime/providers/codex/mod.rs` takes the Codex wiring now spread over `active_app/sessions.rs:40` and `active_app/execution_configuration.rs:33-52`. `runtime/providers/claude/mod.rs` does the same for Claude. |
| `ExecutionEndpoints::new(registrations)`. Add `configuration_source(provider)` and `launch_preparation(provider)` next to `runtime(binding)`. Shutdown stops every local runtime. | `execution_targets/endpoints.rs`, `agent_sessions/application/lifecycle.rs:157`                                                                                                                                    |
| Session code asks the route's provider through the endpoints. The single `profile_source` and launch-hook slots go away.                                                          | `agent_sessions/application/{configuration.rs:199,247,350, quick_features.rs:60,134, direct_user.rs:206, addressed.rs:29,165, mod.rs:55}`, `agent_sessions/session_event_adapter.rs:42`                            |
| Sessions without an execution target route by their pinned configuration's provider. The fixed runtime remains only for unprofiled legacy sessions and test composition.          | `agent_sessions/application/targets.rs:42`                                                                                                                                                                         |
| Profile discovery methods (native skills, model catalogue, inventory) take a route and go through the endpoints. The service no longer holds a source.                            | `execution_configuration/service.rs`                                                                                                                                                                               |
| The model catalogue is keyed by route (device, provider, configuration).                                                                                                          | `execution_configuration/{repository.rs,model_catalogue.rs}`, `storage.rs` migration                                                                                                                               |

### 2. Launch preparation per provider

- `NativeProfileLaunchAuthority` becomes `ProviderLaunchPreparation`, registered per provider. Only `prepare_launch` is required. The Codex-only operations (`prepare_destination_launch`, `commit_destination`, `bound_configuration_ref`) keep default implementations, so Claude implements only what it needs.
- The Codex implementation moves out of `codex/profiles/mod.rs:3831` into `runtime/providers/codex/launch.rs`, as a thin adapter over `NativeProfileService`. Its behavior is unchanged.
- `RuntimeLaunchExtension` gains `executable: Option<String>`. Like `environment`, it is written only by the provider's own launch preparation. Claude uses it for the setup's executable; Codex may keep ignoring it locally.

### 3. Provider setups per device

- Create a neutral read model, `ProviderSetup { device_id, provider, configuration_id, folder, executable, state, detail }`, where state is ready, needs login or unavailable. Add `ProviderConfigurationSource::setups()`, defaulting to empty, and a Tauri command `list_provider_setups` in `execution_configuration/transport.rs`.
- Codex's `setups()` projects its active native profiles: what `localCodexRoutes` computed in the frontend, plus the resolved `codex` executable and its readiness. Codex's own screens and automation stay as they are.
- Claude's setups live in its own table (`claude_setups`: id, folder, executable). The first launch registers the default config folder with `claude` resolved on `PATH`. The user can add another folder or executable. State comes from `claude auth status`: exit code 0 means ready; 1 means needs login, shown with the instruction to run `claude auth login` for that folder.
- Technical Settings:
  - `ExecutionSetupOverview.tsx` lists the setups per device, grouped by provider with descriptor labels.
  - Each provider's setup editor is composed explicitly in `src/features/agentProviders/ProviderSetupSettings.tsx`, following the `ProviderRouteSettings.tsx` pattern: Codex opens its existing profile screen, and Claude gets a small `ClaudeSetupSettings.tsx`.
  - `localRoutes.ts` and its test are removed.
- Remote devices keep their developer-managed host configuration. They get no setup list in this work.

### 4. Capability Profiles across providers

- `CapabilityProfile::validate` (`execution_configuration/capability_profile.rs`) rejects two routes for the same provider on one device and repeated model IDs across one device's routes.
- `CapabilityProfile::route_for_model(device_id, model)` picks the route for a model. On a device, the default route is the one offering the profile's default model, otherwise the first route listed for that device.
- The profile editor's route choices come from `list_provider_setups` (`ExecutionConfigurationScreen.tsx`). It loads model catalogues for both providers' routes. Access-mode options come from each route's reported exposure; Claude reports full access only.
- The per-message model list (`AgentSessionExecutionSettings.tsx`, `PerMessageRuntimeControls.tsx`) shows the models of every profile route on the session's device, labeled by provider. It reads them from the session's Capability Profile, which the frontend already loads.
- Sending a model that belongs to another route: `direct_user.rs` finds the route with `route_for_model`. It records a target change to that route with the same workspace, and the existing prepared-launch path applies it before the prompt: same session, new instance.
- `SessionTargetDialog` chooses device, profile and worktree; it no longer chooses a route.

### 5. One native conversation per provider

- **Domain.** `AgentRuntimeBinding.external_context_id` becomes a list of `NativeConversation { provider, external_context_id, location, last_invocation_id }`, where `location` is the route (`ExecutionRouteRef`) the conversation lives on. The "never changes" rule applies per provider.
- **Storage.** A new table, `agent_session_native_conversations`, is keyed by session and provider. The `storage.rs` migration moves existing `agent_sessions.external_context_id` values in as provider `codex`, with the session's current route (or the local default) as location.
- **Writers.**
  - Context established: `update_sink.rs:220`.
  - Invocation terminal: records `last_invocation_id`.
  - Preparation commit: `repository/preparation.rs:144`.
  - Recovery: `lifecycle.rs:166-205`.
  - Import: `repository/import.rs`.
- **Readers.** `invocation.rs:469,508,609` and `preparation/execution.rs:86,271,312` use the destination provider's conversation.
- **Transfer.** When that conversation's location differs from the destination route:
  - a provider with a continuation port transfers it, as today;
  - a provider without one starts a new native conversation from the session log. Claude registers no port in this work.

  The cross-provider guard in `execution_targets/preparation.rs` stays, but can no longer trigger.

- **History handoff.**
  - A new `agent_sessions/application/history_handoff.rs` builds the catch-up text: for each completed invocation after the provider's `last_invocation_id` (or all earlier invocations when the provider has no conversation), your submitted text and the final agent reply.
  - The session stores the initial prompt prefix its first message was delivered with (a new `agent_sessions.initial_prompt_prefix_json`, written once). Today no prefix is stored; callers only put it in the launch extension.
  - When a provider starts its first conversation for a session that already has history, its first message carries the stored initial prefix, then the catch-up text, then any prefix the caller supplied for this message. These are combined into one `initial_prompt_prefix` with source `orchid_provider_handoff`, so the wire format does not change.
  - When switching back to a provider that already has a conversation, only the catch-up text is added; the stored prefix is not repeated, because that provider loads it through its own conversation.
  - Both launch paths use the same builder.
- **Final reply.**
  - Add one reader, `final_reply(invocation)`, on the session history type. Adopt it in `product_decisions.rs:1929`, `workflows/event_sources.rs:54`, `orchestration/bootstrap_transition.rs:2258` and the handoff.
  - Name the message-role values once in the engine contracts; both normalizers use them. The stored format is unchanged.

### 6. Neutral interaction requests

- In `crates/orchid-engine/src/contracts/interactions.rs`, add a typed `RuntimeRequest` with choices and questions.
  - Each question has `id`, optional `header`, the question text, options, `multiSelect`, `isOther` (a typed answer is allowed) and `isSecret`.
  - It keeps the field names already stored, so older records decode unchanged.
  - `RuntimeControlRecord::RequestOpened` carries the typed request.
- `codex/app_server/requests.rs` builds the typed request instead of `json!`, and validation reads the typed questions.
- Claude questions have no IDs. The Claude adapter assigns them and maps answers back to the question text Claude expects.
- In the frontend, `contracts.ts` gains the new question fields. `SessionInteractions.tsx` gains:
  - checkboxes for multi-select questions;
  - options plus an "other" text field when `isOther` is set;
  - text-only input when a question has no options.

### 7. Provider-neutral remote host

- A `HostProvider` trait in `crates/orchid-engine/src/host/providers.rs` covers:
  - the runtime for a configuration;
  - its launch environment;
  - capabilities (profile and inventory);
  - optional continuation export and install.
- `CodexConfiguration` becomes `HostProviderConfiguration { id, provider, executable, home }`, with the same JSON, so existing host files load unchanged.
- The Codex implementation moves into `providers/codex/host.rs`. `host.rs` keeps dispatch, worktree commands and transport. The host registers Codex only; any other provider gets the existing explicit error.
- The wire protocol does not change, so `HOST_PROTOCOL_VERSION` stays 2.

### 8. The Claude provider

Claude follows the Codex layout, with one responsibility per file.

`crates/orchid-engine/src/providers/claude/`:

| File                                          | Responsibility                                                                        |
| --------------------------------------------- | ------------------------------------------------------------------------------------- |
| `mod.rs`                                      | Module map and the `claude` provider identity.                                        |
| `launch.rs`                                   | Translates launch intent into arguments and environment (table below).                |
| `connection.rs`                               | JSON lines over the `ProcessSupervisor`; correlates control requests by `request_id`. |
| `events.rs`                                   | Claude messages → normalized events and control records.                              |
| `requests.rs`                                 | Permission prompts and `AskUserQuestion` ↔ `RuntimeRequest` and `control_response`.   |
| `runtime.rs`                                  | `ClaudeRuntime: AgentRuntime`, with one process per invocation, as Codex does.        |
| `fixtures/claude-code-<version>/`, `tests.rs` | Recorded streams and adapter tests.                                                   |

The line framing moves from Codex's connection into `crates/orchid-engine/src/processes/json_lines.rs`, and both providers use it.

Launch translation:

| Orchid intent                             | Claude                                                                                                                                                                                                                                                       |
| ----------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Process                                   | `claude -p --input-format stream-json --output-format stream-json --verbose --permission-prompt-tool stdio`                                                                                                                                                  |
| Start / resume                            | Start with an Orchid-generated `--session-id`; resume with `--resume <id>`.                                                                                                                                                                                  |
| Model, reasoning                          | `--model`, `--effort`                                                                                                                                                                                                                                        |
| Full access                               | `--permission-mode bypassPermissions`                                                                                                                                                                                                                        |
| Unattended approval                       | `--permission-prompts none`                                                                                                                                                                                                                                  |
| Managed MCP servers                       | `--mcp-config` with HTTP servers. The bearer is read from an environment variable in the header; enabled tools are pre-approved with `--allowedTools mcp__<server>__<tool>`. Required servers are checked against the `mcp_servers` status in `system/init`. |
| Native MCP suppression                    | `--strict-mcp-config`                                                                                                                                                                                                                                        |
| Ignoring user rules                       | Limit the loaded setting sources; otherwise report it as unsupported.                                                                                                                                                                                        |
| Trusted workspace, sandbox network access | Nothing to do: print mode has no trust prompt, and full access includes the network.                                                                                                                                                                         |
| Initial prompt prefix                     | Same rendering as Codex, in the first user message.                                                                                                                                                                                                          |
| Invoked skills                            | `/skill-name` for skills Claude discovered itself; Orchid's skills through `read_skill`.                                                                                                                                                                     |
| Setup                                     | `CLAUDE_CONFIG_DIR` and the executable, from launch preparation.                                                                                                                                                                                             |

Event translation:

- `system/init` → runtime context established, with the session ID.
- Assistant text → agent message: commentary, and the last one before `result` becomes final.
- `tool_use` and `tool_result` → tool activity, started and completed, keyed by the tool-use ID.
  - Bash → command.
  - Edit, Write and NotebookEdit → file change.
  - WebSearch and WebFetch → web search.
  - TodoWrite → plan.
  - `mcp__<server>__<tool>` → MCP tool.
  - Anything else → other.
- `result` → usage (input, cached input and output tokens) and completion: success means completed, an error means failed, and after an interrupt it means canceled.
- `can_use_tool` control requests → requests opened.
- Turn start → turn active, with the session ID as thread and the invocation ID as turn.
- Stderr → diagnostic evidence, as for Codex.

Runtime operations:

- **Prepare:** start the process, send `initialize`, return the session ID and working directory.
- **Deliver and start:** write the user message.
- **Steer:** write another user message. The invocation completes after the result for its last message.
- **Respond:** write the `control_response`.
- **Cancel:** send the `interrupt` control request, then cancel through the supervisor after a short grace period, as Codex does.

`src-tauri/src/runtime/providers/claude/`:

| File               | Responsibility                                                                                                                                                                                                                     |
| ------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mod.rs`           | `register()`                                                                                                                                                                                                                       |
| `setups.rs`        | The setups table, default detection, `claude auth status` readiness, and Tauri commands to list, add and remove setups.                                                                                                            |
| `configuration.rs` | `ProviderConfigurationSource`. Models and effort levels come from Claude's `initialize` report; skills from `<folder>/skills` and `<cwd>/.claude/skills`; exposure offers full access only; inventory lists the discovered skills. |
| `launch.rs`        | `ProviderLaunchPreparation`: sets `CLAUDE_CONFIG_DIR` and the executable.                                                                                                                                                          |

Frontend:

- `src/application/agentProviders/claude/descriptor.ts`, registered in `descriptors.ts`.
- `src/infrastructure/agentProviders/claude/claudeSetupClient.ts`.
- `src/features/agentProviders/claude/ClaudeSetupSettings.tsx`.

Claude has no route-settings component.

### 9. Adjacent changes

Included, because this work would otherwise add another copy or leave a shared screen tied to Codex:

- the generic provider map (section 1);
- shared JSON-lines framing (section 8);
- one final-reply reader and named message roles (section 5);
- the Codex launch adapter in its own file (section 2);
- moving `AgentSessionRuntimeGuidance.tsx`, which is Codex home guidance inside the shared session feature, into `src/features/agentProviders/codex/`, where it is composed by provider.

Noticed but not planned:

- splitting `codex/profiles/mod.rs` (8.6k lines), since only a small adapter changes there;
- execution-target inventory grouping profiles by their default route's device only (`execution_targets/inventory.rs:153`);
- the unused `ExecutionConnectionFields.tsx`.

## External options considered

- **Official Agent SDK (TypeScript or Python).** It needs a Node or Python helper on every device. Not adopted; the CLI's stream-json mode gives the same features without it.
- **Community Rust crates** (`claude-agent-sdk-rs` 0.6.4, `cc-sdk` 0.8.1). They are tokio-based and own the child process, which bypasses Orchid's supervisor and Windows job handling. They are also unofficial and trail the CLI. Not adopted: the protocol surface Orchid needs is small, and `serde_json` covers it.
- **Agent Client Protocol** (the `agent-client-protocol` crate, with a Claude adapter). It would give one protocol for both providers, but Claude's adapter is a Node program, and adopting it would replace the working Codex adapter. Not now.
- **`rmcp`** stays the MCP library for Orchid's own servers. Nothing new is needed.

## Removals

- `runtime_registry.rs`, `configuration_registry.rs` and the registry half of `continuation.rs`.
- The single `profile_source`, launch-hook and runtime-profile-source slots in the session application, event adapter and Capability Profile service.
- `NativeProfileLaunchAuthority`, replaced by `ProviderLaunchPreparation`.
- `AgentRuntimeBinding.external_context_id` and the `agent_sessions.external_context_id` column, after migration.
- `src/application/agentProviders/codex/localRoutes.ts` and its test.
- Codex hard-coding in `orchid-engine/src/host.rs`.
- The cross-provider rejection as a user-facing failure; a provider change now uses the history handoff.

## Implementation order

Each stage keeps Codex working.

1. Provider registrations, routing and per-provider launch preparation (sections 1 and 2), including the catalogue key.
2. Native conversations per provider and the history handoff (section 5).
3. The neutral interaction request and question UI (section 6).
4. The provider-neutral remote host (section 7).
5. Provider setups and profiles across providers (sections 3 and 4).
6. The Claude provider (section 8), against recorded fixtures.
7. A live local check with an installed Claude CLI.

## Validation

- Registry and routing tests using the existing minimal test provider.
- Migration tests for native conversations, the catalogue key and Claude setups.
- History handoff tests: a first switch, switching back with missed turns, and a restart without a transfer port.
- Codex request typing tests, with old stored records decoding unchanged, plus frontend question UI tests.
- Claude adapter tests on recorded streams: start, resume, tool activity, a question request, steering, cancel, a failed result.
- Host tests using the Codex host provider.
- The usual suites: focused Rust tests, engine tests, Vitest, lint and `npm run build:frontend`. Linux cloud sessions follow [Linux cloud sessions](../development.md#linux-cloud-sessions).
- The live Claude check runs on a machine with Claude installed and logged in.

## Decisions and assumptions

Choices made in this plan (not yet discussed):

- Rust drives the installed `claude` CLI in stream-json mode, including the control messages the official SDKs use for permission prompts, interrupts and `initialize`.
- Each provider has one `register` function and there is one generic provider map. `ExecutionEndpoints` stays the router consumers ask and keeps its name.
- Where a provider has no continuation port, a conversation whose location changed restarts from the session log. Claude registers no port in this work.
- The history handoff carries your prompts and the final replies, without tool detail, through the existing initial prompt prefix, combined with any caller prefix into one block.
- Setups carry their executable through a provider-written `executable` launch field.
- Model choice across routes is resolved in the backend at send time. A model from another route records a target change to that route.
- On each device, the default route is the one offering the profile's default model, otherwise the first route listed.
- Claude models and effort levels come from its `initialize` report; skills come from a folder scan.
- The request type is typed but keeps its stored field names.
- One storage migration adds native conversations, Claude setups and the route-keyed catalogue.
- The adjacent changes are those listed in section 9.

Discussed but not settled:

- **Harness role access modes.** Roles keep requesting their mode. A Claude route without that mode fails with the existing explicit error, so workflow roles run on Codex routes.
- **Remote devices in the setup list.** Only the local device lists setups for now.
- **Claude CLI details to confirm first against the installed version:**
  - `--session-id`;
  - `--strict-mcp-config`;
  - limiting setting sources;
  - whether `AskUserQuestion` reaches the permission channel under full access;
  - environment variables in `--mcp-config` headers;
  - what the `initialize` response contains.

  Where one is missing, the simplest alternative is used: reading the session ID from `system/init`, reporting the intent as unsupported, or a static model list.

Decided by you:

- Capability Profiles cover both providers, with one setup per provider per device. The route follows from device and model.
- Launch environment and harness setup are provider-specific. Technical Settings lists provider setups per device, covering folder and CLI access, with minimal automated setup like Codex today.
- Lifecycle handles any registered runtime.
- The model catalogue feeds the profiles, and the other screens build from profiles.
- The question UI supports both providers' feature sets.
- No remote Claude runtime yet, but a provider-neutral remote host with Codex moved into its own domain.
- The Agent Session instance is the shell; provider domains supply runtime handlers. The layout stays modular, like Codex.
- Keep it simple: happy path, no workarounds for bugs not yet seen.
- Access modes are a profile setting, and Claude need not offer Codex's set.
- Session IDs are an optimization. A provider switch rebuilds context from the session log, and switching back resumes the old conversation with the later turns appended.
- A provider's first conversation in a session receives the session's initial prompt prefix, stored for this if it is not already; a provider returning to its own conversation does not.
- Personal use only: your own provider credentials.

## Documentation handoff

When implemented, update:

- `agent-provider-integration.md`: the registration points, the "Sessions, instances and conversations" section, and a Claude section;
- `docs/agent-session/README.md`: native conversations per provider;
- `docs/execution-configuration.md`: routes across providers and provider setups.
