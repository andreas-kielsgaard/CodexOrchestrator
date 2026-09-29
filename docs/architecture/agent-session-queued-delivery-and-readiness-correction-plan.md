# Agent Session ordered message and target-operation queue

Status: implemented on 2026-09-29. The durable queue, target-operation staging, entry editing and reordering, restart recovery, legacy migration, and explicit steering path are active.

The implementation keeps target-operation actions inside one movable queue entry. It consolidates the planned frontend hooks and entry editors in `SessionQueuePanel.tsx`, and the backend planning and execution services in `application/queue.rs`.

## Target

An Agent Session owns one durable, ordered queue containing:

- messages, with their text and message-local execution choices;
- target operations, which change the device, repository, worktree, and provider context used by later messages.

Queue order defines execution context. Starting from the Session's active target, each target operation changes the projected target for the entries after it. Moving a message above or below that operation changes where the message will run.

Preparation may start early. Activation follows queue order. A device can start and a worktree can be created while an earlier message runs, but the Session does not switch to that target until the target operation reaches the queue head.

## Approved behavior

- Capability Profiles, repository configuration, and the last complete provider catalogue determine which options are eligible before Send.
- Current reachability, process state, worktree existence, and runtime discovery annotate readiness. They do not prevent writing, editing, or durably queueing a message.
- Selecting a different target and sending creates separate queue entries for the target operation and the message.
- Messages own text, model, reasoning level, attachments, sandbox, and other choices that do not change Session target.
- Target operations own device, repository, branch, worktree, migration, and provider-context choices.
- Reordering recomputes the effective target of every later message.
- A blocked operation retains its queue position and every later message. Retry continues the same operation.
- A message may be moved before a blocked target operation and then runs on the preceding target.
- Clicking a queued message or target operation opens an editor for its associated text and choices.
- Provider creation and prompt delivery occur only after the entry reaches the head and its prerequisites succeed.
- Failure after the delivery boundary remains uncertain and is never replayed automatically.

## Core model

### One heterogeneous Session queue

Keep `AgentInvocation` as the canonical message record. It already owns submitted text, options, transcript placement, runtime events, and delivery evidence.

Add `SessionQueueEntry` as the ordering layer:

```text
Message(invocation_id)
TargetOperation(operation_id)
```

The queue entry does not duplicate prompt text or target intent. It provides a common position, lifecycle, and revision boundary for heterogeneous entries.

Queue entry states are:

- `pending`;
- `executing`;
- `blocked`;
- `completed`;
- `canceled`.

Completed messages remain represented by their invocation transcript. Completed target operations remain as compact Session timeline events so target changes are visible between messages. The queue panel shows unfinished entries.

### Effective-target projection

Project the queue from the Session's active target:

1. Start with `session.execution_target`.
2. A target operation replaces the projected target for all later entries.
3. A message uses the current projected target without changing it.

Each pending message displays **Will run on …** from this projection. The projection is advisory until execution. When the message reaches the head, preparation reads the actual activated Session target and persists an exact execution snapshot before provider delivery.

The message record does not store a competing target choice. A target change initiated from the message editor inserts or edits a target operation before that message.

### Target operations and action sets

A `TargetOperation` is one user-meaningful queue entry such as **Switch to Laptop · branch X**. It owns an ordered internal action set:

- prepare or launch the destination device;
- resolve and freeze the selected branch revision;
- inspect source and destination;
- find, create, or verify the destination worktree;
- capture, transfer, and apply a sister-worktree snapshot when explicitly requested;
- prepare the destination runtime and provider context;
- activate the target.

The internal actions are visible in the operation editor and status expansion. They are not independently draggable because their order is required for the target change.

Use ordered actions rather than a generic DAG. The required operation is sequential, and a DAG scheduler would add machinery without improving this behavior.

### Stage early, activate at the head

Target operations have separate staging and activation phases:

- **Staging** may start through **Start now** or automatically after Send. It may proceed while earlier queue entries execute.
- **Activation** begins only when the operation is the earliest unfinished queue entry. It atomically updates the Session's active target and provider context.

Reordering remains available while an operation is pending or staging. Once activation begins, the entry is pinned until it completes or blocks.

