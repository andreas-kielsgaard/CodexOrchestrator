# CLI, process and environment surfaces

## Scope

This catalogue distinguishes commands the release product launches, debug/operator tooling, and substantial retained runtimes that are not reachable from the current product composition.

## Active release process integrations

### Generic Agent Session Codex runtime

Implementation: `src-tauri/src/runtime/codex/` and `runtime/processes/`.

The composition uses `CodexCliRuntime::system("codex", None)`. It:

- probes `codex --version`, `codex exec --help` and `codex exec resume --help`;
- caches successful semantic capability evidence for 30 minutes and unavailable evidence for one minute;
- launches `codex exec --json` or `codex exec resume --json`;
- projects requested model and sandbox options according to discovered support;
- normalizes JSONL runtime output;
- supervises the direct child and supports cancellation/shutdown.

Environment/configuration posture:

- inherits the full parent environment;
- current tip overlays application-selected `CODEX_HOME` after a Native Profile readiness and continuity gate;
- normally does not use strict config, ignored user config or ignored rules;
- project/user Codex configuration may therefore influence effective runtime behavior;
- an unknown model capability silently omits the requested model;
- an unknown sandbox capability fails closed.

### Native-profile Codex runtime

Implementation: integrated into `src-tauri/src/native_profiles.rs`.

It supports:

- browser login and login-status checks;
- Windows sandbox initialization and preprovisioned adoption;
- WorkspaceWrite and danger-full-access canaries;
- MCP readiness exchange;
- projection of a selected `CODEX_HOME` launch target.

Environment/configuration posture:

- clears the environment, then restores selected `CODEX_HOME` and required Windows launch/path variables;
- strongest isolation and danger-mode claims require exact `codex-cli 0.144.0` behavior;
- strict launches ignore user configuration and rules, disable project roots, clear MCP servers and disable hooks/plugins/apps;
- WorkspaceWrite disables network and extra writable roots outside the special MCP probe/reporting route;
- danger mode uses `--dangerously-bypass-approvals-and-sandbox` only after exact durable filesystem-identity authority.

Current tip `9240364` connects selected-profile identity to the generic Agent Session application. It calls the Native Profile readiness gate, persists Session/profile continuity and overlays `CODEX_HOME` while retaining the generic runtime's inherited-environment and normal-configuration posture. It does not consume `project_launch`, selected execution mode or exact danger authority. Identity selection and strict Native Profile launch policy therefore remain separate facts.

### Product Git operations

| Area | Git responsibility | Environment posture |
| --- | --- | --- |
| `execution_support.rs` | create and validate isolated attempt workspaces | targeted validation; inherits more ambient state than File Review |
| `accepted_candidate_authority.rs` | verify and pin accepted candidate refs | exact object/ref/path correlations |
| `accepted_integration.rs` | temporary-index merge, `commit-tree`, CAS `update-ref`, target worktree convergence | serialized against application target authority |
| `sprint_runner_transition.rs` | Sprint Git authority and retry private refs | embedded in transition service |
| `file_review_git_producer.rs` | bounded read/capture of repository comparison | most hardened: clears environment and disables config, hooks, credentials, replacements, optional locks and external diff |

There is no single product Git process wrapper or common environment policy. These helpers have materially different trust assumptions.

## Release-registered but inert legacy runtime

The original Tasks backend in `src-tauri/src/lib.rs` includes Codex launch, Git, validation commands, task runs, artifacts and conversations. Its nine Tauri commands call an unconditional legacy availability guard before opening `codex-orchestrator.sqlite`.

Consequences:

- caller-provided Codex arguments/environment in the old runner are not release-executable;
- arbitrary post-run validation commands are not release-executable;
- the Task Dashboard is not mounted;
- TypeScript `localRuntimeComposition` and its Node child-process/SQLite stack have no product import path.

This is retained compiled migration/test code, not a second active runtime.

## Debug-only application tooling

### Rust Worktree Runtime and Human Review

`worktree_runtime/` provides isolated instance planning, port leases, build/process ownership, health and recovery. `worktree_review/` exposes it through 21 debug Tauri commands and optional proof-control HTTP.

Toolchain discovery/invocation includes:

