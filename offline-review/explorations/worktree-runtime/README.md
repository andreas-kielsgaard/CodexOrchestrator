# Worktree Runtime offline review

Self-contained review package for `codex/explore-worktree-runtime`, inspected on 2026-07-17.
Nothing here calls a provider or requires a network connection.

## Five-minute review

1. Open `worktree-runtime-static-review.html` in a browser. It is a static, non-live rendering.
2. Read `REVIEW-CHECKLIST.md` and record the four product choices.
3. Use `evidence-snapshot.json` when exact branch, proof, or correction state matters.
4. Use the optional live steps below only when the local toolchain and dependencies are already
   installed and cached.

## Package contents

- `README.md` — proof, safe offline steps, boundaries, and candidate product slice.
- `REVIEW-CHECKLIST.md` — review and safety checklist.
- `worktree-runtime-static-review.html` — local static visual; no scripts or external assets.
- `worktree-runtime-static-review.png` — rendered 1280-pixel-wide preview of the static visual.
- `evidence-snapshot.json` — machine-readable evidence and current-state distinctions.

## Current state truth

| Item                               | State                                                                       |
| ---------------------------------- | --------------------------------------------------------------------------- |
| Branch                             | `codex/explore-worktree-runtime`                                            |
| Current committed head             | `4e5027c`                                                                   |
| Exploration commits                | `c25239f`, `673ddf3`, `35587bb`, `4e5027c`                                  |
| Worktree state at final inspection | Clean                                                                       |
| Prepare/fingerprint corrections    | Accepted after Epic review at `4e5027c`                                     |
| Offline correction validation      | `npm run test:worktree-runtime`: 7/7 passed                                 |
| Current ignored `proof-a` manifest | Fresh prepare at `35587bb`, dirty source, no build/test/launch observations |
| Historical two-worktree proof      | Recorded in the committed exploration document                              |

Do not use the current `proof-a` manifest as evidence of the earlier build/test/launch. The proof
below comes from the recorded run against code commit `673ddf3`.

## What the exploration proved

Two Git worktrees, `proof-a` and `proof-b`, used the same committed source and cache keys while
keeping runtime identity and mutable material separate.

- Both debug Tauri builds and focused test actions completed with exit code `0`.
- `proof-a` used Vite/status ports `1640`/`41635`; `proof-b` used `1660`/`41655`.
- Each status owner matched its instance, session, worktree, and commit.
- Each process tree contained its own `codex-orchestrator.exe`.
- Dist, Vite cache, Cargo target, app data, credentials, logs, screenshots, and recordings used
  different instance roots.
- Stopping `proof-a` left `proof-b` healthy and owned.
- Final teardown left zero recorded roots and closed endpoints.
- A separate stale drill observed a dead wrapper while the app remained live, then `recover`
  returned the instance to zero processes.

The proof is incremental developer tooling, not a parallel orchestrator.

## In-app demonstration tab

The development composition supplies one peer application tab named **Worktree Runtime**, alongside
Orchestration and Agent Sessions. It shows:

- worktree, build fingerprint, session, commit, and Tauri identity;
- isolated versus shared-keyed material;
- lifecycle and health evidence;
- teardown/recovery evidence;
- unsupported product boundaries and four review choices.

Product composition does not supply the tab. The tab is inspect-only; it does not register, start,
stop, approve, or recover instances.

## Safest plane workflow: static review

No terminal or local server is required:

1. Open this directory in File Explorer.
2. Double-click `worktree-runtime-static-review.html`.
3. Review it together with `REVIEW-CHECKLIST.md`.

The static view is deliberately labeled **recorded**. It does not claim current health.

## Optional live local review

Use this only if Node/npm, Rust, the Visual Studio C++ toolchain, Tauri prerequisites,
`node_modules`, Cargo dependencies, and WebView2 are already present. Do not run `npm ci` on the
plane: an incomplete npm cache may attempt network access. If a build reports a missing dependency,
stop and use the static review.

From the exploration worktree:

```powershell
Set-Location 'C:\Users\user\.codex\worktrees\1799\Codex Orchestrator'

# Confirm the proposed ports are not already listening. No output is the desired result.
Get-NetTCPConnection -State Listen -LocalPort 2040,42035 -ErrorAction SilentlyContinue

npm run runtime:worktree -- prepare --instance plane-review --session offline-review --slot 31
npm run runtime:worktree -- build --instance plane-review
npm run runtime:worktree -- test --instance plane-review
npm run runtime:worktree -- start --instance plane-review
npm run runtime:worktree -- status --instance plane-review
```

Then open **Worktree Runtime** in the Tauri window. Do not open Agent Sessions or send a message.
When finished:

