# Ad-hoc Rust compilation cache

This checkpoint adds an opt-in compiler-cache route for short-lived Codex Orchestrator worktrees.
It does not change ordinary Cargo, Tauri, or product build behavior.

## Installation and rollback

On 2026-08-04, WinGet installed Mozilla's prebuilt x64 `sccache` 0.17.0 ZIP for the
current user:

```powershell
winget install --exact --id Mozilla.sccache --scope user
```

WinGet verified the archive SHA-256
`e94cfc5b58cbe439302f586c1d1bd7980c2cd371d47bdf385ade657411e6f3ac` and added
the package directory below `%LOCALAPPDATA%\Microsoft\WinGet\Packages` to the persisted user
`PATH`. A reconstructed process environment resolved `sccache.exe` there and reported
`sccache 0.17.0`.

Rollback is explicit:

```powershell
sccache --stop-server
winget uninstall --exact --id Mozilla.sccache --scope user
```

The optional cache data is `%LOCALAPPDATA%\Mozilla\sccache\cache`. Remove that exact directory
separately only when its cached artifacts are no longer wanted.

## Use

From any Codex Orchestrator worktree:

```powershell
.\scripts\cargo-sccache.ps1 check --locked --timings
.\scripts\cargo-sccache.ps1 test worktree_runtime -- --nocapture
```

The default target remains that worktree's `src-tauri\target`. For an already-isolated runtime
target, use `-TargetDir`:

```powershell
.\scripts\cargo-sccache.ps1 -TargetDir .dev\runtime-a\cargo-target check --locked
```

The helper fails if `sccache` is missing, older than 0.17.0, cannot report statistics, or an
existing server is using a different cache directory. It prints the resolved cache, target,
stable Cargo working directory, scoped incremental setting, and executable path. After Cargo
exits, it prints before/after counter deltas for hits, misses, non-cacheable requests,
non-cacheable compilations, and cacheable-request hit rate. These are global server deltas for the
command window and may include concurrent machine activity; the helper never zeroes global stats.

If `sccache` is absent from the inherited process `PATH`, the helper searches persisted user and
machine `PATH` entries and calls the discovered executable by absolute path. It does not replace or
modify the process `PATH`, so inherited Cargo, Codex shims, and Visual Studio developer tools remain
available to Cargo and rustc.

## Why the helper is needed

For Rust, sccache 0.17 hashes the compilation working directory and almost every `CARGO_*`
environment value. Setting only `RUSTC_WRAPPER=sccache` produced no reuse between different
absolute worktree or target paths in this proof. `SCCACHE_BASEDIRS` did not normalize those Rust
key inputs.

The helper therefore:

- runs Cargo from `%LOCALAPPDATA%\CodexOrchestrator\cargo-sccache-cwd` in every worktree;
- passes an absolute manifest and per-worktree target through Cargo's `--target-dir` option;
- removes `CARGO_TARGET_DIR` from rustc's environment;
- sets `RUSTC_WRAPPER`, `SCCACHE_CLIENT_SIDE=1`, and `CARGO_INCREMENTAL=0` only in the helper
  process; and
- uses the shared local sccache directory while leaving build outputs separate.

The local product crate can still miss because its `CARGO_MANIFEST_DIR` differs, and its configured
crate types are not cacheable. Registry dependencies are reusable; that is the measured benefit.

## Benchmark evidence

All commands ran sequentially with Rust/Cargo 1.96.1. Each full run used a fresh target and
`cargo check --locked --timings`. The controlled cross-path pair used identical archived source,
one shared cache, the stable Cargo working directory, CLI `--target-dir`, and
`CARGO_INCREMENTAL=0`.

