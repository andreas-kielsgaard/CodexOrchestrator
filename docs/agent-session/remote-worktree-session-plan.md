# Remote worktree targets for ordinary Agent Sessions

Status: implemented and verified through the native laptop UI against both devices. See [implementation evidence](remote-worktree-session-evidence.md) for results and setup.
Date: 2026-09-14.

## Objective and scope

Launch Orchid on this Windows laptop, create an ordinary Agent Session, select an existing worktree on the laptop or the Hetzner server, and send a prompt through Codex running on the selected device. Follow-up prompts retain that worktree and provider context.

The target modal follows **repository/project -> branch -> device/worktree instance**. It reuses the Worktree Review branch presentation. Selecting an instance also selects its Capability Profile. Each profile describes exactly one device and the connection to its Orchid execution engine. Configure a laptop profile and a server profile for the demonstration.

Included: profile connection editing in Technical Settings, remote capability discovery, existing-worktree discovery, target selection, Codex execution, streamed interaction, follow-up turns, cancellation, and visible target information.

Excluded: remote workflow functionality/callbacks, node-file history, validation/build product features, worktree creation controls or API, Claude implementation, dirty-state transfer, migration of existing sessions, synchronization, scheduling, detached/background remote execution, automatic reconnection/replay, and a security-hardening project. Development checks and ordinary connection/launch errors remain necessary.

Manual setup may create demonstration worktrees. Each new remote worktree starts at an exact published commit; later agent changes remain in that worktree. The application never resets it between prompts.

## Pre-implementation findings and consequences

| Existing location | Finding and consequence |
| --- | --- |
| `src/features/worktreeReview/BranchNavigator.tsx`, `branchSelection/BranchGraphDialog.tsx`, `BranchGraph.tsx` | Presentation is already partly extracted, but repository registration, review targets, graph loading, dialog ownership, and commit-range review are mixed. Extract the branch browsing surface for both consumers. |
| `src-tauri/src/repository_catalog/`, `repository_context/identity.rs` | The catalog registers local Git clones; Git identity uses the canonical common-directory path. Associate a remote clone with an existing catalog repository explicitly instead of equating paths or branch names across devices. |
| `src/features/executionConfiguration/ExecutionConfigurationScreen.tsx`, `CapabilityProfileEditor.tsx` | Every profile currently receives a single selected local runtime. Discovery must use the edited profile's execution binding. |
| `src/app/App.tsx`, `src/features/nativeProfiles/NativeProfileSettings.tsx` | Technical Settings currently mounts native-home management separately from Capability Profiles. Compose the shared profile editor into Technical Settings; retain native-home management for local configuration. |
| `src-tauri/src/execution_configuration/{capability_profile,service,ports,native_codex}.rs` | Capability Profiles own allowed capabilities/defaults; native selection is a separate global source. Add profile execution binding and resolve that profile's device/runtime source. |
| `src-tauri/src/agent_sessions/application/configuration.rs`, `transport/selections.rs` | Ordinary first send goes through `start_direct_user_session` and resolves the default profile. Carry the selected target/profile through this path; do not create a competing session-launch path. |
| `src/features/agentSessions/useAgentSessionController.ts` | The profiled first-send branch takes precedence over `startSession`. Extending only that fallback would miss production execution. |
| `src-tauri/src/active_app/sessions.rs`, `runtime/codex/app_server/` | The active application binds one local Codex app-server adapter. Preserve its behavior and share its implementation between local execution and the remote host. |
| `src-tauri/src/agent_sessions/workspace.rs`, `native_profiles/session_binding.rs` | Working directories and native launch configuration are prepared on the laptop. A remote target must use destination-side path/configuration resolution. |

The active boundaries in `src-tauri/AGENTS.md` apply. New functionality belongs outside the quarantined task/run implementation in `src-tauri/src/lib.rs`.

## Chosen execution shape

```text
Agent Session view and Technical Settings
  -> laptop application services and ActiveDatabase
  -> profile-selected execution endpoint
       local: shared Orchid execution engine
       SSH:   orchid-host on the server -> same execution engine
                                           -> Codex app-server
                                           -> remote worktree
  <- normalized events and provider interaction
```

