# Observation pass: File Review and Human/Worktree Review

## Evidence boundary

- Snapshot: `b28137b66d79121d740267831fc22bf8cdbcbb40` on `codex/orchestration-engine-research`, inspected 2026-08-07.
- Method: start at visible controls, follow their data and control paths in the current tree, then inspect only the recorded, proof, CLI, and sibling-branch material those paths expose.
- This is a static code and history pass. It does not claim that a live Tauri review instance was launched during this pass.
- “Release” below means Rust without `debug_assertions` and a Vite production frontend. Rust debug compilation and Vite `import.meta.env.DEV` are independent switches.
- The relevant frontend test sources were inspected. A targeted Vitest run was attempted, but this research checkout has no installed Vite/Vitest dependencies; startup stopped before test collection with `ERR_MODULE_NOT_FOUND` for `vite`. No passing test result is claimed here.

## Visible starting points

| Visible experience | Immediate trigger | Frontend reachability | Native reachability |
| --- | --- | --- | --- |
| Sprint File Review | `Review files` in a started Sprint | Normal product composition, including release | Both commands are registered in release, but the release contextual producer is deliberately unavailable |
| Recorded File Review | `?file-diff-viewer` plus optional `file-review-fixture` | Vite development only | Fixtures do not require Tauri; the development wrapper still adds the Human Review client |
| Human Review launcher | `Worktree Review Dev` | Injected only when `import.meta.env.DEV === true` | All launcher commands and the entire `worktree_review` Rust module are debug-only |
| Isolated Worktree Review application | Persistent `Worktree build` widget, then Build details or File Review | Enabled by `VITE_HUMAN_REVIEW_INSTANCE=true`, checked before the Vite-development branch | The runtime intentionally builds a Tauri debug executable, so its worktree commands exist |

`src/app/ApplicationRoot.tsx:14-65` is the decisive frontend composition switch. `src-tauri/src/lib.rs:22-25` and `src-tauri/src/active_app.rs:251-302,305-393` are the decisive native switches.

## 1. Sprint File Review behavior

### Visible request and navigation

`createProductApplicationComposition()` always supplies `createTauriContextualFileReviewClient()` (`src/bootstrap/productApplicationComposition.ts:25-35`). Consequently, a started Sprint receives an `onRequestFileReview` callback in normal product boot, not only in development (`src/app/App.tsx:497-523`).

`SprintWorkspace`:

- shows `Review files` only for `planningState.kind === 'started_plan'` and when the callback exists;
- sends only the exact `sprintId`;
- disables duplicate requests while pending;
- keeps the Sprint on screen until the source reports ready;
- exposes the safe failure reason through `data-reason` and user-facing copy.

The relevant implementation is `SprintWorkspace`, `requestFileReview`, and `SprintFileReviewControl` in `src/features/orchestrations/components/SprintWorkspace.tsx:34-54,86-134,859-879`.

After a ready result, `App.requestContextualFileReview` stores the returned `FileReviewSource` and changes the top-level surface to `file-review` (`src/app/App.tsx:391-415`). `FileReviewScreen` itself has no back callback (`src/features/fileReview/FileReviewScreen.tsx:21-28,85-227`).

Static navigation consequence:

- `OrchestrationSection` owns selected Epic, Sprint, revision, and detail location in component-local state (`src/features/orchestrations/OrchestrationSection.tsx:143-216`).
- Switching to File Review unmounts that tree because `App` conditionally renders one top-level surface (`src/app/App.tsx:479-545`).
- Returning through the global `Orchestration` button therefore remounts the orchestration workspace at its defaults rather than returning to the exact originating Sprint.
- The `Files & diffs` peer tab is conditioned on the injected `fileReviewSource` prop, not `contextualFileReviewSource` (`src/app/App.tsx:128-129,463-472`). A contextual review can be displayed without gaining a peer tab, and after leaving it the visible path back is another Sprint request.

The current test explicitly establishes the hidden-peer-tab behavior (`src/app/App.agentSessions.test.tsx:205-247`) but does not exercise returning to the exact Sprint.

### Native production path

The successful contextual path is:

```text
started Sprint id
  -> request_contextual_file_review
  -> load initiated Sprint Git authority
  -> reauthorize the exact review-runtime instance and frozen comparison
  -> store a Git-capture authorization
  -> capture immutable baseline/current Git objects
  -> store changed-files Document + artifact + opaque reference in SQLite
  -> re-load through the opaque reference
  -> return opaque reference
  -> load_scoped_file_review through application-owned frontend ports
  -> validate and render FileReviewSnapshot
```

