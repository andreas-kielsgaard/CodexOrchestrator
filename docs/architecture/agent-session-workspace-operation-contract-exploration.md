# Agent Sessions: workspace operations and delivery contracts

Status: source-grounded exploration, 2026-09-30. Recommendations below are proposals, except where marked as user decisions. Scope: ordinary Orchid Agent Sessions, their workspace queue, and the delivery boundary used by other application callers.

Subsequent user decisions supersede the proposals and open questions in this exploration. The current work is defined in [Target-switch implementation shape and change list](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/docs/architecture/agent-session-target-switch-implementation-shape.md>).

This inspection covers the current main checkout, including the pending correction that removes HEAD from workspace-selection equality. That correction is not a rebuilt running application. No additional runtime behavior was changed for this exploration.

## 1. Agreed direction

- Provider switching belongs implicitly to message delivery. It is not a standalone operation entry.
- Operation entries describe relevant preparation of the workspace in which the agent will work.
- Selecting a device, worktree, or branch must not manufacture a switch because an observed commit changed or is missing.
- Device migration and worktree selection need distinct meanings and presentation.
- Compact queued messages do not need device names or worktree paths. Destination details remain useful in the item dialog.

HEAD still matters when creating a checkout at an exact revision or transferring commits. That is an operation precondition, not destination identity. A detached HEAD has a commit but no attached branch; a missing HEAD may mean an unborn repository or unavailable observation. These are different states.

## 2. The concepts currently bundled into a target

| Concept | Meaning and owner | What a change should imply |
| --- | --- | --- |
| Orchid Session | Product-owned conversation, invocations, interactions, and history | One conversation across workspace and provider changes |
| Message choices | Model, reasoning, sandbox, and intended Capability Profile | Resolve and prepare the later invocation |
| Workspace destination | Device plus registered repository/worktree or product-owned empty directory | Select where the later invocation works |
| Observed workspace facts | Path, attachment, current HEAD, dirty state, availability | Refresh facts and evaluate the requested operation |
| Workspace operation | Durable intention plus necessary preparation and activation | Queue only work required by that intention |
| Provider binding | Native conversation handle owned by an exact provider configuration, endpoint, device, and workspace | Resume that binding or start another with Orchid context |

Today `SessionExecutionTarget` includes all of device, provider/configuration, profile/revision, repository, branch, worktree, path, and HEAD. Comparing this whole aggregate conflates destination changes, inference-policy changes, and changing observations. Merely adding more exceptions to equality will not define those boundaries.

The existing input-to-invocation architecture already puts provider routing and continuity under delivery. The workspace queue partially duplicates that responsibility.

## 3. What happens today

### Queue items and other activity

| Current item/path | Actual behavior | Current presentation |
| --- | --- | --- |
| Message | Durable pending invocation; launches when it reaches the executable head | First line of the message, with device/path metadata |
| Compaction | Application-owned `/compact` invocation; requires an existing native conversation and a Codex route | `Compact conversation` |
| Target operation | Existing workspace, creation, auxiliary workspace, or inferred cross-device migration | Always `Switch to <device>` |
| Steering | Durable interaction against the active invocation's frozen runtime; not an ordinary future queue item in this checkout | Separate interaction/transcript path |
| Approval/question response | Addresses the active invocation and provider request | Interaction controls |
| Active preparation | A target operation staging or activating | `Device activity`, with internal action count |

Storage has message and target-operation kinds; compaction is reconstructed as a command from invocation provenance and `/compact`. It is agent conversation work, not a workspace operation.

### Ordinary queued message

