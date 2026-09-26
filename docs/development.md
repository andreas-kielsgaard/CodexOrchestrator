# Development and validation

Commands below match `package.json`, Vite, Tauri and the retained scripts after the tooling cleanup. npm is the supported package manager; the standalone recorded Session page and its simulator have been removed.

## Setup and startup

Use Node.js 24 or newer with npm, the Rust MSVC toolchain, Visual Studio C++ build tools and Windows WebView2. The [launcher](../launch-dev.bat) starts from its own repository directory, finds Cargo and the local Codex executable when available, and loads the conventional Visual Studio 2022 Build Tools environment if installed there.

For a clean dependency install:

```powershell
npm ci --include=dev
```

`npm run install:app` performs the same clean installation, including the development tools required to build and launch the app. Commit dependency changes in `package.json` and `package-lock.json` together.

Start the desktop application with `launch-dev.bat`, or use `npm run dev:tauri` from a prepared environment. Tauri starts Vite through its `beforeDevCommand`. Vite listens on `127.0.0.1:1420` by default and refuses a conflicting port. `npm run dev` starts only the frontend server; ordinary product IPC needs Tauri.

The optional status server is started explicitly with `npm run dev:status` at port 41415; `npm run mark:stale` and `npm run clear:stale` manage its marker. The normal launcher no longer starts that server, clears the marker or sets `VITE_RUNTIME_STATUS_URL`. The inspector still consumes the endpoint; the old product widget was removed. An endpoint response does not identify a particular application instance.

## Command scope

| Command                                                              | What it does                                                                                                                  |
| -------------------------------------------------------------------- | ----------------------------------------------------------------------------------------------------------------------------- |
| `npm test`                                                           | Runs the Vitest frontend suite; many tests use fake clients/JSDOM.                                                            |
| `npm run lint`                                                       | Runs ESLint.                                                                                                                  |
| `npm run format:check`                                               | Checks repository formatting with Prettier. For a focused change, run the installed Prettier CLI on its changed files.        |
| `npm run build`                                                      | Builds a normal runnable application, retains an independent copy, and prints its path. Launch it separately.                 |
| `npm run build:frontend`                                             | Typechecks and runs Vite only; does not produce a native application.                                                         |
| `npm run check:rust` / `npm run check:rust:release`                  | Compiles/checks native code for the selected profile; does not run tests or a native window.                                  |
| `npm run test:rust:fast -- <filter>`                                 | Runs library tests using the named `test-fast` profile with line-table debug information.                                     |
| `npm run test:rust:full -- <filter>`                                 | Runs library tests with the ordinary test profile.                                                                            |
| `npm run test:worktree-review` / `npm run test:rust:worktree-review` | Runs the named frontend/native Worktree Review coverage.                                                                      |
| `npm run validate:worktree-review`                                   | Builds the frontend, runs the Worktree Review frontend/native suites and checks Rust compilation.                             |
| `npm run build:tauri`                                                | Builds a runnable application and the configured installer bundles.                                                           |
| `npm run validate:release-build`                                     | Builds and retains a normal application without installer bundles.                                                            |
| `npm run test:app-inspector`                                         | Runs the eight Node test files for the optional review companion on Windows with `powershell.exe`; does not launch a browser. |
| `npm run test:app-inspector:browser`                                 | Separately launches installed Microsoft Edge with a disposable profile and local fixture page; skips if Edge is unavailable.  |

A native executable build does not establish installer behavior, provider compatibility or a usable native flow. Record those outcomes separately in the [evidence record](validation-evidence.md).

## Build outputs and compiler caches

Application commands and Worktree Review use the same tool in `scripts/build-tools.mjs`. `npm run build` typechecks, builds the embedded frontend, compiles Tauri, and copies the executable into an independent application directory. Normal builds use release mode; `--debug` selects Tauri debugging mode and retains the application's PDB. The command prints the executable path and never launches it.

```powershell
npm run build
npm run build -- --debug --cache=local
npm run check:rust -- --cache=shared
npm run test:rust:fast -- --cache=auto
node scripts/build-tools.mjs --help
```

The supplied native build, check and test commands accept `--cache=auto|local|shared`. Auto reuses populated local artifacts for the requested profile, otherwise uses shared sccache when available, otherwise compiles normally. Cargo decides which artifacts are still valid. Local mode suits continued edits and keeps ordinary incremental settings. Shared mode can reuse compilation across worktrees and disables incremental compilation for that invocation. Choose local for incremental work or investigating compiler/wrapper problems; choose shared explicitly when testing cross-worktree reuse. An unavailable explicit shared cache is an error; auto reports its fallback.

