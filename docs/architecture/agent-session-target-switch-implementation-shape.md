# Agent Session target switches: implementation shape and change list

Status: proposed implementation shape, 2026-09-30, using `plan-implementation-shape`. Based on the user's completed decisions. No implementation in this turn; earlier uncommitted HEAD-equality corrections remain separate. This document supersedes the behavioral proposals in the preceding exploration.

## Implementation choices proposed by the agent

The product decisions are settled. These are structural choices for achieving them, not additional requested features:

- Extract target-operation orchestration and persistence from the large queue files. Queue ordering and executing a workspace intention change for different reasons and should have separate homes.
- Use one pure target-switch planner with two consumers: read-only projected-plan calculation and head-only execution with fresh facts. Do not maintain separate forecast and execution rules.
- Retain completed operation outcomes in the existing operation store. Add only session history positions to interleave those outcomes with delivered turns; derive provider markers from existing execution snapshots. Do not create an audit-event/attempt log.
- Add a session-level history projector alongside the existing invocation-content projector. Shared turn/range consumers should not have to understand workspace operations.
- Block old pending operations that lack explicit intent/resolution during upgrade. Choosing a migration on their behalf would contradict the new explicit-choice contract.

## Proposed ownership and consumption

Paths in this table identify the proposed source layout; newly created files do not exist yet.

| Disposition / home | Responsibility and consumers |
| --- | --- |
| Retain/adapt `agent_sessions/target_operation/{mod,planning}.rs` | Request, facts, plan, progress, and successful receipt types; one pure `plan_target_switch` calculation consumed by preview and execution. |
| Extract/create `agent_sessions/application/target_operations.rs` | Enqueue/edit requests, read-only observation and Recalculate. Composer acceptance, queue transport, and dispatcher use this through the existing application facade. |
| Extract/create `agent_sessions/application/target_operations/execution.rs` | Head-only evaluation, permitted readiness/materialization/linking/migration, and outcome. Calls Execution Targets and existing engine mechanics. |
| Narrow `agent_sessions/application/queue.rs` | Queue hydration/order, dispatch, retry/discard scheduling, and effective destination. Delegates target work instead of implementing it. |
| Extract/create `agent_sessions/repository/target_operations.rs` | Operation records, revisioned edits, and atomic target completion. Queue repository retains ordinals, acceptance, reorder, and discard transactions. |
| Retain/adapt `execution_targets/{preparation,sisters,endpoints}.rs` | Workspace/device observations, physical workspace preparation, and sister relationships; no provider continuity selection. |
| Retain/adapt `agent_sessions/application/delivery/` and `preparation/` | Message profile/route choice, native binding ownership, context handoff, snapshot and launch; no workspace-operation entry for provider changes. |
| Create `agent_sessions/history.rs` and `repository/history.rs` | Successful outcome read model and session positions referencing existing invocation/operation records. History load consumes these in its existing transaction. |
| Extract/create `features/agentSessions/useSessionQueue.ts` | Queue state/subscription and revisioned commands currently embedded in `useAgentSession.ts`. Existing workspace controller delegates to it; no second queue cache. |
| Create `features/agentSessions/sessionHistoryProjector.ts` | Merge existing invocation panels with successful transitions and a temporary blocking indicator. The full Session view consumes it; turn/range inspectors retain invocation-only content. |
| Create `SessionTransitionIndicator.tsx`, `SessionTransitionDetailsDialog.tsx`, `sessionTransitions.css` | Clickable history elements and outcome-first details, used by the Session renderer. |
| Retain/adapt existing target dialogs, queue panel/item dialog, labels and screen | Intent/resolution selection, edit-in-place, projected/actual details, and compact rows. All quick/browse/device entry points build the same request. |
| Remove early-start API, provider-context workspace step, and retained discard path | Remove competing control paths rather than adding flags or alternate schedulers around them. |

The paths under `agent_sessions/` above are under `src-tauri/src/`; frontend paths are under `src/`. Keep the application facade and repository port, rather than introducing another service registry or an execution framework. Private transaction helpers can be shared across the focused repository modules.

This shape gives a future agent distinct places to change scheduling, migration intent, inference delivery, or history rendering. Extract only these responsibilities; unrelated workflow and provider runtime internals do not need a refactor.

## Reuse and dependencies

Reuse the existing React/native [ModalDialog](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/components/ModalDialog.tsx>), [provider descriptors](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/application/agentProviders/descriptors.ts>), queue labels, history cache, and transcript invocation panels. The modal already owns stacking, keyboard containment, and focus restoration; provider labels already have one registered mapping.