Create one Rust package, `crates/orchid-engine/`, containing a library and an `orchid-host` binary. The desktop depends on the library by path. The binary has no Tauri, WebView, workflow engine, or desktop SQLite composition dependency. No root Cargo workspace is required for this slice.

Here, the **remote session engine** means the execution portion of the existing engine: native configuration discovery, provider start/resume, active requests, and process ownership. The laptop's `AgentSessionApplication` continues owning product sessions, invocations, profiles, and event persistence. Do not instantiate a second product session/history service remotely. This makes the earlier headless-engine proposal concrete without introducing synchronization.

The existing Codex adapter is invocation-scoped and can resume a persisted provider thread. Preserve that lifecycle; a permanently running app-server per conversation is unnecessary. The host keeps active invocation state and a small session/configuration/worktree/provider-ID binding; Codex keeps its native thread data. The laptop stores the returned provider context ID with the session. The host binding is not a second transcript or invocation-history store.

An SSH connection can remain open while a remote turn is active. `orchid-host` reads requests while emitting events, so approvals and cancellation are usable during execution. EOF performs the engine's normal shutdown. Unexpected loss of the connection is shown as disconnected with an unconfirmed remote outcome; it does not resubmit the prompt.

## Records and contracts

### Capability Profile execution binding

Extend `CapabilityProfile` with an execution binding containing:

- device ID and display name;
- connection: local, or SSH target/alias and remote `orchid-host` executable path;
- execution configuration reference: existing local native profile or a named configuration exposed by the remote host;
- provider kind, initially Codex only.

The remote configuration reference maps on the host to the Codex executable and native home. Model choices, allowed capabilities, and defaults remain the profile's existing concerns. Discover capabilities in the selected worktree context before first send because project configuration can affect them.

Use one local and one remote profile in the demo. Do not impose device uniqueness on all existing profiles: each profile has one device, but existing local profiles can share it. Where more than one profile offers a worktree, include the profile name in the choice so selection is explicit. Do not silently pick the first profile.

Connection fields are edited as part of the Capability Profile in Technical Settings. Persist ordinary connection references; use the existing SSH key/agent and destination Codex login. The profile is not a credential store.

### Repository locations and worktree targets

Add `RepositoryDeviceLocation { repositoryId, deviceId, repositoryRoot }` under the repository catalog. For this laptop-owned prototype, `repositoryId` is the existing selected catalog repository. A mapping associates its remote clone with it; no global repository identity migration is needed.

Expose the small mapping editor alongside profile connection configuration: select a registered repository and enter its path on that device. The laptop location comes from current registration. These are clone locations, not new worktree registrations.

The destination reads standard Git branch/worktree facts from that location. Match instances by catalog repository and full branch ref. Device-local Git IDs remain device-local. Return a worktree handle, branch ref, path, and observed HEAD; do not treat identical commits as proof that two instances belong to the same branch.

The laptop's selected repository supplies the branch browser in this slice. Remote instances are matched to those branch refs through the mapping. A remote-only branch becomes selectable when that branch is made available in the laptop repository; multi-device graph merging is outside scope. Main working trees count as instances when attached to the selected branch. Detached worktrees do not acquire a branch merely because their commit matches.

Persist a `SessionExecutionTarget` with the session: Capability Profile identity/revision, resolved device/connection/configuration binding, repository ID, branch ref, worktree handle/path, and observed starting HEAD. This is separate from the existing Session Profile digest. Selection is mutable only in a new-session draft; first send fixes the execution binding. Subsequent sends carry the session ID, not a new target.

Remote paths are opaque to Windows. The destination resolves and checks them. The same applies to Codex home and skill paths.

### Endpoint operations

Keep a small shared request/event contract in `orchid-engine/src/protocol.rs`:

- describe the host and execution configurations;
- list existing worktrees for a mapped repository/branch;
- describe effective runtime capabilities for configuration plus worktree;
- start/resume an invocation with session/invocation IDs, target, prompt, and semantic selections;
- respond to a provider request;
- cancel an invocation;
- stream normalized runtime events, provider context, requests, and terminal outcome.

