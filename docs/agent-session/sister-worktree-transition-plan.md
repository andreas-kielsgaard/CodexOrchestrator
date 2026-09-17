# Sister worktrees and device-transition preparation

Status: planned for `feature/Remote-Development`. This supersedes the no-dirty-state limit of the current Device Continuation mockup. It does not add background synchronization, merge resolution, provider changes, or concurrent editing of the same sister group.

## Target experience

An existing ordinary Agent Session is bound to source worktree A. The user opens Destination device, chooses device B, and sees the source branch in the shared branch graph plus worktree state for A and B: branch/HEAD relationship, staged/unstaged/untracked summary, detached state, and Orchid-owned active turns.

The second step identifies the selected target worktree as an existing sister, a candidate that can become a sister, or a requested new worktree. It shows the exact ordered transition plan: inspect state, capture the source snapshot, materialize or update B, transfer and apply the snapshot, move native conversation continuity, lock the sister group to B, and resume on B. It displays snapshot bytes and a transfer-time estimate from a small measured probe or the latest completed transfer for that device pair.

`Start switching` starts that plan immediately. Closing after selection retains a pending transition. Sending a prompt while the transition is pending starts it if needed; the accepted prompt waits behind the transition and is delivered only after it completes. There is one pending transition and one accepted prompt for a Session. The Composer continues to use the existing preparation panel for live progress.

After completion, B is the active sister. A remains materialized but is locked to the Session's active sister group. Worktree-selection surfaces show that lock and prevent a different Session from selecting A or B until the owning Session moves the group again. This is product-dialogue enforcement only.

## Happy-path assumptions

- The source has no active Orchid turn when a transition begins. The dialog reports active turns and leaves `Start switching` unavailable until they finish.
- The destination is clean and its HEAD is equal to or behind the source HEAD. A diverged or dirty destination is displayed but has no migration action in this slice.
- The source may have staged, unstaged, and nonignored untracked changes. An Orchid snapshot preserves those Git working-tree states and local commits that are ahead of the destination. Ignored files, build products, dependency caches, credentials, and `.git` are not transferred.
- The source and destination repository mappings and Codex configurations already exist. No cloning, Git pushing, merge UI, or recovery protocol is added.
- A transition uses a product-owned virtual snapshot ref and archive. It may update the destination's local branch/worktree to the source HEAD, but creates no user Git commit and pushes no ref.

## Why this is a target transition, not another prompt preparation

`SessionPreparation` is correctly keyed to an accepted invocation and owns the frozen prompt, native readiness, and eventual delivery. It cannot begin before a prompt exists without creating a false invocation. The device switch therefore needs a separate durable record.

`SessionTargetTransition` owns intent, inspection, snapshot migration, sister membership, and the active-device lock. `SessionPreparation` remains the only owner of an accepted user prompt. On Send it reads the current transition: it waits for a running transition, starts a pending one, or continues directly when it is ready. It then reuses the already resolved target and proceeds with configuration, Codex continuation, provider readiness, and delivery.

## Ownership

```text
DeviceContinuationDialog
  -> SessionTargetTransition request and projected action plan
  -> explicit Start, or Send starts the same transition

Agent Session application
  -> persists the transition and one accepted prompt
  -> waits for transition readiness before existing prompt preparation/delivery

Execution Target service
  -> observes worktrees, creates/applies snapshots, materializes target worktrees
  -> records sister instances and active-device locks

Orchid engine + local/SSH host protocol
  -> performs Git inspection and snapshot capture/apply on the owning machine
  -> provides a transport-neutral snapshot stream and measured transfer facts
```

The Session owns its requested transition and queued prompt. `execution_targets` owns physical-worktree facts, sibling identity, and locks so the normal target picker and slash-command discovery use the same truth. The engine owns device-local Git operations. The React modal only renders the projected plan and sends commands.

## Durable model

Create a `sister_worktree_groups` record for a repository and branch when a source worktree first participates. Its `sister_worktree_instances` hold the device, worktree handle, path, last observed branch/HEAD, and detached state. A group has one `active_device_id` and `owner_session_id`; moving the group replaces that active device and does not delete the prior instance.

Create `agent_session_target_transitions`, one current record per Session. It stores the source/selected destination target, sister group ID, planned tasks, snapshot descriptor and estimate, phase (`pending`, `running`, `ready`, `failed`, `canceled`), and resolved destination. It records the worktree transition independently from an invocation. Agent Session preparation references this record rather than copying transition state into an invocation payload.

Persist the snapshot descriptor with source/destination heads, staged and unstaged patch metadata, untracked manifest, bundle metadata, total bytes, and measured throughput. The content archive is short-lived on the source/destination hosts; the descriptor is the durable product record.

## Snapshot and transition semantics

The engine exposes a `WorktreeInspection` for a concrete worktree. It contains the branch or detached state, HEAD, ahead/behind/diverged relationship when comparing two inspections, changed-file counts, byte estimate, and active Orchid usage supplied by the product.

For an eligible migration, capture a `WorktreeSnapshot` from source A:

1. Pack source-only commit objects from B's HEAD through A's HEAD into an Orchid virtual ref.
2. Capture the staged patch, unstaged patch, and nonignored untracked-file manifest/content separately so the destination index and working tree match A.
3. Materialize a requested B worktree when absent, or select the explicitly chosen B candidate.
4. Transfer the snapshot through a `WorktreeSnapshotTransport` port, then apply its commit objects and overlays on B.
5. Reinspect B, record it as the sibling, set the group lock to B, and expose B as the resolved Session target.

The transport port returns bytes and elapsed time. Planning runs a small bandwidth probe where no prior measurement exists; later plans use the most recent matching device-pair measurement. The plan may show an unavailable estimate while inspection is incomplete, but never invents a duration.

The existing Codex continuation transfer stays in ordinary Session preparation after the worktree transition is ready. This keeps provider history separate from source files and lets the actual accepted prompt retain its current readiness/delivery semantics.

## Concrete change map

| Action        | Files / objects                                                                                                                                              | Responsibility                                                                                                                                                                                   |
| ------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------ | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Create        | `src-tauri/src/execution_targets/sisters.rs`, `snapshots.rs`, `transitions.rs`                                                                               | Sister groups, worktree inspection, snapshot plan/execute service, and lock projection.                                                                                                          |
| Adapt         | `src-tauri/src/execution_targets/{domain,endpoints,preparation,transport,mod}.rs`                                                                            | Add inspection/transition DTOs and commands; make materialization and snapshot actions available through local and SSH endpoints. Keep provider runtime routing out of this module.              |
| Adapt         | `crates/orchid-engine/src/{protocol,host}.rs`, `workspaces/`                                                                                                 | Shared local/host commands for inspection, virtual snapshot capture, and apply. Add a transport-neutral snapshot stream boundary; do not put desktop persistence in the engine.                  |
| Create        | `src-tauri/src/agent_sessions/target_transition.rs`, `application/target_transition.rs`, `repository/target_transition.rs`, `transport/target_transition.rs` | Persist/request/start/load the Session transition and notify its state changes.                                                                                                                  |
| Adapt         | `src-tauri/src/agent_sessions/application/preparation/{mod,execution}.rs`, `preparation.rs`                                                                  | Gate an accepted prompt behind its transition, reuse resolved B, and retain the existing provider continuation/readiness/delivery path. Do not create a synthetic prompt for explicit switching. |
| Adapt         | `src-tauri/src/agent_sessions/{ports,repository,transport}/...`, `storage.rs`                                                                                | Add repository queries for product-owned active worktree turns; add sister/transition tables and evolve the active schema version.                                                               |
| Adapt         | `src/application/executionTargets/{contracts,presentation}.ts`, `src/infrastructure/executionTargets/tauriExecutionTargetClient.ts`                          | Expose inspections, locks, transition plan/start/load commands, and live task DTOs.                                                                                                              |
| Adapt/extract | `src/application/agentSessions/preparation.ts`, `preparationPreview.ts`, `SessionPreparationPanel.tsx`                                                       | Render server-projected transition tasks beside existing prompt preparation. Extract the task-list presentation so modal and panel share status rows.                                            |
| Replace/adapt | `DeviceContinuationDialog.tsx`, `deviceContinuation.css`, `AgentSessionScreen.tsx`, `useSessionTarget.ts`                                                    | Replace mock facts with real inspection/plan data; save a pending transition on target selection; provide `Start switching`; show live status and queued-prompt wording.                         |
| Adapt         | `SessionTargetDialog.tsx`, target quick-command source, `WorktreeCreationConfirmation.tsx`                                                                   | Surface sister lock availability everywhere a worktree can be selected; disable locked rows with the owning device/session explanation. Preserve ordinary worktree selection.                    |

Remove the mock-only text in `DeviceContinuationDialog` that says state, relationships, and locks are unavailable. Do not remove the generic target-worktree flow; it remains the different-worktree workflow.

## Validation

- A clean remote target behind the source is inspected, estimated, migrated from source commits plus dirty state, re-inspected, made an active sister, and selected as the Session target without a Git push or new user commit.
- An explicit `Start switching` completes without a user prompt. A later prompt resumes on B and retains the Codex continuation path.
- A prompt submitted during `pending` or `running` transition is durably accepted once, waits, then runs only after B is ready. It is not delivered to A.
- Active Orchid turns, destination dirtiness, detached targets, divergence, and sibling locks are projected accurately. These conditions prevent the happy-path migration action; no merge/recovery behavior is tested or added.
- A second Session's worktree picker and `/device`/`/worktree` discovery show a locked sister as unavailable. The owning Session can move its group back.
- Local and SSH engine/host tests cover snapshot inspection, capture, transfer-port handoff, apply, and source/destination HEAD verification. Agent Session tests cover transition persistence, prompt gating, notifications, and target commit after readiness. Frontend tests cover the two modal steps, explicit start, pending status, and queued prompt copy.

Live acceptance is laptop-to-server and server-to-laptop continuation on the configured Codex profile with a source worktree containing one committed local change and one uncommitted file change. Verify the source worktree remains present but locked after moving, B's files/index match A, and the next prompt executes on B.

## Deferred

No continuous synchronization, concurrent sister editing, branch merging, automatic conflict resolution, device discovery, offline reconciliation, migration retry/recovery design, generalized message queue, ignored-file/cache migration, Claude Code execution, or provider-independent snapshot backend is included.