The inspected manifests use Tauri, Tokio, rusqlite/SQLite, Serde, React, and Vitest/testing-library. They already provide commands, workers, transactions, serialization, and UI/test primitives needed here. No new package or generic queue/workflow library is proposed: destination inheritance, sister resolution, and the no-audit history policy are Orchid domain behavior. Use existing SQL foreign-key cascades and native dialog behavior rather than reimplementing them.

## 1. Keep the behavior already present

| Existing implementation | Disposition |
| --- | --- |
| `projectSessionQueueTargets` changes the effective destination at each unfinished target entry | Retain. Messages remain destination-independent and inherit from queue order. |
| Dispatcher stops at executing/blocked entries; repository pins executing entries during reorder | Retain; apply the active-invocation gate to target execution as well as message execution. |
| Target entries can be edited with a queue revision | Retain the entry ID and queue position; extend editing to intent and migration resolution. |
| Delivery resolves provider/model and exact-route native continuity | Retain its algorithms; move workspace-activation provider bookkeeping into this owner. |
| Engine captures/applies worktree snapshots | Reuse unchanged. Dirty/ahead/divergent destinations currently block; this change does not introduce unconditional overwrite or merge handling. |
| Existing sister inventory and repository/device mapping | Reuse for selecting linked, unlinked, or new destinations. |

## 2. Replace inferred switching with an explicit request

**Modify:** [target operation domain](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/target_operation/mod.rs>), [planning](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/target_operation/planning.rs>), [queue DTOs](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/application/agentSessions/sessionQueue.ts>), and [prepared submission](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/preparation.rs>).

- Add a typed `TargetSwitchRequest`: destination selection plus `ChangeConversationTarget` or `ContinueWork` intent.
- `ContinueWork` also carries an explicit resolution: use an existing sister, link a selected existing instance, or create a linked instance. Record the repository/branch lineage this resolution applies to; do not encode a separate migration queue item.
- Keep stable destination identity separate from observed HEAD, device display name, provider configuration, and Capability Profile revision. Existing-instance selection uses device/repository/worktree identity, with path/attachment checked during resolution. Branch choice and creation intent remain explicit constraints.
- Replace `requires_sister_migration` as the authority for deciding whether to migrate. Repository/branch/device comparisons validate a chosen resolution; they do not infer user intent.
- Distinguish an explicitly queued switch from the optional switch inserted by sending to a genuinely different destination. An explicitly queued request remains an entry even if the destination currently matches; it is evaluated at its turn. Sending again to the same destination must not manufacture another switch because HEAD or inference configuration changed.
- Replace repository/branch-only recognition of a resolved `Create` selection with the created instance's resolution receipt. Carry that resolved identity back into composer state.

**Result:** “Change conversation target” can select another same-branch device without copying files. “Continue this work” cannot be accepted without a migration resolution.

## 3. Separate projected plans from execution plans

**Adapt:** `target_operation/planning.rs` into the one pure plan calculation. Put read-only inspection and preview persistence in the extracted `application/target_operations.rs`; no second planner module is needed.

**Modify:** target-operation domain/planning, [Execution Targets](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/execution_targets/mod.rs>), [selection resolution](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/execution_targets/preparation.rs>), and [endpoint reads](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/execution_targets/endpoints.rs>).

- Replace the single mutable `actions` field with a frozen projected plan and a separate actual execution plan/progress.
- The projected plan stores calculation time, request revision, projected source/destination, facts obtained, required/conditional steps, and unknown facts or blockers.
- Calculate it on initial queue acceptance using read-only inspection. Do not call `ensure_ready`, configured device start, materialization, capture, application, or provider launch preparation.
- Add a revision-checked `recalculate_target_operation_projection` command. This is the only refresh of an existing projected plan. An unreachable endpoint produces an unknown observation, not a device-start attempt or a fabricated readiness result.
- Editing intent/destination or reordering marks the saved projection as belonging to an older request/queue arrangement; retain it visibly as stale until the user clicks Recalculate. Do not silently recalculate it on subscriptions, dialog opening, polling, or reload.
- Use the same calculation rules at execution with fresh facts. Actual execution progress can update normally without replacing the saved projection.
- Stop carrying completed steps forward solely because `replan_actions` finds the same action kind. Reinspect current facts and reuse only a matching resolved resource/effect already known to the existing implementation.

**Result:** a dialog can show “Projected when queued” alongside current status without pretending its old forecast is the execution plan.