Use request IDs for matching replies, invocation IDs for stream routing, and the existing event ordering/persistence path. Reserve stdout for protocol frames and stderr for diagnostics. There is no materialize/create-worktree or workflow command in this protocol.

The desktop's remote adapter exposes the existing `AgentRuntime` semantics. Configuration discovery and workspace access use the same endpoint connection, so the model catalog and actual launch refer to the same destination. Resolve the endpoint before runtime preflight; preflight currently lacks target context and must receive a bound endpoint or explicit context rather than consulting mutable global selection.

## Ownership and file changes

All paths below are repository-relative. Names are the proposed implementation homes; adjust a name only if implementation evidence gives a clearer home.

### Shared engine extraction

| Action | Files/objects | Result |
| --- | --- | --- |
| Create | `crates/orchid-engine/Cargo.toml`, `src/lib.rs`, `src/bin/orchid-host.rs`, `src/host.rs`, `src/protocol.rs` | One reusable execution library and a thin stdio server composition. |
| Extract | Runtime-only IDs, options, outcomes, normalized event/usage types from `src-tauri/src/agent_sessions/domain.rs`; runtime port/data from `agent_sessions/ports/runtime.rs` -> engine `src/contracts/` | Provider code no longer imports product Session records, identity, or Harness definitions. Keep `AgentSession`, persisted invocations/events, and product metadata in the desktop domain. |
| Move | `src-tauri/src/runtime/codex/app_server/`, active normalizer `runtime/codex/protocol.rs`, executable resolution, and `runtime/processes/` -> engine `src/codex/` and `src/processes/` | One Codex implementation and one process supervisor used by both devices. Move relevant tests and fixtures with their owner. |
| Extract | Runtime capability/value types from `execution_configuration/runtime_profile.rs` and `inventory.rs`; pure Codex environment-to-capabilities conversion from `native_codex.rs` -> engine `src/configuration/` | Shared capability interpretation. Saved profiles, restrictions, and application selection remain desktop-owned. Keep a thin local source adapter around `NativeProfileService`. |
| Move | `src-tauri/src/repository_context/` and `git_process.rs` -> engine `src/repository_context/` and `src/git_process.rs` | Reuse read-only branch/worktree facts on Windows and Linux. Update existing catalog and review consumers to the new library imports. Keep physical checkout/build services in their current product modules. |
| Adapt | `src-tauri/Cargo.toml`, module declarations, callers of moved modules, affected test imports | Depend on the engine library; remove the old implementation files after import migration. Test-only legacy `codex exec` fixtures/adapters remain test-only and consume shared contracts where required. |

Expose the minimum public Rust API needed by the desktop and host. Do not move the entire `AgentSessionApplication`, database, Harness engine, native-profile repository, or quarantined task implementation to obtain reuse. Do not include desktop files by alternate `#[path]` trees or copy the Codex adapter into another language.

### Laptop application and persistence

| Action | Files/objects | Result |
| --- | --- | --- |
| Create | `src-tauri/src/execution_targets/{mod,domain,endpoints,remote_runtime,ssh_connection,transport}.rs` | Profile-selected local/SSH endpoint composition, remote runtime adapter, and target discovery facade. SSH transport remains separate from session semantics. |
| Adapt | `execution_configuration/{capability_profile,service,ports,resolution,session_profile,native_codex,repository,transport}.rs` | Save profile execution binding; discovery and selection resolution use the profile's endpoint. Keep per-message choice/default semantics intact. |
| Create/adapt | `repository_catalog/device_locations.rs`, catalog repository/transport | Store explicit repository-to-device clone mappings and compose instance discovery through endpoints. Registration and Worktree Review remain existing consumers of the same catalog. |
| Adapt | `agent_sessions/application/{configuration,creation,invocation,workspaces,interactions}.rs`, `domain.rs`, repository mapping/schema, transport DTOs/selections | Carry target through the existing profiled first-send path; persist it; bind subsequent execution and interaction to it. Resolve remote paths/configuration remotely. |
| Adapt | `active_app/{sessions,execution_configuration}.rs`, `active_app.rs`, `storage.rs` | Compose local/SSH execution; add active migrations and commands. Remove the single-global-runtime assumption for targeted sessions. |

