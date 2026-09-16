# Mutable session targets and preparation on Send

Status: implemented on `feature/Remote-Development`; final native UI acceptance remains outstanding. Cross-device continuation passed with Codex 0.154.0 on both devices. See [validation evidence](session-preparation-evidence.md) for verified behavior and remaining limits.

Inspected 2026-09-14: `feature/Remote-Development` at `66d6047` and conversation import at `bb6fdbd`. Main advanced from `53fabfb` to `a806212` during planning, integrating conversation import; remote targeting remains unmerged. Implement against their integrated source, preserving the main build/cache changes and rechecking current ownership. This plan supersedes the first-prompt target lock and the exclusion of conversation transfer/worktree creation in `remote-worktree-session-plan.md`; it does not replace that plan's remaining remote-runtime boundaries.

## Agreed product behavior

- Device and worktree remain selectable after the first prompt. Changing selections edits the next submission, not the executing turn.
- Changing selections does not migrate history, create folders/worktrees, or start/resume a provider thread. Send starts the required preparation, then delivers that submitted prompt when ready.
- The composer remains editable during submission, preparation, and execution. Preparation never covers it or takes keyboard focus.
- Keep the same Orchid session and transcript across target changes. Transfer Codex continuation history when the destination cannot access the current native conversation. Preserve the native thread ID subject to the compatibility proof below; never substitute a context-free conversation silently.
- Reuse a chosen worktree. Selecting a branch with no instance on the chosen device opens a modal confirming creation at that branch's latest published commit. Confirmation records the exact commit and a creation intent; Send performs materialization. Do not transfer dirty source files, compiler caches, installed tools, or the previous working folder.
- Use the selected light-theme visual: version 2's composer and floating checklist, with version 1's more coherent model/reasoning grouping. Remove the explanatory quick-action prose.

## Visual target and interaction

[Selected version 2 layout](assets/session-preparation/selected-running-layout.png) · [Version 1 model/reasoning treatment](assets/session-preparation/model-reasoning-group-reference.png)

These are design references, not evidence of implemented behavior. Retain Orchid's current shell, typography, light colors, and actual icons; do not adopt incidental generated branding, timestamps, or sample conversation content.

Place a single toolbar below the prompt text. Its left group contains Capability Profile, Device, and Worktree controls; its right group contains Model and Reasoning in one visually joined element, followed by Send. Model and Reasoning remain two independently accessible selectors inside that group. Show selected values, not instructional paragraphs. Identify an actual worktree instance, with repository/path available in its picker; a branch name alone does not identify a checkout.

Use a small amber pending marker on selections that require preparation, without a visible text label. Provide an accessible name and explanation on hover or click. It disappears if the user returns to the current ready target. Selecting a different model alone does not imply history migration. Read-only device/worktree discovery can populate menus; it must not perform the pending operations. If fresh destination facts are unavailable, show that uncertainty instead of claiming that setup is ready.

Default before Send: the indicator can open a preview of required steps. Do not automatically open the full floating panel for every draft change. On Send, open the panel automatically without focusing it. The separate worktree-creation confirmation opens when selecting a branch with no destination instance; it records intent without starting preparation.

That modal identifies the repository, branch, destination device, and resolved published commit (short SHA with the full SHA available), and explains that Send will create a new worktree there. Confirm selects that creation intent; cancel preserves the prior workspace selection. If the published tip cannot be resolved, show the discovery failure rather than substituting an unpublished local commit. Toolbar, target picker, and slash commands share this confirmation behavior wherever they offer branch selection.

| Phase | Progress surface | Prompt/composer behavior |
| --- | --- | --- |
| Draft selections differ | Amber marker without text; optional preview, all steps not started | Editor available; no preparation side effects |
| Submission accepted | Panel appears; submitted message says “Waiting for setup” | Freeze the submitted text/options; clear only that submitted draft |
| Preparing | Actual completed/current/upcoming steps | New typing remains available; another Send is disabled, not queued; no provider thinking indicator yet |
| Destination ready | Compact “Ready on [device]” receipt with View steps | Deliver only the already-submitted prompt; do not submit new draft text |
| Failed/canceled | Keep the failed/canceled step visible; retain submitted text | No delivery to the old device; new draft remains untouched |

Only show relevant steps: connect when needed; create/resolve workspace when needed; copy conversation history when the native store changes; resume/start the conversation; deliver the prompt. The panel follows real dependency order, which may differ from the illustrative screenshots. No timer-driven completion, invented percentage, or fixed four-step ceremony for every Send.