If a staged operation is edited or canceled, its no-longer-valid step outputs are detached from the operation. Created worktrees and prepared devices remain registered resources; canceling a queue entry does not destructively undo them.

### Message execution

Normal Send always appends a queued message, including while a provider turn is running. Steering the active provider turn becomes an explicit **Steer current turn** action using the existing steering transport.

When a message reaches the head:

1. claim it as `executing`;
2. validate its message-local choices against the active target;
3. prepare or resume the workspace-specific provider context;
4. persist the exact execution snapshot;
5. cross the delivery marker and send the prompt;
6. mark the entry completed only when its invocation reaches a terminal state;
7. wake the dispatcher for the next entry.

Worker saturation occurs after durable acceptance. It leaves the message pending.

### Compose and enqueue

The composer projects the target at the queue tail. Its model and reasoning options come from stored eligibility for that projected target.

If the user chooses another destination and clicks Send, one transaction appends:

1. a target-operation entry for the destination, unless the queue tail already projects that destination;
2. a message entry immediately after it.

Both appear separately as soon as the transaction commits. The message can then be dragged above the target operation and will project onto the previous target.

A product Session shell may exist before a native provider conversation. Early staging creates the shell and operation records without starting an agent turn.

## Queue editing and reordering

### Entry editor

Clicking a future entry opens a modal:

- **Message:** prompt text, model, reasoning level, attachments, sandbox, and other message-local choices.
- **Target operation:** device, repository, branch, worktree intent, move-versus-use behavior, and the internal action set with current results and errors.

The message modal shows its derived target. **Change target** creates or edits a target operation before the message; it does not add a target field to the invocation.

Saving an edit is transactional and uses the queue revision. Text and choices remain editable while the entry is `pending` or `blocked` before delivery. A staging target operation remains editable; changing its intent invalidates incompatible staged results and replans its action set. An entry is read-only after activation or message execution begins.

### Reorder rules

- Future pending and blocked entries can move by drag, keyboard move controls, or the editor.
- An executing or activating entry is pinned.
- Entries cannot move before the current execution boundary.
- Reorder rewrites integer ordinals atomically and increments the queue revision.
- A stale revision returns the current queue for reconciliation instead of silently overwriting another edit.
- Every successful edit or reorder recomputes projected targets and compatibility for later messages.

If moving a message makes its model or another choice invalid for the newly projected target, keep the entry and mark it blocked with **Edit message**. Do not reject the move or discard its text.

Reuse the pointer-capture interaction pattern from `useSessionNavigationDrag.ts`. Extract a narrow reorder helper if needed. Do not add a drag-and-drop package.

## Blocking, retry, and restart

The dispatcher executes the earliest unfinished queue entry. A blocked head pauses later execution but does not prevent editing or reordering future entries.

Retry keeps the same queue entry, prompt identity, and ordinal. It resets the failed operation action or message preparation step and any dependent later steps while retaining safe outputs such as a frozen commit or verified worktree.

On restart:

- preserve all pending, blocked, and staged entries in order;
- reconcile interrupted idempotent operation actions;
- leave an operation activated only when the Session target commit is durable;
- return pre-delivery messages to a retryable state;
- classify a message with a delivery marker but no acknowledgement as uncertain;
- wake staging and head dispatch after reconciliation.

## Durable model

Create relational records:

```text
agent_session_queue_state
  session_id, revision, created_at, updated_at

agent_session_queue_entries
  id, session_id, ordinal, kind, state,
  invocation_id?, operation_id?,
  blocked_reason?, created_at, updated_at

agent_session_target_operations
  id, session_id, intent_json, phase,
  staged_target_json?, activated_target_json?,
  error?, created_at, updated_at

agent_session_target_operation_actions
  operation_id, ordinal, kind, status,
  payload_json, error?, attempt,
  started_at?, completed_at?
```

Constraints:

- exactly one of `invocation_id` and `operation_id` matches the entry kind;
- each invocation and operation appears in at most one queue entry;
- `(session_id, ordinal)` is unique;
- only one entry per Session is `executing`;
- queue edits, entry edits, and enqueue operations compare and increment `revision`.

