# Application builds and compiler cache reuse

Status: implemented and validated on refinement/application-build-cache. See the [validation record](../validation/application-build-cache/README.md).

Baseline: main `28ad076`, inspected 2026-09-14. Concurrent Agent Session and documentation edits are outside this work. Recheck affected files before implementation.

## Agreed outcome

- A successful application build produces a complete runnable application, whether requested by an agent or Worktree Review. Build and Launch are separate actions.
- Worktree Review always confirms before building. Debugging is off by default and can be enabled in that confirmation.
- Preserve useful compiler caches to reduce agent compilation time. Disk reduction is secondary.
- Worktree Review automatically prefers the requested profile's local worktree cache, otherwise shared caching when available. Agents can discover and override the same choice through normal compilation commands.
- Use a simple selection rule. Do not add timing-based selection, fine-grained cache analysis, or a performance research project.

Planning assumptions: normal application builds use Tauri release mode; debugging builds use its existing `--debug` behavior and retain debugging symbols. This does not introduce a new inspector. Rust checks and tests retain their existing profiles. Apply automatic cache selection to the provided agent compilation commands as well as Worktree Review, with explicit overrides.

## Current change surface

1. `package.json` routes native checks/tests directly to Cargo and application builds to Tauri. Its generic `build` command produces only the frontend.
2. `scripts/cargo-sccache.ps1` implements shared-cache discovery, invocation-scoped environment handling, stable Cargo working directory, explicit target selection, and statistics. It is documented and has actual agent callers. `cargo-test-fast.ps1` is a separate reduced-debug test entrypoint.
3. `worktree_review/transport.rs` forwards build input through `branch_first.rs` to `ReviewBuildCoordinator`. The coordinator owns source preparation, checkout ownership, durable attempts, output records, and retention.
4. `worktree_review/build_executor.rs` chooses dependency preparation and attempt storage, then calls `PhysicalWorktreeApplication::build`.
5. `worktree_application/build.rs` independently implements typechecking, Vite, Tauri, and publication. It always uses `--debug`, places Cargo and frontend output under a fresh attempt, and renames that entire directory into retained output.
6. `WorktreeReviewScreen.tsx` shows `BuildCheckoutDialog` only for requests needing a new checkout. Other requests build immediately. Completed builds have a separate Open action.
7. Review cleanup already separates application-output retention from physical-checkout retention. Output records store relative executable paths, so launch does not need a fixed `t/debug` layout.

## Proposed ownership

Extract the physical build recipe into a small Node-based tool shared by command-line callers and the native application. Node is already a prerequisite for the existing builder. This avoids compiling the application before agents can access its build tooling.

| File or object                                                                                                           | Change and responsibility                                                                                                                                                                                                                                    |
| ------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| `scripts/build-tools.mjs`                                                                                                | Create the discoverable CLI for application builds, Cargo checks/tests, and explicit cache clearing. Parse options and print help/results; delegate mechanics.                                                                                               |
| `scripts/build/application.mjs`                                                                                          | Extract the typecheck/Vite/Tauri sequence and application-file publication from Rust. Accept explicit source, profile, dependency policy, cache mode, and destination. Never launch.                                                                         |
| `scripts/build/cargo.mjs`                                                                                                | Move the existing shared-cache mechanics here; own target resolution, automatic/local/shared selection, Cargo invocation, and scoped child environments for all consumers.                                                                                   |
| `scripts/build/cargo-runner.mjs`                                                                                         | A narrow Tauri runner adapter to the same Cargo invocation. Preserve Tauri's build arguments while applying the shared-cache helper's stable working directory and explicit manifest/target rules. Use a minimal Windows executable launcher where required. |
| `scripts/cargo-sccache.ps1`                                                                                              | Retain as a thin entrypoint selecting shared mode. Remove its duplicate implementation after porting its behavior and tests. Existing agents already call this name.                                                                                         |
| `scripts/cargo-test-fast.ps1`                                                                                            | Retain as a thin entrypoint preserving its current reduced-debug test semantics; delegate cache/invocation mechanics. Do not silently equate it with the different `test-fast` Cargo profile.                                                                |
| `worktree_application/build.rs`                                                                                          | Reduce to a native adapter: validate its request, invoke the shipped tool, translate result/error data, and verify output containment. Remove the independent command recipe and whole-Cargo-directory publication.                                          |
| `worktree_application/build_tool.rs`                                                                                     | Create a small embedding/materialization adapter for the tool sources. Ship the same checked-in scripts with the controller application and invoke those copies, so an older selected checkout need not contain the new tool.                                |
| `worktree_application/domain.rs`, `mod.rs`                                                                               | Adapt the physical request/result for profile, cache selection, and retained application paths. Keep capture, checkout, and launch ownership here.                                                                                                           |
| `worktree_review/build_executor.rs`                                                                                      | Retain Review's dependency policy, attempt roots, output validation, and error-stage translation. Pass the selected profile and automatic cache mode to the physical builder.                                                                                |
| `worktree_review/build_service.rs`, `build_presentation.rs`, `domain/build.rs`, `storage/builds.rs`, `storage/schema.rs` | Carry and persist the requested profile and show it with the build. Keep source selection, attempt lifecycle, and existing retention policy here.                                                                                                            |