1. Accept the invocation, its preparation record, and any inferred target operation durably in one transaction.
2. Infer an operation if the submitted selection differs from the projected destination after earlier unfinished operations. Profile revision and full execution-binding differences still participate in this comparison.
3. Start operation staging early, potentially while an earlier agent turn is running. This may start a configured remote device and create a destination worktree. It does not yet activate the destination or capture the live source's changes.
4. Preserve queue order. An executing entry is pinned; a blocked head stops later delivery. A staged operation activates at its boundary before following messages.
5. For migration, inspect and capture the current source at activation, after earlier work. Apply the snapshot to the destination, verify it, then register sister activation.
6. Activate the target in product storage. Currently this also clears the current native provider binding and retains it in the provider cache.
7. Prepare the message: resolve the model's route on the selected device, choose native continuity, prepare settings and authorized MCP/skill exposure, persist the immutable execution snapshot, then release the prompt.

Queued messages store their model/reasoning/sandbox choices. Their workspace selection is deliberately omitted from the preparation record and reconstructed from the session target when dispatched. Thus moving a message across a target-operation boundary changes its workspace. Profile choice is also carried by that target rather than independently by each queued message.

### What “prepare the device” means

For local execution, `ensure_ready` immediately succeeds. For remote execution, it probes the configured runtime through the Orchid endpoint. If the probe fails, it runs the device's configured start action and waits for readiness, polling for up to 120 seconds. Concurrent requests share a per-device start gate.

This can start an actual configured device or service. It is not an installation, authentication, source synchronization, or workspace migration step. The probe currently depends on the selected provider's runtime description, so host readiness and provider readiness are coupled.

### What “verify the workspace” means

These checks are currently different:

- **Existing selection:** rediscover the registered worktree on the selected device and match its handle and path within the branch-filtered inventory. Refresh branch/HEAD facts. Do not checkout or restore the old selected commit.
- **Target-operation staging:** require repository targets to report a HEAD; display that HEAD as the verification result. Empty workspaces pass without one. This additional HEAD requirement is inappropriate as a universal selection-readiness rule.
- **New checkout:** verify the exact creation plan, including commit, attachment, top-level path, and common Git directory. Existing attached branches must still point to the planned creation commit.
- **Migration:** require attached source and destination, a clean destination, and destination history equal to or behind the source. The engine rechecks destination cleanliness and its expected HEAD immediately before application. Afterward it checks HEAD and counts of staged, unstaged, and untracked files; the application performs another HEAD comparison.

These are infrastructure and Git checks. They do not compile the project, test the agent's changes, or establish full byte-for-byte equivalence of the migrated source and destination.

### What provider switching does

Provider choice follows model, device, and Capability Profile routing during message preparation. It can select another native conversation, prepare configuration and harness exposure, and start another runtime. It is more than changing a display field, but it is not a reason to migrate project files.

The exact current route/workspace resumes its current handle. A previously used destination resumes its own cached handle with missed Orchid context. A new destination starts a native conversation with initial instructions and earlier Orchid context. Active steering, cancellation, and responses continue to address the original invocation; changing the composer does not move that invocation.

Current missed-context assembly includes earlier terminal user text and final agent replies. It does not replay the complete native tool/reasoning state, and this assembly does not include earlier image payloads or steering interactions. “Full context” currently means this selected conversation representation, not every event recorded by Orchid.

## 4. What a real migration changes

Current migration is inferred by: same nonempty repository ID, same branch ref, different device ID. It is not represented as a separate accepted `move current work` intention.

The source snapshot contains source-only commits, the staged binary patch, the unstaged binary patch, and eligible untracked regular files. Git-ignored files are omitted. Additional untracked exclusions include `.git`, `node_modules`, `target`, `dist`, `.cache`, and `.env` files. These exclusions do not remove such files from committed history or tracked patches.

Application imports the commit bundle, writes an Orchid virtual ref, runs `git reset --hard` to the source commit, reapplies staged and unstaged patches, and writes untracked files. On an attached destination, reset also moves its local branch tip. It does not create a user commit or push a remote ref. The source's work remains present.

Sister registration then records repository/branch ownership, the active device, and both instances. There is no background synchronization or automatic merge. Source and destination being on the same named branch is insufficient to promise they are already linked sisters or that all inconsistencies can be reconciled.

## 5. Supported behavior and limits

