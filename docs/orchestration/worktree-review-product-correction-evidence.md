# Worktree Review product correction evidence

Baseline: `c998c34645847b618ff08354988a219fd00d2d9e`. Branch:
`codex/worktree-review-product-correction`.

## Result

Worktree Review is now normal product composition in Rust, Tauri, and React. Release builds retain
the module, state, product commands, contextual file review, launcher, and settings. Only proof
navigation, proof evidence, and the debug controller are compiled behind `debug_assertions`.

Application startup creates recoverable Worktree Review state without requiring a repository or
toolchain. A selected repository is persisted with an opaque identity derived from its canonical
Git common directory. Repository-scoped runtime and store roots prevent ordinary cross-repository
record reuse. Repository switching validates identity and keeps the last ready service if a new
selection cannot be composed.

The former service monolith no longer owns SQL, adjacent `WorktreeTest*` vocabulary, proof mutexes,
or proof transport. Those responsibilities live behind `WorktreeReviewStore`,
`ReviewInstanceRuntime`, and the debug-only `proof` module. Low-level repository reads in Worktree
Review and Epic Origin use typed, bounded repository-context operations. The runner supplies a
deterministic platform line-ending policy after clearing ambient Git configuration, preventing
false dirty state on Windows without restoring ambient global/system config.

## Validation

| Evidence                                                           | Result                                              |
| ------------------------------------------------------------------ | --------------------------------------------------- |
| `cargo check --manifest-path src-tauri/Cargo.toml --lib`           | Passed                                              |
| `cargo check --manifest-path src-tauri/Cargo.toml --lib --release` | Passed                                              |
| `cargo test ... --lib worktree_review:: -- --test-threads=1`       | 36 passed                                           |
| `cargo test ... --lib repository_context:: -- --test-threads=1`    | 4 passed                                            |
| `cargo test ... --lib epic_origin:: -- --test-threads=1`           | 2 passed                                            |
| Focused frontend product/composition/review suite                  | 7 files, 43 passed                                  |
| `npm run test:worktree-runtime`                                    | 9 passed                                            |
| `npm run build`                                                    | Passed, 2,084 modules transformed                   |
| `npm run validate:product`                                         | Passed                                              |
| `npm run validate:release`                                         | Passed; release check and `tauri build --no-bundle` |
| `cargo fmt` and `git diff --check`                                 | Passed                                              |

The complete frontend suite ran 876 tests: 874 passed and 2 unchanged baseline assertions failed.
`HarnessAwareAgentSessionPane.test.tsx` expects the old `# Epic Plan Builder` heading while the
baseline skill already says `# Product Epic Plan Builder`. `NativeProfileSettings.test.tsx`
expects literal `padding-top: 48px` while baseline CSS already uses `var(--app-top-offset)`. Neither
path was changed by this correction's behavior.

The aggregate `npm run test:rust:fast` did not reach tests because its Cargo process remained idle
behind a target lock. Its verified process tree was stopped; the three sequential suites above
cover all changed Rust modules and passed.

## Release startup proof

The release build produced `src-tauri/target/release/codex-orchestrator.exe`. It was copied to
`src-tauri/target/release-smoke-final/relocated`, started from that directory with isolated application
and Worktree Review storage, and given no repository or proof-controller environment value. It was
still running after six seconds. The active application database and Worktree Review selection
database both existed, and the selection table contained zero rows. The exact observed process was
then stopped.

This is bounded startup evidence. It proves release linking, relocation away from the build output,
and startup without a selected repository; it does not claim a human-observed WebView journey.

## Security and maintainability residuals

- `attach_review_worktree` still runs mutating `git worktree add` directly. It inherits Git
  configuration/hooks, has no timeout/output bound, and may produce raw stderr before transport
  sanitization.
- The shared repository runner bounds output and clears risky Git environment/configuration, but it
  has no wall-clock timeout and its line-oriented parser cannot represent newline-containing paths.
- Repository-local attributes/configuration can still select content filters during Git reads; a
  stricter no-filter execution contract requires a separate design.
- Windows path identity normalizes prefixes, separators, and ASCII case; it is not filesystem
  file-ID equivalence.
- The PowerShell folder picker still uses ambient `PATH`.
- Toolchain discovery is lazy at capability entry but eager within composition, so missing build
  tools also block read-only source browsing.
- `worktree_review/service.rs`, `repository_context/mod.rs`, and the launcher component remain larger
  than ideal. Their extracted ports now provide credible future split points.
- Source refs keep the historical hashing scheme. Per-repository storage supplies practical
  isolation, but source identity should eventually encode repository scope explicitly.

No merge, push, worktree retirement, or user acceptance is implied by this evidence.

## Changed-file manifest

- Product/release composition: `package.json`, `src-tauri/src/{active_app,lib}.rs`,
  `src-tauri/src/worktree_runtime/mod.rs`, `src/app/{App,ApplicationRoot}.tsx`, and related tests.
- Repository boundary: `src-tauri/src/repository_context/*.rs`, `src-tauri/src/epic_origin.rs`, and
  Worktree Review catalog/comparison/source-history/worktree-build modules.
- Worktree Review application: composition, progress, runtime port, service, state, store, transport,
  and the debug-only `proof/*.rs`; the old root-level proof files were moved into `proof/`.
- Frontend product boundary: `src/application/worktreeReview.ts`,
  `src/infrastructure/tauriWorktreeReview.ts`, launcher/readiness/settings files,
  `src/features/technicalSettings/*`, native-profile ownership, child-shell isolation, and tests.
- Support: `scripts/worktree-runtime.node-test.mjs` now scans the actual worktree so deleted files are
  excluded; `src/test/setup.ts` explicitly cleans the DOM after each test.
- Removed obsolete boundary: `src/infrastructure/tauriHumanReviewLauncher.ts`.