Keep prompt-specific capability resolution, execution snapshot, provider context, and delivery evidence in `SessionPreparation`. Add `preparing` to the invocation lifecycle, but keep queue order in `SessionQueueEntry`.

Do not store prompt text in a target operation. Remove `QueuedTransitionPrompt` and `queue_target_transition_prompt`.

## Backend ownership and files

| Action | Files and objects | Intended result |
| --- | --- | --- |
| Create | `src-tauri/src/agent_sessions/queue.rs` | Queue entry types, lifecycle, ordinals, projection inputs, and edit/reorder rules. |
| Create | `src-tauri/src/agent_sessions/target_operation.rs` | Target intent, operation phases, internal actions, invalidation, retry, and activation rules. |
| Create | `src-tauri/src/agent_sessions/application/queue.rs` | Atomic enqueue/edit/reorder, effective-target projection, head dispatch, wake-up, and restart reconciliation. |
| Create | `src-tauri/src/agent_sessions/application/target_operation/{mod.rs,planning.rs,execution.rs}` | Side-effect-free action planning, early staging, and queue-ordered activation. |
| Create | `src-tauri/src/agent_sessions/repository/{queue.rs,target_operation.rs}` | Persistence, revision checks, claims, edits, action results, and atomic target activation. |
| Create | `src-tauri/src/agent_sessions/transport/queue.rs` | Queue read, enqueue, edit, reorder, Start now, Retry, Cancel, and notifications. |
| Adapt | `agent_sessions/domain.rs`, `ports/repository.rs`, `repository/{schema,mapping,preparation}.rs` | Invocation `preparing` state, queue foreign keys, migrations, and execution snapshots. |
| Adapt | `agent_sessions/preparation.rs`, `application/preparation.rs`, `application/preparation/execution.rs` | Start message preparation only at the queue head and use the activated target. |
| Adapt | `application/delivery/continuity.rs`, `repository/native_conversations.rs` | Bind provider continuation to execution route and physical workspace identity. |
| Adapt | `execution_targets/{mod.rs,preparation.rs}` | Split observation from readiness work. Runtime description must not launch devices. |
| Adapt | `active_app.rs`, `storage.rs`, Agent Session composition | Register schema, dispatcher, commands, notifications, and startup reconciliation. |
| Remove after migration | `agent_sessions/target_transition.rs`, `application/target_transition.rs`, `repository/target_transition.rs`, `transport/target_transition.rs` | The general target-operation queue replaces the device-only transition path. |

Git and device mechanics stay in `execution_targets` and `orchid-engine`. Agent Sessions own their scheduling and Session meaning.

## Frontend ownership and files

| Action | Files and objects | Intended result |
| --- | --- | --- |
| Create | `src/application/agentSessions/sessionQueue.ts` | Queue DTOs, revisioned commands, and pure effective-target projection. |
| Create | `src/features/agentSessions/useSessionQueue.ts` | Subscribe to queue state and expose enqueue, edit, reorder, Start now, Retry, and Cancel. |
| Create | `src/features/agentSessions/SessionQueuePanel.tsx` | Unfinished heterogeneous queue with projected targets, blockers, progress, drag handles, and keyboard controls. |
| Create | `src/features/agentSessions/SessionQueueEntryModal.tsx` | Route a selected entry to its message or target-operation editor. |
| Create | `src/features/agentSessions/QueuedMessageEditor.tsx` | Edit text and message-local choices and show the derived target. |
| Create | `src/features/agentSessions/TargetOperationEditor.tsx` | Edit destination intent and inspect or control its internal action set. |
| Create | `src/features/agentSessions/useSessionQueueDrag.ts` | Queue-specific pointer and keyboard reorder behavior using the existing navigation-drag pattern. |
| Extract/replace | `useSessionTarget.ts` -> `useComposerTargetDraft.ts` plus `sessionTargetEligibility.ts` | Keep draft target selection separate from live readiness and queue projection. |
| Adapt | `AgentSessionScreen.tsx`, `AgentSessionComposer.tsx`, `useAgentSession.ts`, `SessionComposerToolbar.tsx` | Send appends durable entries, clears only after acknowledgement, and exposes explicit steering. |
| Adapt | `SessionTargetDialog.tsx`, `draftTargetResolver.ts`, `composerTargetActions.ts` | Produce target-operation intent without performing readiness work or blocking Send. |
| Adapt | `DeviceContinuationDialog.tsx` | Feed move choices into the target-operation editor and shared action set. |
| Adapt | transcript projection and Session event rendering | Show completed target changes in chronological context between messages. |
| Replace | `SessionPreparationPanel.tsx`, `preparationPreview.ts`, `sessionPreparationState.ts` | Queue entries and operation action sets become the single preparation presentation. |
| Remove | `WorktreeCreationConfirmation.tsx` | Worktree creation becomes an editable action inside the queued target operation. |

