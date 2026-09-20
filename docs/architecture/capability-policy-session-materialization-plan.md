# Capability Policy and Session Materialization

Status: proposed implementation shape, 2026-09-20.

## Purpose and scope

Replace the current single-binding, flat-capability Capability Profile with a reusable policy over one or more execution routes. At session creation, resolve that policy with the selected route's live catalogue and the Workflow node's narrower selection into a persisted session manifest. The manifest, rather than a mutable Codex home, determines the MCP tools and skills presented to the Codex runtime.

This is a Capability Profile and session-launch slice. It deliberately does not reopen remote-device control, SSH UI configuration, Composer drafts/tab restoration, Codex-profile registration, or the later sandbox design. New native Codex launches remain fixed to `danger-full-access` until that later work.

## Decisions

- A Capability Profile owns a list of **route policies**. A route is an existing Device -> Harness -> Inference Source binding. Exactly one route is the default; a Session may choose any permitted route and pins that choice when created.
- Every route policy owns its compatible model policies and its MCP/Skill group grants. This avoids inventing one capability catalogue for two different Codex configurations.
- Capability Profile IDs, route IDs, configuration references, source paths, digests, and native MCP server keys remain transport/domain identity only. A new profile gets its UUID when first saved; no profile ID is rendered in the UI.
- The route catalogue is a shared backend projection consumed by Technical Settings and Capability Profiles. It initially projects the local Codex profiles that already exist; it does not add an SSH connection or remote-device editor.
- A model policy is `{ modelId, minimumReasoning, maximumReasoning }`. The range is valid only when every included level is supported by that model on that route. Model rank and reasoning order are data owned by the runtime catalogue, not alphabetical UI rules.
- Capability Profiles grant groups only: one group per OTP and one native-Codex MCP group; one Codex-profile skill-root group, one Orchid skill-root group, and one group per OTP skill root. Node configuration may narrow a permitted group to individual canonical tool or skill IDs, but may never add one.
- The resolver persists a `SessionCapabilityManifest` containing the exact route, catalogue fingerprint, model policy/defaults, selected MCP tools, selected skills, source paths, and content/version fingerprints. A changed skill or missing tool fails with a structured capability-resolution error; it does not silently alter an existing Session. Creating a new Session or explicitly refreshing a Session creates a new manifest.
- One existing Harness Engine sidecar remains the broker. Its virtual, token-bound endpoints may have different per-session masks, but no per-session or per-upstream child sidecars are introduced.
- Orchid and OTP skills are delivered as explicit Codex `skill` input items on each `turn/start`, not through extra skill roots, copied folders, symbolic links, global `CODEX_HOME` edits, or `skills/config` mutations. The current Codex App Server documents a `skill` input with a name and `SKILL.md` path for this purpose. Native Codex-root skills remain only those explicitly selected in the manifest; Orchid does not ask Codex to discover an unrestricted additional root.
- Native-Codex MCP configuration can be switched at a server/group boundary through ephemeral invocation configuration. Exact per-tool node masking is enforced by the broker for Orchid/OTP upstreams that it owns. Do not claim an equivalent exact mask for a native Codex server until that server can be brokered without adding the rejected child-sidecar design; the first slice reports this limitation and keeps those sources group-level.

The final point is intentionally explicit: Codex's native configuration exposes MCP servers, while Orchid's present proxy can mask tools only for upstreams it proxies. Pretending a `tools/list` filter alone is authorization would be unsafe.

## Resulting boundaries

```text
Technical Settings
  ExecutionRouteCatalog (device, harness, inference-source relationships)
        |
Capability Profile
  route policy + model ranges + broad MCP/Skill group grants
        |
Workflow node
  narrower canonical model/tool/skill selection and defaults
        |
SessionCapabilityResolver
  live route catalogue ∩ profile policy ∩ node subset
        |
persisted SessionCapabilityManifest
        |                           |
shared Harness Engine broker       Codex App Server turn/start input
token-bound masked MCP endpoints  explicit selected skill items
```