| Scenario | Current support/limit |
| --- | --- |
| Use an existing registered local/SSH worktree | Supported through destination inventory; no copying except the same-repository/branch cross-device case below |
| Same device, another worktree/branch | Selection and activation; no automatic dirty-state transfer or in-place branch checkout |
| Create a worktree for a selected branch | Resolve an exact local tip or remote published tip, fetch missing objects if needed, and materialize an attached worktree; Git attachment conflicts can block it |
| No repository selected | Product-owned auxiliary directory, not an incidental application cwd |
| Same repository/branch on another device | Automatically enters the snapshot migration path, even though selection itself does not encode migration intent |
| Clean equal/behind destination, attached source/destination | Supported migration path; source may contain local commits and file changes |
| Dirty, ahead, or divergent destination | Blocked; no merge/reconciliation path |
| Detached worktree | Engine can represent and create detached checkouts, but Agent Sessions inventory drops instances without a branch; migration explicitly rejects detached attachments |
| Missing/unborn HEAD | Not a supported general target-operation case; staging requires a reported repository HEAD |
| Provider change for the next message | Delivery has exact-route continuity and context handoff; target operations still contain redundant provider bookkeeping |
| Remote providers | The host currently registers Codex only; remote Claude is unsupported |
| Move an active invocation | Unsupported; destination changes affect later invocations |
| Mutable target on a harness-owned session | Rejected by the ordinary prepared-message path |
| Restart during preparation/migration | In-progress operations become blocked; pending messages survive. Retry is not an atomic rollback/resume guarantee |

Configured repositories and remote paths are prerequisites. A device selection does not clone arbitrary repositories, copy credentials, migrate dependencies/caches, or install providers.

Closing a laptop lid may suspend retained processes and network activity; it is not the same as restarting Orchid. `Ready` can mean prepared and waiting behind the current turn. The earlier inspected same-device operation was caused by stale HEAD equality, not by a genuine device migration or a sleep-recovery requirement.

## 6. Current contract gaps

| Finding | Consequence | Required correction |
| --- | --- | --- |
| Target equality mixes profile/route and workspace facts | A policy/configuration refresh can produce a workspace “switch” | Separate message route intent from workspace intent |
| Pending correction excludes HEAD but still compares branch/path observations | External attachment changes and selection provenance remain ambiguous | Define existing-instance identity separately from requested branch resolution |
| Resolved `Create` is recognized by repository/branch matching | Repeating an already resolved choice is suppressed, but unrelated same-branch worktrees can satisfy the heuristic | Retain the operation's resolved instance receipt |
| Migration is inferred from repository/branch/device | Selecting another workspace can imply copying work without an explicit move contract | Accept `use workspace` and `move current work` as different intents |
| Sister ownership is checked/registered after snapshot application | An ownership conflict can be reported after destination mutation | Reserve and validate ownership before any destination mutation; enforce it in delivery, not only the picker |
| Snapshot application has several filesystem/Git effects and no rollback transaction | Failure or restart can leave a partly or fully changed destination | Record the apply attempt and outcome; inspect it before retry instead of claiming universal resume |
| Retry/replanning retains completed action labels by kind | Completion labels are not proof that current facts still satisfy the step | Revalidate relevant facts; reuse effects only through matching receipts |
| Capture uses the staged destination HEAD despite another inspection at activation | A compatible destination that advanced since staging can fail the stale apply precondition | Build the snapshot precondition from the fresh destination inspection |
| Applied HEAD is not copied back into `staged_target` before activation | Persisted session/sister target facts can retain the pre-migration HEAD | Persist the verified destination observation |
| Removing a message does not remove its preceding inferred operation | An operation can remain with no consumer | Record why an operation exists and which messages require it |
| Activating even an already-selected target clears the native binding; cached cursor is `None` | A redundant operation can cause unnecessary continuity/handoff work | Complete satisfied workspace intent without rebinding; let delivery own provider continuity |
| `Select provider context` is a completed marker before actual delivery selection | Action list suggests work that has not happened there | Remove this workspace action and its activation-side bookkeeping |
| All titles say device switch; transfer progress is bundled into snapshot application transport | Same-device use, creation, migration, and actual progress are hard to distinguish | Present the accepted intention and observable effects |