## 4. Execute preparation only at the queue head

**Extract/modify:** target-specific enqueue/edit/recalculation and execution from [queue application](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/queue.rs>) into `application/target_operations.rs` and its `execution.rs`. Queue dispatch retains ordering and delegates one head operation. Adapt prepared submission and [startup reconciliation](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/lifecycle.rs>) to that boundary.

- Remove calls to `start_target_operation_early` from acceptance, explicit enqueue, editing, retry, and staging-completion callbacks.
- Replace the early-staging worker entry point with head-only operation execution. Before starting a target operation, require that no invocation is running, including one launched through the synchronous delivery path.
- First evaluate the accepted intent against the destination and the source established by preceding queue entries. Read-only inspection precedes necessary device readiness, followed by destination reinspection if readiness was initially unavailable.
- Calculate and execute only necessary steps: readiness, existing-instance verification or materialization, explicit linking/migration when requested, and destination activation.
- A selected instance disappearing does not silently authorize creation or substitution. Incompatible lineage, ownership, or a resolution no longer supported after reordering blocks the entry for editing/resolution.
- Preserve the queue barrier on failure. Retry and an edited alternative reevaluate the intent; neither starts preparations ahead of earlier entries.
- If nothing is required after readiness/availability checks, remove the operation and queue row without resetting provider state or producing a history indicator.
- On restart, keep pending entries pending. Interrupted execution remains blocked under existing recovery rules; do not turn restart into early preparation or an automatic alternative migration.

**Remove:** `start_queue_entry` application/port/client API, `start_agent_session_queue_entry` Tauri registration, and the “Start now” control. Moving an entry toward the front remains the way to change its scheduling.

**Result:** no offline-device startup, worktree creation, or migration occurs merely because a future entry was queued.

## 5. Make the existing migration path conditional on resolution

**Modify:** extracted target-operation execution, [sister store](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/execution_targets/sisters.rs>), and selection resolution.

- For `ChangeConversationTarget`, resolve/select the destination without transferring source changes or establishing sister membership.
- For `ContinueWork`, resolve the recorded linked/adopt/create choice against the current source lineage at execution. If reordering changes that lineage outside the accepted resolution, block for a new resolution.
- Validate group ownership and explicitly establish/claim the conceptual link for the chosen participants before entering the snapshot migration path. Existing unrelated same-branch instances require the accepted link resolution; they are not automatically treated as sisters.
- Keep the source active until the existing migration succeeds, then activate the destination sister and session target.
- Use fresh destination observations for the existing snapshot precondition and persist the verified destination facts after application. This is required by execution-time reevaluation; it does not change the engine's transfer/merge policy.
- Record only the actual successful outcome and performed steps for history. Failed attempts stay with the currently blocked operation and disappear when resolved or discarded.

**Do not change:** snapshot payload/exclusions, destination cleanliness/history validators, detached migration restrictions, Git application mechanics, source retention, or merge/reconciliation policy. Do not add copying, source deletion, automatic cleanup, rollback, or a generalized recovery subsystem.

## 6. Give delivery ownership of provider binding changes

**Modify:** [message preparation](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/preparation/execution.rs>), [continuity](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/delivery/continuity.rs>), [native binding storage](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/repository/native_conversations.rs>), [prepared-binding commit](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/repository/preparation.rs>), and direct delivery binding commits.

- Retain the current native binding's owning execution route/workspace independently of the session's newly selected workspace. Add persisted binding-location data alongside the existing native handle/version; initialize it from the old binding and target during upgrade where known.
- Read that owner during continuity planning instead of deriving it from the current workspace target. Update it atomically when delivery commits a prepared native binding.
- Remove provider-cache insertion and native-handle clearing from `activate_target_operation_record`. Delivery already has source caching and missed-context planning; preserve the launch-accepted cursor there.
- Remove `SelectProviderContext` from the workspace action enum, DTO, labels, planner, and completion handling. Remove the unused target-operation `activated_binding` field.
- Store intended Capability Profile choices with the queued message preparation rather than relying on a target entry to carry them. Validate the selected profile/route at delivery. Profile, provider-configuration, and model changes alone do not create workspace entries.
- Keep workspace inheritance from queue position. This addition must not pin messages to accepted workspace destinations.

**Result:** an actual workspace change preserves enough provenance to resume the correct native context, and an inference-only change uses delivery without a phantom workspace switch.

## 7. Delete discarded queue items transactionally

**Modify:** queue repository/application, extracted target-operation persistence, [repository port](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/ports/repository.rs>), and [live history reduction](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/application/agentSessions/liveHistory.ts>).