`execution_configuration` owns policy, catalogue projection, validation, and manifest resolution. `agent_sessions` owns when a manifest is created, pinned, and used for every turn. `harness_engine` owns only the broker registration and authorization of the resolved MCP leases. `orchid-engine` owns provider-neutral invocation inputs; its Codex adapter serializes them into the app-server protocol. The frontend renders DTOs and never assembles configuration overrides, paths, or masks itself.

## Backend shape

### Catalogue and policy domain

Create focused modules under `src-tauri/src/execution_configuration/`:

- `route_catalog.rs` projects `ExecutionEndpoints` and registered native Codex profiles into device, harness, and inference-source descriptors plus valid route triples. It replaces the frontend-only `localHarnessRoutes` projection and is also the read model for Technical Settings.
- `capability_catalog.rs` describes one observed route: ranked models with their supported reasoning levels, MCP groups and canonical members, Skill groups and canonical members, source provenance, availability, and a stable catalogue fingerprint. It composes the native Codex reader, Orchid workspace root, and OTP catalogue; failed discovery is a typed observation, not an empty successful catalogue.
- `capability_policy.rs` contains `ProfileRoutePolicy`, `ModelAllowance`, `CapabilityGroupGrant`, and `NodeCapabilitySubset`. It replaces the overloaded flat `CapabilitySet` for authoring policy. Sandbox values leave this type entirely for this slice.
- `session_manifest.rs` contains the immutable resolved material: selected binding/route, model and reasoning choices, brokered MCP leases, native group-level server choices, and `SelectedSkill { canonical_id, name, source_path, fingerprint }` records. It owns digest generation and fingerprint revalidation.

Refactor `CapabilityProfile` to persist `route_policies` and `default_route_id` rather than one `execution`, global model list, global reasoning list, flat MCP map, and flat skills list. Require at least one route at save time; removing the default promotes another route before save. `CapabilityProfileService` creates the ID internally with the existing `uuid` dependency, observes each configured route, validates its model range/group grants against its catalogue, and increments revisions as it does today.

Refactor `NodeProfile` from an independent `CapabilitySet` to `NodeCapabilitySubset` plus model/default selections. The subset contains canonical selected tool/skill IDs and model IDs and is validated against the attached route policy at resolution. Keep OTP-specific tool arguments in the separate Workflow configuration, but attach them to the selected canonical MCP upstream only after the subset has passed validation.

`SessionProfileResolver` becomes a composition point, not a place that mutates sandbox fields or independently re-derives capabilities. It receives the selected route binding and a route-specific `CapabilityCatalog`, computes:

```text
route catalogue ∩ profile group grants/model ranges ∩ node subset
```

and emits the manifest plus effective model/reasoning defaults. It must produce individual error kinds for unavailable route, stale catalogue, a profile grant outside the route, a node item outside the profile, a disappeared selected tool, and changed skill content. The resolver no longer calls `force_temporary_sandbox_default`; the fixed native launch default belongs to launch preparation only.

Keep the old schema parser only for one-time storage migration and historical Session decoding. Migrate existing profiles by preserving name/default and turning their single execution binding into one route policy. Do not guess MCP or Skill group provenance from old flat values: migrate them to no group grant and mark the profile as requiring review before it can create a new Session. Historical Session records retain their stored legacy execution facts; they are neither edited nor silently upgraded into a different capability policy.

### MCP launch and enforcement

Extend the existing `HarnessMediationPlan`, `SidecarBindingRegistration`, and `RuntimeLaunchExtension` with resolved MCP lease data rather than teaching callers to assemble raw `-c` arguments. Reuse the existing loopback, token-bound proxy URL flow in `harness_engine/launch.rs`.

In `harness_engine/proxy.rs`, use the manifest lease to do both of the following before forwarding:

- filter `tools/list` (including streamed results); and
- reject `tools/call` whose canonical tool is not leased for that token, before the upstream receives it.

The authorization test must attempt a hidden tool directly after `tools/list`; it is not sufficient that it was absent from the list. Registration cleanup remains tied to the existing Session/invocation lifecycle.