Exact owners:

- Tauri transport: `ContextualFileReviewTauriState`, `request_contextual_file_review`, and `load_scoped_file_review` in `src-tauri/src/orchestration/transport.rs:22-42,363-460`.
- Sprint-context authority lookup: `FileReviewOriginatingEntryService::produce_for_sprint_context` in `src-tauri/src/orchestration/file_review_originating_entry.rs:44-108`.
- Runtime comparison reauthorization: `InitiatedSprintGitAuthorityService` and `WorktreeRuntimeGitComparison` in `src-tauri/src/orchestration/initiated_sprint_git_authority.rs`.
- Concrete runtime provider: `impl WorktreeRuntimeGitComparison for HumanReviewLauncherService` in `src-tauri/src/worktree_review/service.rs:39-90`.
- Git capture and stored contract: `produce_file_review_from_git` in `src-tauri/src/orchestration/file_review_git_producer.rs:21-115`.
- Durable tables and scoped loading: `file_review_documents`, `file_review_changed_files`, `stored_file_review_artifacts`, `file_review_git_capture_authorizations`, and `file_review_git_capture_documents` in `src-tauri/src/orchestration/repository.rs:230-283`; `load_scoped_file_review` at `repository.rs:1983-2023`.

The Git producer accepts only an opaque capture-authorization identity. It canonicalizes the recorded repository and worktree roots, verifies their shared Git common directory, verifies both object IDs are commits, and runs Git with cleared environment, disabled prompts/hooks/config/fsmonitor/replacement objects, bounded stdout, and a minimal `PATH` (`file_review_git_producer.rs:179-335,465-570`). Limits are 500 files, 256,000 bytes per file, 20,000 text lines per side, 256,000 bytes for the changed-path list, and 1,000,000 bytes for the complete stored artifact (`file_review_git_producer.rs:16-19,81-84`; `repository.rs:309-310`).

For text, the stored producer creates one complete-file hunk: all old lines are deletions and all new lines are additions unless both sides are identical (`file_review_git_producer.rs:663-728`). This is a complete comparison contract, not a minimal Git-style patch.

The same producer also supports internal execution inspection. `ProductExecutionWorkspaceResolver::inspect` stores a capture authorization for a clean isolated attempt worktree, produces the same artifact, reloads it, then returns the changed-file manifest, comparison bytes, and capture authorization to execution support (`src-tauri/src/orchestration/execution_support.rs:430-511`). This producer is therefore not only a UI request helper.

### Release behavior is a truthful failure, not absence

In Rust debug composition, `active_app` supplies the contextual state with `FileReviewOriginatingEntryService`, backed by the Human Review service (`src-tauri/src/active_app.rs:251-276`). In release it supplies `ContextualFileReviewTauriState::unavailable` (`active_app.rs:297-302`). Both `load_scoped_file_review` and `request_contextual_file_review` remain in the unconditional Tauri handler (`active_app.rs:346-350`).

Thus the current release behavior is:

1. A started Sprint visibly offers `Review files` because the product frontend client is present.
2. `request_contextual_file_review` finds no service and returns `{status: "unavailable", reason: "not_ready"}` (`transport.rs:408-411`).
3. The frontend leaves the user on the Sprint and displays `File Review is not ready for this Sprint.` (`src/infrastructure/fileReview/tauriContextualFileReview.ts:103-114`).

`load_scoped_file_review` remains a functional release command because it uses the ordinary orchestration application state. No current visible production route accepts an existing opaque reference directly: the only non-test construction of `createTauriScopedFileReviewPorts` is after a successful contextual request.

### Frontend scoped loading and validation

`createTauriContextualFileReviewClient` requests the opaque reference, creates scoped Document/artifact ports, and eagerly calls `source.load()` before returning ready (`src/infrastructure/fileReview/tauriContextualFileReview.ts:31-68`). The first viewer load receives that cached snapshot.

`createTauriScopedFileReviewPorts` calls `load_scoped_file_review` once for `loadDocument()` and again for `loadArtifact()` (`src/infrastructure/fileReview/tauriScopedFileReview.ts:41-96`). Its `cached` variable stores the last decoded Document but does not prevent the second invoke. The ordinary successful contextual sequence therefore performs:

- one contextual production request;
- one backend scoped reauthorization inside that command before success is returned;
- two further `load_scoped_file_review` invokes during frontend source preload.

`createApplicationOwnedFileReviewSource` then independently enforces:

- one eligible `changed_files` Document and one artifact;
- matching Document/artifact identities;
- the 1 MB frontend limit;
- UTF-8 JSON with contract version `stored-file-review-artifact/v1`;
- exact membership of the authorized changed-file set;
- safe display names, supported kinds, coherent hunk ranges/counts/line numbers, and complete new-file content.

See `src/application/applicationOwnedFileReview.ts:12-107,109-199` and `assertCompleteFileReviewFile` in `src/application/fileReview.ts:56-97`.

### A second visible projection of the same durable facts

Stored File Review Documents are also included in `load_orchestration_native_query`. The TypeScript native-query adapter projects them into generic artifact access and Sprint Documents (`src/application/orchestrations/nativeQuery.ts:545-553,579-598,692-725`). `SprintDocumentsPanel` renders Resolve, Open, and Copy path actions (`src/features/orchestrations/components/SprintDocumentsPanel.tsx:9-112`).

Product boot, however, supplies `unsupportedArtifactAccessController` (`src/bootstrap/productApplicationComposition.ts:46-49`; `src/application/orchestrations/artifactAccessController.ts:55-61`). The durable File Review Document can therefore be visible as a generic Document while all three generic artifact operations truthfully report unsupported. The dedicated `Review files` path bypasses that generic controller and uses the opaque scoped-load commands instead.

## 2. The shared File Review renderer receives materially different sources

`FileReviewScreen` is a reusable read-only presentation surface. It owns file selection, Changes/File mode, Unified/Split mode, collapsed-context expansion, totals, and loading/error/empty states (`src/features/fileReview/FileReviewScreen.tsx:28-227`). Text is rendered as source; Markdown is rendered through `AgentMarkdown`; binary and unsupported items receive non-content states (`FileReviewScreen.tsx:261-317`).

| Source path | Selection/authority | Compared material | Notable contract differences |
| --- | --- | --- | --- |
| Contextual Sprint source | Opaque application-owned Document and artifact | Frozen baseline and current commit recorded for the Sprint runtime source | Stored v1 contract; rename support; 256 KB per file / 1 MB artifact; frontend fully decodes membership and identity |
| Human Review launcher comparison | Retained `instanceRef` resolved by the debug launcher | Machine main `HEAD` to selected worktree’s complete current filesystem state | Direct serialized snapshot; no stored Document; no opaque-reference load |
| Isolated worktree comparison | Review-instance environment | Machine main `HEAD` to that worktree’s complete current filesystem state | Direct serialized snapshot; stable client-owned source object |
| Recorded `application-owned` fixture | In-memory recorded Document and bytes | One recorded contract example | Exercises the real application-owned decoder without Tauri |
| Other recorded fixtures | Direct in-memory `FileReviewSnapshot` | working tree, staged, commit range, or generated examples | Bypass application-owned decoding |

The direct Rust worktree comparison is implemented by `src-tauri/src/worktree_review/comparison.rs`. It unions committed divergence with tracked and untracked working-tree paths (`comparison.rs:72-93,297-315`), reads current files directly, uses a 1.5 MB per-file limit, disables rename detection, and emits one full-file hunk after preserving only a common prefix and suffix (`comparison.rs:8,96-252,328-380`). Deleted files become `unsupported` with no old-content view (`comparison.rs:121-161`).

The direct Rust response also carries a `provenance` array (`committed-divergence`, `uncommitted`, or both) on every file (`comparison.rs:18-26,114-120`). `FileReviewFile` has no `provenance` field and `FileReviewScreen` does not display it (`src/application/fileReview.ts:40-50`). That useful distinction crosses Tauri but is currently discarded by the TypeScript/view contract.

Both direct Tauri adapters run `assertCompleteFileReviewFile` but otherwise trust the display-ready response shape (`src/infrastructure/tauriHumanReviewLauncher.ts:24-31`; `src/infrastructure/tauriWorktreeBuild.ts:12-17`). They do not use the stronger Document/artifact identity decoder.

One ownership dependency is easy to miss: Markdown support makes the supposedly generic File Review feature import `AgentMarkdown` from the Agent Sessions feature (`src/features/fileReview/FileReviewScreen.tsx:18,261-271`).

