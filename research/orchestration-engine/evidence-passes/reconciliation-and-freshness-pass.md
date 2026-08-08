# Observation pass: persistence, reconciliation, and frontend freshness

## Anchor

What happens after a Codex process produces an event or reaches a terminal state, and how does that change both durable orchestration state and what the user sees?

This pass followed the notification rather than treating Agent Sessions, bootstrap, Sprint execution, Tauri, or the frontend as separate subjects. Evidence is from the research baseline `b28137b` unless a historical commit is named.

## Observed path

1. The runtime update sink persists an Agent Session event or terminal outcome.
2. The shared Agent Session notifier synchronously dispatches the persisted fact to orchestration services.
3. Bootstrap or Sprint transition services reconcile additional durable workflow state and may launch another application-owned invocation.
4. The notifier emits the original Agent Session update to the Tauri frontend.
5. Agent Session views reload their durable Session state. Most orchestration read models do not automatically reload.

## Concrete observations

### Persistence precedes notification

`src-tauri/src/agent_sessions/application/update_sink.rs` appends events and finishes invocations before calling `AgentSessionNotifier::notify`. Runtime context establishment is also stored in the Session binding before the event is dispatched.

The notification is therefore a wake-up signal about an already-durable fact. It is not the authority for the event or terminal state.

### One notifier became an orchestration lifecycle junction

`src-tauri/src/active_app.rs::ManagedPlanBuilderNotifier` currently fans each Agent Session notification through four consumers:

- the in-memory managed Plan Builder registry for terminal cleanup;
- `PostConfirmationTransitionService`;
- `SprintRunnerTransitionService`;
- the inner Tauri Agent Session notifier.

The type name describes its July origin, not its present responsibility. Git history shows:

- `2c6fd28` (2026-07-17) introduced the notifier with the managed orchestration backend;
- `fe40951` (2026-08-02) attached the Sprint transition service;
- `6cfdaad` (2026-08-06) changed registry-lock handling after a re-entrant launch path was found.

This is evidence that a Plan Builder-specific composition seam grew into a product-wide workflow trigger without being renamed or separated.

### Backend progression occurs before the frontend event

The notifier calls the bootstrap and Sprint services before the inner `TauriAgentSessionNotifier`. A frontend `agent-session-update` can therefore arrive after additional workflow state has already been reconciled or another invocation has already been requested.

All three downstream branches are attempted even if one fails; their errors are combined after dispatch. The UI event is not skipped merely because one orchestration callback failed.

### The notification path is synchronous and can re-enter itself

The comment added in `6cfdaad` records the concrete case: a Bootstrap-terminal transition can synchronously launch the Runner, whose durable launch provenance and fast runtime activity can re-enter the notifier before the outer notification returns. The weak-service registry locks must be released before callbacks are invoked.

This means notification is not only presentation transport. It is inside the runtime delivery and workflow-progress call stack.

### Notification failure does not erase the triggering durable fact

In `update_sink.rs`, notifier failure is returned as runtime update-delivery failure after the event or terminal outcome has been stored. In `agent_sessions/application/lifecycle.rs`, application-side notification failure is recorded as an `agent_session_notification_failed` diagnostic. Diagnostic notification is then attempted separately.

The system can therefore contain a durable Agent Session fact whose immediate orchestration callback failed. Startup and later reconciliation are the recovery mechanisms; notification success is not the durable completion boundary.

### Startup recovery is deliberately staged

`active_app.rs` runs `AgentSessionApplication::reconcile_startup` before the bootstrap and Sprint transition services are constructed and attached. Terminal notifications emitted during that first pass cannot reach those not-yet-attached services.

The later startup sequence compensates explicitly:

- bootstrap `reconcile_startup` loads durable snapshots, observes existing terminals, and reconciles each initiation;
- Sprint `reconcile_startup` observes existing terminals, reconciles each Sprint, then runs Handler, Implementer, review, retry, escalation, candidate, integration, dependency-wave, and settlement reconciliation.

Agent Session startup itself does not reattach to an active Codex process. An invocation left active across application restart is recovered from a known terminal delivery when available; otherwise it is durably marked interrupted. The orchestration services subsequently interpret that persisted lifecycle in their own context.

### Different notification kinds have different workflow meaning