Add a structured native-Codex MCP policy to the launch extension and make `crates/orchid-engine/src/codex/app_server/configuration.rs` serialize only the named server enable/disable changes. Do not use `mcp_servers={}` as a blanket replacement for a user's Codex configuration, and do not write `config.toml`. Existing managed Orchid/OTP MCP servers continue to be injected as loopback URLs.

### Skill delivery

Replace `RuntimeLaunchExtension.skill_roots` and the no-op `capability_roots::apply_skill_roots` path with a provider-neutral supplemental-input type in `crates/orchid-engine/src/contracts/runtime.rs`, for example `RuntimeSupplementalInput::Skill { id, name, path, fingerprint }`.

`agent_sessions` reads the pinned manifest before start and resume, revalidates selected source files, and adds its supplemental skill inputs to every invocation. `CodexAppServerRuntime::initialize_turn` builds the `turn/start.input` array in a stable order: selected skill items followed by product context (when present) and the persisted user text. It never sends `skills/extraRoots/set` and never changes the configured Codex skill roots. A direct user message stays the durable user message; the supplemental items are launch material, not a rewritten draft.

The App Server adapter test fixture should assert the exact ordered payload for fresh and resumed sessions. It should also cover an altered `SKILL.md`: no `turn/start` is sent and the user receives a bounded resolution error that identifies the changed capability without exposing arbitrary filesystem contents.

## Transport and persistence

Update the Tauri and TypeScript contracts together:

- `src/application/executionConfiguration/contracts.ts` gains route-catalogue, group-catalogue, model-descriptor, profile route-policy, and manifest DTOs. `CreateCapabilityProfileInput` omits `capabilityProfileId`.
- `src-tauri/src/execution_configuration/transport.rs` generates profile identity on create and exposes route/capability-catalogue reads. It accepts route/group/model policy, not raw MCP configuration values or filesystem paths from the browser.
- `src-tauri/src/execution_configuration/repository.rs` receives a one-time versioned JSON migration plus storage for the new profile and pinned manifest version. The existing `rusqlite`, `uuid`, and `sha2` dependencies already cover persistence, identity, and fingerprints; no new library is warranted.
- Agent Session repository/transport records receive the manifest/digest alongside the current session profile resolution. Preparation reloads this durable value rather than recomputing from mutable profile state after the Session is committed.

## Frontend shape

Split the present `CapabilityProfileEditor` into composable, policy-specific components under `src/features/executionConfiguration/`:

- `CapabilityProfileEmptyState.tsx` is rendered on initial entry when no profile is selected. It contains only selection/create guidance; it never renders the editor's fields.
- `CapabilityProfileRouteList.tsx` lists the profile's routes with device, harness, and inference-source labels. It identifies the default. Keyboard focus and hover reveal the accessible **Set default** and remove controls.
- `AddCapabilityRouteDialog.tsx` uses the shared route catalogue. Device, harness, and inference-source selectors narrow one another; it returns a valid route triple, not an ad-hoc address or SSH value.
- `RouteCapabilityPolicyEditor.tsx` owns the selected route's models, group toggles, and defaults. This is the details area after a route is selected from the list.
- `ModelAllowanceList.tsx` orders models by runtime strength and uses an ordered minimum-to-maximum reasoning range control. **Add model** opens a modal containing only not-yet-added observed models. Row removal is hover/focus accessible. Default model and reasoning selectors are filtered to the allowed pairs and present reasoning lowest-first.
- `CapabilityGroupToggles.tsx` renders separate **MCP tools** and **Skills** sections. Each group has provenance and a concise statement that selection is broad policy; actual session delivery is resolved at creation. It does not expose a field called Runtime profile, raw paths, runtime reference, locks, sandbox modes, or profile IDs.

`ExecutionConfigurationScreen.tsx` owns selection/new-profile state and loading of the shared route catalogue. Entering the screen with no selection displays the empty state. Choosing **New profile** deliberately enters an unsaved editor; only its first successful save receives an internal ID. The profile-list default indicator remains in the list, not duplicated in details.

Remove `RuntimeProfileInspector` from Capability Profiles and Workflow node editing, retire `HarnessInferenceRouteFields`, and replace `CapabilitySetFields`, `OtpMcpToolsPicker`, and the generic `RuntimeDefaultsFields` use in this flow. Keep generic presentation helpers only where a different feature still consumes them; do not leave a second Capability Profile policy editor behind.

