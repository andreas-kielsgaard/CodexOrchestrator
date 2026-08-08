# Observation pass: startup, shutdown, and user observability

## Anchor

What work can the backend recover or start before the React product is available, what evidence can the mounted frontend later reconstruct, and what does application exit settle without a usable frontend control surface?

This pass follows one cold-start-to-exit interval at research baseline `9240364`. It records implemented behavior and visibility boundaries; it does not propose a target lifecycle design.

## Executive observation

The backend lifecycle is operationally authoritative. During Tauri setup it can terminalize stale Agent Session invocations, create application-owned Sessions, prepare orchestration material, open scoped MCP servers, start Codex, and continue later workflow stages. None of those effects waits for a mounted frontend or for user acknowledgment after reopen.

The frontend usually learns the resulting state after the fact through durable queries. That catch-up is uneven:

- Agent Session views combine durable initial loads, live `agent-session-update` events, and a 1.5-second fallback poll while an invocation is active.
- The orchestration overview takes a mount-time composition of three sequential snapshots and has no general event subscription or periodic refresh.
- Native Profile recovery is not run by the backend's general startup sequence. Several pending-attempt reconciliations run when the Technical Settings query is made, so their durable cold-reopen result may not be established until that secondary view is opened.

Exit protection is also uneven. Orchestration MCP registries are explicitly drained, then the Agent runtime waits, terminates, reaps, and can prevent Tauri exit on a returned shutdown error. Native Profile child cleanup and debug review cleanup rely on best-effort `Drop` behavior whose failures are not raised to the frontend or used to prevent exit.

## Ordered startup boundary

`src-tauri/src/active_app.rs:83-304` is the productive startup composition root. Its ordering is part of the observed recovery protocol:

| Order | Backend action | Immediate effects and evidence | Frontend state at this boundary |
| --- | --- | --- | --- |
| 1 | Open the database and Native Profile service (`active_app.rs:85-109`) | Durable repositories become available. Native Profile in-memory child maps start empty. | No product acknowledgment is required. |
| 2 | Construct Agent Session runtime/application and call `AgentSessionApplication::reconcile_startup` (`active_app.rs:123-159`) | Every invocation still recorded active is settled from known terminal delivery or marked `interrupted` (`agent_sessions/application/lifecycle.rs:761-839`). | The Tauri event emitter exists, but React listeners are not the recovery authority. |
| 3 | Construct orchestration services and attach bootstrap/Sprint callbacks to the shared notifier (`active_app.rs:160-220`) | The transition callbacks did not receive the earlier Agent Session startup notifications because they did not yet exist. | No UI dependency. |
| 4 | Run bootstrap startup reconciliation (`active_app.rs:221-223`) | Durable terminal state is observed again in orchestration context; material, Sessions, MCP servers, and Codex invocations can be prepared or started. | No UI confirmation or startup gate. |
| 5 | Run Sprint startup reconciliation (`active_app.rs:224-226`) | Sprint, Handler, Implementer, review, retry, escalation, candidate, integration, dependency-wave, and settlement reconcilers run in a deliberate order. Some stages can create Sessions/worktrees, manipulate Git state, open MCP servers, and launch Codex. | No mounted overview is needed. |
| 6 | Manage services and, in debug builds, compose Worktree Review infrastructure (`active_app.rs:227-303`) | Long-lived state becomes reachable by Tauri commands. Debug review process ownership is also established. | The product can mount only after setup succeeds. |

Startup reconciliation uses durable facts to compensate for the period when listeners are absent. `ManagedPlanBuilderNotifier::notify` later dispatches terminal cleanup, bootstrap progression, Sprint progression, and only then the Tauri Agent Session event (`active_app.rs:28-80`). During the earlier Agent Session reconciliation, however, the bootstrap and Sprint weak references are still unattached. The explicit later reconciliation passes are therefore not redundant notification replay.

If any setup reconciliation returns an error, the `?` propagation in `active_app.rs` aborts setup. There is no already-mounted product error surface for that failure, and an earlier stage may already have written durable state or started external work before a later stage fails.

## Work that startup may perform

### Agent Session recovery settles; orchestration recovery may relaunch

Agent Session startup does not reattach to a previously owned Codex process. For each durably active invocation it:

- applies a known terminal delivery when one exists;
- otherwise writes `interrupted`;
- records `runtime_startup_without_launch_acceptance` when the stale invocation never durably reached launch acceptance;
- removes the update lane and emits a terminal notification.

The implementation is in `src-tauri/src/agent_sessions/application/lifecycle.rs:761-839`. A notification failure does not undo the lifecycle fact: `notify_or_record` writes an `agent_session_notification_failed` diagnostic (`lifecycle.rs:1042-1052`).