A worktree keeps one persistent Cargo target across builds: an explicit `--target-dir`/`CARGO_TARGET_DIR`, its previously selected target, an existing populated `src-tauri/target`, or a short tool-owned path under `%LOCALAPPDATA%/CodexOrchestrator/build-cache/<worktree-key>/t`. Branch-tip changes do not discard it. Different worktrees have separate targets; a new worktree can reuse the shared cache without copying its parent's target. Tool commands refuse overlapping builds for the same worktree.

Application copies default to `%LOCALAPPDATA%/CodexOrchestrator/builds/<worktree-key>/<build-id>/output`; `--output DIR` selects a new attempt directory outside the checkout and compiler cache. Worktree Review supplies its existing retained-output location. Copies contain runtime files and, for debugging builds, application symbols. Compiler intermediates stay in the cache. Rebuilding or clearing a cache does not change an older application copy.

```powershell
node scripts/build-tools.mjs clear-cache --worktree C:\path\to\checkout
```

Clear only when explicitly needed. This command refuses active builds and arbitrary or linked target directories. Build failure and dependency/toolchain changes do not trigger deletion: caches can remain useful for other inputs. Old Review outputs retain their existing layout and retention policy.

The shared helper requires sccache 0.17.0 or newer, a compatible local cache server and working Cargo/MSVC tools. It preserves the stable Cargo working directory at `%LOCALAPPDATA%/CodexOrchestrator/cargo-sccache-cwd`. Environment changes apply only to child processes. Logged cache counters are machine-wide command-window deltas and can include other builds.

The PowerShell entrypoints delegate to the same tool:

```powershell
.\scripts\cargo-sccache.ps1 check --locked
.\scripts\cargo-test-fast.ps1 -Cache auto --lib --no-run --locked
```

The latter preserves its older ordinary-test profile with `CARGO_PROFILE_TEST_DEBUG=0`; it differs from npm's named `test-fast` profile, which retains line-table debug information. Checks and tests keep their existing semantics; neither publishes an application. `npm run test:build-tools` verifies cache selection, invocation, wrappers, locking and output independence.

Frontend-only output is `dist/`. `VITE_RUNTIME_ROOT` redirects development Vite cache/output and `VITE_RUNTIME_VITE_PORT` selects its port. Application builds use stable frontend staging beside their compiler cache and disable the duplicate Tauri frontend hook.

## Installed and live checks

Default Rust library compilation excludes the `live-tests` feature. Enabling that feature compiles ignored installed/live proof entry points; it does not execute them automatically. Installed-CLI compatibility probes require an installed executable and an explicit exact ignored-test selection. Live/paid drivers additionally require their documented environment opt-in.

`npm run test:codex-app-server` runs the installed executable contract test only when `CODEX_APP_SERVER_CONTRACT_PROGRAM` names a native Codex executable. Otherwise it skips. The executable talks to a local fixture provider using a disposable unauthenticated native home. A skipped run proves no installed compatibility; a passed fixture run does not prove an authenticated provider session.

The older `src-tauri/src/agent_sessions/live_smoke.rs` is a test-only `CodexCliRuntime` lifecycle probe, not the production app-server adapter. Its four-invocation driver requires `CODEX_AGENT_SESSION_LIVE_SMOKE=true`; optional `CODEX_AGENT_SESSION_LIVE_SMOKE_TIMEOUT_SECS` is bounded to 1–300 seconds per wait, default 180. It owns a temporary database/workspace and emits redacted evidence. Quota rejection is a failed/incomplete provider proof, not successful continuation or cancellation. Preserve this historical probe's scope when explicitly choosing it; do not use its existence as the current Session implementation description.

