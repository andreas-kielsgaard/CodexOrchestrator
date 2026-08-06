# NCHP-3 controlled-live evidence

Date: 2026-08-06. This is a partial evidence checkpoint, not readiness acceptance.

## Provenance and isolation

- Candidate: detached clean `6943d5dfa6af9017e0baf1ab79ee5d7373ca8cc8`, with accepted
  baseline `249c899ef8bdfb473977e779b974e6f36aaa58f9`, native producer
  `ffd7f684742aa75c1fa523845385d0b5519b328b`, and strict consumer
  `c21d41603554ab32782e98048d2df44858f65529` as ancestors.
- Task-owned state was rooted at `.dev/worktree-runtime/nchp3-live`: isolated application data,
  `CODEX_HOME`, existing homes, dedicated home, probe roots, build targets, and package target.
  No retained `pip02live557` or `pip02live658` path, credential, secret, or provider payload was
  opened, copied, parsed, printed, or changed.
- The isolated `tauri build --debug --no-bundle` path built the frontend and native app, then
  launched the real Tauri composition. It was stopped, recovered, and relaunched against the same
  isolated data; both launches observed a `codex-orchestrator.exe` child and WebView2 process.
- The release `npm run build:tauri -- --bundles nsis` path passed TypeScript/Vite, produced the
  release executable and `Codex Orchestrator_0.1.0_x64-setup.exe`, and emitted only existing
  dead-code warnings plus the pre-existing `.app` bundle-identifier warning. The installer was not
  installed. The release executable was cold-started with the isolated persisted application-data
  path, remained alive for eight seconds, then its owned process was stopped.
- Final-source deterministic checks: native profiles **25/25**; focused TypeScript/client/settings
  tests **8/8**. The release package build also passed the TypeScript/Vite production build.

## Observed / unobserved matrix

| Boundary | Evidence level | Result |
| --- | --- | --- |
| Existing-home registration and selection | Native controlled-live service | Observed with one task-owned absolute existing-home path and one selected profile id. |
| Dedicated-home creation and selection | Native controlled-live service | Observed under task-owned application data with a distinct application-dedicated profile id. |
| Continuity/replacement | Native controlled-live service | Observed: replacing a task-owned registered directory at the same path projected `replaced` and `profile_continuity_lost`. |
| Application-consumer resolution | Native controlled-live service | Observed fail-closed: selected dedicated profile returned `The selected native Codex home is not ready for an application consumer`; no `CODEX_HOME` was exposed to a consumer. Deterministic product tests remain the accepted evidence for the ready success path and its one absolute `CODEX_HOME` plus separate readiness facts. |
| Authentication/browser login | Unobserved | No browser login request or provider-state inspection was made. No account identity was inferred. |
| Native Windows sandbox request | Native controlled-live service | Observed failure: request first recorded pending UAC/human attention, then reconciled to `native_sandbox_attempt_failed` and `attention_required`. |
| UAC/explicit confirmation | Native controlled-live service | Not confirmed. The product rejected confirmation because no completed application-owned sandbox request existed. |
| Workspace-write canary | Native controlled-live service | Blocked as designed: `workspace_write_canary_requires_observed_sandbox_initialization`; no write canary passed. |
| MCP/reporting readiness | Native controlled-live service | The application-owned probe request was persisted and remained pending; no correlated receipt, MCP call, or `ready` claim was observed. |
| Service cold reopen | Native controlled-live service | Observed: a fresh service instance reopened the same SQLite state and retained existing, replaced, dedicated, selected, readiness, and attention facts. |
| Application restart | Real isolated Tauri composition | Observed process restart against the same app-data route. UI-command continuity is unobserved because host desktop enumeration failed with Windows `0x80070003`. |
| Packaged cold startup | Release executable | Observed process survival for eight seconds against the isolated persisted app-data route. Installer execution and package UI/profile-query continuity are unobserved. |

The controlled-live service harness was temporary test-only instrumentation around the existing
native service and was removed before this checkpoint. Its retained JSON output is task-owned,
ignored runtime state, not a product fixture or provider observation.

## Actionable correction boundary

The installed CLI actually reached by this checkpoint was `codex-cli 0.130.0-alpha.5`, not the
earlier recorded `0.144.0`. Its public diagnostic for the product-shaped sandbox command requires
`--sandbox-state-json <JSON>`. The candidate's native sandbox command supplies
`--sandbox-state-disable-network` and `--sandbox-state-readable-root`, but no required JSON state,
so the native child exits before any UAC confirmation or canary can be meaningful.

The next correction should make the Windows sandbox invocation compatible with the actually
selected supported CLI surface (including the required sandbox state and any required Windows
subcommand), then repeat this exact isolated profile flow. It must continue to keep confirmation,
canary, MCP receipt, provider authentication, and consumer resolution as separate facts. This
checkpoint does not authorize a sandbox bypass, wider roots/network, provider-state access, MCP
receipt fabrication, implementation/Handler work, settlement, integration, publication, or user
acceptance.
