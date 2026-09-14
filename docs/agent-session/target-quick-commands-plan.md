# Device and worktree quick commands

Status: implemented and verified in the native app against the laptop and Hetzner server. See acceptance evidence below.
Baseline: `feature/Remote-Development` at `32ddfe3`, rebased onto main `3ecfd94`.

## Agreed behavior

- `/worktree` opens local worktree choices. The composer becomes `/`, ready to filter and select a worktree.
- `/device` opens configured device choices. Selecting a device opens that device's worktrees, again with `/` ready for filtering.
- A worktree choice identifies its branch and instance. Selecting it applies the same complete Session execution target as the existing modal, consumes the command, and leaves the composer ready for a prompt. It does not send a prompt or create a Session.
- The device selection is provisional until a worktree is selected. Back or Escape leaves the current target unchanged.
- Target commands are available only before the first prompt creates a Session. A completed or failed existing Session keeps its target; being idle does not make it retargetable.
- Existing model, reasoning, skills, navigation, folder placement, and target-modal behavior remain available.

## Assumptions and boundaries

The quick route lists instances across registered repositories, with repository identity visible in each choice; it adds no repository-selection stage. This differs intentionally from the modal's repository-first browsing route. A navigation folder does not silently filter the list or move when a target is chosen. The explicit target continues to determine execution location.

Local means the laptop running Orchid, determined by local execution connections, even when the current draft target is remote. Configured Capability Profiles remain the source of devices and execution connections. No second device registry is introduced.

For remote enumeration, only registered repositories with a configured path on the selected device can be inspected. This is not a server-wide filesystem search. Devices with no mappings or no instances remain visible with an explanatory empty state.

Several Capability Profiles may expose the same worktree. Offer separate, clearly profile-qualified choices instead of silently choosing a profile or adding another selection stage. A choice retains repository, device, profile revision, execution binding, worktree ID, path, branch, and HEAD.

Use branch name plus the worktree directory name as the primary label, for example `feature/Remote-Development · remote-development`. Show repository, profile, path, and a short instance ID as supporting information; retain the full ID for identity and search. Labels never serve as execution identifiers. Include available branch-backed primary checkouts and linked worktrees; detached instances remain outside this slice because the current target contract requires a branch.

Keep the existing command-only entry convention: commands start an otherwise empty message. This slice does not introduce inline command parsing inside prose or the literal chained syntax `/device /server /branch`. Each selection replaces the current query with `/` and the menu heading shows the current stage. Slash characters in branch names must work while filtering worktree choices.

No worktree creation, workflow integration, retargeting existing Sessions, migration/synchronization, remote native model/skill discovery, or new provider support. No schema migration or new persisted command state.

## Findings that determine the shape

1. `useComposerQuickMenu.ts` already owns nested selection, filtering, keyboard handling, focus, and stale-context rejection. Its action children are synchronous, and one provider catalog's loading/error state currently blocks the whole menu. Product-owned target commands must not depend on that catalog; remote native quick-feature discovery is currently unavailable.
2. The root query regex rejects additional `/` characters. That preserves literal paths, but cannot be used unchanged inside a worktree picker because branch names commonly contain slashes.
3. `ExecutionTargetService::targets(repository, branch)` enumerates configured profiles and repository paths. It is tailored to the modal and queries all devices. Using it once per branch from the composer would duplicate discovery and connect to unrelated devices.
4. `ExecutionEndpoints::worktrees` requires a branch, but the shared engine and deployed host already support `ListWorktrees { repository_root, branch_ref: None }`. All-branch inventory needs a desktop adaptation, not another server API.
5. `useSessionTarget` owns draft target selection and subsequent profile/runtime loading. The screen uses that state for the modal, capability controls, and first-send readiness. The commands should call this same owner.
6. Main's `useProfiledAgentSession` and `useAgentSession` already retain folder placement and pass an execution target on first send only. Command parsing does not belong in session execution or provider adapters.

## Proposed ownership and changes

### 1. Device-oriented inventory in the existing target service

Retain `ExecutionTargetService` as the application owner. Extract its profile grouping, repository-root resolution, and inventory projection from `src-tauri/src/execution_targets/mod.rs` into `inventory.rs`, shared by the modal query and the new device-oriented query. Keep endpoint transport in `endpoints.rs`.

Add two read operations through `domain.rs`, `transport.rs`, `active_app.rs`, `src/application/executionTargets/contracts.ts`, and `src/infrastructure/executionTargets/tauriExecutionTargetClient.ts`:

- `listDevices()` returns configured devices and their profile identities, without contacting remote hosts.
- `listWorktreeChoices(scope)` accepts either local connections or a specific device ID. It returns repository/profile groups of existing instances, with repository labels and per-group discovery errors. It enumerates each applicable registered repository directly; it does not enumerate branches first.

Keep `listTargets(repositoryId, branchRef)` for the modal. Both operations consume the extracted inventory behavior; the modal keeps its existing response shape and interaction.

Adapt `ExecutionEndpoints::worktrees` to accept an optional branch filter, pass it to the existing host/local engine operation, and preserve each returned instance's actual branch. Filter detached instances deliberately when constructing selectable targets; never manufacture a branch from the query. Query only the requested device's connections. A failed repository/profile should not hide successful groups.

No new server command, daemon, dependency, deployment, or configuration is expected. Verify the existing remote host with the unfiltered read request during acceptance.

### 2. Shared target choice presentation

Create `src/application/executionTargets/presentation.ts` for conversion from repository/profile/instance facts to `SessionExecutionTargetDto`, display labels, search terms, and stable choice keys. Both `SessionTargetDialog.tsx` and the command action factory adopt it. Remove the modal's inline target-object construction so the two entry points cannot drift in which profile revision or instance fields they bind.

Keys include repository, device, profile, and instance identity. Search covers branch, directory name, repository, profile, path, and instance ID. No label parsing is used to reconstruct a target.

### 3. Target actions separate from native quick features

Create `composerTargetActions.ts` alongside `composerQuickActions.ts`. It defines the two product-owned commands from a small source containing inventory loaders, current selection, availability, and an `onSelectTarget` callback. It does not resolve profiles, construct SSH commands, or send prompts.

`AgentSessionScreen.tsx` composes this source from the existing execution-target client and `targetDraft.setTarget`. Pass it through optional props on `AgentSessionWorkspace.tsx`, `ConversationViewport.tsx`, and `AgentSessionComposer.tsx`. Do not inject target commands into workflow or harness composers that do not already offer ordinary Session target selection.

Retain `useSessionTarget` for target/runtime/profile state. Both the modal and commands call its selection operation. Existing target changes reset message-local model/effort choices and reload target capabilities through the established path. Choosing only a device does not call it.

Use the same availability rule as the target control, based on the existence of a Session and send state. Existing Sessions may show the commands disabled with the explanation that their target is fixed. The two commands are independently usable when a native quick-feature loader is absent or fails.

### 4. Extend the existing menu's navigation, not a second command parser

Retain `ComposerQuickAction` and its static-child and local-action cases. Add a narrow asynchronous-child case for device and worktree pages. Move shared action/page types to `composerQuickMenuTypes.ts` if needed to keep the provider and target action factories independent. This is a menu extension, not a general command registry or plugin framework.

Adapt `useComposerQuickMenu.ts` to hold the current page and its parent navigation frames, with page-specific loading/error/retry state. Load devices when `/device` is entered; load inventory when `/worktree` or a device is selected. Discard responses after backing out, changing devices, changing Session/draft identity, or selecting a target. Reload on re-entry; do not add a background cache or synchronization process.

Compose target roots immediately. Native catalog loading/error must affect native choices only; it must not clear target roots or prevent selecting them. Preserve direct skill shortcuts after native discovery completes. Remove the current global `loading || error` selection gate and the renderer condition that hides every item when native discovery fails; replace them with the relevant page/action state.

Keep root recognition strict so ordinary slash-containing paths remain text. Once inside worktree selection, accept a slash-prefixed query containing additional slashes and spaces. Store selected device/worktree identity in menu state, not in a parsed command string. Escape and Backspace return to the actual prior stage and query; opaque action IDs must not appear in the composer.

Retain the existing listbox, highlighted choice, Arrow keys, Enter/Tab selection, IME handling, Shift+Enter, focus return, and form-submit interception. Loading, empty, or failed target pages never submit their query as a prompt. Escape at the root still permits intentional literal slash text.

Update `ComposerQuickMenu.tsx`, its CSS only as required for stage/choice descriptions, and composer hints for the new commands. A completed choice clears the command and announces the selected branch/device; the existing target control remains the durable visible indication.

## Delivery order

