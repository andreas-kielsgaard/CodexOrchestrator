# Epoch Control Surface Discovery final handoff

> Historical provenance: this accepted discovery predates Sprint 2 and intentionally preserves its
> original Epoch, Orchestration-instance, Plan, and Work Slice vocabulary. Those terms are not current
> product authority; use `terminology.md`. Retain this record while its acceptance history is needed,
> and remove it only through an explicitly authorized archive-retirement pass.

Status: complete. Parent re-review accepted WU-ECS3 task
`019f6195-3f0d-7e42-a433-6fccadba555d`, G3, and the Epoch completion audit. ECS-R4 is the final
active and accepted presentation revision. The next Epoch remains unlaunched pending separate
explicit authorization.

## Objective and completion frame

This Epoch discovered the application surface for understanding and supervising one started Epoch,
then expanded the accepted hierarchy through Plan and Work Slice detail. The implementation is a
recorded, non-executing evaluation surface. Parent re-review accepted the corrected handoff at G3
and confirmed the Epoch complete.

The accepted surface is mounted in the actual application and backed by deterministic recorded data.
Evidence lives primarily in `src/features/orchestrations/`,
`src/application/orchestrations/epochControlSurface.ts`,
`src/application/orchestrations/planWorkflow.ts`, and
`src/dev/orchestrationSection/disposableRecordedOrchestrationView.ts`, with focused coverage beside
those files. The user accepted WU-ECS2D with "Looks good," and the parent confirmed that integrated
evaluation also satisfies G2.

## Accepted information and interaction direction

- The hierarchy is Orchestration detail -> Epoch detail -> Plan detail -> Work Slice detail. Back
  navigation restores focus to the opener at each level.
- The Epoch context rail contains only the Epoch name and summary. The control bar owns the Flow,
  Concerns, and Documents tabs. The compact Epoch Agent Session occupies the main-column lower
  panel.
- Flow treats Work Units as the substantial nodes. Plans visually own their Work Units, and the
  whole Plan region is actionable. Orthogonal dependency lanes show sequential work, parallelism,
  convergence, review markers, and the conversation-produced plan-change event. The revision
  selector stays fixed while the map scrolls and can inspect retained ECS-R1 through ECS-R4 history.
- Plan detail presents fixed ready scope as an actor/conversation workflow: Planner start, parallel
  Work Slice initiators, workers, return/review/correction loops, repository integration or handoff,
  settled slices, Planner completion, and return to the Epoch.
- Work Slice detail places handler/initiator and worker Agent Sessions side by side, coordinates
  their expansion, stacks them narrowly, and reuses the shared Agent Session conversation
  components.
- Concerns use summary boxes, a maximized detail view, and links to affected Work Units. Their state
  is derived from linked Work Unit presentation states, with explicit accepted/deferred decisions
  taking precedence.
- Documents are projected newest first with planner/execution provenance and linkage. Opening is an
  injected application boundary; the recorded adapter reports unsupported and never resolves a path
  or opens a system program.
- Tabs use semantic tab roles and roving keyboard focus. Dialogs and nested detail views restore
  focus, interactive regions expose accessible names and states, desktop/narrow layouts remain
  contained, and reduced-motion styles remove nonessential transitions.

Automatic continuation remains two distinct projections. Orchestration Auto-flow describes whether
the Orchestrator may proceed between Epochs; Epoch Auto-flow describes continuation within one
Epoch. Both controls are local, non-persisted, and non-executing. Neither proves eligibility,
authorization, prompt delivery, or a transition.

## Review and revision history

G1 accepted the initial direction and produced ECS-R2. Later conversation feedback retained the
accepted foundation while producing ECS-R3 and ECS-R4. WU-ECS2C retains its rejected first visual
direction and correction. WU-ECS2E retains both returned attempts and the accepted second visual
result. WU-ECS2D retains its real task id `019f617e-0a11-7011-9d36-f18630f85dfd`, returned attempt,
parent review, user acceptance, and accepted state. G2 is recorded as accepted. ECS3 retains its own
real task id, returned attempt, correction review, parent acceptance, and accepted state. G3 is
recorded as accepted, and the Epoch completion audit passed. No next-Epoch task was launched.

## Contract classification

Carry forward these concepts into the state-contract Epoch:

- separate planned structure from observed execution facts;
- retain revision, Plan ownership, dependency, gate, attempt, review/correction, provenance, and
  Agent Session linkage in application-owned read projections;