Cancel setup stops before prompt delivery where still possible; it does not delete the user's draft or established workspace. Once the turn has started, use the existing turn-cancellation behavior. Retry is available only for a known preparation failure before delivery, and reuses the accepted submission and any completed materialization. An uncertain provider delivery remains an explicit uncertainty, not an automatic retry.

## Current evidence and structural consequences

| Current owner | Finding | Consequence |
| --- | --- | --- |
| `AgentSessionScreen.tsx`, `SessionTargetControl.tsx`, `composerTargetActions.ts` | Screen supplies `fixed`/disabled reasons for started sessions; a header control and slash actions expose the same target separately. | Remove the first-prompt lock and drive toolbar, picker, and slash commands from one draft-selection owner. |
| `useSessionTarget.ts` | Target resets on session change; runtime/profile discovery only runs for a new-session draft. | Initialize draft choices from the current successful binding; support editing them for existing sessions and distinguish loading facts from preparation. |
| `AgentSessionComposer.tsx`, `useAgentSession.ts` | Textarea is disabled by `sending` and some active-turn states. Send awaits remote/provider setup; any active invocation is treated as a steering target. | Separate editor availability, submission acceptance, preparing state, and actual provider-turn steering. |
| `PerMessageRuntimeControls.tsx`, `AgentSessionExecutionSettings.tsx` | Message options live in a separate explanatory fieldset, used by ordinary and embedded sessions. | Reuse their option semantics in a compact grouped control; remove duplicate ordinary-session controls while retaining shared embedded behavior. |
| `direct_user.rs`, `transport/selections.rs` | First send accepts a target; follow-up sends accept only message-local model/reasoning/sandbox choices. | Carry an explicit draft execution selection through the ordinary send path for both first and later messages. |
| `application/workspaces.rs`, `invocation.rs`, `repository/mod.rs` | Target changes are rejected; directory recovery updates only a NULL directory. | Add an explicit successful-target update, rather than weakening the historical-directory recovery operation. |
| `application/targets.rs` | Interaction routing resolves an invocation through its session's current target. | Freeze execution routing with each accepted invocation so later draft/session changes cannot redirect cancellation or approvals. |
| `crates/orchid-engine/src/host.rs` | A persisted session binding rejects changed configuration, cwd, or provider ID. | Permit an explicit idle rebind during preparation; keep active invocation routing fixed. |
| `crates/orchid-engine/src/protocol.rs` | Discovery and invocation operations exist; native-history transfer and worktree creation do not. | Add narrow preparation operations to the shared engine and existing SSH protocol. |
| Import branch `runtime/codex/app_server/history.rs` | `thread/read` and `thread/fork` are native operations; decoding imports product types. Import creates an independent Orchid session. | Share native access after engine integration; keep product import decoding/lifecycle separate from continuation transfer. |
| `worktree_application/checkout.rs` | Exact-commit creation, attachment checks, and verification already exist locally. | Extract the physical checkout primitive for local/remote use, leaving Worktree Review output/retention ownership in the desktop. |

## Ownership and data flow

```text
Toolbar / slash actions / target picker
  -> draft execution selection (no preparation)
  -> Send: accept prompt + selection snapshot
  -> existing pending invocation + preparation record
  -> ordinary-session preparation service
       -> destination workspace operation
       -> native Codex continuation transfer, if required
       -> existing runtime start/resume, with readiness evidence
  -> provider accepts the submitted turn

Persisted preparation updates -> existing session notifications
  -> waiting message + floating checklist

The composer edits the next draft throughout.
```

Keep this an ordinary Agent Session send operation, not a generic migration platform, workflow engine, or queue service. Reuse the existing session repository, invocation identity, notifier, runtime supervisor, and SSH transport.

### Draft, accepted submission, and current binding

Create a typed `SessionExecutionSelection` for desired profile/device and workspace choice. Workspace choice is either an existing instance or a confirmed create request naming its repository, published branch, destination device, exact commit SHA, and attachment choice. The branch provides display/provenance; the confirmed SHA determines materialization. It is not a resolved `SessionExecutionTarget`: a requested new worktree has no path/handle yet.