The later orchestration reconcilers interpret that settled state. Bootstrap startup loads initiation snapshots, observes terminal outcomes, and reconciles each transition (`src-tauri/src/orchestration/bootstrap_transition.rs:1205-1215`). Its normal reconciliation can:

- prepare bootstrap files/material (`bootstrap_transition.rs:1309-1333`);
- create the Bootstrap Generator Agent Session (`bootstrap_transition.rs:1342-1355`);
- open and retain a scoped loopback MCP server, prepare an invocation, and start Codex when no attempt was persisted (`bootstrap_transition.rs:1375-1424`);
- accept semantically complete material and ensure the Epic Runner (`bootstrap_transition.rs:1436-1477`);
- prepare or recover an Epic Runner invocation, open its MCP endpoint, and start Codex (`bootstrap_transition.rs:1479-1565`);
- create a recovery invocation when a Runner ended without selecting a Sprint (`bootstrap_transition.rs:1570-1687`).

Sprint startup is broader. `src-tauri/src/orchestration/sprint_runner_transition.rs:1652-1695` first reconciles each Sprint, then its Work Unit handlers, implementer outcomes, reviews, retries, no-progress handbacks, Epic escalations, accepted candidates, integrations, dependency waves, and Work Slice settlements. Representative launch paths include Sprint pre-start (`sprint_runner_transition.rs:2077-2127`), Epic/Sprint continuation (`sprint_runner_transition.rs:2139-2178`), and planning control (`sprint_runner_transition.rs:2181-2208`). The same service owns scoped MCP preparation for the Epic Runner, Sprint Runner, Work Slice Planner, Handler, Implementer reporting, review, retry, handback, and escalation roles (`sprint_runner_transition.rs:1469-1552`).

The resulting distinction is important: restart recovery can first mark the old process ownership interrupted and then, from durable orchestration semantics, start the next eligible application-owned work. `interrupted` is not by itself evidence that startup remained passive.

### Startup effect and product evidence matrix

| Backend effect | Durable or live evidence | Where the user can eventually see it | Freshness/control boundary |
| --- | --- | --- | --- |
| Stale invocation becomes interrupted | Agent Session invocation lifecycle and diagnostic | Agent Session selector/transcript | Visible after the Agent Sessions controller loads the Session. The early event need not have been received. |
| Application-owned Session is created | Session summary/details and orchestration correlations | Agent Sessions tree; embedded Agent Session views; orchestration transition labels | Agent Sessions are not the initial surface. Embedded views appear only when the relevant route is mounted. |
| Scoped MCP endpoint is opened | In-memory registry plus durable launch/configuration evidence around its invocation | No direct MCP-server inventory or stop control was found in the frontend | The user sees the associated Session/stage, not the server lifecycle itself. |
| Codex is launched | Durable launch acceptance, invocation events/lifecycle, orchestration transition state | Agent Session conversation and projected orchestration stage | The overview can remain at its mount-time snapshot while the Agent Session updates live. |
| Bootstrap material is prepared/accepted | Durable bootstrap transition and material paths | Epic bootstrap status/reason in the orchestration view | No explicit statement says that startup recovery performed the action. |
| Sprint/Work Unit progression occurs | Durable Sprint transition, Sessions, attempts, candidates, integration/settlement state | Orchestration views and related Agent Sessions | No general orchestration-state-changed event; several views require remount or a specific refresh path. |
| Worktree/Git activity occurs in a transition | Durable workflow records and repository state | Later Work Unit/review/integration projections | No startup activity panel or pre-mount approval surface was found. |

The product generally exposes the current durable outcome, not a causal startup ledger. A stage label can say that a Runner is active or material was accepted, but it does not distinguish work performed before the frontend mounted from work performed after a live notification.

## Frontend catch-up and temporal skew

### The orchestration overview is a mount-time composition

`App` defaults to the `epics` surface and immediately invokes `useOrchestrationLoad` (`src/app/App.tsx:119-140`). The hook loads once when its client mounts and exposes a refresh callback (`src/app/useOrchestrationLoad.ts:11-46`). In the current product, the explicit overview refresh is used after Epic initiation confirmation (`App.tsx:275-317`); no periodic refresh, general orchestration event listener, or overview refresh control was found.

The client does not receive one atomic backend snapshot. `src/infrastructure/orchestrations/tauriOrchestrationNativeQuery.ts:33-65` invokes, in order:

1. the large native orchestration query;
2. the bootstrap-transition query;
3. the Sprint-transition query.

The calls use separate service/repository reads and share no transaction or snapshot token. Backend reconciliation and child activity can advance between them. Only the native query carries its own `generatedAt`; that timestamp does not establish that the two later transition reads represent the same instant.

Planning drafts are loaded separately (`App.tsx:158-179`), adding another freshness point to the rendered product state. The composed screen is therefore a useful durable projection, but not proof of a single-time system snapshot.

### Agent Session visibility is more live, but route-dependent