## 3. Human Review launcher behavior

### Frontend composition

For every Vite-development route except the special review instance, `loadDevelopmentReviewComposition` adds:

- `HumanReviewLauncherView` using `tauriHumanReviewLauncher`;
- a `humanReviewLauncherNavigation` proof callback.

This is `src/app/ApplicationRoot.tsx:30-60,79-88`. `App` then adds `Worktree Review Dev` and polls the launcher proof route every 300 ms (`src/app/App.tsx:141-157,453-462`). A browser-only Vite development server can render this tab even when no Tauri commands exist; failures are caught in the views, not prevented by composition capability detection.

### User-facing lifecycle

`HumanReviewLauncherView` loads selectable Git worktrees and retained review instances. The overview supports:

- Prepare a named isolated instance from a compatible worktree;
- Build;
- Open;
- Focus window;
- Check status;
- Stop;
- Recover;
- inspect Build details and retained sanitized operation output;
- compare files through the shared File Review screen.

The view labels itself `Development tool` and explicitly says a future troubleshooting Agent Session and dedicated Harness are not implemented (`src/features/humanReviewLauncher/HumanReviewLauncherView.tsx:240-316,318-453`).

`HumanReviewLauncherClient` is an application-layer contract, while `tauriHumanReviewLauncher` maps it to 16 Tauri commands (`src/application/humanReviewLauncher.ts:75-107`; `src/infrastructure/tauriHumanReviewLauncher.ts:10-44`).

### Native composition and retained state

`worktree_review::compose` discovers toolchain programs and Git worktrees, opens a worktree-runtime registry at `registry.sqlite`, creates a Rust `WorktreeRuntimeApplication`, builds a `WorktreeTestInstanceFacade`, creates/loads an authority secret, and opens launcher state at `launcher.sqlite` (`src-tauri/src/worktree_review/composition.rs:10-76`). Default material lives below `dev.codex-orchestrator.human-review`, unless `CODEX_ORCHESTRATOR_REVIEW_RUNTIME_DIR` selects an isolated root (`src-tauri/src/active_app.rs:253-265`).

`HumanReviewLauncherService` persists review session identity, source, built state, and lifecycle history in SQLite, while the current `ProgressRegistry`, operation results, and proof-navigation state are in process memory (`src-tauri/src/worktree_review/service.rs:251-328`). Detail assembly additionally reads retained build/test/start log files (`src-tauri/src/worktree_review/detail.rs:289-319`). “Retained review build” therefore combines durable registry/session/log material with volatile live progress/proof state.

The visible UI operation commands use caller-created operation references and a blocking native call moved to Tauri’s blocking pool:

- frontend starts `pollProgress` every 500 ms, then awaits Prepare/Build/Open (`HumanReviewLauncherView.tsx:172-213,503-535`);
- native `prepare_human_review_instance`, `build_human_review_instance`, and `start_human_review_instance` call `spawn_blocking` and return the completed `ReviewInstanceView`, not an acceptance receipt (`src-tauri/src/worktree_review/transport.rs:63-86,167-217`);
- the service records stages in `ProgressRegistry` during that blocking operation (`service.rs:350-550`).

A separate operation interface exists for the proof controller: `begin_prepare`, `begin_build`, and `begin_open` allocate their own operation references, spawn named background threads, retain terminal results in memory, and expose `operation_status` (`service.rs:553-711`). The visible React UI does not use this accepted-operation interface; `debug_controller.rs` does.

### Build and open behavior

The Rust runtime’s Build plan runs TypeScript type checking, a Vite build into the instance’s private dist, and `tauri build --debug --no-bundle` with a generated Tauri config (`src-tauri/src/worktree_runtime/planning.rs:339-430`). Open plans start a Vite server, status server, and the verified debug executable with a cleared, explicitly reconstructed environment (`planning.rs:435-487`; `execution.rs:61-110`).

The environment supplies isolated app data, temp, caches, credentials home, ports, readiness path, navigation path, worktree identity, and `VITE_HUMAN_REVIEW_INSTANCE=true` (`planning.rs:595-752`). The generated Tauri build window begins hidden and unfocused (`planning.rs:521-548`). Open is recorded only after the exact window and readiness marker are established, and then the launcher focuses it (`src-tauri/src/worktree_review/service.rs:494-550`).

### File comparison route and a likely reload loop