- Replace queued-item cancellation-as-retention with a discard transaction.
- For a never-delivered queued message, delete its invocation. Existing foreign-key cascades delete preparation, queue entry, snapshots, runtime events, and diagnostics. Refuse this discard path once launch acceptance exists; cancellation of an actual running turn remains existing session history.
- For a discarded target entry, delete its operation and queue row instead of saving `Canceled`.
- After removing a queued message, count remaining pending/blocked message entries anywhere in that session queue. If any remain, retain all switches even when those messages precede them. If none remain, remove all remaining nonexecuting switches, including independently queued ones.
- Apply this cleanup to message removal, not ordinary completion of the last message or standalone switch acceptance. Do not delete an executing operation or completed history.
- Advance queue revision and publish removals together. Clear stale frontend invocation/entry projections so late notifications cannot restore discarded bubbles.
- A no-action result follows the same physical deletion of its operation/entry, without a historical “completed/no-action” record.

**Result:** canceled queued messages and discarded/no-action switches do not survive as an audit trail.

## 8. Update the existing dialogs and queue presentation

**Modify:** [device dialog](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/features/agentSessions/DeviceContinuationDialog.tsx>), [target dialog](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/features/agentSessions/SessionTargetDialog.tsx>), [screen wiring](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/features/agentSessions/AgentSessionScreen.tsx>), [controller](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/features/agentSessions/useAgentSession.ts>), [target state](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/features/agentSessions/useSessionTarget.ts>), queue labels/panel/item dialog, and workspace activity presentation.

Extract the queue state/commands/subscription from `useAgentSession.ts` into `useSessionQueue.ts` while retaining the workspace controller interface. The hook owns the existing queue snapshot and notifications; target-draft state and invocation history stay in their existing owners.

- Add explicit intent selection to target-switching dialogs. Enable Continue only with an explicit use/link/create resolution supported by the chosen destination. Retargeting remains available independently of sister membership where the target is usable.
- Reopen the same dialog with the queued request when editing. Save intent and resolution into the existing entry, retaining its order.
- Route quick-target controls through the same request construction; they cannot bypass intent/resolution validation.
- Use intent-specific titles: “Continue this work at <device>” and “Change conversation target to <device> <branch>.” Internal migration steps are details of that entry.
- Keep compact rows at intent/message plus status. Remove repeated device/path metadata from message rows.
- In the item dialog, show the saved projected plan with calculation time, stale/unknown facts, and Recalculate. Keep actual execution status/steps separate.
- Show only current applicable work in the activity view; remove local readiness no-ops and provider-context pseudo-steps. A blocked entry exposes the current reason and Edit/Retry/Remove resolution controls.
- Retain drag reorder and its effective-target calculation. No destination-pinning field is added to messages.

**Transport changes:** add intent/resolution to enqueue/update/prepared-message commands; add the projection-recalculation command; remove early-start. Update [Tauri queue transport](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/transport/queue.rs>), [client bridge](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/infrastructure/agentSessions/tauriAgentSessionClient.ts>), contracts, exports, and [active app registration](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/active_app.rs>).

## 9. Add actual transition history

**Create:**

- `src-tauri/src/agent_sessions/history.rs`: successful workspace outcome and ordered history-position contracts.
- `src-tauri/src/agent_sessions/repository/history.rs`: their persistence/load projection.
- `src/features/agentSessions/sessionHistoryProjector.ts`: session timeline composition using existing invocation projections.
- `src/features/agentSessions/SessionTransitionIndicator.tsx` and `sessionTransitions.css`: centered workspace/left-aligned provider elements with hover/focus affordance.
- `src/features/agentSessions/SessionTransitionDetailsDialog.tsx`: outcome-first details with expandable actual steps/diagnostics.

**Modify:** repository history loading, history ports/DTOs, [invocation projection](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/features/agentSessions/transcriptProjector.ts>), [renderer](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/features/agentSessions/AgentSessionTranscript.tsx>), live-history reducer/source, workspace wiring, and controller subscriptions. Keep `TranscriptAnchor`, `ProjectedTranscriptContent`, and turn/range selection invocation-scoped. Session timeline items have their own types and are passed separately to the full Session view; they are not synthetic invocation content.