- derive concern presentation from linked work plus explicit decisions;
- keep recorded and product adapters behind the same visual component tree;
- keep document opening and Agent Control behind replaceable application ports;
- continue using shared Agent Session transcript and viewport components.

The exact `EpochPlannerOutputV1`, `EpochExecutionSnapshotV1`, `PlanWorkflowV1`, projection, and
feature presentation shapes remain provisional. They are useful discovery inputs, not approved
persistence records, command schemas, transition models, or provider protocols. Layout coordinates,
CSS geometry, illustrative concerns and Work Units, recorded transcripts, presentation labels, and
the single-file development fixture are disposable presentation structures.

The recorded document opener, local Agent Session message behavior, continuation toggles, and
top-level view injection are intentionally replaceable. Fixture ids, display status unions, revision
selection state, derived concern state, local navigation state, and `recorded_theoretical` workflow
data must not become durable schema merely because the current UI consumes them.

## Truthfulness boundaries

The app currently imports the orchestration view from `src/dev/orchestrationSection` in
`src/main.tsx`. The feature and application orchestration directories do not invoke Tauri, use
persistence, launch Codex, or import the quarantined legacy task/run implementation. Recorded demo
ids are labelled as such; ECS2D and ECS3 use the actual delegated task ids supplied by their parent.
The snapshot records observed review bookkeeping but does not itself launch tasks or persist state.

No click in this surface means a task was launched, a document was opened, data was saved, a prompt
was delivered, or a transition occurred. `src-tauri/AGENTS.md` remains authoritative: later work
must not build on or add orchestration behavior to the legacy task/run code in `src-tauri/src/lib.rs`.

## Deferred product work

- durable Orchestration and Epoch state contracts;
- real application data clients and Agent Control controllers;
- artifact identity/path resolution and system-default opening;
- persistence, authorization, prompt delivery, and idempotent transitions;
- automatic-continuation eligibility and execution at both levels;
- extraction or retirement of legacy task/run capabilities after active replacements exist.

## Input to the state-contract Epoch

Bound the next Epoch to questions required by the accepted views:

1. Which identities, revisions, attempts, reviews, gate decisions, and provenance are durable facts,
   and which values are projections?
2. Which application read models supply Orchestration, Epoch, Plan, Work Slice, Concern, Document,
   and Agent Session linkage without leaking provider or persistence shapes?
3. Which commands need authorization, prompt provenance, idempotency, and explicit observed
   outcomes? How are requested, launched, returned, reviewed, accepted, and integrated states kept
   distinct?
4. How are explicit concern decisions combined with derived state without storing presentation
   labels as truth?
5. What artifact reference is sufficient for safe path resolution and system-default opening?
6. What separately makes Orchestration-level and Epoch-level automatic continuation eligible, and
   which actor owns each decision and execution?
7. Which legacy capabilities are still genuinely required and should be extracted behind new ports,
   and which can be retired?

Do not begin that Epoch by turning the current fixture or TypeScript interfaces into a product
schema. Start from these questions and the accepted UI requirements.

## Validation and repository state

ECS3 focused orchestration/projection validation passed: 5 files and 44 tests. The test set includes
revision projection, review history, G2/ECS3 bookkeeping, Flow interaction, Plan and Work Slice
navigation, Concern derivation, Document ordering/opening, focus behavior, accessibility semantics,
narrow containment, and non-executing continuation copy.

Completion-bookkeeping validation passed the affected projection and integrated-surface suites: 2
files and 36 tests.

The final full `npm test` rerun passed 65 files and 398 tests. An immediately preceding full run
passed 397 tests and timed out only the Plan/Work Slice navigation test at Vitest's 5-second default
under full-suite load; that test passed in the focused run and in the full rerun. This is retained as
a load-dependent timing sensitivity rather than reported as a product assertion failure.

`npm run format:check`, `npm run lint`, `npm run build`, and `git diff --check` passed. The build
emits the application and neutral Agent Session harness entries. No Rust validation was required
because ECS3 changed no Rust/backend file.

This handoff remains unstaged and uncommitted in the existing dirty `main` checkout alongside the
accepted Preparation, Orientation Discovery, and ECS work. ECS3 performed no branch creation,
staging, commit, merge, push, reset, checkout, Rust/backend change, persistence, live Codex call,
artifact opening, or next-Epoch implementation.
