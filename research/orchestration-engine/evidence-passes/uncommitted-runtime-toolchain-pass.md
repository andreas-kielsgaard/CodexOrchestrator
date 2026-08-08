# Observation pass: uncommitted runtime toolchain experiment

> **Status: uncommitted, detached and moving evidence.** This document describes the dirty state observed on 2026-08-07 in `C:/Users/user/.codex/worktrees/430c/Codex Orchestrator`. The worktree is detached at `b86a8ac`; none of the changes below has a commit identity or named integration authority.

## Evidence boundary

The dirty tree contains 12 modified tracked files relative to `b86a8ac`:

- `docs/agent-session/README.md`
- `package.json`
- `scripts/worktree-runtime.mjs`
- `src-tauri/Cargo.toml`
- `src-tauri/Cargo.lock`
- `src-tauri/src/agent_sessions/mod.rs`
- `src-tauri/src/orchestration/application.rs`
- `src-tauri/src/orchestration/bootstrap_transition.rs`
- `src-tauri/src/runtime/codex/capabilities.rs`
- `src-tauri/src/runtime/codex/mod.rs`
- `src-tauri/src/runtime/codex/runtime.rs`
- `src-tauri/src/runtime/codex/tests.rs`

The diff is 76 insertions and 400 deletions. Nearly all deletions are dependency-lock contraction. No release product module, Tauri command, runtime launch argument, process environment, database schema or frontend experience is changed.

Validation performed without editing the source:

- `npm run test:rust:fast -- --no-run --locked` compiled successfully in 2m10s;
- `cargo test --manifest-path src-tauri/Cargo.toml --features live-tests --lib --no-run --locked` compiled successfully in 1m48s;
- default feature test discovery listed 250 tests;
- `live-tests` feature discovery listed 260 tests, exactly 10 more.

These are compile/list observations, not full deterministic execution or live Codex proof.

## What the patch attempts

This is a developer-feedback and test-compilation optimization with four related moves:

1. keep paid or installed-Codex test harnesses out of default Rust test compilation;
2. remove TLS support that current Rust test/example HTTP clients do not use;
3. offer a reduced-debug, filter-friendly Rust test lane while preserving a full lane;
4. stop the legacy Node Worktree Runtime test command from accidentally selecting Rust `worktree_runtime::` tests through substring matching.

The work changes which test code and dependencies are compiled. It does not change what the shipped application does.

## Behavior 1: compile-time ownership for live tests

`src-tauri/Cargo.toml` introduces:

```toml
[features]
default = []
live-tests = []
```

The feature is then applied only under `cfg(test)` or to test-local items.

### Exact gated artifacts

| Artifact | Feature-owned content |
| --- | --- |
| `agent_sessions/mod.rs` | the entire `live_smoke` module |
| `runtime/codex/mod.rs` | test-only public re-export of `CodexCliCapabilities` and `CodexCliCapabilityProbe` used by the live-smoke module |
| `runtime/codex/capabilities.rs` | the convenience `discover()` method used by installed-CLI tests; normal capability discovery remains compiled |
| `runtime/codex/runtime.rs` | `active_direct_child_count`, a test-only observation used by the live-smoke driver |
| `runtime/codex/tests.rs` | installed Codex help compatibility test |
| `orchestration/application.rs` | live Plan Builder notifier/harness plus three paid installed-Codex tests |
| `orchestration/bootstrap_transition.rs` | live transition notifier plus the paid Bootstrap-to-Runner test |

The ten tests added by `--features live-tests` are:

- four deterministic tests inside `agent_sessions::live_smoke::tests`;
- the ignored four-invocation Agent Session live-smoke driver;
- three ignored installed-Codex Plan Builder tests;
- one ignored installed-Codex Bootstrap/Runner transition test;
- one ignored installed-Codex CLI help compatibility test.

Default compilation therefore omits six ignored provider/compatibility tests **and four deterministic tests of the live-smoke harness itself**. The documentation compensates by adding a feature-enabled command for those deterministic harness checks before any live run.

The feature does not authorize cost or provider activity. Feature-enabled live tests remain `#[ignore]`; the Agent Session driver additionally requires `CODEX_AGENT_SESSION_LIVE_SMOKE=true`. Compilation, ignored-test selection and runtime opt-in remain separate controls.

## Behavior 2: dev-only HTTP dependency contraction

The manifest changes the dev dependency from:

```toml
reqwest = { version = "0.13", default-features = false, features = ["json", "rustls"] }
```

to JSON support without Rustls.

The lockfile consequently removes the TLS/QUIC/platform-verifier subtree, including packages such as:

- `aws-lc-rs` and `aws-lc-sys`;
- `hyper-rustls`, `rustls`, `tokio-rustls` and certificate/platform-verifier crates;
- `quinn`, `quinn-proto` and `quinn-udp`;
- `ring`, `cmake`, `jobserver` and associated platform dependencies.

`reqwest` remains available. Current call sites in orchestration MCP tests, bootstrap tests and `examples/worktree_review_controller.rs` connect to loopback `http://127.0.0.1`. The one `https://wrong.example` occurrence is an Origin-header denial fixture, not a network destination. The observed callers therefore do not need a TLS backend.

Ownership is strictly developer/test/example-side because `reqwest` is a dev dependency. Release networking, MCP serving and Codex processes do not use it. The change does, however, make future HTTPS test/example calls unavailable unless TLS support is restored deliberately.

## Behavior 3: reduced-debug Cargo profile and npm routes

`src-tauri/Cargo.toml` adds:

```toml
[profile.test-fast]
inherits = "test"
debug = "line-tables-only"
```

`package.json` exposes two repository-root commands:

- `test:rust:fast`: Cargo library tests under `--profile test-fast`;
- `test:rust:full`: ordinary Cargo library tests.

The intended implementation loop is `npm run test:rust:fast -- <module-filter>`. The module filter reduces executed tests but not compilation of the library test harness. Because `test-fast` is a distinct Cargo profile, its first run builds a separate `src-tauri/target/test-fast` artifact set; it is not a warm shortcut into ordinary `test` artifacts.

`line-tables-only` retains source-line debugging while reducing debug-information volume. The full lane remains available for richer debugging and integration-boundary evidence.

`docs/agent-session/README.md` changes the suggested matrix accordingly:

- use the fast filtered lane during implementation;
- use the full npm lane at Slice/integration boundaries;
- omit a preceding `cargo check` because `cargo test` already compiles the crate;
- compile `live-tests` explicitly for live-smoke harness verification and authorized ignored runs.

This is developer guidance, not an enforced CI, Harness, orchestration or application policy.

## Behavior 4: Node Worktree Runtime test-scope correction

`scripts/worktree-runtime.mjs` changes both its projected command description and actual child invocation from:

```text
cargo test --manifest-path src-tauri/Cargo.toml runtime::
```

to:

```text
cargo test --manifest-path src-tauri/Cargo.toml --lib runtime:: -- --skip worktree_runtime::
```

The original `runtime::` filter also substring-matches `worktree_runtime::`. The revised command:

- restricts Cargo to the library test target;
- keeps the generic `runtime::` and process-boundary selection;
- explicitly excludes the separate Rust Worktree Runtime suite;
- records the same narrowed command in the generated instance manifest that it actually executes.

The Node tool already runs its own `scripts/worktree-runtime.node-test.mjs` suite first. The exclusion avoids recursively broadening its Rust boundary check into the later Rust Worktree Runtime implementation.

This modifies only the retained Node developer tool. It does not change the release/debug Rust Worktree Review service or its test suite.

## Lineage classification

### Dirty base

The worktree was created detached at `b86a8ac` on 2026-08-04. File modification timestamps place this experiment on 2026-08-07. It is based on the then-current `main` snapshot rather than the engine-rich continuation used by the Orchestration research checkout.

### Closest committed current line

The named branch `codex/rust-build-acceleration` ends at `6b7e95e` and was committed on 2026-08-04. That line is an ancestor of the current engine-rich continuation and owns:

- `scripts/cargo-test-fast.ps1` and its contract tests;
- an opt-in `CARGO_PROFILE_TEST_DEBUG=0` lane;
- optional isolated target selection;
- removal/restoration of ambient `RUSTC_WRAPPER` state;
- explicit preservation of ordinary Cargo/CI/product behavior;
- separate optional `cargo-sccache.ps1` tooling and evidence.

The uncommitted `test-fast` profile was written later against the stale base. It is therefore not a chronological predecessor to the committed helper. It is a parallel/later alternative for the same reduced-debug goal.

### Per-part disposition

