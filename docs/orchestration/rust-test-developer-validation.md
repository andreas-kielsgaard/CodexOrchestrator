# Rust test developer validation

This repository provides an opt-in reduced-debug test lane for short-lived developer validation:

```powershell
.\scripts\cargo-test-fast.ps1 --lib --no-run --locked
.\scripts\cargo-test-fast.ps1 focused_test_name -- --nocapture
.\scripts\cargo-test-fast.ps1 -TargetDir .dev\agent-a\cargo-target --lib --locked
.\scripts\cargo-test-fast.ps1
```

The script runs ordinary `cargo test` without sccache. It owns the Codex Orchestrator manifest and
target arguments, sets `CARGO_PROFILE_TEST_DEBUG=0`, removes ambient `RUSTC_WRAPPER`, and leaves
Cargo's incremental setting and `PATH` unchanged. Its environment changes are process-scoped and
the exact prior presence and values are restored after success, Cargo failure, or exception.

This does not change `Cargo.toml` or the default test profile. Use normal Cargo with default debug
information when diagnosing failures. The reduced-debug lane is explicit; nothing routes Cargo,
CI, agents, or product workflows to it automatically.

## Returned performance evidence

The following measurements were returned by testing task
`019fcc24-b0dd-7240-a423-f435cc54a1af` at commit
`b86a8ac8f3e7483214b13e75b47397ca4df35074`. They are preserved here as returned facts, not as
independently reproduced results.

| Returned checkpoint, in reported order | Measurement |
| --- | --- |
| Default-debug cold `--no-run` | 192.842 s wall; 144.30 s before the local crate; 48.30 s local harness |
| Default-debug warm `--no-run` | 10.220 s wall; 9.24 s local unit |
| Default-debug full execution | 16.24 s; 252 passed; 8 ignored |
| Reduced-debug cold `--no-run` | 187.995 s wall; 38.74 s local unit |
| Reduced-debug artifacts | 26.455 MiB EXE; 18.410 MiB PDB; 1.687 GiB target |
| Default-debug cold artifacts | 28.597 MiB EXE; 213.879 MiB PDB; 2.841 GiB target |
| Orchestration execution | 13.02 s |
| Known Git child-process proof | approximately 5 s |

The warm no-run and full execution reused preceding target output. A fresh Cargo target does not
make package, filesystem, OS, or linker caches cold. The reduced-debug cold run occurred later than
the default-debug cold run, so their wall times are not a controlled profile comparison. Artifact
sizes are snapshots at those checkpoints, and the Git child-process duration is approximate.

## Local capability validation

No Cargo, rustc, or linker process was active, and the ignored target did not exist, before this
worktree ran:

```powershell
.\scripts\cargo-test-fast.ps1 `
  -TargetDir .dev\cargo-test-fast-validation-target `
  --lib --no-run --locked --timings
```

The command exited 0 in 185.707 seconds; Cargo recorded 186 seconds. It compiled but did not execute
tests. The fresh target contained 4,183 files totaling 1.687 GiB. The test executable was 26.455
MiB and its PDB was 18.418 MiB.

One warmed focused execution then used the same target:

```powershell
.\scripts\cargo-test-fast.ps1 `
  -TargetDir .dev\cargo-test-fast-validation-target `
  --lib --locked legacy_task_commands_are_fail_closed_in_the_reset_baseline
```

It exited 0 in 11.753 seconds; Cargo reported 8.82 seconds. One test passed, none failed or were
ignored, and 259 were filtered out. No broader Rust suite or sccache benchmark was run.

The fake-only PowerShell contract suite covers six cases: no-run arguments, focused arguments,
full-test failure, manifest ownership, target ownership, and Cargo launch exception. It verifies
reduced-debug mode, wrapper removal, unchanged incremental and PATH values, environment restoration,
and exact Cargo exit propagation.

## Explicitly deferred and non-operative

Later orchestration-product ownership may decide validation-lane selection or enforcement across
agent workflows, skills, Harness behavior, and CI. It may also define behavioral test tiers or
redesign expensive Git, SQLite, and process fixtures. This developer-tooling checkpoint implements
none of those controls or test-design changes.