- Freeze a successful target operation's outcome receipt at activation: intent, resolved source/destination, actual effects/steps, and completion time. Add no receipt for no-action, failure, or discard.
- Extract the existing invocation-panel markup from `AgentSessionTranscript.tsx` into a local `AgentSessionInvocation.tsx` so the full timeline and scoped rendering reuse it. The turn inspector remains scoped to its chosen invocation and receives no unrelated transition items. A transition-only Session still renders its history instead of returning the empty state.
- Persist per-session history positions referencing delivered invocations and successful operation receipts. Allocate them in the existing launch-acceptance and target-completion transactions. These are ordering references to what happened, not queue-attempt/audit events.
- Use execution order for newly executed queued turns; current creation-time sorting must not present dragged messages in the old order. Preserve imported-turn ordering and existing invocation anchors.
- Derive provider-change markers from launch-accepted invocation snapshots and their prior provider identity. A prepared but unlaunched route is not a history transition. Configuration/model changes within the same provider produce no marker; unknown legacy provenance does not invent one.
- Insert workspace outcomes as centered siblings between turns, outside input/output containers. Place provider markers on the agent side before the corresponding output. Where both occur, the workspace outcome precedes the provider marker.
- Project a currently blocked action as a temporary centered indicator from current queue state. Retry, successful resolution, edit to an alternative, or discard removes that temporary failure indicator. Do not persist a failed/resolved/skipped action-history record.
- The modal opens immutable actual outcome facts for completed transitions, and current details for a blocking one. Hover/focus marks clickability. Do not add a navigation list.
- Extend live updates to carry successful receipts and removed invocation IDs so the history cache updates without retaining discarded content or relying on polling. Reload must reconstruct the same history.

## 10. Storage upgrade and existing dirty work

**Modify:** [active storage composition](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/storage.rs>), repository schemas/mapping, and storage migration tests.

- Add the native binding-location column, per-session history-position storage, and new serialized operation/preparation fields. Use the next active schema version; the inspected version is 63.
- Convert retained records in one explicit upgrade. Pending old operations lack the new user intent/resolution: retain their destination and block them for explicit resolution rather than silently infer migration. Retire canceled, never-delivered queue records.
- Preserve existing delivered/imported invocation history. Backfill its previous displayed ordering; use actual execution positions for new work. Do not fabricate outcome details/provider switches absent from old records.
- Old preparation/staging state must not cause early work after reopening. Existing in-progress execution still uses the existing blocked restart policy.
- Preserve the unrelated attachment-schema repair already modified in `storage.rs` and `storage/session_migration_tests.rs`. Integrate schema changes without overwriting those edits.
- Fold the earlier pending HEAD-equality fix into the explicit identity contract; do not treat its existing helper or repository/branch creation heuristic as the finished design.

## 11. Verification and implementation order

| Pass | Work | Required evidence |
| --- | --- | --- |
| 1 | Typed request, identity, projection, transport, schema | Head drift/missing observation does not create a switch; explicit same-target request remains queued; old ambiguous requests block; projection reads have zero startup/materialization calls |
| 2 | Head-only dispatcher and linking resolution | Earlier active turn prevents preparation; retarget performs no snapshot/link; Continue requires accepted resolution; changed destination blocks; existing migration fixture still passes unchanged validators |
| 3 | Delivery-owned binding and discard transactions | Correct native route/workspace resumes after switches; provider-only changes create no target entry; deletion cascades; before/after-switch messages both retain switches; last removed message deletes all nonexecuting switches |
| 4 | Dialogs and queue projection | Edit retains entry/order; unsupported intent disabled; frozen preview survives reload/reorder until explicit refresh; dragging a message across a switch changes execution destination |
| 5 | History receipts, ordering, indicators/modal | Action-first centered/left alignment; provider identity only; no phantom prepared-provider indicator; no-action/discard absent; blocking indicator disappears; actual reordered execution survives reload |

Extend the existing Rust queue/repository/preparation/target tests and frontend queue/dialog/transcript tests for these contracts. Add projection and history-position tests around unknown observations, transaction races, restart, and ordering. Keep imported transcript and range-inspector regression coverage because the renderer is shared.

Run the root frontend build, Rust check and focused fast tests, then build and launch an isolated native instance for the queue/history walkthrough. Verify visible hover/click/modal behavior, edit/recalculate, blocking/resolution, and reload persistence. Exercise one real linked two-device migration without changing its existing mechanics. Automated component tests alone do not establish those native or remote outcomes.

The implementation order above follows dependencies. Completion means the new contracts work together in the native flow, not merely that labels have changed. Root/module declarations, public DTO exports, in-memory test repository implementations, and recorded fixtures should adopt the new fields directly; do not keep dual request formats or a second planner for compatibility.