Migrate saved profiles to a local execution binding that preserves their current native configuration behavior. Existing session history remains local and readable; do not recompute stored Session Profile digests or reinterpret old paths as remote. Add the target as a separate optional session field, with existing sessions following the established local path. Profile changes affect new bindings; active sessions keep their stored device/worktree/configuration choice.

The old ordinary local path remains available for untargeted local drafts. A remote profile requires an existing remote worktree selection. The remote launch never runs through the laptop's workspace allocator or laptop native-home launch authority. Only native capabilities actually available on the destination are exposed; this slice does not advertise Orchid workflow tools remotely.

Existing workflow callers remain local-only. If they encounter a remote profile, the shared execution boundary reports that target as unsupported for that caller; it must not run it locally. This is a scope guard in profile resolution, not remote workflow integration.

### Shared branch UI and session targeting

| Action | Files/objects | Result |
| --- | --- | --- |
| Extract | `worktreeReview/BranchNavigator.tsx`, branch list rendering, branch graph browsing/selection from `branchSelection/BranchGraphDialog.tsx` -> `src/features/branches/BranchBrowser.tsx` and related shared files | A repository-scoped browser with selected branch/callback and a narrow branch read source. Preserve the existing visual branch presentation, including graph browsing. |
| Move/adapt | `BranchGraph.tsx`, `branchGraphLayout.ts`, `branchLineage.ts`, shared styles/types/tests -> `features/branches/` and `application/branches/` | Shared branch rendering stops depending on the complete Worktree Review client. Repository registration, review actions, commit-range review, and review dialog orchestration stay in Worktree Review. |
| Adapt | `WorktreeReviewScreen.tsx`, `BranchNavigator.tsx`, `BranchGraphDialog.tsx`, review client adapters | Existing review consumes the shared browser. Replace extracted implementations rather than retaining a second branch list/graph. Existing read-only graph queries can back the narrow adapter; no duplicate Git graph parser. |
| Create | `src/application/executionTargets/contracts.ts`, `src/infrastructure/executionTargets/tauriExecutionTargetClient.ts` | Typed target/profile/device discovery contracts. No SSH or Git execution in React. |
| Create | `src/features/agentSessions/SessionTargetControl.tsx`, `SessionTargetDialog.tsx`, `useSessionTarget.ts` | Repository choice, shared branch browser, device groups, existing instance rows, and selection state. |
| Adapt | `AgentSessionWorkspace.tsx`, `AgentSessionScreen.tsx`, `useAgentSessionController.ts`, `useSessionExecutionSelection.ts`, `application/agentSessions/selections.ts`, Tauri session client | Put the target control in the reusable session view, enabled for new ordinary sessions and read-only for a started targeted session. Extend the production profiled first-send input. Reuse transcript, requests, and cancel UI. |
| Create/adapt | `src/features/technicalSettings/TechnicalSettingsScreen.tsx`, existing Capability Profile editor/screen/types/client, `App.tsx`, `productApplicationComposition.ts` | Technical Settings composes the shared profile connection editor and existing local native settings. The Capabilities entry uses the same saved profile model/editor. |

The modal has one enclosing dialog. Choosing a repository resets branch/instance selection; choosing a branch refreshes the device groups. Display each configured device, profile label where needed, and existing matching instances with path and HEAD. Distinguish an empty list from an unavailable connection. Do not auto-select another device when one is unavailable.

Selecting a target preserves the prompt draft and updates the effective Capability Profile/model controls together. Before first send, selection is draft state only. Reopening the modal restores it. After first send, show the bound repository/branch/device/path and disable retargeting. Keep target selection optional at the shared component boundary so embedding the session view does not introduce workflow functionality.

### Concrete removal boundaries

