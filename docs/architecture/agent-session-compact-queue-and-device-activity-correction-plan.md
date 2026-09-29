# Agent Session compact queue and device activity correction

Status: proposed correction to the implemented ordered queue. This plan does not replace the durable queue model in `agent-session-queued-delivery-and-readiness-correction-plan.md`; it corrects target reconciliation and separates the queue, active message, and device activity presentations.

## Target

Agent Sessions should present three related but distinct things:

1. **The future queue** immediately above the composer: messages and target operations that still affect future ordering.
2. **The active agent turn** in the conversation transcript: the message currently being processed and its runtime updates.
3. **Active device work** in the top-right activity surface: device startup, worktree preparation, transfer, and target activation that are currently running.

The backend should retain its durable execution barrier. An executing message may remain the queue head internally until its provider turn is terminal, while the frontend omits that message from the future-queue presentation.

## Confirmed behavior

- The queue is a flat, compact list above the composer.
- Its collapsed state shows the first queue entry and a `+N more` indicator.
- Hovering the queue expands it to one line per entry. Keyboard focus within the queue should expand it as the accessible equivalent.
- Clicking a line opens a modal containing that entry's details and controls.
- A message line stays concise: state, first line of text, and its projected destination where space allows.
- A target-operation line stays concise: state and destination. Its internal preparation actions are visible in the modal and active device surface, not as nested queue rows.
- Messages and target operations remain peer entries. Reordering either changes the effective target of later messages.
- Internal preparation steps remain part of one target operation and cannot be reordered independently.
- A target operation may remain as a compact queue boundary while it stages early because its position still controls when the Session activates the target.
- Once a message starts, it disappears from the queue and appears only as the active turn in the transcript.
- The top-right surface lists only active device or worktree actions. It does not represent the future queue and does not show agent messages.
- A blocked operation remains in the queue as a concise line. Clicking it reveals the error, completed actions, remaining actions, edit controls, and retry.
- `Ctrl+Enter` steers the active turn when steering is available. Otherwise it follows ordinary Send behavior, which queues the message when a turn is active.
- `Shift+Enter` continues to insert a newline.

## Causes found in the current implementation

### A stale create-worktree choice can survive target activation

`useSessionTarget.ts` marks cached and manually selected targets as dirty. `adoptCurrent` then refuses later target updates, and `acceptReady` only rebases when the selected JSON still exactly matches the accepted selection. A successful queued target activation can therefore leave the composer holding its earlier `workspace.kind = create` choice.

The backend compounds this in `application/queue.rs`: `selection_matches_target` treats every create-worktree selection as different from the active target. The next Send inserts another target operation even when the Session already runs on the worktree produced by the first operation.

Git then correctly rejects a second worktree for the same checked-out branch. The current checkout adapter reduces the useful Git diagnostic to `Git could not create the exact physical worktree`, so the visible error does not explain the conflict.

### The operation plan is a universal checklist

`TargetOperation::planned_actions()` currently creates every possible action for every operation. A same-device worktree change consequently carries source inspection, destination inspection, snapshot capture, transfer, sister activation, and provider actions even when most cannot apply.

When an early action blocks, the queue card renders all later universal actions as pending. This makes the error look like a much larger migration request than the user made.

### One frontend slot represents unrelated state

`ConversationViewport` has one `preparationPanel` slot. `AgentSessionScreen` gives that slot to `SessionQueuePanel` whenever the queue is unfinished and suppresses `SessionPreparationPanel`. Both panels use absolute top-right positioning.

As a result, the future queue occupies the device-processing location, and there is no stable ownership boundary between scheduled work and work that is active now.

### Durable queue state is rendered without a presentation projection

The backend intentionally keeps a message queue entry in `executing` until the provider emits its terminal update. This is the correct ordering barrier, but `SessionQueuePanel` renders every nonterminal entry. The active message is therefore duplicated in the queue and transcript.

The Session header similarly uses broad controller preparation state rather than the active invocation's state. Preparation belonging to a later queued message can label a currently working Session as `Waiting for setup`.