The Workflow node editor changes from generic "Exposed capabilities" to a subset picker sourced from the selected profile route catalogue. It shows individual selected tools/skills only after a route/profile is chosen, and provides a clear unavailable/stale state instead of presenting empty dropdowns as real capability choices.

## Concrete change map

| Action | Files | Result |
| --- | --- | --- |
| Create | `src-tauri/src/execution_configuration/{route_catalog,capability_catalog,capability_policy,session_manifest}.rs` | Explicit route, catalogue, policy, and immutable-manifest ownership. |
| Refactor | `capability_profile.rs`, `node_profile.rs`, `resolution.rs`, `session_profile.rs`, `service.rs`, `repository.rs`, `transport.rs`, `mod.rs` | Versioned multi-route policy and resolver replace the flat capability model. |
| Adapt | `src-tauri/src/execution_targets/`, `src-tauri/src/active_app.rs`, `src/application/executionTargets/contracts.ts` | Shared device/harness/inference route projection powers both settings screens. |
| Adapt | `src-tauri/src/agent_sessions/application/{preparation/execution,invocation}.rs` and Session persistence/transport | Persist, revalidate, and deliver manifest material for every invocation. |
| Adapt | `src-tauri/src/harness_engine/{domain,exposure,launch,proxy,service}.rs` | One broker gains token-bound per-session tool enforcement. |
| Replace | `crates/orchid-engine/src/contracts/runtime.rs`, `src/codex/app_server/{mod,capability_roots,configuration}.rs` | Explicit skill input and structured native-MCP policy replace root injection/raw overrides. |
| Replace/create | `src/features/executionConfiguration/{CapabilityProfileEditor,ExecutionConfigurationScreen,types,presentation}.tsx`, new route/model/group components and tests | Empty-state-first, no internal IDs, route list/modal, model ranges, and group toggles. |
| Remove | `HarnessInferenceRouteFields.tsx`, `RuntimeProfileInspector.tsx` from this flow, obsolete flat-capability editor paths/tests | One visible policy model, no misleading runtime/sandbox editor. |

## Validation

- Persistence: create generates a non-rendered profile ID; save/reopen preserves ordered route policies/default route; migration preserves existing profiles safely but blocks unreviewed legacy tool/skill policy from launching a new Session.
- Catalogue/UI: no profile selection shows only the empty state; add-route filtering produces valid triples; a non-default row can become default; the last route cannot leave a saveable profile; no Runtime profile, profile ID, sandbox, or raw source path appears.
- Models: the observed route list is complete rather than a hand-written subset; Astra, Sol, Terra, Luna, and 5.5 appear when the selected runtime reports them; strength/reasoning ordering and unsupported-range rejection are covered; model defaults cannot name a disallowed pair.
- Groups/subsets: profile grants only groups; node subsets cannot widen them; unavailable route/catalogue states are explicit; each resolved manifest is stable and digest-verified.
- MCP: one broker serves two distinct Session tokens with different `tools/list` results; a direct hidden `tools/call` is rejected before upstream; token reuse and post-cleanup calls fail; selected native Codex groups change only ephemeral per-invocation configuration.
- Skills: chosen Codex/Orchid/OTP skills appear as ordered explicit `skill` inputs for first and resumed turns; no root-setting request or filesystem copy occurs; unselected roots are absent; changed content stops launch with a structured error.
- Regression: focused Rust resolver/repository/harness/app-server/Agent Session tests plus focused Vitest component tests pass, then `npm run build:frontend` and `npm run check:rust`. Inspect the running desktop flow at normal width for the list, modal, hover/focus controls, and empty state.

## Deferred

- Exact per-tool masking for arbitrary native-Codex MCP server configurations that Orchid cannot broker yet.
- Multi-device coordinator/control-plane networking, Android, and any SSH user interface.
- Capability Profile sandbox modes, approvals, health/canary setup, and safer default policy.
- Composer draft persistence and tab-return work, which remains covered by the existing device/harness/inference and Composer plan.