| Dirty behavior | Relationship to committed/current lines |
| --- | --- |
| reduced-debug test lane | parallel alternative, largely superseded by committed `6b7e95e` for current integration ownership |
| npm `test:rust:fast/full` aliases | unique convenience layer; not present in committed current lines |
| `live-tests` feature fence | unique uncommitted experiment; not adopted by the current engine-rich line |
| Reqwest Rustls removal | unique uncommitted dependency optimization; current line retains Rustls |
| Node Worktree Runtime filter correction | unique and still absent from the retained Node script in the current line |
| Agent Session README commands | tied to the uncommitted manifest/npm design; superseded as current guidance by the committed developer-validation document where the two disagree |

### Node versus Rust Worktree Runtime

The edited `scripts/worktree-runtime.mjs` is itself an earlier developer-runtime generation. The current codebase also contains the committed Rust `worktree_runtime/` and debug `worktree_review/` product tooling. The dirty patch does not migrate or unify them; it merely stops the Node tool's test filter from pulling the Rust Worktree Runtime suite into a generic runtime check.

The filter fix may remain technically useful if the Node tool is retained. It should not be mistaken for evidence that Node remains the desired runtime owner.

## Artifact ownership map

| Artifact | Owner / effect class | Release effect |
| --- | --- | --- |
| `src-tauri/Cargo.toml` features | Rust test compilation boundary | none unless tests are built |
| `src-tauri/Cargo.toml` test profile | developer validation performance/debug policy | none |
| `src-tauri/Cargo.toml` Reqwest feature | test/example HTTP client capability | none |
| `src-tauri/Cargo.lock` contraction | reproducible Rust dev dependency graph | no intended product behavior change |
| `agent_sessions/mod.rs` | Agent Session live-smoke compilation ownership | none |
| `orchestration/application.rs` | Plan Builder live-test fixtures | none |
| `orchestration/bootstrap_transition.rs` | Bootstrap/Runner live-test fixtures | none |
| `runtime/codex/*` | installed-CLI/live-smoke test affordances | production runtime methods remain unchanged |
| `package.json` | repository-root developer command UX | none |
| `scripts/worktree-runtime.mjs` | retained Node internal runtime/test operator | none in product composition |
| `docs/agent-session/README.md` | historical Agent Session verification guidance | none |

## Tensions and unresolved questions

### Deterministic checks move behind an opt-in feature

The feature name suggests only live/provider behavior, but four safe deterministic live-smoke contract tests also disappear from default compilation and execution. A future tier design may want separate `live-harness` and `paid-live` ownership rather than one feature.

### The stale base undercounts current live surfaces

The engine-rich line added further native-profile, transition and execution evidence after `b86a8ac`. Applying this patch conceptually to current code would require a new inventory; the observed `live-tests` fence does not automatically classify later live or controlled-live artifacts.

### The acceleration mechanisms make different tradeoffs

The dirty Cargo profile is cross-platform and simple to call through npm, retains line tables, and creates a named profile. The committed PowerShell helper uses debug level 0, can isolate the target, clears a wrapper for reproducibility and leaves the manifest unchanged. Combining them without one owner would create competing caches, commands and evidence language.

### Lockfile savings are capability removal, not only performance

The removed packages are currently unnecessary for loopback HTTP, but the manifest no longer supports HTTPS in Rust tests/examples. That should be an explicit test-tooling boundary if adopted, not an incidental lockfile cleanup.

### Historical documentation is not current test evidence

The edited README retains a 2026-07-12 claim of 84 Rust tests and two ignores while the observed dirty tree now discovers 250 default tests and 260 with the feature. Those figures describe an older checkpoint, not validation of this patch.

### A narrower Node test command does not settle Node retention

The filter correction is coherent on its own. The larger keep/archive/extract decision for `scripts/worktree-runtime.mjs` remains separate from whether this command is accurate.

## Current interpretation

The dirty state is best understood as a coherent but unintegrated test-tier experiment. Its release-risk surface is low because every source conditional is test-only and the dependency/profile changes are developer-only. Its architectural value is evidence for three future decisions:

- whether live/provider test code should be excluded from default compilation;
- whether loopback-only Rust test clients should deliberately lack TLS;
- whether fast Rust validation should be owned by Cargo profiles, repository scripts, npm aliases, or a centralized application/orchestration policy.

It should not be described as current product behavior or merged current tooling. The committed `6b7e95e` helper is the present integration-owned acceleration route; the remaining unique parts require fresh evaluation against the current engine-rich codebase.