The Tauri notifier emits `agent-session-update` after the durable update and after orchestration callbacks (`src-tauri/src/agent_sessions/transport/mod.rs:17-65`). Tauri events are wake-up signals, not replayed history.

When an Agent Session controller is mounted, it subscribes and loads durable summaries/details (`src/features/agentSessions/useAgentSessionController.ts:200-251`). While the selected Session has an active invocation it also polls every 1.5 seconds (`useAgentSessionController.ts:372-391`). Those durable reads recover events missed before subscription.

The route still matters. Agent Sessions render only when `surface === 'agent-sessions'` (`App.tsx:526-540`), and Plan Builder-specific proposal refresh behavior exists only while that route is mounted (`src/features/orchestrations/EpicPlanBuilder.tsx:99-121`). A startup-created Session is discoverable, but its screen is not automatically brought forward.

Standalone Agent Sessions expose Cancel for the active invocation (`useAgentSessionController.ts:343-359`; `AgentSessionComposer.tsx:60-66`). The orchestration overview does not expose a corresponding stop control for the transition-owned MCP server or startup-launched workflow. Reaching Cancel requires navigating to and selecting the correlated Session, assuming its invocation is still active.

### Event coverage is narrower than backend progression

The productive Tauri event surface includes `agent-session-update` and the Epic initiation confirmation event (`src-tauri/src/orchestration/transport.rs:88-104`). No general orchestration-state-changed or startup-recovery event was found.

The confirmation listener is installed only when its React hook mounts (`src/app/useEpicInitiationConfirmation.ts:57-76`). Agent Session durability gives that subsystem a query-based catch-up mechanism. The overview instead relies on its own explicit durable reload points. Backend workflow progression can consequently be newer than the surrounding orchestration read model even when a nested Agent Session transcript is current.

### Native Profile recovery is query-triggered

`NativeProfileService::open` opens schema/storage and initializes empty in-memory owned-child maps (`src-tauri/src/native_profiles.rs:1569-1602`). It does not perform the full pending-login/setup/full-access-attempt reconciliation used by `query`.

`NativeProfileService::query` revalidates each profile and reconciles sandbox adoption, login attempts, setup attempts, full-access canaries, and expired MCP probes (`native_profiles.rs:1608-1623`). On a cold reopen, a pending attempt whose owned process no longer exists can then become `recovered_unobserved` with durable attention, such as the login path at `native_profiles.rs:3467-3540`.

The frontend invokes that query when `NativeProfileSettings` mounts and on manual Refresh (`src/features/nativeProfiles/NativeProfileSettings.tsx:11-24,50`). Technical Settings is a secondary route (`App.tsx:541-543`), not part of initial Epic-surface loading. Agent launch authority performs narrower profile/continuity/readiness checks (`native_profiles.rs:2393-2427,3804-3818`), not the full query reconciliation. Therefore Native Profile recovery evidence may remain unresolved until the user opens or refreshes Technical Settings even while orchestration startup is already operating.

## Ordered shutdown boundary

`src-tauri/src/active_app.rs:398-424` handles `RunEvent::ExitRequested` in this order:

1. `ManagedPlanBuilderService::shutdown` drains managed Plan Builder MCP handles.
2. `PostConfirmationTransitionService::shutdown` drains bootstrap MCP handles, then delegates to Sprint Runner shutdown for its scoped MCP handles.
3. `AgentSessionApplication::shutdown_runtime` asks the Codex process supervisor to shut down.
4. Only an Agent runtime shutdown error is logged and followed by `api.prevent_exit()`.

The MCP server handles explicitly cancel their loopback server and join its task (`src-tauri/src/orchestration/mcp.rs:589-620`, `bootstrap_transition.rs:1895-1926`, `sprint_runner_transition.rs:4440-4458`). The registries are drained before the Codex child-process grace period begins (`orchestration/application.rs:101-133,175-177`; `bootstrap_transition.rs:1117-1149,1276-1283`; `sprint_runner_transition.rs:1555-1561`). Their shutdown methods do not expose progress or stop failures to the frontend.

The Agent runtime uses a two-second graceful period (`src-tauri/src/runtime/codex/runtime.rs:39,454-464`). The process supervisor then:

- marks itself shutting down, rejecting later process starts;
- waits up to the grace deadline for active children and terminal callbacks;
- marks remaining direct children as shutdown, terminates them, and waits for reaping and callback completion;
- returns an error after draining if termination failed.

That behavior is in `src-tauri/src/runtime/processes/supervisor.rs:211-310`. Shutdown terminal outcomes are projected as interrupted Agent invocations, and callback completion remains inside the drain boundary (`runtime/processes/monitoring.rs:235-273`; `runtime/codex/runtime.rs:467-488`). The wait for final reap/callback after termination has no second timeout in this path.

