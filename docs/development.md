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

| Command                                                              | What it does                                                                                                                                                                |
| -------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `npm test`                                                           | Runs the Vitest frontend suite; many tests use fake clients/JSDOM.                                                                                                          |
| `npm run lint`                                                       | Runs ESLint.                                                                                                                                                                |
| `npm run format:check`                                               | Checks repository formatting with Prettier. For a focused change, run the installed Prettier CLI on its changed files.                                                      |
| `npm run build`                                                      | Typechecks and runs Vite. This is a frontend build.                                                                                                                         |
| `npm run check:rust` / `npm run check:rust:release`                  | Compiles/checks native code for the selected profile; does not run tests or a native window.                                                                                |
| `npm run test:rust:fast -- <filter>`                                 | Runs library tests using the named `test-fast` profile with line-table debug information.                                                                                   |
| `npm run test:rust:full -- <filter>`                                 | Runs library tests with the ordinary test profile.                                                                                                                          |
| `npm run test:worktree-review` / `npm run test:rust:worktree-review` | Runs the named frontend/native Worktree Review coverage.                                                                                                                    |
| `npm run validate:worktree-review`                                   | Builds the frontend, runs the Worktree Review frontend/native suites and checks Rust compilation.                                                                           |
| `npm run build:tauri`                                                | Runs `tauri build`, including the configured frontend prerequisite.                                                                                                         |
| `npm run validate:release-build`                                     | Runs `tauri build --no-bundle` to build the native release executable. Tauri runs the frontend prerequisite once; release-profile Rust checking remains a separate command. |
| `npm run test:app-inspector`                                         | Runs the eight Node test files for the optional review companion on Windows with `powershell.exe`; does not launch a browser.                                               |
| `npm run test:app-inspector:browser`                                 | Separately launches installed Microsoft Edge with a disposable profile and local fixture page; skips if Edge is unavailable.                                                |

A native executable build does not establish installer behavior, provider compatibility or a usable native flow. Record those outcomes separately in the [evidence record](validation-evidence.md).

## Outputs and optional Rust helpers

Ordinary frontend output is `dist/`; native output is normally below `src-tauri/target/`. `VITE_RUNTIME_ROOT` redirects Vite cache/output for an isolated runtime and `VITE_RUNTIME_VITE_PORT` selects its port. Worktree Review's build runner has its own explicit typecheck/frontend/Tauri sequence and disables the duplicate Tauri hook while using isolated outputs; keep that responsibility with [Worktree Review](worktree-review.md).

The named fast/full test lanes compile the library test harness even when a filter selects few tests. Two optional PowerShell helpers remain:

```powershell
.\scripts\cargo-test-fast.ps1 --lib --no-run --locked
.\scripts\cargo-test-fast.ps1 -TargetDir .dev\local-check\cargo-target --lib --locked
.\scripts\cargo-sccache.ps1 check --locked --timings
.\scripts\cargo-sccache.ps1 -TargetDir .dev\cached-check\cargo-target test worktree_application -- --nocapture
```

The older fast helper runs ordinary `cargo test` with process-scoped `CARGO_PROFILE_TEST_DEBUG=0`, removes ambient `RUSTC_WRAPPER`, restores its environment and preserves Cargo's exit code. It does not select the named `test-fast` profile, whose debug setting differs.

The sccache helper requires an installed `sccache` at least 0.17.0, a compatible running cache server and working Cargo/MSVC tools. It uses a stable Cargo working directory under `%LOCALAPPDATA%/CodexOrchestrator/cargo-sccache-cwd`, separate per-worktree targets and a shared compiler cache. It scopes/restores wrapper, incremental and cache variables without changing PATH. Counters are machine-wide command-window deltas and may include concurrent activity.

Ordinary incremental Cargo remains appropriate for repeated local edits. Cache reuse depends on matching command/profile inputs; the historical representative cache-assisted `test --no-run` run was slower, not faster. See [tooling evidence](validation-evidence.md#developer-tooling) for the measurements and limits. Cache contents and build outputs have different producers and purposes; this documentation rewrite does not delete either.

Fake-only helper checks are available through `scripts/cargo-test-fast.tests.ps1` and `scripts/cargo-sccache.tests.ps1`. These verify script contracts and environment/exit handling, not application behavior.

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

## Ownership and provenance

The manifest, `vite.config.ts`, `src-tauri/tauri.conf.json`, the launcher and script implementations own command behavior. The documentation does not add CI/workflow enforcement or change build policy. `review-tools/app-inspector/` owns observation and development interaction details.

Developer-only optimization scope was set in task `019fcbd6-6e6a-74a1-9514-dad527bb9e36` (raw line 872). The bounded follow-up `019fcc24-b0dd-7240-a423-f435cc54a1af` and publication `019fe0c7-2bda-7ee2-9c08-47b32fa8820e` retained opt-in helpers and separate proof tiers. Original operating/benchmark records remain retrievable at `e2bfc6c:docs/orchestration/rust-test-developer-validation.md` and `e2bfc6c:docs/orchestration/ad-hoc-rust-compilation-cache.md`.