- Remove duplicate active Codex, process, and read-only Git implementations from their old locations once consumers use the engine library.
- Remove branch rendering/loading that was extracted from the review wrappers; wrappers retain only review-specific composition.
- Replace global local-runtime discovery in profile editing and targeted first send with profile-specific resolution.
- Do not introduce a separate device connection store that competes with Capability Profiles, a second remote transcript database, a generic inference adapter, or an alternate session composer/controller.
- Keep unrelated dirty files, ongoing repository-session navigation work, and the separate legacy-retirement plan untouched. Recheck the checkout before implementation.

## Fresh server work

The server setup task, `Generate Hetzner SSH key` (`01a0a031-42ba-7d61-89a1-bfd4230b3cd3`), confirms interactive SSH to `root@2.28.122.21`, hostname `ubuntu-8gb-nbg1-1`, using `C:\Users\user\.ssh\orchid_hetzner`. It does not establish installed Git/Codex/build tools or provider authentication. A subsequent check in this conversation found the Windows SSH agent disabled and noninteractive access failing.

Complete the companion [fresh-server setup plan](remote-worktree-server-setup.md) during implementation. Its result is an SSH-accessible host binary, authenticated Codex configuration, mapped clone, and manually created branch worktree. No server change is claimed by this document.

## Implementation sequence and proof

1. **Make the shared engine headless.** Extract contracts, Codex/process code, capability interpretation, and read-only Git readers. Switch the desktop's local Codex path to the library. Build library/binary without Tauri and run the moved focused tests. Prove an ordinary local session still works before adding SSH routing.
2. **Prepare the server and host transport.** Enable usable SSH access, inspect/install prerequisites, deploy the host, complete provider sign-in, and manually create the demonstration worktree. Exercise host description, worktree listing, first prompt, and continuation through SSH. Record actual versions/paths; CLI success alone is not the final acceptance.
3. **Bind Capability Profiles and sessions.** Add profile/device connection persistence, repository locations, endpoint selection, and stored Session targets. Route capabilities, preflight, launch, responses, and cancellation through the selected endpoint. Test the profiled first-send and follow-up paths, including a Linux path handled from Windows.
4. **Reuse the branch view and wire the modal.** Update both review and session consumers, compose Technical Settings, and configure the laptop/server profiles through the product UI. Test selection propagation and prompt-draft preservation.
5. **Launch Orchid and exercise the actual flow.** Open Technical Settings, then New Session -> Target worktree -> repository -> branch -> remote instance. Send a prompt that reports its actual directory, reads repository content, and makes a small identifiable edit. Verify the file on the server, then send a follow-up that observes it with the same provider thread and worktree. Repeat target selection for a laptop instance. Exercise one provider request/response and cancellation while connected. Reload the completed session and confirm the displayed history and target remain.

Focused automated coverage: profile-selected discovery, migration/local defaults, target serialization, first-send binding/follow-up routing, shared branch UI reuse, device grouping/empty state, host request/event/response framing, and cancellation. Use one real remote smoke test for the happy flow. No robustness matrix, fuzzing, automatic recovery, or workflow tests are added as prototype features.

Run the relevant existing frontend tests and `npm run build`, the affected Rust suites, and `cargo check --manifest-path src-tauri/Cargo.toml`. Build/test the new engine independently and build `orchid-host` for the actual server architecture. Review the modal and settings visually in the launched app; focused tests alone do not establish UI or remote-provider acceptance.

Record implementation evidence in `docs/agent-session/remote-worktree-session-evidence.md`: tested source revision, host/Codex versions, selected profile/target, screenshots, remote directory/file evidence, continuation and cancellation outcome. Do not record credentials. Describe any remaining untested behavior rather than broadening the acceptance claim.

## Remaining implementation facts to resolve

- Actual server OS release/architecture and installed prerequisites; the hostname alone does not establish these.
- Codex sign-in availability and the installed app-server version used for the demonstration.
- Remote clone access for the chosen repository and the exact published starting commit.
- Reconcile file ownership with concurrent checkout changes immediately before implementation; the plan assumes the active modules inspected on 2026-09-14.

These are setup/discovery tasks in the sequence, not requests to redesign the agreed UI or expand scope.