The launcher’s Files route creates its source inline on every parent render:

`<FileReviewScreen source={client.comparison(detail.instanceRef)} />` (`HumanReviewLauncherView.tsx:215-237`).

At the same time, the launcher continues polling `listProgress()` every 500 ms and stores the newest returned object in state (`HumanReviewLauncherView.tsx:93-115`). `tauriHumanReviewLauncher.comparison` returns a new source object on every call (`src/infrastructure/tauriHumanReviewLauncher.ts:24-32`). `FileReviewScreen` treats source identity changes as a full reload and resets snapshot, selected file, mode, and expanded context (`FileReviewScreen.tsx:38-57`).

Static consequence: once `listProgress()` has at least one retained operation, each poll is expected to rerender the launcher, construct a new comparison source, invoke `human_review_instance_comparison` again, and reset the File Review interaction state. The isolated `WorktreeBuildShell` does not have this problem because it passes the stable `client.comparison` property. This is code-path reasoning, not a live timing observation; current tests use `listProgress() => []` while visiting files and do not cover it (`HumanReviewLauncherView.test.tsx:239,308`).

## 4. Isolated Worktree Review application behavior

`VITE_HUMAN_REVIEW_INSTANCE=true` takes precedence over all Vite-development query routes. `ApplicationRoot` immediately creates normal product composition, dynamically loads `tauriWorktreeBuild` and `WorktreeBuildShell`, and wraps the entire `<App>` (`src/app/ApplicationRoot.tsx:18-29,67-75`). It does not inject the `Worktree Review Dev` launcher into the child.

`WorktreeBuildShell`:

- loads worktree identity/context;
- on the next animation frame calls `markReady`, then loads detailed retained state;
- keeps the normal product application mounted but visually hidden while showing Build details or File Review;
- always docks an `ApplicationWidget` over the application;
- polls proof navigation every 300 ms;
- supplies an explicit `Build details` return path around the shared File Review screen.

See `src/features/worktreeBuild/WorktreeBuildShell.tsx:13-138`.

Its direct Tauri commands are `worktree_build_context`, `worktree_build_detail`, `worktree_build_comparison`, `mark_worktree_build_ready`, and `worktree_review_proof_navigation` (`src/infrastructure/tauriWorktreeBuild.ts:9-21`; registered at `src-tauri/src/active_app.rs:371-380`). `mark_worktree_build_ready` changes the exact window title/size, shows it without activation on Windows, and atomically writes `application-surface-rendered` to the runtime-provided readiness path (`src-tauri/src/worktree_review/transport.rs:126-165`).

This is not a lightweight review-only executable. It is a full Codex Orchestrator product build from the selected worktree, with an additional shell and debug-only native commands.

## 5. Proof and control seams woven into the experiences

Three different proof-navigation mechanisms reach these visible surfaces:

1. `App` polls `human_review_launcher_proof_navigation` every 300 ms to enter the launcher.
2. `HumanReviewLauncherView` separately polls typed proof presentation and detail navigation every 300 ms to select source, detail, and retained operation output.
3. `WorktreeBuildShell` polls `worktree_review_proof_navigation` every 300 ms. That command reads and validates a JSON file named by the runtime environment.

The first two are in-memory state on `HumanReviewLauncherService`; the third is a file under the isolated instance. See `src/features/humanReviewLauncher/HumanReviewLauncherView.tsx:49-89,117-149`, `src/features/worktreeBuild/WorktreeBuildShell.tsx:52-84`, and `src-tauri/src/worktree_review/debug_controller.rs:36-75`.

When `CODEX_ORCHESTRATOR_REVIEW_CONTROLLER=enabled`, debug composition also starts a loopback Axum controller with a generated capability, descriptor file, strict command shapes, bounded output, and shutdown cleanup (`debug_controller.rs:32-35,77-120` and following). Two Rust examples drive evidence capture:

- `src-tauri/examples/worktree_review_controller.rs` reads that descriptor, sends capability-protected commands, watches operations, writes new proof output files, and records foreground-window ownership.
- `src-tauri/examples/worktree_review_background_launcher.rs` starts the launcher without activation and terminates it if foreground ownership changes.

These examples and proof routes are active code artifacts for controlled review evidence, but none is part of the release command surface.

## 6. Adjacent parallel and historical surfaces reached from this behavior

### The older Node worktree-runtime CLI remains exposed

`package.json` still exposes:

- `npm run runtime:worktree -- ...` -> `scripts/worktree-runtime.mjs`;
- `npm run test:worktree-runtime` -> `scripts/worktree-runtime.node-test.mjs`.

The CLI supports `prepare`, `install`, `build`, `test`, `start`, `status`, `stop`, and `recover`, storing a manifest below `.dev/worktree-runtime/<instance>` (`scripts/worktree-runtime.mjs:136-166,194-312,1073-1074`). It implements its own process ownership checks, ports, status server, environment, build/test commands, and lifecycle. The visible Human Review launcher does not call this script; it uses the separate Rust `worktree_runtime` module and review-root SQLite registries.

This leaves two operational implementations of similar worktree-isolation behavior with different state roots and entry points:

- manually exposed Node CLI and JSON manifest under `.dev/worktree-runtime`;
- debug Tauri Human Review launcher backed by Rust and SQLite under the review runtime root.

### An earlier Worktree Runtime proof view remains in the current tree but has no composition path

The following current files reference only one another and their tests:

- `src/application/worktreeRuntime.ts`;
- `src/dev/worktreeRuntime/createDevelopmentWorktreeRuntimeSource.ts`;
- `src/features/worktreeRuntime/WorktreeRuntimeExplorationView.tsx` and CSS.

The source reads `VITE_RUNTIME_*` values and a status endpoint, otherwise producing a recorded fallback. The screen labels itself `Development proof` and is inspect-only. No current `ApplicationRoot`, `App`, or other production/dev composition imports or renders it. It was visibly composed as a `Worktree Runtime Dev` tab in commit `673ddf3` on the exploratory lineage; the current launcher experience replaced that route without removing these artifacts.

### Compatibility contract

`src-tauri/worktree-review-contract.json` is not merely documentation: `ReviewWorktreeCatalog` checks it to mark candidate worktrees compatible or incompatible (`src-tauri/src/worktree_review/catalog.rs:239` and surrounding code). The launcher disables Build/Open for incompatible sources. This file is configuration that participates in application behavior.

## 7. Sibling-branch lineage

The exploratory branches are useful design evidence, but they are not literal ancestors of the corresponding current implementation commits.

| Branch | Tip | Relationship to current HEAD | What it shows |
| --- | --- | --- | --- |
| `codex/explore-file-diff-viewer` | `ba130cf` | merge base `4a3d5df`; 2 branch-only commits | Initial `?file-diff-viewer` recorded viewer, then application-owned source |
| `codex/explore-worktree-runtime` | `44de3ce` | merge base `4a3d5df`; 8 branch-only commits | Early runtime proof view, Node runtime, then first Human Review launcher |
| `codex/worktree-review-progress` | `d2e50cd` | merge base `4a3d5df`; 12 branch-only commits | Progress, background proof, File Review integration, and retained evidence work |
| `codex/integration-batch-07-file-review-scoped-source` | `d4eb0de` | tip is current ancestor | Scoped source convergence |
| `codex/integration-batch-09-file-review-facts` | `0f20243` | one branch-only commit; current has parallel `3cfbd7a` | Active-v3 durable File Review facts work |
| `codex/integration-batch-14-worktree-runtime-convergence` | `89fd514` | tip is current ancestor | Large convergence commit `8ce6c09` plus Unicode correction |
| `codex/integration-batch-15-initiated-sprint-git-authority` | `5d7479f` | tip is current ancestor | Runtime-backed initiated Sprint Git authority and frozen baseline |

Notable current-line commits, in behavior order:

- `7bd5545` — reconcile Agent Sessions with Sprint detail review;
- `d4eb0de` — scope File Review sources;
- `3cfbd7a`, `20f9f95` — durable native facts and typed invoke;
- `7464021`, `c255180`, `82d6a78` — authorize, produce, and isolate Git capture;
- `8ce6c09`, `89fd514` — converge the worktree runtime review surface;
- `e970e35`, `cd33bfd`, `5d7479f` — bind and freeze initiated Sprint Git authority;
- `a6140de` — originate File Review from Sprint authority;
- `4e38435` — add contextual invocation;
- `8a69194` — link accepted candidates to exact capture evidence.

The branch topology matters: sibling exploration history explains intent and discarded variants, but should not be treated as proof that its exact code was merged unchanged.

## 8. Artifact map