The sister store has a source `claim` method, but this inspection found no production caller. Ownership is exposed in inventory and checked during final sister activation; it is not a filesystem lock against external tools. The current group is unique per repository/branch, so it also assumes one shared lineage for that branch.

Early preparation already uses generation checks to reject obsolete staging results after edits/cancellation. Physical work completed before cancellation may remain registered. Cancellation should report this fact and prevent subsequent activation, rather than imply that creation was undone.

## 7. Proposed behavioral contracts

### A. Message delivery

Accept durable semantic input, message choices, and provenance. Provider selection is implicit. Resolve one supported route, prepare its settings and authorized exposure, select exact-route continuity, freeze the invocation snapshot, and release once.

Keep the intended Capability Profile with the message rather than using a workspace operation to carry it. Revalidate its accepted revision before delivery; an unavailable/incompatible revision blocks that message with an actionable reason. It does not create workspace work. A later change to these choices cannot redirect an active invocation.

### B. Use a workspace

Select an existing instance by stable device/worktree identity. A branch picker resolves to an instance or an explicit creation request; the branch displayed on an existing instance is an observation. Worktree relocation or attachment changes need refreshed inventory and a defined mismatch result, not an inferred migration.

Verify existence, repository identity, accessibility, and Orchid ownership as applicable. Preserve its current commit and dirty state. Do not require equality with a previously observed HEAD. Missing Git facts can be diagnostic without preventing agent work when the directory is otherwise usable.

For the minimal change to today's queue, retain a genuine destination boundary as `Use worktree <branch>`, with required availability/ownership checks and later activation. An already-satisfied boundary completes without resetting provider state. Whether such lightweight selection should have its own visible entry is a product decision below.

### C. Create a workspace

Keep explicit repository/branch creation intent. Resolve its starting revision once, record the plan, materialize, and retain the resolved instance identity. A supplied exact commit is pinned; an unspecified branch tip is resolved at preparation, then pinned for that attempt. Later branch advancement does not silently retarget the created checkout or create another one.

Physical effects that survive cancellation remain visible as prepared resources. Existing-instance selection and new-instance creation must not be made equivalent solely because their branch names match.

### D. Move current work to another device

Accept an explicit source lineage/sister group and destination instance or creation intent. Capture the latest source work when the operation reaches its boundary, after the prior invocation is terminal. Reordering must not silently change which lineage will be moved.

Before mutation, verify ownership, source availability, repository/branch compatibility, destination cleanliness, and history relationship. Use fresh destination facts for the snapshot's apply precondition. Reuse a known sister, or explicitly adopt a compatible destination into the group. Ambiguous candidates require selection. An independent same-branch worktree is a candidate, not an automatically linked sister.

The source must remain unchanged during capture and the destination must remain exclusive during application. Serial Orchid turns do not freeze external editors or Git processes; concurrent external mutation is outside this initial contract. Stronger guarantees require capture-consistency checks and coordination, rather than stronger wording on the current lock annotation.

Transfer the defined snapshot, verify the defined result, persist current destination facts, then change active sister and session destination. Native provider continuity is resolved by the next message. This contract initially retains the supported equal/behind path; dirty/divergent or detached migration remains an explicit unsupported result until separately designed.

### E. Queue causality and relevance

- Distinguish explicitly requested operations from preparation inferred for messages.
- Record consumers of inferred preparation. Cancel it when no unfinished consumer remains; preserve independently requested operations and already-created resources.
- Revalidate queue projection after edits/reorder. If a message crosses a destination boundary, its changed destination must be clear in the editing/reorder flow.
- Start independent readiness/materialization early only for still-relevant intent. Do not capture live source state or apply migration ahead of the boundary.
- A prepared operation waiting behind a turn is `Ready · waiting for current turn`, not active work or a hang.
- Persist effects and receipts separately from action labels. Retry inspects what actually happened and reuses only a compatible result.