- Git;
- Node/npm;
- TypeScript;
- Vite and Vitest;
- Tauri CLI;
- Cargo and rustc;
- the built desktop executable.

The Worktree Review composition exists only under `debug_assertions`. Generic Worktree Runtime modules compile in release but have no release route.

## Repository operator/developer tools

| Artifact | Responsibility | Classification |
| --- | --- | --- |
| `scripts/worktree-runtime.mjs` | historical prepare/install/build/test/start/status/stop/recover runtime with generated isolation and Tauri overlay | developer prototype; overlaps Rust runtime |
| `scripts/runtime-status-server.mjs` | unauthenticated loopback health/status/stale server, default port 41415 | development only |
| `review-tools/app-inspector/` | inspect, compare, wait, launch-wait, bounded click and WebView2 CDP control | review/operator CLI |
| `src-tauri/examples/worktree_review_controller.rs` | manual debug proof client | example/proof |
| `src-tauri/examples/worktree_review_background_launcher.rs` | non-activating Windows launch proof | example/proof |
| `scripts/cargo-sccache.ps1` | opt-in shared compiler cache with isolated target and restored environment | developer helper |
| `scripts/cargo-test-fast.ps1` | per-invocation lower-debug Cargo test wrapper | developer helper |
| ignored live Rust tests using `CODEX_AGENT_SESSION_LIVE_SMOKE*` or `CODEX_PIP01*` | controlled verification | test seam, not product configuration |

`review-tools/app-inspector` is substantial enough to treat as a maintained internal tool rather than an incidental script, but it has no product transport.

## Static and dynamic build/runtime configuration

| Artifact or variable | Effect |
| --- | --- |
| `src-tauri/tauri.conf.json` | product identity/version/window; CSP currently `null` |
| `src-tauri/capabilities/default.json` | main-window event listen/unlisten permission |
| `vite.config.ts` | default port 1420, runtime overrides, product and Agent Session harness build inputs |
| `agent-session-harness.html` | deterministic verification entry emitted in frontend builds without normal product navigation |
| `CODEX_ORCHESTRATOR_APP_DATA_DIR` | absolute product app-data override |
| `CODEX_ORCHESTRATOR_REVIEW_RUNTIME_DIR` | debug review-root override and composition trigger |
| `VITE_RUNTIME_*`, `RUNTIME_STATUS_*` | generated isolated review/runtime configuration |
| `CODEX_ORCHESTRATOR_MCP_*` | child-only orchestration MCP bearer values |
| selected `CODEX_HOME` | native-profile probes and current shared Agent Session runtime identity |

Both Node and Rust worktree runtimes generate Tauri overlays and isolation environments. They represent two generations of the same internal architecture rather than two intentionally composed product services.

## Environment-policy matrix

| Process family | Parent environment | User/project config | Network/sandbox posture | Principal risk/question |
| --- | --- | --- | --- | --- |
| generic Agent Session Codex | inherited, with selected home overlay | influential by default | projected from requested options/capability probe | profile identity is governed, but ambient configuration can still change effective Harness behavior |
| native-profile strict routes | cleared/allowlisted | explicitly ignored | tightly projected and evidence-gated | stricter mode/authority is not shared by ordinary Agent Session launch |
| File Review Git | cleared/hardened | disabled | read/capture only | strongest product Git isolation is not shared elsewhere |
| execution/integration Git | varies/inherited more broadly | partially controlled by explicit args | mutates authorized worktrees/refs | consolidate policy without breaking Git authority semantics |
| debug Worktree Runtime | generated isolated environment | generated overlays | local ports/process ownership | internal-tool lifecycle and cleanup |
| Node worktree prototype | script-generated | generated overlays | developer-controlled | duplicated implementation and retention purpose |

## Architecture and cleanup questions

- Should generic Agent Sessions receive an application-owned environment/configuration policy rather than ambient inheritance?
- Which process isolation requirements are common, and which are intentionally capability-specific?
- Can Git execution share a hardened launcher while retaining exact per-operation allowlists and authority checks?
- Is the Node worktree runtime still useful as evidence, or should it be archived/extracted after Rust parity is confirmed?
- Should the Agent Session harness remain a production build input?
- Which operator tools belong beside the product versus in a separate internal-tooling package?