The draft hook owns this selection and model/reasoning overrides. On Send, capture an immutable snapshot with a stable submission/invocation ID and expected current session binding. Changes made afterward belong to a new draft. Return the session/invocation acknowledgement after durable acceptance, before slow preparation. Schedule the bounded worker through Tauri's existing asynchronous runtime; the preparation service owns its cancellation and shutdown, rather than leaving an untracked detached task.

Use one preparation row keyed by the existing pending invocation ID. Store the accepted selection, source binding, resolved destination when known, current phase, step outcomes, and an error/cancellation result. Store the resolved execution snapshot with the invocation so runtime routing and later inspection refer to what actually executed. Ordinary messages retain user provenance; do not label them application/workflow inputs to reuse a helper.

Keep the existing single-active-invocation rule. A second draft is writable during acceptance and preparation, but another Send is disabled and is not queued. It must not be misrouted as steering into a provider turn that does not yet exist. Once a real turn is active, preserve existing steering only for the same active execution configuration. Changes to device, worktree, capability profile, model, or reasoning belong to a later turn; keep that draft editable and disable its submission until the active turn finishes. A general multi-message queue is outside this slice.

After a new-session acknowledgement, moving from draft ID to session ID must preserve text typed during acceptance and the selected navigation context. Keep the existing conditional draft clearing, and fix the session-change reset path so it cannot erase the next draft. Text editing and target selection never change the frozen in-flight request.

Update the session's current target/cwd/provider binding only when destination readiness is confirmed. Retain the original creation-profile snapshot for historical meaning; expose the current successful execution resolution separately. Each invocation stores its own resolution, so a profile change does not relabel prior turns. Destination failure leaves the last successful binding truthful.

For first Send, persist the new session identity, requested selection and pending message before remote setup. Finalize its creation-profile resolution when the destination workspace and effective configuration are known; do not invent a resolved profile for an uncreated workspace. During that pending state, the toolbar displays the accepted requested selection.

### Profile and device relationship

Retain the current device-bound Capability Profile model for this slice. The separate controls are coordinated selectors, not an assertion that any profile can run on any device. Device choice filters compatible profiles; preserve a valid current choice, choose an unambiguous available match, or request a profile choice when multiple matches exist. Do not silently choose the first profile or leave a stale incompatible profile displayed.

Both toolbar controls and slash choices use the same capability filtering and validation. Keep message-local model/reasoning override semantics; changing the selected profile refreshes available choices and makes an invalid override explicit. Do not silently replace an explicit requested model. Revalidate against the destination at Send using its resolved configuration and workspace. No capability-profile/device schema redesign is included.

## Native conversation continuation

The transfer unit is native Codex continuation data, not Orchid's rendered transcript. Reading/forking a thread and copying normalized chat items are not proof of portable continuation. Orchid already owns its conversation display/history; do not create another Orchid session or import duplicate historical invocations when switching devices.

The canceled experiment in the **Build handling** conversation (`01a0a024-39ab-7a32-833d-eb2542988dfb`) reported that remote Codex `0.144` rejected history produced by local `0.154`. This is reported experimental evidence, not a fresh verification of installed versions or a successful transfer. Compatible source/destination versions are a prerequisite for supported continuation.

Native portability evidence must cover isolated source and destination Codex homes using compatible Windows/Linux versions: export the native persisted history, install/resume it with an explicit destination cwd while preserving its thread ID, then transfer back after another settled turn. Record the tested versions and payload. Do not use a real user conversation as a transfer experiment. Prefer native app-server operations; if a native rollout file must be transferred, isolate its format handling inside the Codex adapter.

Preserving the ID is the intended implementation. The exact portable payload and native installation mechanism remain unverified. If the tested versions cannot preserve continuation this way, revisit that decision before implementing a silent fork, new provider conversation, or transcript-summary fallback.

The engine owns source-native-history lookup, export, destination installation, identity/boundary checks, and resume. The desktop owns which session is moving and relays the transfer through existing endpoints. Identify a native conversation by device, native configuration/home, and provider thread ID; the ID alone does not locate its history. Resolve native homes and paths on their owning device; Windows must not interpret remote paths. Source selection comes from the last successful provider binding, not a newly changed global native-profile preference.

Use a settled conversation boundary. A device switch does not execute while an old turn is writing that conversation. Copy only native continuation material required for this conversation, including required conversation attachments if the native format references them; do not copy credentials, the entire Codex home, unrelated sessions, code files, or caches. Historical paths remain historical evidence: supply the new current cwd without globally rewriting transcript text or tool arguments.