Bootstrap responds only to terminal invocation notifications. Sprint execution also consumes persisted runtime events for Work Unit Handler invocations so provider activity can be projected from the durable event seam. Diagnostic notifications do not advance these paths.

For non-Handler workflow stages, terminal lifecycle is routed through correlation queries that decide whether the invocation belongs to reporting, review, retry, handback, escalation, or another managed continuation. The notification carries an invocation identity; durable orchestration tables supply its semantic role.

### Reconciliation is invoked from several kinds of edge

Progress is not owned by one scheduler. It can be entered from:

- persisted Agent Session notifications;
- application or MCP semantic operations such as completing bootstrap material or recording a pre-start outcome;
- startup reconciliation;
- explicit transition methods and repeated idempotent reconciliation inside services.

By contrast, the Tauri transition query commands are read-only snapshots. `load_orchestration_native_query`, `load_epic_bootstrap_transition_query`, and `load_sprint_runner_transition_query` do not themselves reconcile workflow state.

### Frontend Agent Session freshness and orchestration freshness differ

`src/infrastructure/agentSessions/tauriAgentSessionClient.ts` establishes one shared listener for `agent-session-update`. Agent Session controllers subscribe before loading and reload durable Session state for correlated updates. The Plan Builder separately refreshes its durable proposal source on any terminal Agent Session update.

`src/app/useOrchestrationLoad.ts`, however, loads orchestration state on mount and exposes a manual `refresh` function. In the baseline application, the only call to `orchestrationLoad.refresh()` follows Epic initiation confirmation. No orchestration overview subscription, periodic refresh, or user refresh action was found.

The same loader and refresh behavior remains on the sampled Product Decisions (`82d9351`) and final-settlement (`8965191`) sibling lines. Git history shows the loader was introduced in `e54430e` on 2026-07-17 and has not changed, while backend notification-driven workflow depth expanded materially afterward.

An embedded Agent Session can therefore update while the surrounding orchestration read model remains the snapshot taken when the application or client instance mounted.

### The frontend composes three non-atomic snapshots

`src/infrastructure/orchestrations/tauriOrchestrationNativeQuery.ts::createNativeQueryOrchestrationClient` loads, in order:

1. the large native orchestration query;
2. the bootstrap transition query;
3. the Sprint transition query.

The three commands use different service/repository connections and are not bound by a shared read transaction or snapshot token. Only the native query carries `generatedAt`; the transition contracts do not provide a common version. Workflow reconciliation may occur between these sequential calls.

The product read model is consequently a useful composition of durable snapshots, but not evidence that every projected field was observed at one atomic instant.

### Tauri event coverage is narrow

The productive backend exposes two named Tauri events: `agent-session-update` and the Epic initiation confirmation event. There is no general orchestration-state-changed event. Most orchestration communication is command/query based even though backend progression is notification driven.

## Unexpected connections

- Agent Session transport failure handling also functions as orchestration callback-failure evidence.
- The lifecycle junction sits in the Tauri composition root, but most work it triggers is domain persistence, process launch, Git manipulation, and MCP lifecycle—not Tauri behavior.
- Startup correctness relies on the ordering of three separate reconciliation systems, including the deliberate period when orchestration listeners are unattached.
- A frontend can receive a fresh transcript event after the backend has already advanced several hidden workflow facts, while its surrounding product overview remains stale.
- The native query is called a snapshot boundary in Rust, but the frontend-facing orchestration view is a composition of three independently timed snapshot boundaries.

## Questions opened by the pass

- Which product states are intended to refresh live, on navigation, on explicit user action, or only after application reopen?
- Is synchronous lifecycle dispatch an intentional transaction-like progression boundary, or accumulated coupling that should eventually become a durable work queue?
- Should one callback failure cause runtime delivery-failure diagnostics when the underlying provider event was persisted successfully?
- Is the current startup order part of the supported recovery protocol or merely the order imposed by composition dependencies?
- Does the composed frontend read model need a shared snapshot/version boundary, or is bounded temporal skew acceptable for the intended experience?
- Should the lifecycle junction remain an Agent Session notifier, become an explicit orchestration reconciler, or remain conceptually hidden behind application composition?

These are not disposition proposals. The pass establishes that persistence, progression, notification, and presentation freshness are related but distinct mechanisms.