The composer may refresh runtime and model observations in the background, but those promises never feed `sendUnavailableReason`. It consumes the last complete stored catalogue. Dispatch revalidates against the active destination and blocks the message for editing if conditions changed.

## Migration

Add one schema migration that:

- creates queue state, entries, target operations, and operation actions;
- places current unfinished invocations in their stable existing order;
- converts the current target transition into a target-operation entry;
- converts any `QueuedTransitionPrompt` into a normal invocation and adjacent message entry in the same transaction;
- maps existing preparation phases to queue and invocation states;
- replaces the broad one-active-invocation index with one-executing-entry enforcement;
- removes the old target-transition table after successful conversion.

Preserve accepted prompt text, submission IDs, and delivery markers. Do not retain a runtime compatibility layer for the old transition model.

## Implementation sequence

1. Add pure projection and repository tests for heterogeneous ordering, revision conflicts, edits, and atomic enqueue.
2. Add queue and target-operation persistence, migrate old transitions, and introduce the per-Session head dispatcher.
3. Move device/worktree work into target-operation staging and activation. Make provider continuity workspace-aware.
4. Replace frontend preparation gates with stored eligibility, the queue panel, entry modals, and revisioned drag reorder.
5. Make normal Send append during active turns and expose explicit steering.
6. Remove the old transition, preview, and worktree-confirmation paths.
7. Run native queue scenarios and rebuild the repository executable.

## Verification and acceptance

- While a message runs on target A, select target B and Send: the queue shows **Switch to B** followed by the new message.
- Add another message, drag it above **Switch to B**, and verify it projects to and executes on A; the later message executes on B.
- Reorder messages across A -> B -> A operations and verify every displayed and persisted target projection.
- Click a queued message, change its text, model, and reasoning, close and reopen the modal, restart the app, and verify persistence.
- Click a staged target operation, change its branch or device, and verify invalid steps are replanned while unrelated created resources remain registered.
- Move a message to a target where its model is unavailable: the move succeeds, the prompt remains, and the message shows an editable blocker.
- Start B early while A is running: device/worktree staging proceeds, but Session activation remains A until the operation reaches the head.
- Fail B staging: later entries remain intact; Retry resumes it, and a message moved before it can execute on A.
- Force runtime and model refresh promises to hang: typing, editing, Send, and reorder remain available.
- Select a local-only branch with no worktree: the operation freezes the local revision, creates and verifies the worktree, then activates it.
- Select an offline remote device: selection and Send succeed; failed startup retains the operation and prompt across restart.
- Exhaust preparation workers: another Send is accepted and later dispatched without resubmission.
- Restart during device startup, worktree creation, target activation, and message preparation without losing order or duplicating a prompt.
- Fail after the message delivery marker: report uncertain delivery and do not retry automatically.
- Switch to another same-device worktree: no file migration occurs, and a workspace-specific provider context starts.
- Use drag, keyboard move controls, modal edits, Retry, and Cancel in the packaged native UI. Capture evidence under `docs/validation/agent-session-ordered-queue/`.

Run focused frontend tests, Agent Session repository and application suites, worktree engine tests, `npm run build:frontend`, `npm run check:rust`, and the packaged native flow.

## Non-goals

- No generic workflow engine or arbitrary operation graph.
- No independently draggable low-level steps inside one target operation.
- No automatic Git push, clone, merge, or conflict resolution.
- No background synchronization between sister worktrees.
- No destructive rollback of devices or worktrees prepared by a canceled operation.
- No automatic replay after uncertain provider delivery.
- No redesign of Capability Profile authoring.
- No second transcript or remote product-session database.