These contracts can reuse the existing durable queue. They do not require a second scheduler or a second canonical session history.

## 8. Presentation based on intention and work

| Intention | Proposed entry | Relevant detail/progress |
| --- | --- | --- |
| Same workspace, different provider/model/profile | No operation entry | Message delivery preparation/blocked reason |
| Select another existing workspace | `Use worktree <branch>` if retaining a boundary entry | Check availability/ownership; activate for following messages |
| Create an instance | `Create worktree <branch>` | Resolve starting revision, create, verify; then ready/waiting |
| Prepare an empty directory | `Prepare empty workspace` when creation is necessary | Create/reuse directory; readiness |
| Move current work between devices | `Move current work to <device>` | Source/destination inspection, snapshot bytes/files, apply, verification, activation |

Show applicable steps only. Skip a local readiness no-op; show a configured start when it actually runs. Do not label rediscovery as creation or a progress marker as provider selection. Snapshot transfer/application can be one step until there is separately observable transfer progress.

Rename the activity area to `Workspace preparation`, since it includes local work. Keep the future entry and activity view tied to the same operation identity. Compact message rows show message content and queue state; omit the repeated device/path. Keep source, destination, reason, consumer links, revision details, and retry implications in the dialog.

Use distinct states for queued message, delivery preparation, running agent, workspace preparation, ready/waiting, migration application, and blocked. `Start now` currently starts staging rather than bypassing the activation boundary; `Prepare now` is a truthful label for that behavior.

## 9. Ownership and implementation shape

| Owner | Retain/change |
| --- | --- |
| Agent Sessions delivery | Own all provider route/continuity/harness preparation; retain per-message profile choices and context cursors |
| Execution Targets | Own workspace facts, device readiness, physical materialization/migration, and sister ownership checks |
| Queue application | Classify explicit workspace intents, project boundaries, track consumers, schedule relevant staging, and revalidate at activation |
| Queue repository | Atomic acceptance/reordering/activation, durable resolved-instance and apply receipts; remove native-binding reset from workspace activation |
| Engine workspace module | Git/filesystem mechanics with explicit preconditions/results; describe partial outcomes and verification scope |
| Frontend projection/presentation | Share typed operation intention/status, truthful titles, compact rows, and one activity identity |

Adapt the existing modules rather than adding another universal “switch” subsystem. Remove `SelectProviderContext` from workspace action contracts and move the provider-cache write/clear responsibility to delivery. First retain the current native binding's owning route/workspace independently of the active workspace destination: preparation currently derives that location from the session target, so simply deleting the reset would risk associating the old handle with the new workspace. Replace scattered satisfaction comparisons with explicit intent/fact resolution shared through backend results. Creation and migration receipts should be narrow domain records, not a general workflow framework.

Once decisions are made, update the input-to-invocation document and queue correction documents to point to one accepted workspace contract. The older sister-transition plan and preparation evidence describe earlier native-continuation experiments; they are historical evidence, not current provider-state portability guarantees.

## 10. Extensions with explicit boundaries

| Extension | Contract needed before support |
| --- | --- |
| Another remote provider | Host registration, route catalogue, launch/settings, authentication on its device, interactions, harness transport, and execution evidence; no workspace-switch item |
| Detached/unborn existing workspace | Inventory and identity independent of branch attachment; directory-readiness rules independent of committed HEAD |
| Detached migration | Define destination attachment and exact revision behavior separately from existing branch-sister rules |
| Dirty/divergent destination | Explicit reconciliation/merge plan preserving both states; independently visible workspace work, never a silent send-time merge |
| Independent copies of the same branch | Explicit lineage/group identity; do not infer sister relationships solely from repository/branch |
| Concurrent/offline sister editing | A coordination/conflict contract beyond the current single-owner database annotation |
| Richer cross-provider context | Define which semantic turns, steering, images, and references transfer, plus cursor fidelity; keep native handles route-local |

