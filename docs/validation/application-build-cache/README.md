# Application build and cache validation

Validated on Windows on 2026-09-14 in branch `refinement/application-build-cache`, worktree `C:\Users\user\.codex\worktrees\build-cache-refinement`, based on `28ad076`.

The shared Node tool now owns application compilation and Cargo cache selection for agent commands and Worktree Review. Runnable application copies are independent of persistent compiler targets. Normal builds use release mode; debugging is an explicit confirmation choice. Building and launching are separate.

## Automated checks

| Check                                               | Result                               |
| --------------------------------------------------- | ------------------------------------ |
| Build-tool and PowerShell entrypoint tests          | 20 passed                            |
| Worktree Review frontend and native-client tests    | 25 passed                            |
| Rust `worktree_` tests under `test-fast`            | 66 passed                            |
| TypeScript checking and Vite production compilation | Passed as part of application builds |
| Focused frontend ESLint                             | Passed                               |
| Normal and debugging Tauri builds                   | Passed                               |

The checks cover automatic/local/shared selection, fallback and explicit failures, preserved profiles and argument boundaries, child exit codes, locking, cache-clear ownership/junction checks, independent copies, Windows PDB naming, confirmation/cancellation/reset behavior, profile persistence, older-record migration and retained-output resolution.

## Live application checks

- Started the independently copied release controller and used native Worktree Review. Confirmed debugging is unchecked initially; checked it, cancelled, and reopened to verify reset. Cancellation left zero builds.
- Built committed source `28ad076ffa005b9227c5bd5b3f63c8df23b43bdf` through that controller. Its retained checkout had no `scripts/build-tools.mjs`. The shipped tool installed dependencies, built the application, and returned a successful release result.
- The fresh target selected shared caching. The log reported 286 hits and 108 misses during its command window; these counters are machine-wide and are not a timing or per-build savings measurement.
- The result recorded profile `release`, displayed **Normal** and **Compilation completed**, and offered **Launch**. There were zero processes using its retained executable before Launch.
- Cleared only that validation checkout's compiler target with the provided clear-cache command. The target was absent and the retained executable remained present. Clicking **Launch** then opened the application, which loaded repository facts and its active-build context.
- In the final normal instance, also opened and cancelled confirmation for the existing live worktree; debugging remained unchecked and no additional build was created.
- Rebuilt the implementation worktree using its populated local release/debug targets. Launched both final normal and debugging copies separately and checked their native UI.

[Final existing-worktree confirmation](build-confirmation.png) · [Completed normal build and separate Launch action](normal-build-result.png)

The retained native test build is `build-f9061d62-8a7f-4ac5-a838-378a074ea91a`, attempt `19392504ec4fcd49b541b790b6195e20`, output `output-ce32e437-3214-46ad-8d60-2414cb5ee077`.

Final applications are under `%LOCALAPPDATA%\CodexOrchestrator\build-validation\refinement-normal-final\output` and `refinement-debug-final\output`. The normal executable is 39,217,664 bytes. The debugging output contains the 66,944,000-byte executable and its 520,417,280-byte `codex_orchestrator.pdb`. Compiler intermediates are outside both output directories.

Raw command logs and inspection receipts are retained locally under `.dev/build-validation/`. Existing Vite bundle-size and Rust unused-code warnings remain. Installer packages and live agent-provider sessions were not exercised; the live checks establish native startup, repository navigation, build/launch behavior and cache independence.

## Windows details verified

Tauri frontend paths are relative so the frontend is embedded instead of interpreted as a Windows file URL. Publication copies Rust's underscore-named application PDB for a hyphenated executable, and reports an error if a Windows debugging build has no symbols. Both cases have regression coverage.

Useful implementation-worktree caches and the shared sccache remain. Cache clearing was limited to the disposable native validation target; no old user application output or unrelated worktree was removed.

Automatic approval review blocked the final combined temporary-output cleanup command with "blocked by policy"; the temporary output folders were retained.