```powershell
npm run runtime:worktree -- stop --instance plane-review
npm run runtime:worktree -- status --instance plane-review
```

Expected final status: zero processes, status/Vite health unavailable, and `stale: false`. If the
instance is stale, use:

```powershell
npm run runtime:worktree -- recover --instance plane-review
```

Do not re-run `prepare` over a live or stale instance. The accepted correction at `4e5027c` is
intended to refuse that operation.

## Isolation and reuse

| Material                       | Boundary                                                      | Evidence                            |
| ------------------------------ | ------------------------------------------------------------- | ----------------------------------- |
| Git source and `node_modules`  | Worktree-local                                                | Projected and exercised             |
| npm download cache             | Shared only under lock/toolchain/OS key                       | Projected and exercised             |
| Rust output                    | Instance-local `CARGO_TARGET_DIR`                             | Observed                            |
| Rust compilation cache         | Shared only when keyed `sccache` is available                 | Unsupported on proof machine        |
| Vite cache and dist            | Instance-local                                                | Observed                            |
| Database and application state | Instance-local app-data override                              | Projected and exercised             |
| Ports                          | Explicit strict slot mapping                                  | Observed                            |
| Processes                      | Manifest roots plus observed descendant tree                  | Observed, non-atomic                |
| Logs/screenshots/recordings    | Instance-local directories                                    | Paths observed; capture unsupported |
| Credentials                    | Empty instance `CODEX_HOME`; known ambient variables scrubbed | Projected and exercised             |

Cache keys prevent reuse across declared dependency/toolchain changes. They do not make mutable
application state shared.

## Evidence vocabulary

- **Projected** — requested configuration, path, port, or action. It is not completion.
- **Observed** — verified during the live proof, such as matching health or a completed test.
- **Recorded** — retained historical evidence; it may not describe the current machine state.
- **Unsupported** — the prototype does not provide the capability.

The static HTML turns prior observations into recorded evidence. The optional live tab can show
current owner matching as observed.

## Bounded corrections implemented now

These changes were accepted after Epic review at `4e5027c`:

1. **Safe re-prepare**
   - Reads and observes an existing manifest before replacement.
   - Rejects mismatched identity/worktree routes.
   - Refuses live owned roots, live unowned roots, endpoint-only live state, and stale state.
   - Allows a clean stopped instance.
2. **Complete untracked fingerprint**
   - Uses Git porcelain with `--untracked-files=all`.
   - Hashes each regular untracked file and fails closed on unreadable/non-regular entries.
   - Includes a real temporary-Git-repository test proving nested untracked content changes alter
     the fingerprint.

The harness test file now contains seven cases, including safe/refused re-prepare and nested
untracked invalidation. All seven passed offline during package creation. Treat this as implemented
accepted correction evidence.

## Safety limits

- Detached wrappers start before their PID ownership route is persisted. That interval is not
  crash-atomic and can leave a tree the manifest cannot recover.
- Windows ownership lookup plus `taskkill /T` is not atomic. Product work needs a Job Object.
- Port slots are explicit, not durably leased.
- `sccache` was unavailable, so shared Rust compilation was not proved.
- Provider credentials are isolated but not provisioned.
- Installer, updater, protocol registration, notifications, and other OS-global identity were not
  exercised.
- Screenshot/recording directories exist; capture is not implemented.
- Pause means stop at a safe boundary and explicitly restart. Suspension/resumable actions are
  unsupported.
- The roughly 1,000-line JS harness is disposable exploration tooling, not the future product
  module structure.
- Direct Tauri-window capture remains a manual gate because Windows was locked. The indirect
  1280×820 live render and both titled Tauri windows were accepted as sufficient exploration
  evidence.

## Product choices

1. May parallel test instances receive provider credentials?
2. Which failures, health changes, or approval gates deserve attention?
3. Is stop plus explicit restart an acceptable first pause model?
4. Which observed gates may continue automatically?

## Candidate next slice

Build a focused **Worktree Test Instance Registry and Windows Process Owner** outside legacy
`lib.rs`, using separate identity, registry, cache, launch, ownership, health, and recovery modules.
Do not extend the exploration script into product infrastructure.

The slice should:

1. persist projected instance/worktree/build/session identity, cache keys, paths, and port leases;
2. launch Vite and Tauri in a named Windows Job Object with kill-on-close;
3. record observed build, test, launch, health, stop, and recovery transitions;
4. expose explicit, idempotent read/start/stop/recover application ports;
5. prove two simultaneous instances, isolated stop, and stale recovery after application restart.

Defer parallel scheduling, automatic approvals, provider credential injection, visual capture, and
full pause/resume until ownership and evidence are accepted.