On return to a device, use the latest settled continuation, not a stale local copy merely because its ID matches. Refuse conflicting active/divergent native state rather than overwriting it. No continuous synchronization, branch merging of conversations, or independent simultaneous execution of the same session is required.

Split native thread readiness from prompt delivery inside the existing Codex adapter. Report successful start/resume with its actual thread ID and effective cwd before `turn/start`; the application accepts that readiness and commits the resolved binding before releasing the submitted prompt. Reuse that same invocation connection rather than creating a second speculative model turn. Provider acceptance remains separate evidence from successful preparation.

## Destination working folders

Keep read-only discovery and creation distinct. When the selected branch has no instance on the chosen device, resolve its latest published tip from the repository's configured remote and present the creation confirmation. Confirmation stores the exact SHA in draft state; materialization runs only after Send. Do not silently advance to a newer branch tip between confirmation and execution. A changed repository, branch, or destination requires confirmation of the new intent.

Use the mapped destination repository and the confirmed published commit. Any required retrieval of that commit occurs during preparation using the configured repository connection. No implicit repository cloning, publishing, credential provisioning, or transfer of dirty files is included. If the commit cannot be obtained, report a preparation failure rather than using another revision.

Reuse the existing physical checkout checks through a shared engine primitive. Do not force a branch into two worktrees: reuse its selected attached instance, or honor an explicit new-branch/detached attachment choice supported by that primitive. Preserve existing files and report an unavailable source revision or conflicting path as the failed preparation step.

If a non-repository local session needs an auxiliary folder, allocate it on the execution device through the same destination workspace boundary. Never fall back to the desktop application's cwd. A completed folder/history installation can be reused on retry; do not delete useful worktrees because a later step failed.

## Concrete change map

Paths are relative to the repository after integration; keep names open to a more focused owner discovered during implementation.

| Action | Files / objects | Intended responsibility |
| --- | --- | --- |
| Adapt | `src/application/executionTargets/{contracts,presentation}.ts`, `useSessionTarget.ts` | Desired selection versus resolved target, compatible profiles, confirmed creation intents with exact published SHAs, and pure pending-step preview. |
| Create | `src/features/agentSessions/SessionComposerToolbar.tsx` | Controlled profile/device/worktree controls and joined model/reasoning presentation. No endpoint operations inside rendering. |
| Adapt | `PerMessageRuntimeControls.tsx`, `useSessionExecutionSelection.ts`, `composerQuickActions.ts`, `composerTargetActions.ts`, `useComposerQuickMenu.ts` | Share option state and actions between toolbar and quick menu. Retain ordinary slash commands; remove fixed-target disabling. |
| Adapt/remove | `SessionTargetDialog.tsx`, `SessionTargetControl.tsx`, `AgentSessionScreen.tsx` | Reuse the branch/instance picker from toolbar and commands; confirm creation at the resolved published SHA when no destination instance exists. Remove the obsolete fixed header target control once its consumers move. |
| Adapt | `AgentSessionComposer.tsx`, `ConversationViewport.tsx`, `AgentSessionWorkspace.tsx`, `useAgentSession.ts`, `useProfiledAgentSession.ts` | Keep textarea editable, acknowledge accepted sends promptly, carry preparation separately from execution, preserve the next draft. |
| Create | `SessionPreparationPanel.tsx`, `sessionPreparation.css` | Nonmodal right-side checklist, pending preview, cancel/retry, collapsed receipt. Render persisted facts; never orchestrate work. At narrow width, place it above the transcript, not over the input. |
| Adapt/remove | `AgentSessionExecutionSettings.tsx`, `ProfiledSessionPane.tsx`, `agentSession.css`, `perMessageRuntimeControls.css`, `sessionTarget.css` | Remove duplicated ordinary composer option fieldset and helper prose; retain profile inspection, identities, event delivery views and shared embedded controls. Use optional toolbar targeting for ordinary sessions only. |
| Adapt | `src/application/agentSessions/selections.ts`, client contracts, `src/infrastructure/agentSessions/tauriAgentSessionClient.ts`, `src-tauri/src/agent_sessions/transport/{selections,mod}.rs` | First/follow-up Send accepts a desired selection and stable ID; return durable acceptance and expose preparation updates/cancel/retry. |
| Create | `src-tauri/src/agent_sessions/preparation.rs`, `application/preparation.rs` | Typed step/snapshot data and bounded ordinary-session preparation orchestration. Reuse current invocation lifecycle and failure handling. |
| Extract/adapt | `application/{direct_user,invocation,targets,workspaces,update_sink}.rs` | Separate accepting a pending user invocation from launching it; route by frozen invocation binding; commit only confirmed current target. Keep historical directory recovery distinct. |
| Create/adapt | `repository/preparation.rs`, repository mapping/schema/ports, active storage migration | Persist preparation and invocation execution snapshot, notify through existing session updates, retain old rows and original creation provenance. Recheck migration version at implementation time. |
| Adapt | `transcriptProjector.ts`, `AgentSessionTranscript.tsx`, shared status contracts | Display Waiting for setup separately from provider work; preparing invocations cannot expose steering. |
| Create/adapt | `src-tauri/src/execution_targets/preparation.rs`, `endpoints.rs`, `ssh_connection.rs`, `remote_runtime.rs` | One local/SSH facade for read-only published-tip discovery, workspace preparation, and native continuation transfer; stream typed step outcomes over existing transport. |
| Create/adapt | `crates/orchid-engine/src/protocol.rs`, `host.rs`, new `session_preparation.rs` | Read-only published-tip lookup and narrow workspace/history preparation requests, explicit idle binding replacement, unchanged active-invocation routing. No second remote product history service. |
| Extract/adapt | `crates/orchid-engine/src/codex/app_server/history.rs`, new Codex continuation module, `app_server/mod.rs` | Reusable native read/fork access, portable continuation adapter, start/resume readiness before turn delivery. Keep imported product DTO decoding in the desktop. |
| Extract | `src-tauri/src/worktree_application/checkout.rs` and its required Git/value helpers into `crates/orchid-engine/src/workspaces/` | One exact-checkout implementation for Session preparation and Worktree Review; desktop wrapper retains existing Review API/retention. Do not move build, capture, or launch recipes. |
| Adapt | `active_app/sessions.rs`, `active_app.rs`, module declarations | Composition only: wire preparation, repository, endpoints, notifier. Keep implementation out of `lib.rs`. |

