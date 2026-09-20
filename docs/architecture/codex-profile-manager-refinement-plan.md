# Codex Profile Manager Refinement

Status: proposed implementation shape, 2026-09-20.

## Target

Turn **Local Codex homes** into a readable local **Codex profiles** manager. A Codex profile is one registered local `CODEX_HOME` folder: it supplies Codex configuration and sign-in state, but does not own Orchid's model, inference, sandbox, capability, or remote-device policy.

```text
Codex profile: registered local CODEX_HOME + login
  -> local Codex CLI harness: Orchid launch mechanism on this device
       -> inference source: OpenAI through that harness
            -> Capability Profile: later policy owner
```

The normal screen is a master-detail view: the registered profile list is the primary navigation; the selected profile's concise details fill the rest of the screen. Setup diagnostics appear only inside the modal operation that produced them.

## Decisions and assumptions

- Rename the product term to **Codex profile**. Keep `native profile` only as an implementation name while the larger Device/Harness catalogue migration is pending.
- A profile's stable database ID and filesystem identity remain internal: they protect continuity and existing Session bindings but are not displayed.
- The profile list owns the **Default** indicator and default-selection action. The detail pane does not repeat it.
- Render a conventional Windows path (`C:\…`), not Rust's canonical `\\?\…` path. Preserve the canonical path internally for equality and continuity checks.
- Adding a profile happens in an **Add Codex profile** modal. It offers an address field, a nested discovered-folder picker, and a *Create new Codex home folder* toggle that disables the address field.
- Profile health is a read-only, visible operation: continuity, CLI resolution/surface, login status, and runtime inventory. It must not initialise a sandbox, start a canary, request a browser, or mutate profile configuration.
- Login verification is a visible operation. It offers browser login only after an unauthenticated result. It is unavailable when the last checked result is authenticated.
- **Immediate simplification:** new Orchid native-Codex launches use `danger-full-access` with unrestricted network by default. Remove profile-level mode selection, separate Orchid danger authorisation, sandbox setup/adoption, canaries, and the private MCP reporting probe.
- This is intentionally a temporary, unsafe policy. The later Capability Profile sandbox design will replace the fixed launch default with allowed modes and defaults. Existing historical Session/invocation snapshots retain their recorded mode; new launches do not honour old profile-level sandbox state.
- Reuse `src/components/ModalDialog.tsx`; no new dialog dependency is warranted.

## Ownership after the change

| Owner | Responsibility | Does not own |
| --- | --- | --- |
| `native_profiles` | Register/create/select a local Codex profile; retain continuity; run health and login operations | sandbox policy, capability policy, provider credentials, model choice |
| `execution_configuration/native_codex` | Inspect the native runtime and expose the discovered harness tool inventory for a specified profile | profile registration or browser login |
| Capability Profiles | Future reusable runtime policy, including sandbox choices | CODEX_HOME registration, sign-in, SSH, device secrets |
| Agent Sessions | Persist the resolved profile reference and historical launch facts | mutable profile configuration |

## Backend shape

Refactor `src-tauri/src/native_profiles.rs` into focused modules under `src-tauri/src/native_profiles/`:

- `repository.rs`: registered profile rows, default selection, canonical path/identity continuity, and the migration from the existing schema.
- `service.rs`: register, create, select, check-health, and request-login orchestration.
- `health.rs`: one structured, read-only health result. It converts raw CLI/configuration errors into step results suitable for the progress modal; raw diagnostics remain operation-local.
- Retain `discovery.rs` for the bounded discovery sources (`CODEX_HOME`, `.codex`, `.codex-*`, existing registrations), but return display-normalised paths.
- Retain `session_binding.rs` only for profile identity/home binding; remove its execution-mode payload.
- Delete the reporting-server and sandbox/canary implementation from the current monolithic file.

Define a small profile query DTO for list/detail consumption:

- `id` (transport-only), display path, ownership, lifecycle, `isDefault`, and last login/health summary.
- No filesystem identity, danger authority, setup attempts, canary state, raw attention facts, or MCP-probe state.

Add explicit commands for `check_native_profile_health`, `request_native_profile_login`, `register_native_profile`, `create_dedicated_native_profile`, `discover_native_codex_homes`, and `select_native_profile`. Health and login must accept any active profile; selecting a default must not be a prerequisite for signing it in.

Move the tool inventory query to `execution_configuration/native_codex.rs`, where the Codex runtime reader already owns configuration inspection. Expose it through a narrow profile-ID command and return only tools/capabilities actually reported by the installed CLI/configuration: filesystem scope, shell, skills, and MCP server/tool names when observable. Do not turn the old Orchid reporting handshake into a claimed Codex capability.

Replace the current profile-level execution mode projection with a single native-launch policy in the launch authority: `danger-full-access`, unrestricted network, and no per-profile authorisation receipt. Update all native-Codex new-launch resolution paths to use this one policy, including configured runtime defaults and Agent Session launch preparation. Do not rewrite historical Session/invocation records.