### The queue component owns too many responsibilities

`SessionQueuePanel.tsx` currently owns queue classification, target projection, card rendering, nested action progress, drag behavior, message editing, target editing, and operation controls. Those responsibilities now need different display locations and lifecycles.

## Proposed model boundaries

### Keep the durable queue as the execution source of truth

Retain `SessionQueueEntry`, queue ordinals, revisions, dispatcher ordering, and the rule that the executing head completes only after the provider turn is terminal. These are persistence and execution semantics, not display semantics.

Do not change message delivery or mark an entry complete early merely to remove it from the visible queue.

### Add one pure presentation projection

Create `src/application/agentSessions/sessionQueuePresentation.ts`. Given the durable queue and active invocation, it should return:

- `futureEntries`: queue lines that remain reorderable or affect future target projection;
- `deviceActivities`: target operations currently in `staging` or `activating` with their applicable action progress;
- `queuedInvocationIds`: pending or blocked message invocations that should not also appear in the transcript;
- `activeInvocationId`: the executing message shown through the transcript;
- the count and first item needed by the collapsed queue.

This projection should not mutate DTOs or infer backend lifecycle changes. It gives the queue, transcript, header, and device activity list one shared classification so the surfaces cannot drift.

Target operations that are staging early may appear in both outputs for different reasons: the queue line represents their scheduling boundary, while the device activity item represents work happening now.

### Distinguish selected intent from resolved target

Replace the frontend's coarse dirty flag with explicit selection provenance:

- **adopted**: follows the Session's active or projected target;
- **user override**: a later destination choice the user intentionally made;
- **operation intent**: a choice currently represented by a target operation.

When a target operation activates, rebase a matching operation intent to the resolved existing worktree. Preserve a genuinely newer user override.

On the backend, replace the boolean `selection_matches_target` result with a small relation owned by target-operation planning:

```text
Exact
SatisfiedByResolvedWorkspace
Different
```

A create-worktree selection is `SatisfiedByResolvedWorkspace` when the active target is the worktree previously resolved for that operation or has the same repository, branch revision, device, and registered workspace identity. In that case Send should not append a duplicate operation.

### Plan only applicable target actions

Extract target-operation planning from execution. The planner should compare source target, destination selection, workspace intent, and migration choice, then persist only required actions in their required order.

Examples:

- existing worktree on the same ready device: verify worktree, select provider context, activate target;
- new worktree on the same device: resolve revision, materialize worktree, verify worktree, select provider context, activate target;
- remote destination without state transfer: prepare device, resolve or verify workspace, select provider context, activate target;
- explicit sister-worktree transfer: add source inspection, destination inspection, snapshot capture, transfer, apply, and migrated-state verification.

The planner should be used for creation, material edits, and retry invalidation. Presentation should render the persisted applicable plan, not filter a universal list heuristically.

### Preserve useful checkout errors

The workspace checkout boundary should return a stable error category plus sanitized Git detail. A branch-already-checked-out failure should name the branch and conflicting worktree path when Git provides them. The target operation should keep that detail for its modal while its queue line uses a short summary.

## Frontend shape

### Compact queue

Refactor `SessionQueuePanel.tsx` to own only the compact disclosure and ordering interaction:

- render the first future entry while collapsed;
- show `+N more` when more entries exist;
- expand on hover and focus-within;
- render one line per future entry when expanded;
- open the item modal on click;
- retain drag reorder in the expanded list;
- expose keyboard move controls in the modal.

The collapsed first line should remain usable without hover. A blocked first line should show a compact blocked marker, with its full reason in the modal.

Use the existing pointer/native drag approach and `ModalDialog`; no new package is needed.

### Queue item modal

Create `src/features/agentSessions/SessionQueueItemDialog.tsx` and move both editors and entry controls out of `SessionQueuePanel.tsx`.

For messages, the modal should own:

- prompt text;
- model, reasoning, sandbox, attachments, and other message-local choices already supported;
- derived target summary;
- move up/down and remove controls;
- a change-target action that edits or inserts the preceding target operation.

For target operations, the modal should own:

- destination device, repository, branch, and worktree intent;
- phase and applicable action results;
- full blocked error;
- choose destination, start now, retry, move up/down, and remove controls where lifecycle permits.

### Device activity list

Create `src/features/agentSessions/SessionDeviceActivityList.tsx` with its own stylesheet. It should render only target operations in active preparation or activation phases.

Each item should show the operation summary, current applicable action, and compact progress. It should disappear when the operation becomes staged, blocked, completed, or canceled. A blocked operation remains accessible through its queue row and modal.

This surface occupies the current top-right processing area.

### Conversation layout

Adapt `ConversationViewport.tsx` to accept separate slots:

- `deviceActivityPanel`, rendered in the top-right activity position;
- `queuePanel`, rendered immediately before the composer.

Remove the ambiguous `preparationPanel` slot after its callers migrate.

Retain draft preparation preview as a dialog opened from the composer target status when useful. Do not let it take over the top-right device activity surface.

### Transcript and Session header

Adapt transcript projection to exclude invocation IDs classified as pending or blocked future messages. Raw persisted history remains unchanged.

An executing invocation is not excluded: it appears in the transcript with its ordinary working and interaction states. Completed messages continue to appear normally.

Derive the Session header badge from the active invocation and active target operation projection. Later queued preparation must not replace an active turn's `Working` state. A device operation may have its own activity indicator without changing the message status.

### Keyboard submission

Adapt `AgentSessionComposer.tsx` after the quick-menu handler has had the first chance to consume the key:

1. `Shift+Enter`: insert a newline.
2. `Ctrl+Enter` with an active turn, available steering, and an `onSteer` handler: steer once.
3. Other `Ctrl+Enter`: invoke the existing submit path once.
4. Plain `Enter`: invoke the existing submit path once.

The fallback intentionally inherits existing Send behavior: it sends when idle and queues when a turn is active. Update the keyboard hint to describe steering only when available; the ordinary Send/Queue label remains the source of truth for fallback.

## File ownership changes

| Change | File or module | Resulting responsibility |
| --- | --- | --- |
| Retain | `src-tauri/src/agent_sessions/application/queue.rs` | Durable queue dispatch, execution barrier, and transactional queue mutations. |
| Extract | `src-tauri/src/agent_sessions/application/target_operation/planning.rs` | Target relation, applicable action planning, and staged-result invalidation. |
| Extract | `src-tauri/src/agent_sessions/application/target_operation/execution.rs` | Execution of the persisted target-operation plan. |
| Adapt | `src-tauri/src/agent_sessions/target_operation.rs` | Persist target intent, phase, applicable actions, results, and error categories; remove the universal action constructor. |
| Adapt | `src-tauri/src/agent_sessions/repository/queue.rs` | Replan target operations on material edit or retry without changing queue identity or ordinal. |
| Adapt | `crates/orchid-engine/src/workspaces/checkout.rs` and its caller | Preserve categorized, sanitized checkout failure detail. |
| Retain | `src/application/agentSessions/sessionQueue.ts` | Queue DTO helpers and effective-target projection. |
| Create | `src/application/agentSessions/sessionQueuePresentation.ts` | Shared queue, transcript, header, and device-activity classification. |
| Adapt | `src/features/agentSessions/useSessionTarget.ts` | Track selection provenance and rebase completed operation intents without losing later user choices. |
| Simplify | `src/features/agentSessions/SessionQueuePanel.tsx` | Compact queue disclosure and ordering only. |
| Create | `src/features/agentSessions/SessionQueueItemDialog.tsx` | Entry details, editing, errors, retry, removal, and accessible movement. |
| Create | `src/features/agentSessions/SessionDeviceActivityList.tsx` | Top-right active device and worktree progress. |
| Adapt | `src/features/agentSessions/ConversationViewport.tsx` | Separate queue and device-activity layout slots. |
| Adapt | `src/features/agentSessions/AgentSessionScreen.tsx` | Consume the shared projection and compose the three surfaces. |
| Adapt | `src/features/agentSessions/transcriptProjector.ts` | Omit future queued invocations from transcript presentation. |
| Adapt | `src/features/agentSessions/AgentSessionWorkspace.tsx` | Derive the badge from the active invocation and active target activity. |
| Adapt | `src/features/agentSessions/AgentSessionComposer.tsx` | Route `Ctrl+Enter` to steering with Send/Queue fallback. |
| Split | `src/features/agentSessions/sessionQueue.css` | Compact queue and modal styles only. |
| Create | `src/features/agentSessions/sessionDeviceActivity.css` | Top-right processing-list layout and progress styles. |