These are extension seams, not work to implement as part of presentation cleanup. Existing synchronous application delivery also uses the delivery facade but has different sequencing from prepared queue delivery. New callers should state whether they need queued workspace preparation; sharing the facade alone does not give them its lifecycle semantics.

## 11. Decisions and acceptance

Three product decisions remain:

1. **Existing-worktree selection:** retain a `Use worktree` boundary with meaningful checks/activation (smallest change), or show only physical preparation operations and store destination intent directly with messages (changes reorder semantics).
2. **First sister adoption:** allow explicit adoption after compatibility checks (recommended), or require a previously registered sister. In either case, `Use workspace` must not imply adoption/migration.
3. **Destination inheritance:** retain current order-based inheritance with clear reorder consequences (smallest change), or pin each message to its accepted destination. Provider/profile choices should remain message-owned under either rule.

After those decisions, implement in bounded passes: intention/identity and message choices; labels/projection; queue consumer cancellation and satisfied boundaries; migration ownership/receipts/retry. Each pass has an observable outcome.

Acceptance should cover: changed or absent HEAD without a phantom operation; provider/profile change without workspace activation; existing-worktree use preserving files; creation reuse by resolved identity; explicit equal/behind sister migration preserving committed/index/working/untracked state; independent same-branch selection without transfer; ownership rejection before mutation; canceling the last consumer; edits/reorder during staging; failure/restart after apply without blind replay; and ready/waiting presentation behind an active turn.

Use focused contract tests for these cases and a native queue walkthrough for presentation. Real two-device migration evidence is needed for that path. This exploration inspected source and dated evidence documents; it did not perform a new native or two-device run. Earlier automated HEAD-equality checks do not establish the larger contracts proposed here.

## Source anchors

| Evidence | Source |
| --- | --- |
| Existing ownership boundary | [Input-to-invocation pipeline](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/docs/architecture/input-to-invocation-pipeline.md:1>) |
| Acceptance and omitted message selection | [Prepared acceptance](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/preparation.rs:159>) |
| Queue operation inference and staging | [Queue application](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/queue.rs:424>) |
| Migration/apply/sister ordering | [Target activation](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/queue.rs:816>) |
| Provider cache/reset in activation | [Queue repository](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/repository/queue.rs:1056>) |
| Action planning and identity correction | [Target planning](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/target_operation/planning.rs:13>) |
| Message route/settings/launch preparation | [Preparation execution](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/preparation/execution.rs:75>) |
| Native continuity and context fidelity | [Continuity](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/delivery/continuity.rs:35>), [context](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/delivery/context.rs:23>) |
| Readiness/materialization/inventory | [Execution Targets](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/execution_targets/mod.rs:85>), [selection resolution](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/execution_targets/preparation.rs:197>), [inventory](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/execution_targets/inventory.rs:110>) |
| Sister identity and ownership | [Sister store](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/execution_targets/sisters.rs:15>) |
| Snapshot contents, mutation, and verification | [Snapshot engine](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/crates/orchid-engine/src/workspaces/snapshot.rs:32>), [checkout engine](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/crates/orchid-engine/src/workspaces/checkout.rs:15>) |
| Remote provider registration | [Host providers](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/crates/orchid-engine/src/host/providers.rs:84>) |
| Queue titles, metadata, and activity projection | [Labels](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/features/agentSessions/sessionQueueLabels.ts:27>), [queue panel](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/features/agentSessions/SessionQueuePanel.tsx:110>), [presentation](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/application/agentSessions/sessionQueuePresentation.ts:29>) |
| Order-based destination inheritance | [Queue projection](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src/application/agentSessions/sessionQueue.ts:122>) |
| Restart classification | [Lifecycle reconciliation](<C:/Users/user/Documents/Code Projects/Codex Orchestrator/src-tauri/src/agent_sessions/application/lifecycle.rs:64>) |