The tool accepts explicit worktree/manifest paths. It must not infer its build target from its own script location: native callers use a shipped tool to build a different checkout. Request/result files provide a small structured process boundary; compilation logs remain separate from the result. No new service or daemon is needed.

Use Tauri's existing `--runner` and forwarded Cargo arguments for the native build. Preserve its frontend configuration and feature handling rather than recreating Tauri with a bare Cargo command. The installed CLI exposes these seams. Verify their actual invocation and Windows argument handling during implementation.

## Cache rule and storage

Expose `--cache=auto|local|shared` on the supplied Rust/application compilation commands; default to `auto`.

1. Resolve a persistent target for the physical worktree. Respect an explicit target, reuse an existing populated `src-tauri/target` by default, and otherwise allocate a short tool-owned path under `%LOCALAPPDATA%/CodexOrchestrator/build-cache/<worktree-key>/`. The key identifies the worktree location, not its branch tip or build attempt. Both callers use this resolver.
2. For `auto`, check whether the requested profile has populated Cargo artifacts in that target. If so, use ordinary local Cargo behavior. Otherwise use shared caching if its preflight succeeds; if unavailable, use ordinary compilation and report that choice.
3. Cargo decides which individual artifacts are reusable. Profile-directory presence is a cheap routing hint, not a claim that every artifact is fresh. Do not hash the dependency graph or predict cache hit rates.
4. Local mode leaves normal incremental settings intact and bypasses the shared wrapper. Shared mode preserves the existing helper's scoped incremental disablement, stable Cargo working directory, target handling, cache-location checks, and environment restoration behavior.
5. Explicit shared mode reports an unavailable helper as an error. Auto can fall back before compilation; a compiler failure is not grounds for a hidden retry in another mode.

Keep short Windows paths: the current builder contains a real MSVC path-length workaround. Record the owning worktree in tool-created cache roots so cache clearing can identify its target. Do not move existing warm targets merely to adopt the new layout.

Frontend staging also uses a stable worktree/profile location. Its Tauri configuration path must not change with every output attempt. Each build refreshes those frontend files before native compilation. Application destinations and Review IDs do not belong in compiler-cache identity.

Shared caching still writes ordinary build artifacts into the selected worktree target. That target can be reused on subsequent calls. Preserve compatible command/profile inputs without adding an adaptive optimizer.

Log the selected mode, reason, target path, and shared-cache availability. Keep existing shared-cache statistics as diagnostics, without presenting machine-wide counters as per-build savings. The product modal needs no cache controls.

## Runnable output and cleanup

After compilation, copy the executable and required runtime files into a fresh application staging directory, then publish that directory. Include the application's debugging symbols for debugging builds. The current app embeds its frontend; do not retain frontend tooling or an entire Cargo tree as runtime requirements. Verify the concrete file set by launching the copied Windows application.

Worktree Review keeps its existing attempt/output roots and records the executable's new relative path. Command-line builds receive an equivalent independent output folder and print its location. Existing `open.rs` remains the launch mechanism for Review; it does not need compiler caches to exist.

Never move or hard-link a mutable compiler output into a retained application: subsequent compilation must not change an older instance. Protect the shared build/staging/copy sequence for the same worktree with one tool-owned lock; Cargo's internal lock alone does not cover copying after its process exits. Reuse ordinary OS locking, with no new queue or runtime coordinator. Different worktrees keep independent targets.

Preserve caches on build failure and after successful publication. Remove only temporary publication files on failure. Review's existing retention cleanup continues to remove owned application outputs and attempt data, not compiler-cache roots.