No database schema change should be needed. Existing target operations remain readable. Editing or retrying an older operation should replace its universal action list with a newly applicable plan.

## Implementation sequence

1. Add backend tests for target relations and operation-specific action plans, including an already-resolved create-worktree intent.
2. Extract the target-operation planner and use it for append, edit, and retry. Preserve the current dispatcher and activation boundary.
3. Reconcile completed operation intent in `useSessionTarget` so a resolved worktree becomes the adopted current target.
4. Add the pure frontend presentation projection and tests for pending, blocked, executing, staging, activating, staged, completed, and canceled entries.
5. Split the queue item modal from the compact queue panel, then add hover/focus expansion and retained reorder behavior.
6. Add the device activity list and separate viewport slots. Remove absolute top-right queue positioning.
7. Apply the shared projection to transcript visibility and header status.
8. Add `Ctrl+Enter` steering and fallback behavior after quick-menu handling.
9. Improve checkout error mapping and verify its concise queue summary and detailed modal rendering.
10. Rebuild the root executable and run the packaged app through the acceptance scenarios.

## Verification

### Target reconciliation

- Activate a newly created worktree, then send another message without changing device, branch, or worktree. No second target operation is appended and Git is not asked to create another worktree.
- Choose a genuinely different branch or device after activation. The user override remains and produces one target operation before the message.
- Retry an older blocked operation. Its action plan contains only actions applicable to that transition.
- Force a branch-already-checked-out conflict. The modal identifies the branch and conflicting checkout; the queue line remains concise.

### Queue and transcript

- With one future item, show one line and no more indicator.
- With three future items, show the first line and `+2 more`; hover and keyboard focus expand to three flat lines.
- Clicking each line opens the correct message or target-operation modal.
- Reorder a message across a target operation and verify its projected destination changes.
- Start a queued message. It disappears from the queue and appears once in the transcript while the backend retains its execution barrier.
- A pending or blocked message appears in the queue and not as a duplicate transcript turn.
- A later blocked setup does not replace an active turn's `Working` header state.

### Device activity

- Start target preparation early. Its compact queue line remains in schedule order while its current work appears in the top-right device activity list.
- Complete staging. The top-right item disappears while the queue boundary remains until activation reaches the head.
- Block preparation. The top-right item disappears and the queue line becomes blocked with retry and detail in its modal.
- Agent messages never appear in the top-right list.

### Keyboard behavior

- During an active steerable turn, `Ctrl+Enter` sends one steering input and creates no queue entry.
- During an active turn where steering is unavailable, `Ctrl+Enter` creates one normal queued message.
- While idle, `Ctrl+Enter` follows ordinary Send and sends once.
- `Shift+Enter` inserts a newline.
- An open quick menu consumes Enter before submission shortcuts run.

### Persistence and regression

- Restart with pending, blocked, staging, and executing records. The same three-surface classification is restored without duplicating transcript messages.
- Keep queue revision checks, delivery uncertainty handling, early staging, and activation ordering intact.
- Run focused Rust queue, preparation, repository, frontend queue, transcript, composer, target-selection, and viewport tests before the broader application suites.

## Scope limits

- Keep the existing ordered queue and relational records.
- Do not introduce a generic workflow engine or independently draggable preparation steps.
- Do not add a drag-and-drop, disclosure, or keyboard-shortcut dependency.
- Do not automatically delete worktrees or undo prepared devices when a queue entry is canceled.
- Do not make current reachability a prerequisite for writing or durably queueing a message.
