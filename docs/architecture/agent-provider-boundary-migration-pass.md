# Agent provider boundary: migration pass

Status: implemented on `feature/claude-support`, 2026-09-25, on top of the boundary foundation commit `95d0448`:

- Stage A (references, native options, model-independent skills, host): commit `7423986`.
- Stage B (launch intent): commit `f5703fb`.
- Stage C (event consumption): the commit that follows `f5703fb`.

See [Implementation notes](#implementation-notes) for where the implementation differs from the plan below and for validation. This pass follows the boundary foundation recorded in [the implementation shape](agent-provider-boundary-implementation-shape.md).

## Purpose

Remove the remaining Codex vocabulary from shared contracts, so that a Claude Agent SDK bridge implements only provider-owned modules and does not change shared session, profile, orchestration, or transcript code. This is Codex migration work. It adds no Claude code and no new product capability.

The pass covers four discussed items:

- typed configuration references;
- neutral launch intent in place of Codex configuration strings;
- normalized event consumption;
- a provider-options envelope in place of `codex_personality`.

It also makes the small remote-host change that typed references force, and one adjacent port cleanup (see A4). One SQLite migration (schema v59) converts stored data. Old event streams are never rewritten.

## Decisions, in review order

### Agent-proposed choices awaiting review

1. **Items 2 and 4 are delivered as one stage (A).** Both change the same types: `RuntimeProfileSnapshot`, `SessionProfile`, `ProfileRoutePolicy` and their frontend DTOs. Both also need the same digest reseal. Delivering them as one stage means one migration and one set of fixture updates. Resulting order: A (references, options, host) → B (launch intent) → C (events).
2. **Orchid's own runtime records get a typed decoder rather than new normalized kinds.** `runtime_request_*`, `session_steering_*`, `runtime_turn_active`, `runtime_process_exit` and `runtime_working_directory_resolved` are Orchid's control vocabulary. They are carried in `raw_payload` with `source = runtime`. A `RuntimeControlRecord` enum encodes and decodes exactly the JSON already stored. No data changes, and old interaction history stays readable. Process-exit evidence comes from the existing `runtime_process_exit` record, so the `runtime_transport == codex_app_server` gate is dropped and old app-server history keeps its evidence. This replaces the earlier idea of a new normalized `ProcessExited` event.
3. **Import provenance comes from the existing `agent_session_imported_turns` table.** It is loaded into invocation history. This replaces the `codex_history_import` marker event and makes a new invocation column unnecessary. New imports stop writing the marker event.
4. **`NormalizedToolActivity` gains a neutral `kind`, and Codex fills tool activity for every tool item.** Today only `mcp_tool_call` carries it. The transcript takes its labels from `kind`, because `tool` is set only for MCP. Records without a `kind` default to `mcp_tool`, since that was the only item type that ever carried tool activity. This is a truthful default, not a compatibility layer.
5. **Remove `mcp_tool_activity_partial`.** It is the only place observation reads Codex `itemType`, and no UI consumes it.
6. **The neutral managed-MCP entry carries name, URL, optional bearer, optional tool list and the required flag.** The startup timeout (10 s), tool timeout (300 s) and auto-approval of managed tools stay Codex constants, because every current caller uses identical values. This revises the earlier proposal, which listed timeouts and approval as neutral fields. Three launch intents replace the other Codex strings: unattended approval, a trusted application workspace, and sandbox network access.
7. **Codex uses one encoding for all managed servers:** thread configuration, which the Harness Engine path already uses, with bearers delivered through the process environment. The test-only CLI runtime reuses the same translator. Risk: the Implementer currently uses process-level `-c` values, including `mcp_servers={}`. Prove the change with `npm run test:codex-app-server` and a live Implementer reporting run. If process-level encoding turns out to be necessary, that stays inside Codex.
8. **Rename the native capability-group sentinels** `codex-profile-mcps` and `codex-profile-skills` to `native-mcps` and `native-skills` in the same migration.
9. **Extract `orchestration/managed_mcp.rs`** from the 1462-line Plan Builder `mcp.rs`. It holds the neutral server builder, the Implementer reporting check and `transport_denial`, which bootstrap and sprint-runner transitions also use. Add `ConversationHarnessProfile::launch_extension()` to replace about twelve duplicated extension literals.
10. **The migration code lives in the Codex provider area** (`runtime/providers/codex/legacy_migration.rs`) and can be deleted once no v58 database remains. It does not verify old digests before resealing, which follows the agreed stance that the data is experimental.
11. **Bump `HOST_PROTOCOL_VERSION` to 2,** because the `Capabilities` response shape changes. Redeploy the host with `deploy-orchid-host.ps1`.
12. **Guards:** an ESLint `no-restricted-syntax` rule on the frontend, which uses existing tooling, and one Rust source-scan test on the desktop side.
13. **The Codex native-launch preparation authority receives the typed reference and rejects other providers.** No per-provider preparation registry is added until a second implementation exists.
14. **Adjacent port cleanup (A4) is included.** `SelectedRuntimeProfileSource` returns `CodexSkillCatalogue`, and shared application code parses `$name` skill mentions. The Claude bridge would have to implement both. The catalogue shape is already neutral, so the change is small. Assumption: included.

### Discussed and still open

- Claude Agent SDK bridge packaging and API mapping. This pass does not depend on them.
- Legacy sessions with neither an execution target nor a pinned profile. They still fall back to the Codex `selected` configuration, but now in a single helper. Resolving them properly belongs to the historical-binding migration in the base plan.

### User-prescribed or approved decisions

- Orchestration produces a provider-unaware list of managed MCP servers. Conversion to native configuration happens only in `providers/<provider>/`. `config_overrides` leaves the shared contract.
- Configuration references become a typed `{provider, configuration_id}`. Stored `native-codex:` strings are converted in the migration, and there is no permanent legacy decoder.
- The transcript pairs rows by normalized item ID and phase. Import and process-exit detection no longer rely on Codex markers. No compatibility layer for old events. A raw-payload guard is added.
- `provider_options` replaces `codex_personality` in the engine snapshot, capability profile and session profile. No contract-version bump and no old-format reader. Capability profiles are migrated in SQLite so that profile setup is preserved. Pinned snapshots are rewritten and their digests recomputed. One migration test replaces old-fixture digest tests.
- The remote host gets a light touch: `Capabilities` uses the neutral reference and `CodexConfiguration` moves under `providers/codex/`. The configuration format, the provider factory and the `"codex"` checks wait for the remote Claude work.
- One migration pass. Standing rules from the base plan still apply: provider subdirectories, no bundled exceptions object, no plugin loader, Codex feature parity only, full access.

## Stage A: configuration identity and native options

### A1. Shared contracts

| Change | Location |
| --- | --- |
| Add `ProviderConfigurationRef { provider, configuration_id }`. `Display` renders `provider/configuration_id`, for labels and opaque keys only. It is never parsed back. | `crates/orchid-engine/src/contracts/provider.rs` |
| `RuntimeProfileSnapshot`: `profile_ref: String` becomes `configuration: ProviderConfigurationRef`. `codex_personality` becomes `provider_options: Option<ProviderNativeOptions>`, meaning provider defaults reported by the configuration source. Validation checks that the options' provider matches the configuration's provider. The contract version stays unchanged. | `crates/orchid-engine/src/configuration/runtime_profile.rs` |
| Move the Codex projection of its environment into a neutral snapshot, from `configuration/native_codex.rs` to `providers/codex/runtime_profile.rs`. It takes a `ProviderConfigurationRef`. `configuration/` then has no Codex import. | engine |
| `SessionProfile`: `runtime_profile_ref` becomes `configuration`, and `codex_personality` becomes `provider_options`, with matching accessors and the same provider-match validation. | `src-tauri/src/execution_configuration/session_profile.rs` |
| `ProfileRoutePolicy.codex_personality` becomes `provider_options`. Validation requires `provider_options.provider == execution.provider`. | `execution_configuration/capability_profile.rs` |
| `NATIVE_MCP_GROUP` and `NATIVE_SKILL_GROUP` constants replace the `codex-profile-*` literals. | `capability_profile.rs`. Consumers: `resolution.rs`, `session_skills.rs`, `service.rs`, `agent_sessions/application/quick_features.rs`, `runtime/providers/codex/configuration/quick_features.rs` |
| `ExecutionBinding::configuration()` and `ExecutionRouteRef::configuration()` return the typed reference. Their stored shape does not change, because they already hold the two parts separately. | `execution_targets/domain.rs` |

### A2. Consumers

- `resolution.rs`: the profile identity check compares typed references. `RuntimeProfileChanged` renders them with `Display`. The personality merge becomes whole-envelope precedence: `route.provider_options.or(runtime.provider_options)`. Codex never writes an empty envelope; inherit means none.
- **One session configuration accessor.** It returns the execution target's configuration, otherwise the pinned profile's configuration. It replaces the four `strip_prefix("native-codex:")` sites: `direct_user.rs` (about lines 249–265), `invocation.rs` (about 463–480), `addressed.rs` (about 163–166) and `agent_sessions/application/quick_features.rs` (about 56–59). The legacy `selected` fallback exists only inside this accessor.
- Registry dispatch: `configurations.source(&reference.provider)?`, then per-provider port methods called with `reference.configuration_id`. The per-provider port keeps plain configuration IDs, because the registry entry already fixes the provider.
- `runtime/providers/codex/configuration/mod.rs`: the three `format!("native-codex:…")` sites build typed references. It sets snapshot `provider_options` from the Codex profile's personality preference through `CodexNativeOptions::encode`.
- `agent_sessions/application/configuration.rs`: launch copies `pinned.provider_options()` directly, with no Codex import. `addressed.rs` rebuilds the original snapshot from the pinned profile's typed fields.
- `harness_engine/service.rs`: `runtime_instance_id` uses the reference's `Display` form. Existing bindings keep their stored strings, because the value is compared only against its own record.
- Codex native-launch preparation (`runtime/providers/codex/profiles/session_binding.rs`): receives the typed reference and rejects any other provider.

### A3. Remote host (light touch)

- Move `CodexConfiguration` from `host.rs` to `crates/orchid-engine/src/providers/codex/host.rs`, as a struct only.
- `HostCommand::Capabilities` builds `ProviderConfigurationRef { provider, configuration_id }` and calls the moved Codex projection. `host.rs` no longer formats `native-codex:`.
- Bump `HOST_PROTOCOL_VERSION` to 2 in `protocol.rs`. Leave the configuration format, the construction in `Host::new` and the `"codex"` dispatch checks unchanged.

### A4. Model-independent skills

User direction (2026-09-25): skill content is not changed. Skills from every configured source are exposed through capability profiles and chat quick actions, and each is delivered to the selected provider in the format that provider expects.

- **Sources.**
  - Provider-native discovery returns the neutral `ProviderSkill` and `ProviderSkillCatalogue` types in engine contracts. Codex's `skills.rs` projects into them.
  - Product skill roots (Orchid and each OTP package) belong to execution configuration as `ProductSkillRoots`. The Codex configuration source no longer holds them, and `skill_roots_for_configuration` leaves the provider port.
- **Exposure.** Shared quick-feature assembly appends product-root skills for any provider. The capability groups are `native-skills`, `orchid-skills` and `otp:<package>:skills`. `$name` is Orchid's mention syntax for every skill. Shared code produces `QuickSkill.invocation_text`.
- **Delivery.**
  - Shared code parses `$name` mentions against the pinned manifest and the provider's native catalogue. It pins newly mentioned native skills, which is the current behavior, and sets `RuntimeLaunchExtension.invoked_skill_ids`.
  - The manifest guidance becomes provider-neutral, because `read_skill` serves every pinned skill.
  - Codex converts invoked skills that its catalogue can load into native skill input items, and no longer parses `$name` itself.
- **Consumers.** Update `session_skills::pin_discovered_skill`, rename `CapabilityProfileService::codex_skills_for_configuration` to `skills_for_configuration`, and change the return type of the Tauri `load_native_profile_skills` command.
- **Left for later** (see Remaining): `configuration_home`, and the auxiliary workspaces under a Codex home.

### A5. Frontend

- `src/application/agentProviders/contracts.ts`: add `ProviderConfigurationRefDto` and `ProviderNativeOptionsDto`.
- `src/application/executionConfiguration/contracts.ts`: `profileRef` and `runtimeProfileRef` become `configuration`, and `codexPersonality` becomes `providerOptions`. Follow the change through `presentation.ts`, `SessionProfileInspector.tsx`, `AgentSessionScreen.tsx` (around line 300), `selectedTargetQuickFeatures.ts`, `application/agentSessions/quickFeatures.ts` and the test fixtures.
- `features/agentProviders/codex/CodexPersonalityField.tsx`: reads and writes `route.providerOptions` through a small Codex encode/decode helper. Inherit writes `null`.
- Create `src/features/agentProviders/index.ts` with an explicit map from provider ID to route-settings component (`{ codex: CodexPersonalityField }`). `CapabilityRouteEditor.tsx` renders the slot and stops importing Codex.
- `CapabilityProfileEditor.tsx`: use the neutral group IDs.

## Stage B: launch intent

### B1. Shared contract (`crates/orchid-engine/src/contracts/runtime.rs`)

- `RuntimeManagedMcpServer { name, url, bearer_token: Option<String>, enabled_tools: Option<Vec<String>>, required: bool }`. `Debug` redacts the bearer. The bearer never appears in errors, launch provenance or persisted configuration; existing leak tests carry over.
- `RuntimeLaunchExtension`:
  - Remove `config_overrides`.
  - Add `approval: RuntimeApprovalIntent { Inherit (default), Unattended }`.
  - Add `trusted_workspace: bool`, meaning the application has authorized this isolated working directory.
  - Add `sandbox_network_access: bool`. It is currently requested only by the Work Unit Implementer reporting continuation.
  - Keep `native_mcp_enabled`; `Some(false)` means suppress.
  - Keep `environment`, now documented as written only by the selected provider's launch preparation (Codex home). Product code must not write it.

### B2. Codex translation (`crates/orchid-engine/src/providers/codex/app_server/configuration.rs`)

One translator turns the extension into native configuration entries plus process-environment additions. It covers:

- managed servers: URL, generated `bearer_token_env_var`, `enabled_tools`, `required`, `default_tools_approval_mode = "approve"`, and the 10 s and 300 s timeouts;
- native MCP suppression;
- `approval_policy = "never"`;
- the project trust override, moved here from `work_unit_execution_harness.rs::workspace_trust_configuration`;
- the two sandbox network keys;
- reasoning.

The app-server applies these through thread configuration. The existing collision check covers all managed servers. The `ignore_user_rules` rule inspection is unchanged. The test-only `runtime/providers/codex/arguments.rs` emits the same entries as `-c` pairs.

### B3. Orchestration

- Create `src-tauri/src/orchestration/managed_mcp.rs`:
  - `managed_server(scope, url, bearer, tools, required) -> RuntimeManagedMcpServer` replaces `CodexMcpInjection`. It keeps the current `scope_<uuid>` naming.
  - An `implementer_reporting` constructor sets `sandbox_network_access`.
  - `is_exact_implementer_reporting(&RuntimeLaunchExtension)` checks semantic fields instead of TOML strings.
  - `transport_denial` moves here.
  - `mcp.rs` keeps only the Plan Builder server.
- `conversation_harness.rs`: `runtime_config_overrides()` becomes `launch_extension()`, carrying unattended approval, reasoning mode and the initial prompt prefix.
- Call sites become `let mut extension = harness.launch_extension(); extension.managed_mcp_servers.push(server);`:
  - `application.rs`, around line 493;
  - `bootstrap_transition.rs`, around lines 1410, 1571 and 1671;
  - `sprint_runner_transition.rs`, around lines 2315, 2717, 2934, 3201, 3211, 4029 and 4482;
  - `work_unit_execution_harness.rs`, lines 628–655.

  In the Implementer package, `mcp_servers={}` becomes `native_mcp_enabled: Some(false)` and the trust string becomes `trusted_workspace: true`. The `injection()` accessors on the managed-invocation traits (`application.rs:50`, `bootstrap_transition.rs:1107`) return the managed server.
- `harness_engine/launch.rs`: proxy entries set `required: true` and no bearer, because the token is in the URL. `reject_caller_mcp_configuration` drops its override-string check.
- Tests: replace TOML-array assertions with semantic assertions. Tests that scrape the URL or bearer from override strings, such as `bootstrap_transition.rs` around line 6437, read the managed server instead. Exact native keys are asserted once, in the Codex translator tests.

## Stage C: event consumption

### C1. Contracts and Codex production

- `contracts/domain.rs`: add `ToolActivityKind { Command, FileChange, WebSearch, Plan, McpTool, Other }` to `NormalizedToolActivity`. Missing values default to `McpTool`.
- Create `contracts/control.rs` with `RuntimeControlRecord`. It uses the serde tag `kind`, with exactly the current names and fields. It provides an event-draft constructor and a decoder for runtime-sourced events. The diagnostic-only `runtime_transport` and `runtime_effective_configuration` records are not modeled.
- `providers/codex/protocol.rs` fills `tool_activity` for `command_execution`, `file_change`, `web_search`, `plan_update` and `mcp_tool_call`. The Codex app-server emits control records through the typed constructors.

### C2. Desktop consumers

- `agent_sessions/interactions.rs`, `application/interactions.rs` and `application/update_sink.rs` (lines 160–210) decode `RuntimeControlRecord` instead of matching `raw_payload["kind"]`.
- `AgentInvocationHistory` (`agent_sessions/ports/repository.rs`) gains `import_provenance: Option<ImportedTurnProvenance>`, loaded from `agent_session_imported_turns`. `repository/import.rs` stops writing the marker event and keeps the `importedContent` details on imported items.
- `application/observation.rs`:
  - Imported invocations are recognized from `import_provenance`.
  - Process terminal comes from the last `RuntimeProcessExit` record, otherwise from the invocation terminal.
  - MCP observations filter on `kind == McpTool`.
  - Remove `mcp_tool_activity_partial` here and in its DTO.
- Guard: one test scans `src-tauri/src` outside `runtime/providers/` and test files for `raw_payload[`, `raw_payload.get(` and `raw_payload.as_`. It fails on any match.

### C3. Frontend

- `application/agentSessions/contracts.ts`: add `toolActivity.kind` and `importProvenance` on invocations. Remove `mcpToolActivityPartial`, including from the fixtures in `src/dev/`.
- Create `application/agentSessions/runtimeControlRecords.ts`, which decodes the runtime-sourced kinds the UI uses. `sessionAttention.ts` uses it.
- `transcriptProjector.ts`:
  - Pair start and completion rows by `toolActivity.itemId` and `phase`.
  - Take the label from `toolActivity.kind`, plus `tool` for MCP.
  - Delete `lifecycleIdentity`, `toolLabelFromActivity` and the `details.itemType` reader.
  - Move the stderr text extraction (`technicalLabel`) into `features/agentSessions/runtimeDiagnostics.ts`.
- `importedTranscript.ts`: the ordinal and source start time come from `importProvenance`.
- `eslint.config.js`: `no-restricted-syntax` on `rawPayload` member reads. Exempt `runtimeControlRecords.ts`, `runtimeDiagnostics.ts`, tests, `src/dev/` and `**/agentProviders/**`. Raw evidence passed to the diagnostics view travels with the event object rather than being read field by field.
- Accepted degradation: older non-MCP tool rows are not paired and lose their Codex item-type label. Their text is still shown.

## Migration (schema v59)

`src-tauri/src/storage.rs` bumps `ACTIVE_SCHEMA_VERSION` to 59 and, when upgrading from 58 or earlier, calls `runtime/providers/codex/legacy_migration.rs` inside the existing evolution transaction. The migration:

1. Rewrites capability profiles (`execution_capability_profiles.profile_json`):
   - `routePolicies[].codexPersonality` becomes `providerOptions: {provider: "codex", settings: {personality}}`; absent or null stays absent.
   - Group IDs in `mcpGroups` and `skillGroups` are renamed.
2. Rewrites stored session-creation resolutions in `agent_sessions.session_profile_json`, `agent_session_current_execution.resolution_json` and `agent_session_preparations.payload_json.currentResolution`:
   - `sessionProfile.runtimeProfileRef` becomes `configuration`: `native-codex:<id>` maps to `{codex, <id>}`, and any other string maps to `{codex, <string>}`.
   - `codexPersonality` becomes `providerOptions`.
   - The resolution is resealed through a new `SessionCreationResolution::reseal` in `resolution.rs`, which validates it and computes the digest. The migration records a map from old digest to new digest.
3. Rewrites `agent_session_preparations.payload_json.resolution.sessionProfileDigest` using that map.

The migration never touches event rows.

Test: extend `src-tauri/src/storage/session_migration_tests.rs` with seeded v58 rows. Seed a capability profile with a personality and Codex group IDs, a pinned session with a `native-codex:` reference and a personality, and a preparation carrying the old digest. Assert that the reopened rows deserialize, that `verify_digest()` passes, and that the digest reference in the preparation matches. This replaces the old-fixture digest tests.

## Validation

For each stage, run the focused Rust and Vitest suites for the touched modules.

At the end:

- `npm run check:rust`, `npm run test:rust:fast`, and `cargo test --manifest-path crates/orchid-engine/Cargo.toml`;
- `npm run build:frontend`, `npm test` and `npm run lint`;
- `npm run test:codex-app-server`, which is required because Stage B changes the Codex encoding.

Live checks:

- an ordinary session with and without a route personality override;
- a Plan Builder or sprint-runner flow using managed MCP;
- a Work Unit Implementer reporting run (workspace write, sandbox network, suppressed native MCP);
- an imported Codex session view;
- a restart with a pending request;
- after redeploying the host, `scripts/remote-host-smoke.mjs` against the SSH device.

Audit remaining Codex references:

- `rg "native-codex:|codex_personality|codexPersonality|config_overrides|CodexMcpInjection|codex_history_import|codex-profile-(mcps|skills)"` should match only `providers/codex/`, the migration, and migration test fixtures.
- `rg "raw_payload\[|rawPayload" src src-tauri/src` should match only allowed files.

## Remaining after this pass

These are not part of this pass:

- Remote host provider factory, neutral host configuration entries, and removal of the `"codex"` checks. This belongs to the remote Claude work.
- Retirement of the test-only `CodexCliRuntime`, after live coverage moves off it.
- Auxiliary workspaces under the Codex home through `configuration_home`.
- The single configuration source in `CapabilityProfileService`, and a model catalogue cache keyed by configuration ID alone. Both must become provider-addressed when a second provider registers.
- Destination-instance paths and the historical binding migration from the base plan.
- The base plan's live desktop, restart, import, interaction and SSH checks.

## Implementation notes

Differences from the plan above, and the facts behind them:

- **Codex encoding (choice 7).** Managed MCP servers moved to thread configuration for every caller, with bearers in the process environment. A probe against the real `codex-cli 0.154.0` app-server confirmed that the MCP server received `Authorization: Bearer <token>` and that `approval_policy` is honored in thread configuration. Workspace-write resolves to read-only on this Windows machine with either encoding, so the effect of the trust and network keys could not be observed locally. Approval, project trust and sandbox network access therefore keep their existing process-level `-c` values. Only Codex code would change if that is revisited.
- **Skills.** Product skill roots moved out of the Codex configuration source into `ProductSkillRoots`, and shared quick-feature assembly adds them for every provider. `$name` resolution moved into `execution_configuration::skill_mentions`, and Codex converts `invoked_skill_ids` rather than parsing text.
  - The manifest guidance wording is now provider-neutral: "A $name mention refers to the skill with that name; read its instructions with read_skill unless they are already attached". Skill files and Harness skill guidance are unchanged.
  - A profiled Session now invokes only skills its manifest pinned. The old direct-send path already dropped ad hoc native additions for pinned Sessions, while the preparation path kept them; both paths now follow the pinned policy.
- **Import provenance.** New imports no longer write the `codex_history_import` marker event. The imported-turn table is the provenance, and it is exposed on invocation history and the invocation DTO.
- **Old records.** Process-exit evidence still reads from existing `runtime_process_exit` records, so older app-server history keeps it. Older non-MCP tool rows are not paired and lose their item-type label. Older MCP tool activity defaults to `kind: mcp_tool`.
- **Adjacent fix.** The shared session target dialog showed the raw provider ID after the foundation commit. It now uses the existing `AgentProviderDescriptor` through `agentProviderLabel`, which also fixed a failing frontend test.

Validation on 2026-09-25:

| Check | Result |
| --- | --- |
| `npm run test:rust:fast` | 760 passed, 1 ignored |
| `cargo test --manifest-path crates/orchid-engine/Cargo.toml` | 58 passed |
| `npm test` | 768 passed |
| `npm run lint` | No errors; 7 pre-existing warnings |
| `npm run build:frontend` | Passed |
| `npm run check:rust -- --all-targets --features live-tests` | Passed |
| `npm run test:codex-app-server` with `codex-cli 0.154.0` | 2 passed |

The live desktop checks and the host redeployment and SSH smoke test were not run.

## Documentation handoff

After implementation:

- Update the provider guide (`agent-provider-integration.md`) with the typed reference, launch intents, control records, tool-activity kinds, the route-settings slot, and the neutral skill port.
- Update the status list in the base plan.
- Update `docs/execution-configuration.md` for `providerOptions` and the native group IDs.
