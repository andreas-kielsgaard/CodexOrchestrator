# Agent Review Exploration

## Scope

- Baseline: `f23f5fdcd3ae9298261e81db52366854c00dc4a0`
- Branch: `codex/explore-agent-app-review`
- Worktree: `C:\Users\user\.codex\worktrees\5634\Codex Orchestrator`
- Platform: Windows `10.0.26200`, x64
- Date: 2026-07-17
- Checkpoint resumed: 2026-07-27

This exploration separates deterministic renderer verification, Windows-native inspection, and
native shell/IPC verification. Driver commands do not belong in the review contract.

The three proofs were launched by dedicated worktree-local adapters. The semantic handoff through
the worktree runtime is defined in [worktree-runtime-seam.md](worktree-runtime-seam.md), but its
application integration is not yet proven and no cross-worktree code was merged into this branch.

## Current findings

| Lane                              | Current disposition                                                                         | Direct evidence                                                                                           |
| --------------------------------- | ------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------- |
| Deterministic renderer            | Verified for the recorded Plan Builder behavior and development review tab evidence capture | Playwright Test 1.61.1, Edge 150.0.4078.99, 1920 × 1080, two passing scenarios                            |
| Windows Tauri/WebView2 attachment | Verified on this Windows stack                                                              | Tauri debug host, WebView2 150.0.4078.99, one discovered page target, semantic interaction and screenshot |
| Native Tauri shell/IPC            | Verified for the active native-query command on this Windows stack                          | WebdriverIO 9.27.1, Tauri service/plugins 1.2.0, embedded provider, one passing real-shell IPC test       |

### Renderer

`npm run review:renderer` starts an isolated Vite server on port 1437. It exercises the existing
`?recorded-plan-builder` composition and the `?agent-review` development tab. The run:

- enters Plan Builder through the visible application action;
- asserts the workspace, conversation, and proposal rail;
- collapses and expands a Sprint through its semantic button;
- captures screenshots, semantic snapshots, traces, console output, network failures, layout
  measurements, and manifests;
- verifies that the tab keeps the three lanes and request/evidence/disposition stages distinct;
- verifies that it grants no synthetic Run or Attach authority.

Generated runs are under `.dev/agent-review/renderer/`. Deliberately retained files are under
`docs/agent-review/evidence/renderer/`. The behavior is verified. The screenshots are valid visual
evidence, but exact fidelity to the older source images remains `user-review-required` because those
images represent a different viewport and content state.

### Windows attachment

The actual debug executable was launched with environment variables scoped to its process tree:

```text
WEBVIEW2_ADDITIONAL_BROWSER_ARGUMENTS=--remote-debugging-port=0
WEBVIEW2_USER_DATA_FOLDER=<worktree>\.dev\agent-review\native\webview2-user-data
```

WebView2 created `EBWebView/DevToolsActivePort` with ephemeral port `52232` for the retained run. The
endpoint reported one page:

```text
title: Codex Orchestrator
url: http://127.0.0.1:1438/?recorded-plan-builder
browser: Edg/150.0.4078.99
protocol: 1.3
```

Playwright 1.61.1 attached through CDP, observed Tauri internals, opened Plan Builder, exercised the
Sprint control, and captured semantic and screenshot evidence. Stopping the owned launch tree
stopped 18 descendants, ended `codex-orchestrator.exe`, closed the debug port, and removed the
worktree-scoped WebView2 profile.

Reproduce the owned lifecycle with:

```powershell
npm run review:native-attach
```

Generated runs are under `.dev/agent-review/native/runs/`. The selected manifest, lifecycle record,
semantic snapshot, and screenshot are retained under
`docs/agent-review/evidence/windows-attachment/`. Authenticated profiles and credentials are not
retained. Before build and launch, the adapter removes `CODEX_HOME` and environment variables whose
names indicate tokens, secrets, passwords, credentials, API keys, or authentication.

This disproves the broad hypothesis that Playwright necessarily exposes no usable WebView2 page for
the current application. It does not prove that every WebView2 host works, that the route is
portable, or that Chrome DevTools MCP itself has been invoked.

The development renderer and attachment consoles each retain one known `/favicon.ico` 404. Neither
run recorded a page exception or failed request, and the behavioral assertions passed; the console
entry is retained rather than hidden.

### Native shell and IPC

`npm run review:native` builds an alternate release Tauri shell with the explicit `native-review`
Cargo feature, alternate Tauri config, and alternate frontend entry. WebdriverIO 9.27.1 with
`@wdio/tauri-service` 1.2.0 and its embedded provider then:

- launches the real Windows shell;
- observes the application root;
- invokes the active Rust `load_orchestration_native_query` command through
  `browser.tauri.execute`;
- asserts `orchestration-native-query/v2` and all 12 empty durable collections in fresh isolated
  app data;
- retains a 27,402-byte screenshot plus assertion, frontend, backend, build, and driver evidence;
- verifies the selected loopback port closed and removes the isolated database and WebView2
  profile.

The accepted run passed 1/1. Its generated evidence is under
`test-results/native-tauri-wdio/latest/`; the approved subset is retained under
`docs/agent-review/evidence/native-tauri-wdio/`.

The normal configuration never enables `withGlobalTauri`, WDIO permissions, or the embedded
driver. Both Rust plugins are optional and registered only under `native-review`; the normal Cargo
tree and a normal release-binary scan contained no WDIO markers.

Two compatibility findings remain explicit:

- `@wdio/tauri-service` 1.2.0 declares `@wdio/native-utils` 2.4.0 but imports an export first present
  in 2.5.0, so the documented npm override is required;
