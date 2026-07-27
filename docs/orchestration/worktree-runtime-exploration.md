# Worktree-aware application runtime exploration

Status: developer-tooling proof plus an uncomposed product ownership core. Neither is a parallel
orchestrator, and the core is not evidence that product UI or scheduling controls exist.

## Current architecture evidence

- Vite used one fixed port (`1420`), one `dist`, and its default cache.
- Tauri used one identifier, one development URL, and the shared `dist`.
- The active Tauri composition resolved one application-data directory and opened the active
  database and orchestration materials below it.
- The development status server used one port (`41415`) and one `.dev/runtime-status.json`.
- The active Rust process supervisor owns and reaps direct children. Its module contract correctly
  states that portable `Child::kill` does not own a Windows descendant tree.
- Git worktrees isolate tracked files, `HEAD`, and index state, but share repository metadata. This
  does not isolate ignored dependencies, build output, ports, processes, or application state.

## Minimal direction

`scripts/worktree-runtime.mjs` creates one ignored manifest under
`.dev/worktree-runtime/<instance>/manifest.json`. The manifest names the worktree, commit, dirty
source fingerprint, branch, session, Tauri identifier, strict ports, paths, cache keys, projected
commands, and observed lifecycle facts.

The lifecycle is explicit:

1. `prepare` computes identity, cache keys, paths, ports, and a Tauri config overlay.
2. `install` restores worktree-local `node_modules` through a keyed shared npm download cache.
3. `build` compiles one debug Tauri application into the instance Cargo target and frontend output.
4. `test` runs focused worktree-runtime, status-client, and Rust runtime/process suites against that
   instance target.
5. `start` launches owned status and Tauri wrapper processes.
6. `status` compares projected identity with owned PIDs, descendant processes, status ownership,
   Vite health, and the observed Tauri executable.
7. `stop` tears down only wrapper trees whose command lines prove the manifest and role.
8. `recover` clears stale launch state only after the same ownership check.

The development composition also adds one peer **Worktree Runtime** application tab. It consumes a
small `WorktreeRuntimeExplorationSource` contract and is absent from product composition. The live
development adapter combines launch-time manifest metadata with the current status owner. It labels
manifest paths as projected, completed build/test and matching health as observed, the earlier
teardown drill as recorded, and absent product controls as unsupported.

Example:

```powershell
npm run runtime:worktree -- prepare --instance worker-a --session session-a --slot 11
npm run runtime:worktree -- install --instance worker-a
npm run runtime:worktree -- build --instance worker-a
npm run runtime:worktree -- test --instance worker-a
npm run runtime:worktree -- start --instance worker-a
npm run runtime:worktree -- status --instance worker-a
npm run runtime:worktree -- stop --instance worker-a
```

## Isolation and reuse

| Concern                        | Prototype boundary                                                                |
| ------------------------------ | --------------------------------------------------------------------------------- |
| Source and modules             | Git worktree plus worktree-local `node_modules`                                   |
| Vite cache and dist            | Instance-local paths selected by `WORKTREE_RUNTIME_ROOT`                          |
| Rust outputs                   | Instance-local `CARGO_TARGET_DIR`                                                 |
| Database and application files | Instance-local `CODEX_ORCHESTRATOR_APP_DATA_DIR`                                  |
| WebView and Tauri identity     | Unique config-overlay identifier and window title                                 |
| Ports                          | Explicit slot mapped to strict Vite and status ports                              |
| Processes                      | Manifest-named wrapper PIDs and observed descendant tree                          |
| Logs and evidence              | Instance-local logs, screenshots, and recordings directories                      |
| Credentials                    | Empty instance-local `CODEX_HOME`; known ambient provider credentials are removed |

The shared npm cache key includes `package-lock.json`, Node, npm, OS, and architecture. npm's cache
is content-addressed and integrity checked; `node_modules` remains isolated.

The optional Rust cache key includes `Cargo.lock`, `Cargo.toml`, `rustc -vV`, `RUSTFLAGS`, profile,
OS, and architecture. When `sccache` is available, its cache is shared only inside that key. Without
it, the manifest records `isolated-target-only`; it does not imply shared Rust compilation.

