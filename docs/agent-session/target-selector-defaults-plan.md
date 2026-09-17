# Agent Session target-selector defaults and ordering

## Target

Make the composer target controls describe the selected configuration clearly:

- The Capability Profile select shows only actual profiles. It no longer offers the empty `Capability Profile` option once a profile is selected.
- The worktree control displays the selected branch name. Its existing instance suffix remains available when several worktrees share a branch.
- A new draft defaults to the repository default branch's existing local worktree when one is available. Existing sessions retain their current target.
- Opening the worktree picker starts on the current target's repository, branch, device, and instance. A new draft starts on its default-branch candidate.
- Branches are ordered: repository default branch; branches with an instantiated worktree on the selected device; branches with a dirty instantiated worktree; newest relevant commit. Stable branch-name ordering breaks ties.

`main` means the repository's configured default branch, not a hard-coded `refs/heads/main` name. If that branch has no local instance, it remains first and is shown as a branch that needs materialization; the slice does not create it automatically.

## Shape

### Extend execution-target inventory facts

Adapt `crates/orchid-engine/src/protocol.rs`, `crates/orchid-engine/src/host.rs`, and the local/SSH execution-target boundary so a discovered `WorktreeInstance` carries:

- whether its index, working tree, or eligible untracked set is dirty;
- the HEAD commit timestamp.

`list_worktrees` owns this Git inspection because it already discovers each local or remote instance. It should collect the facts while inspecting that worktree, so React never performs per-row Git or SSH calls. The existing host command and local endpoint use the same DTO.

Adapt `src-tauri/src/execution_targets/domain.rs`, `inventory.rs`, and the TypeScript execution-target contracts to project those facts without changing target identity or the sister-lock contract.

### Give the picker a branch-plus-instance read model

Keep Worktree Review as the source of repository branches, commit dates, and default branch discovery. Expose its default branch reference in the branch-graph read model instead of assuming the name `main` in Agent Sessions.

Create a small Agent Session presentation helper next to `src/application/executionTargets/presentation.ts`. It combines the branch graph with one `listWorktreeChoices` response for the selected device, deduplicates instances represented by more than one Capability Profile, and returns ordered branch rows plus their instances. This is the only ranking implementation; `BranchBrowser`, `SessionTargetDialog`, and quick-command presentation consume its result. The existing generic `orderReviewTargets` remains Worktree Review's ordering policy.

The helper determines the relevant timestamp from the instance HEAD when instantiated and from the branch tip otherwise. A dirty instance only outranks a clean instance after the instantiated/non-instantiated comparison. An instance belongs to a branch only through its explicit `branchRef`; detached worktrees stay outside this branch selector.

### Simplify composer controls and retain current context

Update `src/features/agentSessions/SessionComposerToolbar.tsx` so the profile select has no empty selectable placeholder when profiles exist. If no usable profile exists, show a disabled explanatory state rather than allowing a blank selection.

Move branch and instance label formatting into the shared presentation helper. The toolbar's worktree button uses that label, beginning with the branch name rather than the generic `Worktree`. Keep the full repository path and instance handle in its title for disambiguation.

Adapt `src/features/agentSessions/useSessionTarget.ts` and `AgentSessionScreen.tsx` to resolve the local default-branch instance for a fresh draft after the default profile and inventory load. It must not replace an existing session's target or a draft selection the user has already changed.

Update `SessionTargetDialog.tsx` to load the target-device inventory alongside the branch graph, apply the shared ordering, and initialize from the current selection. Its existing selected repository and branch initialization stays the source of truth; add the selected instance to the initial candidate. The dialog still requires an explicit `Use worktree` confirmation before changing a target.

### Commands and tests

Keep `/worktree` and `/device` behavior intact, but use the shared labels and ordered inventory so their autocomplete mirrors the modal. Add focused tests for:

- no blank Capability Profile option when a profile is active;
- current session opens its target's repository, branch, device, and instance;
- a clean new draft selects the default-branch instance without overwriting a user selection;
- default branch, instantiated branch, dirty instance, and newest commit ordering;
- local and SSH inventory projection of dirty and commit-time facts.

Run the affected engine tests, Agent Session target/dialog/quick-command tests, and the frontend production build. Launch the isolated remote-development app and verify the selected values appear in the reopened picker.

## Deferred

This does not create missing worktrees, alter sister-worktree migration, synchronize configuration, or reinterpret detached worktrees. It only makes the existing selection and branch inventory easier to choose and inspect.
