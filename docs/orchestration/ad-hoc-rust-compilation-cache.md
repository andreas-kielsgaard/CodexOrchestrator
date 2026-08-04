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
stable Cargo working directory, scoped incremental setting, and server totals after Cargo exits.

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

Only the compiler cache was shared. Simultaneous Cargo execution was not stress-tested.

The committed helper was then validated from this worktree with its default cache and two fresh,
separate targets:

```powershell
.\scripts\cargo-sccache.ps1 check --locked --timings
.\scripts\cargo-sccache.ps1 -TargetDir .dev\sccache-helper-validation-target check --locked --timings
```

The first Cargo report was 167 seconds with 375 misses. The second was 144 seconds; its server-stat
delta was 269 hits, 106 misses, and 86 non-cacheable calls. The helper also rejected an active
server configured for a different cache before Cargo started.

## Incremental policy

sccache 0.17 rejected `CARGO_INCREMENTAL=1` with `incremental compilation is prohibited`.
Incremental ordinary Cargo was also about 2.1 times faster for the measured local edit. Therefore:

- use this helper, with process-scoped incremental disablement, for fresh or short-lived parallel
  worktree validation where dependency reuse matters;
- use ordinary Cargo with its normal incremental behavior for sustained edit/recheck loops; and
- do not add global `RUSTC_WRAPPER`, `CARGO_INCREMENTAL`, Cargo config, or a shared target directory.

The cache is disposable, server statistics are machine-wide, and the installed package remains a
machine prerequisite rather than a repository dependency.