Use an active-database migration to remove the now-dead execution-mode, danger-authorisation, sandbox-adoption/setup-attempt, canary, and profile-reporting records/tables after preserving the registered-profile and session-binding tables. Rebuild the readiness projection into a compact health/login state rather than retaining unused columns as a hidden second configuration model.

## Frontend shape

Replace `src/features/nativeProfiles/NativeProfileSettings.tsx` with a small feature area:

- `CodexProfilesScreen.tsx`: loads the list and owns master-detail selection.
- `CodexProfileList.tsx`: profile rows, the Default column, and **Add Codex profile**.
- `CodexProfileDetail.tsx`: path with system-options menu, ownership/lifecycle summary, health, login, and Harness-provided tools.
- `AddCodexProfileDialog.tsx`: address/create toggle and the nested discovery picker.
- `ProfileHealthDialog.tsx` and `ProfileLoginDialog.tsx`: visible step progress and concise completion/error outcomes. They own raw diagnostics only for the active operation.
- `codexProfiles.css`: master-detail layout and practical minimum widths; remove the wrapping action-strip/card styling.

The path menu invokes Windows Explorer through the existing Tauri shell/open capability, not a custom explorer. The platform adapter receives the canonical path, while the label is normalised for display.

`TechnicalSettingsScreen.tsx` changes its tab label from **Local Codex homes** to **Codex profiles** and mounts the new screen. `ExecutionSetupOverview.tsx` retains its device/harness projection but changes its copy to use “Codex profile”; it must not reintroduce profile setup actions.

Remove sandbox selectors from the native-Codex route and current Capability Profile editing paths for this interim. Show neither a false per-profile setting nor a deferred setting that currently does nothing. The generic Conversation Harness editor is outside this slice; leave it intact until the later Capability Profile sandbox project gives it one policy contract.

## Concrete changes

| Action | Files | Outcome |
| --- | --- | --- |
| Replace | `src/features/nativeProfiles/NativeProfileSettings.tsx`, `nativeProfileSettings.css` | Master-detail manager replaces registration fields, cards, and button strip. |
| Create | `src/features/nativeProfiles/CodexProfile{List,Detail}.tsx`, `AddCodexProfileDialog.tsx`, `Profile{Health,Login}Dialog.tsx`, `codexProfiles.css` | Independently editable list, details, and guided operations. |
| Reuse | `src/components/ModalDialog.tsx` | Native modal/focus behaviour for all new dialogs. |
| Adapt | `src/infrastructure/nativeProfiles/nativeProfileClient.ts` | Compact DTOs and five user-facing actions; remove sandbox/canary/MCP client methods. |
| Split/remove | `src-tauri/src/native_profiles.rs`, `src-tauri/src/native_profiles/{session_binding,readiness}.rs` | Preserve profile/session continuity, delete per-profile sandbox, canary, and reporting machinery. |
| Create/adapt | `src-tauri/src/native_profiles/{repository,service,health}.rs`, `src-tauri/src/storage.rs`, `src-tauri/src/active_app.rs` | Compact durable profile state, schema migration, health/login transport registration. |
| Adapt | `src-tauri/src/execution_configuration/{native_codex,configured_runtime,capability_profile,service,transport}.rs` | Profile-specific harness inventory; fixed danger-full-access interim launch default; no present sandbox editor. |
| Adapt | `src-tauri/src/agent_sessions/{application,transport,repository}/` and `src/application/agentSessions/` | New launches get the interim default; historical resolved facts remain unchanged. |
| Adapt | `src/features/{technicalSettings,executionConfiguration,agentSessions}/` | New terminology and removal of sandbox choices from active native-Codex profile/session flow. |
| Delete/rewrite tests | `src/features/nativeProfiles/NativeProfileSettings.test.tsx`, native profile/service tests, execution-configuration/editor tests | Remove assertions for retired controls and cover the new flows. |

## Validation

- UI: registered profiles appear in a readable list; the default column changes only through the list; selecting a row preserves its detail pane; no internal ID or `\\?\` path is visible.
- Add flow: manual registration, nested discovery selection, and dedicated-folder creation each produce a profile; creating a home never sends the disabled address value.
- Health/login: each modal shows its ordered steps; health is read-only; an unauthenticated result offers browser login; a verified login disables the redundant action.
- Tools: only reported native runtime tools are shown. Missing inventory is an actionable, bounded result, not an invented capability or raw protocol dump.
- Migration: registered paths/default/session bindings survive; retired per-profile sandbox data is no longer queried, written, or exposed.
- Launch: a new native-Codex Agent Session uses the fixed danger-full-access policy; historical session and invocation records retain their original resolved facts.
- Regression: focused Vitest and Rust suites for native profiles, execution configuration, and Agent Session launch pass; then build the frontend and inspect the running UI at desktop width.

## Deferred

- Capability Profile-owned sandbox modes, allowed capability ceilings, defaults, and per-session override semantics.
- Any restoration of safer defaults or an approval workflow.
- Device Agent/Coordinator work, remote connectivity, account editing, and credential copying.