Source edits do not reuse a prepared identity silently. Build, test, and launch fail until
`prepare` records the new source fingerprint. The fingerprint asks Git to enumerate every untracked
file, hashes each regular file's content, and fails closed when an untracked entry cannot be read as
a regular file.

## Executed two-worktree proof

On 2026-07-17, `proof-a` and `proof-b` used the same committed source
(`673ddf321d230f4ff497b5603efef42208d30dc4`) in two Git worktrees.

- Both selected the same Node and Rust cache keys. The npm download cache was shared by key;
  `sccache` was unavailable, so both manifests truthfully selected isolated Cargo targets.
- Concurrent installs completed on the harness commit. After the peer view was added without
  dependency-lock changes, concurrent debug Tauri builds and focused test actions completed with
  exit code `0` for both instances on the final proof commit.
- Both launched simultaneously. `proof-a` owned Vite/status ports `1640`/`41635`; `proof-b` owned
  `1660`/`41655`. Each status owner matched its instance, session, worktree, and commit, and each
  process tree contained its own `codex-orchestrator.exe`.
- Dist, Cargo target, application data, credentials, logs, screenshots, and recordings resolved to
  distinct instance roots.
- After stopping `proof-a`, `proof-b` remained healthy with two owned wrapper roots and its Tauri
  process. Final teardown left both manifests with zero processes and both port pairs closed.
- A separate stale-instance drill killed one owned status tree while its application stayed live.
  `status` reported stale, `recover` stopped the remaining owned tree, and the final observation had
  zero processes and closed endpoints.

Focused validation covered 7 Node harness tests, 2 development-source tests, 2 application-surface
tests, 15 application status tests, 2 Rust app-data override tests, TypeScript, ESLint, formatting,
the production frontend build, and the per-instance Rust runtime/process test action. The production
composition does not supply the peer view; its feature implementation is excluded from the
production bundle.

## Visual inspection

The running Tauri processes exposed separate windows titled `Codex Orchestrator [proof-a]` and
`Codex Orchestrator [proof-b]`. The Windows desktop was locked, so safe interaction with and capture
of the Tauri window itself was unavailable.

The same live `proof-a` application URL was rendered non-interactively at the intended 1280 by 820
window size. The peer tab showed the correct instance, build, session, commit, worktree, and Tauri
identity; shared-keyed and isolated material were visually distinct; lifecycle evidence separated
observed, projected, and recorded states; and the unsupported boundaries and four user-review
decisions were readable. The live `proof-b` view independently reported `proof-b`, `session-b`, its
second worktree, and its distinct Tauri identity. A direct Tauri-window screenshot remains a manual
review gate when the desktop is unlocked.

## Human-control model

The manifest separates requested or projected work from observed facts. A future control surface
can show both without turning a launch request into launch evidence.

- Attention routes on a changed fact: failed build/test, ownership mismatch, unhealthy endpoint,
  stale instance, or an approval requirement. Healthy parallel instances need no repeated prompt.
- Launch and credential provisioning remain explicit user or policy decisions. Teardown is automatic
  only after ownership proof; an unowned PID blocks rather than broadening kill authority.
- Current pause is stop-at-a-safe-boundary, followed by explicit restart. Process suspension and
  resumable application actions are not implemented.
- Intervention should target one named instance and expose its logs and evidence roots. It must not
  infer authority over another worktree or session.
- Automatic continuation may cross a gate only when the required observed facts exist and no
  approval, pause, or intervention is pending.

## Prior art and constraints