| Behavior/layer | Primary artifacts and symbols |
| --- | --- |
| Shared display contract | `src/application/fileReview.ts`: `FileReviewSource`, `FileReviewSnapshot`, `assertCompleteFileReviewFile` |
| Shared display | `src/features/fileReview/FileReviewScreen.tsx`, `fileReview.css` |
| Stored source decoding | `src/application/applicationOwnedFileReview.ts`: `createApplicationOwnedFileReviewSource`, `STORED_FILE_REVIEW_ARTIFACT_V1` |
| Contextual application boundary | `src/application/contextualFileReview.ts`: `ContextualFileReviewClient`, result/reason types |
| Contextual Tauri adapters | `src/infrastructure/fileReview/tauriContextualFileReview.ts`, `tauriScopedFileReview.ts` |
| Sprint entry and top-level routing | `SprintWorkspace.tsx`, `App.tsx`, `productApplicationComposition.ts` |
| Native contextual commands | `src-tauri/src/orchestration/transport.rs`: `request_contextual_file_review`, `load_scoped_file_review` |
| Durable producer | `file_review_originating_entry.rs`, `initiated_sprint_git_authority.rs`, `file_review_git_producer.rs`, `repository.rs` |
| Other producer consumer | `execution_support.rs`: `ProductExecutionWorkspaceResolver::inspect` |
| Recorded fixtures | `src/dev/fileReview/recordedFileReviewClient.ts`; route selection in `ApplicationRoot.tsx` |
| Human Review application contract | `src/application/humanReviewLauncher.ts` |
| Human Review view/adapter | `HumanReviewLauncherView.tsx`, `src/infrastructure/tauriHumanReviewLauncher.ts` |
| Human Review native composition | `src-tauri/src/worktree_review/composition.rs`, `service.rs`, `transport.rs`, `catalog.rs` |
| Rust isolation runtime | `src-tauri/src/worktree_runtime/*` |
| Review detail/progress/evidence | `worktree_review/detail.rs`, `progress.rs`, `proof_evidence.rs` |
| Direct worktree comparison | `worktree_review/comparison.rs`, `worktree_build.rs` |
| Isolated child shell | `src/application/worktreeBuild.ts`, `tauriWorktreeBuild.ts`, `features/worktreeBuild/*` |
| Debug proof controller | `worktree_review/debug_controller.rs`, `src-tauri/examples/worktree_review_controller.rs`, `worktree_review_background_launcher.rs` |
| Parallel Node CLI | `scripts/worktree-runtime.mjs`, `scripts/worktree-runtime.node-test.mjs`, `package.json` |
| Unmounted development proof | `src/application/worktreeRuntime.ts`, `src/dev/worktreeRuntime/*`, `src/features/worktreeRuntime/*` |
| Behavior-driving compatibility config | `src-tauri/worktree-review-contract.json` |

## 9. Open ambiguities for later passes

1. Is release intended to keep a visible, predictably `not_ready` Sprint action until a release-safe runtime comparison provider exists, or should command capability determine visibility?
2. Is `load_scoped_file_review` meant to gain a visible route for already-recorded Documents, especially since the native query already projects them into Sprint Documents?
3. Should contextual File Review retain and restore the exact Epic/Sprint/revision/detail origin, and should its source gain a peer tab after success?
4. Are generic Document Resolve/Open/Copy actions the intended future entry to File Review, or is the dedicated opaque-source path intentionally separate?
5. Should Human Review comparison snapshots be memoized by `instanceRef` to prevent progress polling from reloading the viewer?
6. Should committed/uncommitted provenance produced by direct worktree comparison be surfaced or removed from the native DTO?
7. Should direct worktree comparison and stored Sprint comparison converge on one contract, limits policy, rename policy, and validation boundary, or remain intentionally different?
8. Is the Node `runtime:worktree` CLI still an operationally supported tool, a fallback, or retained exploration material now that the Rust runtime backs the launcher?
9. Is the unmounted `WorktreeRuntimeExplorationView` intentionally retained for future use, or is it an obsolete presentation layer from the predecessor flow?
10. Which proof-controller examples, polling routes, and navigation fields must remain product-adjacent, and which can be segmented into explicit review/proof tooling?
11. Should live operation progress survive launcher restart? Review sessions and logs are durable, while the current progress registry and proof navigation are volatile.
12. Does every supported deployment pair Vite development with Rust debug, or must composition explicitly handle mismatched combinations such as browser-only Vite dev and production frontend/debug Rust?