The frontend has no exit-progress, MCP-drain, child-termination, or exit-prevented status surface. A late Agent Session event may be emitted while the webview is closing, but durable lifecycle persistence is the authority. If runtime shutdown eventually returns an error, the app can remain open because exit was prevented; the only direct explanation in this handler is written to stderr.

The bootstrap and Sprint transition services do not expose a shutdown latch in these artifacts. Runtime terminal callbacks remain part of the synchronous drain, while the process supervisor rejects new child starts after its shutdown flag is set. Any resulting diagnostic or callback failure is backend evidence rather than an interactive exit decision.

## Native Profile exit contrast

`Drop for NativeProfileService` locks the login, setup, and full-access child maps, calls `terminate` on each owned process, ignores every error, and clears the maps (`src-tauri/src/native_profiles.rs:3771-3791`). System termination uses child kill/wait; successful release detaches inherited stream-drain threads (`native_profiles.rs:684-750`).

This cleanup is materially different from the Agent runtime contract:

| Property | Agent runtime | Native Profile owned children |
| --- | --- | --- |
| ExitRequested call | Explicit | None; cleanup occurs on `Drop` |
| Grace period | Two seconds before forced termination | No equivalent service-level grace protocol |
| Durable lifecycle settlement during shutdown | Runtime update sink records terminal/interrupted outcomes | `Drop` does not update attempt disposition or attention |
| Error handling | Returned error can prevent Tauri exit | Termination errors are ignored |
| Frontend visibility | No progress UI; later Agent Session state is durable | No progress/error UI; cold-reopen query may later classify a missing process as unobserved recovery |

Explicit Native Profile invalidation paths do perform durable cancellation/cleanup (`native_profiles.rs:3574-3627`). That stronger behavior is not called by the application exit handler.

## Debug Worktree Review exit contrast

This surface exists only in debug composition. `src-tauri/src/worktree_review/composition.rs:10-57` creates its durable review root/SQLite registry, a `WindowsJobProcessOwner`, runtime application, and launcher services. `active_app.rs:252-297` optionally starts the debug review controller and manages the services; production composition reports contextual review unavailable instead.

Explicit Worktree Review Stop/Recover operations are durable and user-visible (`src-tauri/src/worktree_review/service.rs:659-675`; `src-tauri/src/worktree_review/detail.rs:101-118,168`). Exit does not invoke that operation. Instead:

- Windows-owned review processes are assigned to Job Objects configured with `JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE` (`src-tauri/src/worktree_runtime/ownership.rs:253-320`). Dropping the stored handles closes them (`ownership.rs:473-505`), which gives the OS ownership boundary its process-tree cleanup semantics.
- `DebugReviewController::drop` sends a graceful HTTP server shutdown signal and removes its descriptor, ignoring errors (`src-tauri/src/worktree_review/debug_controller.rs:77-90`).

The ExitRequested handler does not explicitly advance the review lifecycle registry or display an exit outcome. Build material and durable review records remain; the next launcher/recovery read determines what the retained state means. The evidence supports implicit process cleanup plus later recovery, not a claim that exit recorded the same durable Stop result as the explicit user action.

## Resulting observability boundaries

The current implementation separates several facts that can easily look like one lifecycle from the product surface:

- backend setup completion versus frontend mount;
- stale-process interruption versus orchestration eligibility to start new work;
- durable state versus delivery of a non-replayed Tauri event;
- a fresh Agent Session transcript versus a stale surrounding orchestration projection;
- Native Profile service open versus query-triggered attempt recovery;
- user-visible explicit Stop versus implicit process-owner cleanup at exit;
- MCP endpoint shutdown versus Codex child shutdown;
- exit prevention versus a user-visible explanation for why exit did not complete.

No startup recovery ledger, background-work inventory, shutdown progress surface, or direct MCP lifecycle control was found in the frontend. That absence does not mean the lifecycle is unowned: the backend services and durable repositories own it. It means the user-visible product primarily presents reconstructed current state, with different refresh rules per subsystem, rather than one temporally coherent account of what startup and shutdown did.

## Questions preserved for later analysis

- Which pre-mount effects are intentionally automatic recovery behavior, and which are incidental consequences of calling the ordinary reconcilers during setup?
- Which user-visible views are expected to represent live state, navigation-time state, or mount-time state?
- Is the setup order a supported recovery contract or only a dependency order in the current Tauri composition root?
- What semantic relationship, if any, is intended between orchestration-level ownership and standalone Agent Session Cancel?
- Should Native Profile attempt recovery remain query-triggered, given that its child cleanup is also outside the explicit ExitRequested protocol?
- Which debug review state is expected after implicit Job Object cleanup compared with explicit durable Stop?

These are classification questions for the later keep/tune/prune/refactor work. This pass establishes the implemented temporal and observability seams without assigning a disposition.