The inspector has its own optional [Node/browser checks](../review-tools/app-inspector/README.md#checks). They are excluded from Vitest and Worktree Review validation and require separate invocation. A skipped browser check does not verify browser behavior.

## Recorded review routes

Run Vite and select a route explicitly in development:

| Query/entry                                          | Current recorded surface                                                                  |
| ---------------------------------------------------- | ----------------------------------------------------------------------------------------- |
| `?recorded-plan-builder`                             | Plan Builder/Epic recorded composition.                                                   |
| `?harness-inspector`                                 | Recorded Harness inspection.                                                              |
| `?recorded-work-unit-review`                         | Work Unit review examples.                                                                |
| `?file-diff-viewer&file-review-fixture=working-tree` | File Review; variants include `staged`, `commit-range`, `generated`, `application-owned`. |

These routes supply recorded application facts. They establish presentation behavior rather than live provider, production persistence or semantic acceptance. File Review includes explicit unavailable/binary states; recorded Work Unit review has typed test-detail examples, superseding the old packet's blanket claim that test detail is absent.

The retained previews read fixed Session histories; Session creation, sending and cancellation reject as unsupported. The obsolete standalone Session scenario page and its simulator are absent, and the normal application is the only production HTML entry. Focused Session tests use application DTOs and explicit client responses rather than a simulated Session lifecycle.

The [offline packet](../offline-review/README.md), [regression probes](regression-review/README.md) and [walkthrough captures](ux/session-event-model-walkthrough/README.md) describe their original checkpoints. Old absolute worktree paths and screenshots are not a current launch recipe.

## Linux cloud sessions

The project is developed and validated on Windows. Claude Code cloud sessions run it in a Linux container, where some setup differs and a fixed set of Rust tests fails for environmental reasons. Treat a failure as a regression only if it is outside the list below, or if it also fails on Windows.

Setup:

- Tauri needs the WebKitGTK system packages: `apt-get install -y libwebkit2gtk-4.1-dev libgtk-3-dev libayatana-appindicator3-dev librsvg2-dev libssl-dev pkg-config`.
- `tauri::generate_context!()` requires `src-tauri/icons/icon.png` on Linux, and only `icon.ico` is committed. Create any small PNG there and add it to `.git/info/exclude`; do not commit it.
- `npm ci --include=dev`, `npm test`, `npm run build:frontend` and the `crates/orchid-engine` tests run normally.

Known failures in `cargo test --profile test-fast --lib` on Linux (observed 2026-09-25 at `9ab4706` and later, 24 to 26 of about 755):

| Tests | Cause |
| --- | --- |
| `execution_devices::tests::{idle_shutdown_runs_once_after_the_last_orchid_activity, failed_idle_shutdown_is_released_for_a_later_retry, keep_awake_and_activity_lease_prevent_idle_shutdown}` | The fixtures use Windows paths such as `C:\Orchid\stop-device.exe`, which are not absolute on Linux ("Device lifecycle program must be an absolute path"). |
| `runtime::providers::codex::profiles::tests::windows_login_environment_is_allowlisted_and_keeps_the_product_selected_home` | The allowlist is compiled only under `#[cfg(windows)]`. |
| `orchestration::accepted_integration::tests::*` (10 tests) and `orchestration::accepted_integration::accepted_integration_proof_tests::*` (6 tests) | The Git fixture's integration commit is rejected with `integration_identity_metadata_mismatch`, and dependent assertions then fail. Cause not yet identified. It is not the container's commit signing (`GIT_CONFIG_GLOBAL` pointing to a plain config still fails) and not the time zone (`TZ=Europe/Copenhagen` still fails). |
| `orchestration::sprint_runner_transition::accepted_integration_gateway_tests::accepted_integration_full_gateway_revalidates_retained_lineage_without_attempt_worktree`, `orchestration::bootstrap_transition::tests::terminal_authority_fixture_converges_product_materialization_and_real_git_gateway`, `orchestration::repository::tests::git_capture_authorization_is_private_replay_safe_and_reauthorized` | Same Git-fixture family as above. |
| `runtime::providers::codex::profiles::tests::reporting_dispatch_calls_the_exact_mcp_tool_then_settles_the_real_receipt` | "The MCP reporting exchange completed without a correlated receipt". Fails on every Linux run seen so far. |

These fail on some Linux runs and pass on others:

- `runtime::providers::codex::profiles::tests::concurrent_reporting_dispatches_adopt_one_pending_request`
- `runtime::providers::codex::profiles::tests::two_services_claim_one_reporting_exchange`
- `orchestration::bootstrap_transition::tests::no_progress_handback_delivers_one_epic_receiver_without_higher_effects`

To check whether a change caused a failure, run the same filter on the parent commit in a separate worktree with its own `CARGO_TARGET_DIR`, and compare the two lists of failing tests.

## Ownership and provenance

The manifest, `vite.config.ts`, `src-tauri/tauri.conf.json`, the launcher and script implementations own command behavior. The documentation does not add CI/workflow enforcement or change build policy. `review-tools/app-inspector/` owns observation and development interaction details.

Developer-only optimization scope was set in task `019fcbd6-6e6a-74a1-9514-dad527bb9e36` (raw line 872). The bounded follow-up `019fcc24-b0dd-7240-a423-f435cc54a1af` and publication `019fe0c7-2bda-7ee2-9c08-47b32fa8820e` originally retained opt-in helpers and separate proof tiers. The application-build refinement now makes automatic cache selection the default on the supplied native commands. Original operating/benchmark records remain retrievable at `e2bfc6c:docs/orchestration/rust-test-developer-validation.md` and `e2bfc6c:docs/orchestration/ad-hoc-rust-compilation-cache.md`.
