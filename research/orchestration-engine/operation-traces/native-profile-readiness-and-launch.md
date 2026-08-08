# Operation trace: Native Profile readiness and launch authority

## Purpose

Native Profiles provide application-owned control over a Codex home, its filesystem continuity, authentication, sandbox readiness, execution mode, canary evidence and MCP reporting. The implementation deliberately treats these as separate facts rather than a single “ready” flag.

The visible settings capability is productive. It was not connected to ordinary Agent Session launch at the initial `b28137b` anchor. Current inspected tip `9240364` connects selected, fully ready profile identity to every release-composed Agent Session launch through the shared Rust application boundary.

## Registration and selection

The Technical Settings frontend calls Tauri commands in `native_profiles.rs` through `nativeProfileClient.ts`. A profile can be:

- a registered existing `CODEX_HOME`;
- an application-dedicated home under `native-codex-homes/`.

SQLite stores canonical path, filesystem identity, ownership, lifecycle and a single selected profile. Selection does not itself prove continuity or readiness. Later operations call `require_selected_active` and revalidate the filesystem identity.

## Readiness dimensions

The product projects at least these independent dimensions:

- lifecycle/continuity;
- authentication;
- sandbox initialization;
- WorkspaceWrite canary;
- Danger Full Access canary;
- MCP reporting;
- execution-mode selection;
- exact danger authorization;
- structured attention.

Login, setup, canary and MCP work have separate attempt tables so request, launch acceptance, terminal observation, receipt and final readiness are not conflated.

## Login and authentication

`request_native_profile_login` creates a durable login attempt and launches the native Codex login process. Browser handoff is explicitly unobserved. A returned request, accepted child launch, terminal process result and later authentication observation remain different facts.

`refresh_native_profile_readiness` performs safe status reconciliation and updates authentication without claiming browser behavior or provider workflow completion.

## Sandbox initialization and WorkspaceWrite canary

The product can request Windows sandbox setup through the supported Codex sandbox command. It records the exact CLI provenance, phase, deadline and terminal classification. User/UAC confirmation is a separate action.

An already-provisioned sandbox follows a different path:

1. observe compatible external configuration;
2. record a sandbox-adoption fact;
3. require explicit product adoption confirmation.

The WorkspaceWrite canary launches an application-authored command in an application-owned probe root. Readiness depends on the expected receipt, not merely exit code. Cleanup and missing-receipt failures receive their own classifications.

## Danger Full Access authority and canary

Danger mode is selected independently from authorization. Authorization is bound to:

- the current profile filesystem identity;
- an explicit full-machine-filesystem plus unrestricted-network scope;
- a versioned authority contract and correlation;
- authorization and revocation timestamps.

The full-access canary uses an application-owned work/receipt pair outside the profile root. It tracks request, launch acceptance, process activity, terminal classification, exact receipt, cleanup and readiness. Provider activity stays `unobserved` unless separately proven. A passing canary requires both receipt observation and owned-artifact removal.

## MCP reporting readiness

### Intended backend lifecycle

The backend separates two commands:

1. `probe_native_profile_mcp_reporting` calls `begin_mcp_reporting_probe`, creating a durable `pending` request with exact profile, capability, server, tool, probe root, correlation and deadline.
2. `reconcile_native_profile_mcp_reporting` claims an already-pending request as `dispatching`, hosts the one-tool MCP server, launches strict Codex, receives the application-bound receipt and settles the request as `received`.

The child is instructed to call only `codex-orchestrator-reporting.report_native_profile_readiness` with `{}`. Tool success returns `ready:false`; only later correlated application settlement changes durable MCP readiness to `ready`.

If the process ends without a receipt, the request becomes `cancelled` and readiness becomes `probe_failed`. Expired requests remain distinct.

### Baseline frontend seam

The visible “Start MCP/reporting probe” action calls `probeMcp`, but `nativeProfileClient.ts` maps that method directly to `reconcile_native_profile_mcp_reporting`. No TypeScript caller invokes probe creation. With no existing pending request, reconciliation simply reloads unchanged state.

This is a confirmed incomplete path in the baseline, not evidence that the backend exchange is absent.

### Post-baseline correction

The `dbe321d` engine continuation serializes probe creation against profile selection and adds race proof. It does not by itself change the baseline frontend command mapping. The final current-state assessment should refresh both lines after active work settles.

## Launch projection

`resolve_selected_home` requires active continuity plus authenticated, sandbox-initialized, WorkspaceWrite-canary-passed and MCP-ready state before returning a selected home to an application consumer.

`project_launch` additionally requires an application-owned target root and mode-specific authority. It builds strict Codex arguments that ignore user config/rules, disable project roots/MCP/hooks/plugins/apps and either:

- enforce WorkspaceWrite with network disabled and no extra writable roots; or
- use the explicitly authorized danger bypass for unrestricted mode.

The method only returns a command projection; it does not start Codex or prove launch acceptance.

At `b28137b`, no non-test ordinary Agent Session caller consumed `resolve_selected_home` or the projected target. At `9240364`, the shared `AgentSessionApplication` calls a Native Profile launch authority that resolves the selected home, durably binds the Session and Invocation to profile identity, and overlays `CODEX_HOME` on the common runtime request.

That connection uses `resolve_selected_home`, not `project_launch`. Agent Sessions still inherit the desktop environment and normal Codex configuration behavior; selected execution mode, exact danger authorization, strict configuration flags and application-owned target policy are not applied by this seam. Profile identity is connected, while the broader Native Profile launch-policy model remains parallel.

## Product truth table

| Fact | Proves | Does not prove |
| --- | --- | --- |
| profile selected | application choice | continuity or readiness |
| login launch accepted | child process accepted | browser handoff or authentication |
| sandbox setup terminal success | process result | UAC observation or product confirmation |
| sandbox adoption verified | compatible external configuration observed | product created it |
| WorkspaceWrite canary passed | exact bounded receipt | MCP or ordinary workflow readiness |
| danger authorized | exact profile has versioned full-machine/network authority | canary, launch or UAC |
| danger canary passed | receipt plus cleanup under authorized mode | provider activity or normal task success |
| MCP tool receipt received | exact server/tool exposure worked | general provider readiness |
| selected home resolved | all required readiness dimensions currently satisfy the consumer gate | launch acceptance or provider use |
| Session/profile binding prepared | durable profile/filesystem continuity was selected for the Session | runtime preflight, process launch or provider activity |
| launch projected | safe command can be constructed for an application-owned target | process was launched |

## Architecture and experience questions

- Should Agent Session launch consume the full Native Profile projection and mode authority, or intentionally only its selected-home identity?
- Should the visible MCP action atomically create and reconcile one request, or deliberately present two steps?
- Which readiness facts belong in calm default UI versus diagnostic disclosure?
- Is exact CLI-version gating a temporary compatibility policy or a durable product contract?
- How should effective profile identity appear in Agent Session and orchestration evidence once connected?