1. Add device-oriented inventory and shared target projection; move the modal onto the shared helpers. Preserve its repository/branch route.
2. Extend menu pages for asynchronous product choices while preserving native command behavior and independent failure states.
3. Wire `/worktree` and `/device` into ordinary new-Session composers through the existing target owner.
4. Run focused checks, then verify the laptop/server flow through the native app. Update `composer-quick-features.md` with implemented behavior and record acceptance evidence.

## Verification

Backend tests should prove local-only enumeration never reaches SSH; device scope excludes other devices; optional branch filtering returns actual branches; repository/profile errors remain localized; and a complete target preserves profile revision and instance identity. Test duplicate branch names across repositories and profiles. Keep existing branch-filtered modal query coverage.

Composer tests should exercise the full keyboard paths for both commands, branch names containing `/`, profile-qualified duplicate instances, Back/Escape without target mutation, no send on selection/loading/empty/error, and stale results after device/draft changes. Prove target commands still work when native discovery rejects, and existing model/reasoning/skill selection and literal-path behavior remain intact.

Screen integration should prove modal and quick commands produce the same binding, only final worktree selection changes the target, a new draft clears the old selection, first send retains folder placement, and existing completed/failed Sessions cannot retarget. Keep shared workflow composers free of these actions.

Run affected frontend tests, TypeScript/production build, changed-file lint, and focused Rust inventory/session tests. No broad unrelated test expansion is needed.

Native acceptance on this laptop: select a local demo instance with `/worktree`; select Hetzner then its demo instance with `/device`; inspect branch, path, and profile in the existing target control; back out without changing a selection; try the commands after a remote draft has made native quick discovery unavailable. Send a read-only `hostname`, `pwd`, and `git rev-parse HEAD` prompt to each selected target and compare the results independently through local Git/SSH. Do not create worktrees or change remote files for this feature's acceptance.

## Acceptance evidence

- Focused validation passed: 35 frontend tests, changed-file ESLint, production frontend build, eight inventory/connection Rust tests, four target-session Rust tests, and the native `test-fast` build. The shared presentation/modal checks also passed (nine tests, overlapping the frontend coverage).
- The existing host accepted unfiltered worktree inventory without deployment. `.dev/target-command-preflight.json` records the host response and independent Git observations on both devices.
- The native executable at `src-tauri/target/test-fast/codex-orchestrator.exe` ran as PID 21016 against the isolated prototype database. The inspector verified its debugger ownership in `.dev/target-commands-native-owner.json`. The development configuration has a distinct application identifier so main's single-instance plugin does not activate another worktree's app.
- Keyboard `/device` selected Hetzner, then filtered `/codex/remote-development-demo`; Tab applied the instance. Escape from worktrees restored `/Hetzner` without changing the target. The choice displayed repository, profile, branch, directory, path, and instance ID.
- With that remote draft selected, native quick-feature discovery reported unavailable. `/worktree` still listed laptop instances and selected the local demo. The command disappeared, the target control updated, and no Session existed until the read-only prompt was sent.
- Local Session `413c904a-4324-440d-9a6a-1b450940de51`, invocation `de40a44f-d96d-4291-86bd-8e5dc3271afb`, completed in provider thread `01a0a190-cbd9-7321-b9fd-76b19a1afcf5`. It reported `DESKTOP-VSTHML8` and `C:\Users\user\.codex\worktrees\remote-development-demo`.
- Remote Session `edf6bba9-ac42-47cf-88ec-a5b2c8604c64`, invocation `252981e8-9865-48f3-bf1c-a56f92570bac`, completed in provider thread `01a0a192-1a09-7b30-aced-f536d449d109`. It reported `ubuntu-8gb-nbg1-1` and `/root/.codex-orchestrator/repositories/orchid/worktrees/remote-development-demo`.
- Both returned HEAD `e2bfc6cb584a9ce7ada0d762f768c605a33b4160`, independently confirmed with local Git and SSH. Local status remained clean; remote status retained only the four pre-existing demo/smoke files. These prompts made no worktree changes.
- After local completion, `/device` was disabled, Enter left the query unchanged, and the target button stayed fixed. A new draft allowed target selection again. Persisted bindings and outcomes are recorded in `.dev/target-commands-acceptance.json`.
- Screenshots: `.dev/target-commands-devices.png`, `target-commands-remote-choice.png`, `target-commands-local-after-remote.png`, `target-commands-local-result.png`, and `target-commands-remote-result.png`.

Remote native model/skill discovery, workflow targeting, and changing existing Session targets remain outside this slice.
