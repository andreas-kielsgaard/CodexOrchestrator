# Worktree-aware application runtime exploration

Status: adjacent product exploration. The prototype is developer tooling, not a parallel
orchestrator or evidence that product-owned controls exist.

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
`prepare` records the new source fingerprint.

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

- Windows teardown currently uses verified wrapper command lines plus `taskkill /T`. This is
  reversible developer tooling, but process lookup and teardown are not one atomic ownership
  operation. A product runtime needs a Job Object.
- `sccache` was not installed during implementation. Shared Rust compilation must remain reported as
  unavailable until installed and measured.
- Port slots are explicit, not leased by a durable broker.
- The Tauri override isolates active database/application data and WebView identity. Installer,
  updater, protocol registration, notification identity, and other OS-global integration were not
  exercised.
- Credential isolation is fail-closed, not credential provisioning. Product work needs an explicit
  per-instance secret source and approval policy.
- Screenshot and recording roots are isolated, but capture is not implemented.
- No product UI, durable instance registry, attention router, approval queue, or pause/resume
  controller was added.
- Local manifests are developer evidence. They are not authoritative orchestration events.
- The full frontend suite reached 601/602 tests in this checkout but one `EpicPlanBuilder` text
  lookup exceeded its one-second timeout in every full-suite run. The same file passed 10/10 in
  isolation. The prototype records this as an unrelated timing-sensitive gate; it does not weaken or
  rewrite that product test.

User review should decide whether parallel test instances may receive provider credentials, which
events deserve attention, whether stop/restart is an acceptable first pause model, and which gates
may continue automatically.

## Exact next product slice

Build a focused **Worktree Test Instance Registry and Windows Process Owner** outside legacy
`lib.rs`.

The slice should:

1. persist projected instance identity, worktree/build/session links, cache keys, paths, and port
   leases;
2. launch Vite and Tauri inside a named Windows Job Object with kill-on-close;
3. record observed process, health, build, test, stop, and recovery transitions without inferring
   success;
4. expose read/start/stop/recover application ports with explicit authority and idempotency;
5. prove two worktrees can run simultaneously, one can be stopped without affecting the other, and a
   stale owner can be recovered after application restart.

Defer parallel scheduling, automatic approvals, provider credential injection, visual capture, and
full pause/resume until that ownership and evidence slice is accepted.