- [Codex worktrees](https://learn.chatgpt.com/docs/environments/git-worktrees) describe parallel
  worktree chats, setup, handoff, and cleanup. The public material does not establish a concurrent
  multi-instance Tauri harness or its safety properties.
- [Git worktrees](https://git-scm.com/docs/git-worktree.html) share repository metadata while
  maintaining per-worktree `HEAD` and index state.
- [npm cache](https://docs.npmjs.com/cli/cache/) is content-addressed and integrity checked, but is a
  cache rather than durable state.
- [Cargo build cache](https://doc.rust-lang.org/cargo/reference/build-cache.html) supports isolated
  target directories and recommends `sccache` for shared compilation.
- [Vite build](https://vite.dev/config/build-options.html) and
  [server](https://vite.dev/config/server-options) options provide explicit output and strict-port
  boundaries.
- [Tauri CLI configuration](https://v2.tauri.app/reference/cli/) supports ordered config overlays;
  the [Tauri identifier](https://v2.tauri.app/reference/config/) must be unique because it participates
  in system and WebView data identity.
- [Windows Job Objects](https://learn.microsoft.com/en-us/windows/win32/procthread/job-objects)
  provide the durable process-tree ownership model: grouped accounting and whole-job termination.

## Prototype limits and review points

- The prototype still uses verified wrapper command lines plus `taskkill /T`; its process lookup and
  teardown are not atomic. The uncomposed Rust core has a separately tested named-Job-Object
  replacement.
- Prototype launch is not crash-atomic. The Rust core separately proves route-before-launch and
  assign-suspended-before-resume ordering; it has not launched a real Tauri build through the new
  facade.
- `scripts/worktree-runtime.mjs` began as a 981-line prototype. It is disposable exploration
  tooling, not the module structure for a product registry or process owner.
- `sccache` was not installed during implementation. Shared Rust compilation must remain reported as
  unavailable until installed and measured.
- Prototype port slots are explicit. The Rust core separately proves durable SQLite leases.
- The Tauri override isolates active database/application data and WebView identity. Installer,
  updater, protocol registration, notification identity, and other OS-global integration were not
  exercised.
- Credential isolation is fail-closed, not credential provisioning. Product work needs an explicit
  per-instance secret source and approval policy.
- Screenshot and recording roots are projected per instance, but capture is not implemented.
- The Rust core contains a durable instance registry and semantic application facade, but neither is
  composed into product boot. There is no product UI, attention router, approval queue, or
  pause/resume controller.
- Local manifests remain developer evidence. The uncomposed registry proves local lifecycle
  persistence; neither source is an orchestration event stream.
- The full frontend suite reached 601/602 tests in this checkout but one `EpicPlanBuilder` text
  lookup exceeded its one-second timeout in every full-suite run. The same file passed 10/10 in
  isolation. The prototype records this as an unrelated timing-sensitive gate; it does not weaken or
  rewrite that product test.

User review should decide whether parallel test instances may receive provider credentials, which
events deserve attention, whether stop/restart is an acceptable first pause model, and which gates
may continue automatically.

## Rust ownership checkpoint on this branch

The bounded candidate core lives under `src-tauri/src/worktree_runtime/`, outside legacy `lib.rs`.
Only its module registration was added to `lib.rs`. Ordinary `npm run dev`, `npm run build:tauri`,
the production Tauri identifier, development URL, frontend output, and bundle default remain
unchanged.

### Verified in the focused Rust proof

- `SystemSourceInspector` fingerprints the commit, tracked binary diff, every Git-enumerated
  untracked regular file, Node/package-lock state, and Rust/Cargo state. A nested untracked content
  change changes the identity. The facade re-inspects before build, test, and start and refuses a
  changed source or toolchain.
- Per-instance projections isolate frontend output, Cargo target, application data, credentials,
  temporary files, logs, evidence, screenshots, recordings, status state, and strict port pairs.
  Worktree-local `node_modules` is not shared. A package-lock/Node/OS keyed npm download cache is
  shared when its directory is usable; otherwise Node falls back to an instance cache. Rust
  compilation and Cargo home remain instance-local because no measured compiler cache is available.
- The SQLite registry uses immediate transactions for exact identity, hashed authority, two-port
  leases, idempotent lifecycle commands, durable ownership routes, and observed transitions.
- One partial unique index and the same immediate reservation transaction allow only one pending
  start/stop/recover command per instance. The registry holds a path-keyed Windows named mutex so a
  second live application execution cannot misclassify an executing start as abandoned. Within
  that lease, process-lifetime start ownership makes stop/recover return `OperationInProgress`
  while launch is executing. After the OS releases the lease on exit, restart recovery atomically
  fails the abandoned start before reserving recovery; ordinary stop refuses that repair path.
- Focused facade tests hold start or stop execution open across the real registry boundary. They
  verify start/stop, stop/stop, stop/recover, and interrupted-start recovery behavior, including
  the required semantic errors and zero pending commands after the owning operation completes.
- The Windows owner creates three helper roots suspended, assigns them to one exact named Job Object
  with kill-on-close, and resumes only after assignment. Its integration proof starts two isolated
  jobs, stops one while the other remains healthy, drops the owner, reopens SQLite, and recovers the
  stale record.
- Start becomes `Running` only after the exact job is active and both projected TCP endpoints are
  reachable. Stop/recover complete only after the job is absent and both endpoints are closed.
- The system action executor runs with an explicit cleared environment, writes an instance action
  log, and returns semantic pass/fail plus the failed step.
- The semantic facade proof requests two sources, receives opaque handles, projects distinct mutable
  state and ports, uses the same safe Node cache key, runs build/test through an injected executor,
  starts both through an injected lifecycle port, and stops one without changing the other.

The focused command is:

```powershell
cargo test --manifest-path src-tauri\Cargo.toml worktree_runtime -- --nocapture
```

Its current expected result is 16 passed and 2 ignored helpers. The original developer script,
not this Rust facade, remains the evidence for two real concurrent Tauri builds/windows.

Checkpoint validation on 2026-07-27 also passed `cargo fmt --check`, `cargo check`, the ordinary
`npm run build`, and `npm run build:tauri -- --debug --no-bundle`. The latter used the unchanged
production Tauri config and produced the normal debug application binary. Existing unrelated
dead-code warnings and the pre-existing macOS `.app` identifier warning remain.

### Intended but not yet established

- No product composition supplies the registry location, authority secret, semantic source resolver,
  or facade instance. There is no Tauri command or UI route to this core.
- Build/test plans point at the exact worktree and isolated outputs, but the focused suite does not
  execute those real Node, Vite, Cargo, or Tauri plans. Worktree-local dependencies must already
  exist; dependency restoration and its lock-to-modules proof are not owned yet.
- Build/test outcomes and logs are synchronous results, not durable registry evidence. Launched
  Vite/status/Tauri stdout and stderr are not redirected to the projected log root.
- TCP reachability does not prove that the endpoint belongs to the named job. The preflight and
  strict ports fail closed for normal collisions, but an untrusted local bind race remains.
- Source reinspection narrows but cannot eliminate the edit-after-check race during a running
  build/test command.
- Port leases are durable and never silently reassigned, but pruning/releasing retired instance
  records is not implemented.
- Unique Tauri configuration and projected application paths are planned, but database/WebView
  isolation has not been re-proven through this Rust facade.

## Review/test host integration seam

A review driver should receive an application-composed `Arc<dyn WorktreeTestInstances>`. It should
not construct the concrete facade or import planning, registry, projection, ownership, or process
types.

1. Create `TestSourceRef` from an application-owned source route and call
   `request(IsolatedTestRequest { source, purpose })`.
2. Retain the returned opaque `TestInstanceHandle`.
3. Call `build` or `test` and consume only `TestActionResult`: `Passed` or `Failed`, optional failed
   step, and current semantic status.
4. Call `start`, `status`, `stop`, or `recover` and consume only `TestInstanceStatus`: lifecycle
   phase, `NotObserved`/`Healthy`/`Unhealthy`/`Closed` health, and stale ownership/endpoint state.
   Concurrent terminal callers may instead receive `OperationInProgress` for the already-running
   operation or `Conflict` for the opposite operation; refresh status rather than constructing a
   second low-level command.

That seam intentionally supplies no worktree path, port, cache path, Tauri identifier, manifest,
Job Object name, authority secret, or raw launch description. Detailed durable build/test evidence
and evidence-file access are future application ports, not fields for this lifecycle facade.

## Next bounded product slice

Compose the semantic facade behind an application-owned source resolver, registry location, and
durable authority source, then:

1. bind status/Vite endpoint identity to the exact owned job;
2. own dependency restoration and prove its lock-to-worktree-local-modules invariant;
3. execute two real worktree builds/tests/launches through the facade and persist observed outcomes;
4. add the smallest review-host adapter while keeping ordinary development and packaging defaults
   unchanged.

Defer parallel scheduling, automatic approvals, provider credential injection, visual capture, and
full pause/resume until that ownership and evidence slice is accepted.