| Checkpoint | Wall time | Hits | Misses | Non-cacheable calls | Result |
| --- | ---: | ---: | ---: | ---: | --- |
| No-wrapper fresh-target baseline, before cache integration | 143.128 s | n/a | n/a | n/a | passed |
| Cold controlled workspace E | 414.090 s | 0 | 375 | 86 | passed |
| Warm fresh workspace F, different absolute source and target | 351.619 s | 269 | 106 | 86 | passed |
| Negative control: different target through `CARGO_TARGET_DIR` | 146.800 s | 0 | 375 | 86 | passed, no reuse |
| sccache with `CARGO_INCREMENTAL=1` | 1.926 s | 0 | 0 | 0 | failed clearly |
| Ordinary Cargo incremental local edit | 14.009 s | n/a | n/a | n/a | passed |
| sccache non-incremental equivalent local edit | 29.256 s | 0 | 0 | 2 | passed |

The successful second-workspace run had a 71.73% Rust hit rate and was 62.471 seconds (15.1%)
faster than its controlled cold pair. It remained slower than the earlier no-wrapper baseline;
later runs experienced substantial machine contention, including Cargo package-cache lock waits,
so these wall times are checkpoints rather than a universal speedup claim.

Cargo timing reports and logs remain ignored, local evidence below
`.dev\sccache-benchmark-20260804`. The controlled targets were distinct:

- `workspace-e\src-tauri\target`
- `workspace-f\src-tauri\target`

Only the compiler cache was shared. Simultaneous Cargo execution was not stress-tested in the
original checkpoint; the hardening follow-up below covers it.

The committed helper was then validated from this worktree with its default cache and two fresh,
separate targets:

```powershell
.\scripts\cargo-sccache.ps1 check --locked --timings
.\scripts\cargo-sccache.ps1 -TargetDir .dev\sccache-helper-validation-target check --locked --timings
```

The first Cargo report was 167 seconds with 375 misses. The second was 144 seconds; its server-stat
delta was 269 hits, 106 misses, and 86 non-cacheable calls. The helper also rejected an active
server configured for a different cache before Cargo started.

## Hardening follow-up evidence

The follow-up started from commit `b395d9ff6cc077fb4db655a9202c1e61eda504c4`. It did not zero
server statistics, change Rust code or profiles, share a target directory, or run Rust tests.
Ignored logs, Cargo timings, independent clone worktrees, and fresh targets remain under
`.dev\sccache-hardening-20260804`.

### Deterministic helper checks

The focused test uses executable stubs for Cargo, `link.exe`, and controlled sccache counters:

```powershell
.\scripts\cargo-sccache.tests.ps1
```

All three checks passed:

1. With sccache removed from inherited `PATH` but present in persisted user `PATH`, the helper
   reported the persisted absolute executable. Stub Cargo observed the exact inherited `PATH`, and
   a current-only `link.exe` stub remained executable.
2. Deterministic counters produced deltas of 3 hits, 2 misses, 3 non-cacheable requests, and 1
   non-cacheable compilation, with a 60.00% hit rate.
3. A simulated active-cache mismatch failed before the Cargo stub ran.

The current shell did not expose a real `link.exe` through `Get-Command`, so a Visual Studio
developer shell was not launched as a separate test. Exact PATH equality inside stub Cargo proves
the helper preserves such current-only entries; the real concurrent and benchmark builds also
completed MSVC linking successfully.

### Simultaneous worktree validation

Two new independent clone worktrees at the baseline commit received the hardening script and ran
at the same time:

```powershell
# Started together in separate PowerShell processes.
<worktree-a>\scripts\cargo-sccache.ps1 check --lib --locked --timings
<worktree-b>\scripts\cargo-sccache.ps1 check --lib --locked --timings
```

| Pair result | Worktree A | Worktree B |
| --- | ---: | ---: |
| Cargo timing duration | 212 s | 210 s |
| Target files | 3,596 | 3,596 |
| Package-cache lock messages | 2 | 2 |
| Cargo result | `Finished` | `Finished` |

Pair wall time was 214.979 seconds. Both Cargo logs had no `error:` line, produced independent
timing reports, and ended in `Finished`; no Cargo or rustc process remained. The launcher did not
retain numeric child exit codes, so correctness rests on those Cargo artifacts rather than a
reconstructed exit-code claim.

The authoritative global sccache delta belongs to the overlapping pair, not either helper output:

| Hits | Misses | Non-cacheable requests | Non-cacheable compilations | Hit rate |
| ---: | ---: | ---: | ---: | ---: |
| 538 | 212 | 170 | 0 | 71.73% |

Both commands used `%LOCALAPPDATA%\CodexOrchestrator\cargo-sccache-cwd` and the shared default
cache, but their output roots were distinct absolute directories:

- `worktree-a\src-tauri\target`
- `worktree-b\src-tauri\target`

The only observed contention was Cargo's shared package-cache lock, twice in each log. It delayed
startup but did not serialize the separate targets or interfere with either result.

### One representative no-run benchmark

Exactly one representative command shape was compared: `cargo test --lib --no-run --locked
--timings`. Both legs ran sequentially from the stable Cargo cwd, with identical source and
`CARGO_INCREMENTAL=0`, no other observed Cargo/rustc process, and a fresh separate target. The
baseline removed `RUSTC_WRAPPER`; the helper leg used:

```powershell
<worktree-b>\scripts\cargo-sccache.ps1 -TargetDir <fresh-helper-target> `
  test --lib --no-run --locked --timings
```

| Mode | Wall time | Hits | Misses | Non-cacheable requests | Hit rate | Result |
| --- | ---: | ---: | ---: | ---: | ---: | --- |
| No wrapper | 191.127 s | 0 | 0 | 0 | n/a | passed, executable produced |
| Helper, cache prewarmed by `cargo check` | 257.627 s | 40 | 286 | 88 | 12.27% | passed, executable produced |

The helper was 66.500 seconds (34.79%) slower. The prior `cargo check` cache did not broadly match
the `cargo test --no-run` code-generation and linking arguments, so this was mostly a cold-key-shape
run rather than a warm reuse proof. No favorable third run was added. Cargo timing recorded 458
units in each leg. The local library test unit started at 147.83 seconds and took 43.06 seconds in
the baseline; with the helper it started at 209.29 seconds and took 47.27 seconds. `--no-run`
performed no test execution. Each result emitted a 29,935,616-byte (28.55 MiB) test executable and
an approximately 212.4 MiB PDB, showing that substantial target-local MSVC link/debug output
remains outside the useful compiler-cache hit path.

### Non-cacheable classification and stop recommendation

For each concurrent `cargo check`, Cargo timing showed 45 build-script compilation units and 29
resolved proc-macro units. Together with the local library's `staticlib`, `cdylib`, and `rlib`
configuration, these account for the 75 `crate-type` refusals per worktree. The no-run benchmark
added 78 `crate-type`, 5 `missing input`, and 5 opaque `-` reasons; its timing had 48 build-script
compilation units, the same 29 proc macros, and the local test library. The standard logs cannot
map the five `missing input` or five `-` entries to exact rustc probe commands. Eight compile-fail
counters during the successful concurrent pair and four during the successful no-run command are
consistent with non-fatal compiler probes, but exact attribution remains unproven.

Stop here: retain the existing policy. Use ordinary Cargo with incremental compilation for
iterative edit/recheck loops. Use the helper for fresh or short-lived worktree validation when the
same Cargo command/profile shape is likely already represented in the shared cache. Do not infer a
universal speedup, automatically route all Cargo through the helper, change profiles, or add a
shared target. Warm cross-worktree `cargo test --lib --no-run` reuse remains unmeasured.

## Incremental policy

sccache 0.17 rejected `CARGO_INCREMENTAL=1` with `incremental compilation is prohibited`.
Incremental ordinary Cargo was also about 2.1 times faster for the measured local edit. Therefore:

- use this helper, with process-scoped incremental disablement, for fresh or short-lived parallel
  worktree validation where dependency reuse matters;
- use ordinary Cargo with its normal incremental behavior for sustained edit/recheck loops; and
- do not add global `RUSTC_WRAPPER`, `CARGO_INCREMENTAL`, Cargo config, or a shared target directory.

The cache is disposable, server statistics are machine-wide, and the installed package remains a
machine prerequisite rather than a repository dependency.