- real IPC passed, but command mocking remains unproven after the frontend interception warning.
  The forwarded channel also emitted non-fatal JSON deserialization warnings.

The 2026-07-27 npm audit reports one high production package finding and 28 full-tree package
findings (one moderate, 27 high). The production `postcss` finding and five other affected packages
are present at the unchanged baseline versions. The other 22 affected packages are in the new
development-only WDIO chain. Top-level fixes are unavailable for the current WDIO CLI, local
runner, and Tauri service; npm offers a breaking Mocha-framework downgrade. See
[dependency-audit.md](dependency-audit.md).

### Dependency audit

The npm lock change adds 431 package entries and changes or removes no pre-existing package entry.
Playwright supplies the renderer and CDP adapters. The pinned WDIO CLI, local runner, Mocha
framework, Tauri service/plugin, and `get-port` supply the native route; the 2.5.0
`@wdio/native-utils` override is the compatibility fix described above. The Cargo lock adds 17
package-version entries and removes none; both WDIO Rust plugins remain optional. No runtime or
production dependency was added to `package.json`.

After the native proof, a normal featureless release was rebuilt. Its normal Cargo graph contained
zero WDIO references, its feature graph contained both 1.2.0 plugins, and its 15,990,784-byte binary
contained none of the review markers. SHA-256:
`182652124F73CEBF75BDA3A2D654B0DC2B5C6B9EE7E3118B10D71682095C6F6F`.

## Architecture seams

- `src/app/ApplicationRoot.tsx` owns the existing development-only composition gate.
- `src/app/App.tsx` owns peer application-surface navigation.
- `src/dev/` owns recorded data and development-only UI.
- `src/application/` is the correct location for a neutral review/evidence contract.
- `src-tauri/src/active_app.rs` owns active Tauri composition. The legacy implementation in
  `src-tauri/src/lib.rs` remains quarantined.
- `src-tauri/capabilities/default.json` is the normal capability boundary; review-driver
  permissions must remain separate.

## Development feature tab

`Agent Review` is a peer application surface only when development/test composition supplies it.
`?agent-review` selects it directly. The normal product composition supplies no review surface, and
a normal production build contains none of the recorded lab, remote-debugging, or WDIO strings
checked by:

```powershell
npm run review:production-exclusion
```

The tab reports retained facts and reproduction commands. It does not directly execute a driver or
grant inspection, native, orchestration, credential, or production authority. Its worktree
convergence section explicitly labels the application/runtime integration as unproven.

## Review boundary

A review request should identify revision, worktree, surface, scenario, platform or viewport,
claims, granted capabilities, and required evidence. An evidence bundle should identify application
mode, driver/version, start state, actions, assertions, observations, produced files, and unverified
claims. Evidence acquired from an owned instance should also link its instance ID and runtime
manifest; `null` means the evidence predates that integration. Agent judgement returns one of:

- `accepted`
- `changes-required`
- `user-review-required`
- `blocked`
- `inconclusive`

Exploratory control, deterministic verification, and review judgement remain separate. A driver
cannot grant itself authority to mutate orchestration state.

## Security and lifecycle

- Renderer scenarios use recorded, effect-limited clients.
- WebView2 remote debugging is development-only, loopback-bound, process-scoped, and uses an
  OS-assigned port.
- The WebView2 profile lives in the assigned worktree and contains no authenticated production
  state.
- `DevToolsActivePort` is discovered from the owned profile; fixed global ports and registry
  overrides are unnecessary.
- The debug endpoint inherits the page's authority while active. Only trusted inspection agents may
  receive it.
- Evidence must not retain credentials, authenticated profiles, or production data.
- Native launch adapters scrub `CODEX_HOME` and credential-shaped process variables before build
  and launch; manifests retain only the number removed, never their names or values.
- Native drivers and permissions must shut down with their owning test process and stay out of the
  normal production build.

After generating all accepted runs, retain only the approved evidence file set with:

```powershell
npm run review:retain-evidence
```

## Checkpoint validation

- Renderer review: 2/2 Playwright scenarios passed at 1920 × 1080.
- Windows attachment: one owned WebView2 page attached; semantic assertions and lifecycle cleanup
  passed.
- Native Tauri: 1/1 WebdriverIO scenario passed; real native-query IPC and cleanup passed.
- Frontend/unit: 90 Vitest files, 612 tests passed.
- Rust: 168 tests passed, six paid/live probes ignored; the capability integration test passed.
- Static: TypeScript, full ESLint, touched-file Prettier, Rust format, PowerShell parsing, and
  `git diff --check` passed.
- Production: frontend exclusion passed across eight files and two normal Tauri configs; normal
  Cargo and release-binary scans found zero WDIO markers.

One initial full Vitest run shared the machine with Cargo tests and timed out on an existing Epic
Plan Builder async assertion. That file then passed 10/10 alone, and the full suite passed 612/612
without competing compilation. No product change was made for the transient failure.

## Primary references

- [Playwright Test Agents](https://playwright.dev/docs/test-agents)
- [Playwright agent CLI](https://playwright.dev/agent-cli/commands/test-debugging)
- [Microsoft WebView2 with Chrome DevTools MCP](https://learn.microsoft.com/en-us/microsoft-edge/web-platform/devtools-mcp-server)
- [Microsoft WebView2 environment overrides](https://learn.microsoft.com/en-us/microsoft-edge/webview2/reference/win32/webview2-idl)
- [Tauri WebDriver](https://v2.tauri.app/develop/tests/webdriver/)
- [WebdriverIO Tauri](https://webdriver.io/docs/desktop-testing/tauri/)
- [WebdriverIO Tauri plugin setup](https://webdriver.io/docs/desktop-testing/tauri/plugin-setup/)