Provide an explicit cache-clear command for a selected worktree/target. It identifies the owner and refuses an active build; it does not remove retained applications or source. Do not automatically equate changed dependencies, profiles, or toolchains with permanently useless caches. Existing old per-attempt outputs remain launchable and follow existing retention; this implementation does not bulk-delete or relocate them. A general abandoned-cache sweeper is outside this slice.

## Worktree Review confirmation and records

- Replace `BuildCheckoutDialog.tsx` with `BuildConfirmationDialog.tsx`, reusing `ReviewDialog`. Every Build click opens it. It summarizes the frozen source request, includes checkout-creation disclosure when applicable, and offers an unchecked **Enable debugging** option plus Cancel/Build.
- Adapt `WorktreeReviewScreen.tsx` so confirmation is the only build dispatch path. Reset debugging to off for each new confirmation. Cancellation creates neither checkout nor build.
- Adapt `BuildComposer.tsx` and `buildDraft.ts` only where needed to compose the confirmed request; keep branch/commit selection intact.
- Add release/debug profile to `application/worktreeReview/contracts.ts`, native `CreateBuildInput`, and the physical build request. Pass it unchanged through the existing client/transport/coordinator.
- Persist profile on new build records and display Normal/Debugging in `BuildHistory.tsx`. Existing records without trustworthy profile data remain unspecified; do not relabel them as new normal builds.
- Keep the separate launch endpoint and existing focus-if-open behavior. Label the user action **Launch** and its progress **Launching…**. Building never invokes it.
- Preserve current source-based output retention. Cache mode does not create a new retention identity or change application behavior.

## Agent commands and discovery

Make `npm run build` produce an application through the shared tool. Add `build:frontend` for the current TypeScript/Vite-only operation. Point Tauri's frontend prerequisite and frontend-only validation callers to that explicit command to avoid recursive application builds.

Route `check:rust`, `check:rust:release`, and the Rust test scripts through the shared Cargo entrypoint while preserving their command/profile/filter semantics. Route native application build entrypoints through the shared application recipe; preserve the existing explicit installer use of `build:tauri` rather than making ordinary review builds install packages.

Example proposed usage:

```powershell
npm run build
npm run build -- --debug --cache=local
npm run check:rust -- --cache=shared
npm run test:rust:fast -- --cache=auto
node scripts/build-tools.mjs --help
```

Parsing must keep tool options distinct from Cargo filters and arguments after `--`. Frontend-only compilation has no Rust cache switch.

Update `README.md`, `docs/development.md`, `docs/worktree-review.md`, and the concise compilation guidance in `src-tauri/AGENTS.md`. Explain the default in place: local artifacts suit continued work; shared caching can help a new target; auto applies that rule. Show overrides alongside the ordinary commands, not in a separate optional-tool appendix. Distinguish an application build, a compilation check, and compiling/running tests.

## Implementation order and acceptance

1. Extract the shared tool and cache resolver. Port the existing helper's invocation/environment checks; keep both PowerShell entrypoints thin. Verify Tauri runner forwarding before switching the native builder.
2. Move application compilation/publication to that tool. Embed its sources for native use, adopt persistent targets/staging, and remove the duplicate Rust recipe. Add profile persistence and preserve old output paths.
3. Replace checkout-only confirmation, add the debugging choice, and retain separate Launch. Update command callers and discovery documentation.
4. Validate the ordinary supported Windows flows with isolated application data and task-owned outputs.

Focused automated checks should cover auto selection with/without local profile artifacts, unavailable shared caching, explicit overrides, scoped environments and exit codes, persistent target identity, output independence, and the same-worktree build/publication lock. Retain useful existing helper contract coverage instead of duplicating it.

Frontend/native checks should cover confirmation for existing and new checkouts, cancel without dispatch, debugging off by default and reset on reopening, selected profile reaching compilation/persistence, and Build never launching. Existing output records must still resolve.

Live acceptance: produce a normal and debugging application, launch each separately, rebuild the same worktree with local reuse, and exercise shared-cache selection for a fresh target when sccache is available. Launch a copied application after clearing only its disposable test cache to prove runtime independence. Exercise the feature from a release controller, including a selected source without the new CLI files. Visually check the confirmation and completed-build Launch flow.

Record mode, successful reuse/compilation behavior, and command outcomes. No timing targets, broad comparison matrix, or benchmark gate are required. Build success establishes produced application files; startup and usable UI are separate live evidence.