Remove fallback text such as “Type /worktree or /device…” and “Type / for model, reasoning, and skills,” plus duplicate “Next message” prose now represented by selected toolbar values. Keep actual errors and meaningful action feedback. Do not remove slash functionality or actionable errors just to remove explanatory copy.

## Validation expectations

Validate the full flow in a runnable Orchid application using the existing application build/cache commands. Build and Launch remain separate. Record screenshots and actual provider behavior separately from automated tests.

Focused automated checks must establish:

- Changing each selector or running a target slash command performs no creation, history transfer or provider start/resume. Pending markers reflect actual required work and clear when selections revert.
- Selecting a branch without a destination instance shows the creation modal with its latest published SHA. Cancel preserves the prior selection; confirm records intent only. Send uses that confirmed SHA even if the published branch advances. Unresolvable published tips cannot become confirmed creation intents.
- First and later sends capture the chosen profile/device/worktree/model/reasoning consistently; invalid combinations cannot send to a silently substituted destination.
- The accepted message appears while setup is blocked. Typing, caret/focus and new-session draft handoff survive acknowledgements, progress events, cancellation and completion.
- A preparing invocation cannot receive steering or a duplicate Send; disabled submissions are not queued. Changed execution settings cannot steer the old configuration. Later draft changes cannot alter active target, approval/cancel routing, or the submitted prompt.
- Same-store worktree changes skip history transfer; new worktree requests materialize once after Send. Review's existing exact-checkout checks still pass after extraction.
- Failure before readiness leaves the previous current binding intact and delivers no prompt. Known retry reuses completed work; uncertain delivery does not duplicate a turn. A stopped application does not silently replay pending prompts at restart.
- Creation-profile history and prior invocation targets remain unchanged. Current target survives reload and matches confirmed native cwd/context identity.
- Embedded/workflow session callers retain their supported local behavior; adding toolbar reuse does not enable remote workflows.

Live acceptance covers a local worktree switch, local-to-remote and return device switches with conversation continuity, deferred destination worktree creation, a blocked transfer while actively typing, cancel/retry before delivery, and collapsed/completed progress. Verify full native continuation rather than inferring it from Orchid's displayed transcript. Check the light toolbar, joined model/reasoning control, keyboard operation, long worktree names, and a narrow window. No code/dirty-file migration, generic task queue, automatic reconnect/replay system, or migration dashboard is included.
