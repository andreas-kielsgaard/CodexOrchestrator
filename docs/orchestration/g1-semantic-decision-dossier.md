# Gate G1 semantic decision dossier: Orchestration and Epoch state contracts

> Historical provenance: this accepted Gate G1 record predates Sprint 2 and intentionally preserves
> the vocabulary in which its decisions were made. Its Epoch, Orchestration-instance, Work Slice,
> Planner Episode, and Orchestrator terms are superseded for active product use by `terminology.md`.
> Retain it while Gate G1 provenance is needed; remove it only in an authorized archive-retirement pass.

Status: accepted user-supplied Gate G1 semantics and the basis for WU-OESC2 through WU-OESC5.
This record does not authorize implementation, persistence, delivery, execution, document opening,
or continuation.

## Scope and evidence boundary

The accepted Epoch Control Surface supplies vocabulary and relationship evidence only. Its
`EpochPlannerOutputV1`, `EpochExecutionSnapshotV1`, `PlanWorkflowV1`, display labels, recorded
adapters, fixture ids, and local UI state are not product records or a provider protocol.

The user supplied the semantic decisions below. Repository evidence continues to establish the
recorded/non-executing boundary, provider-neutral Agent Session boundary, document-opening port,
and legacy task/run quarantine. No live provider, persistence, document-opening, launch, handoff,
integration, or continuation behavior was inspected or asserted.

## Accepted G1 semantics

- Product contracts and user-facing definitions use **Work Unit**. A work slice is, at most,
  orchestration-process shorthand for execution of a Work Unit; it is not a competing planned or
  responsibility entity in the durable product vocabulary.
- The plan is the **Epoch Plan**. The Epoch Planner estimates remaining work, currently completable
  Work Units, and currently completable portions of their tasks, then manages execution. It may
  revise the Epoch Plan after user feedback, blockers, or new insight.
- One Epoch Plan has many Work Unit planners/executions. A Work Unit planner has fixed ready scope
  once instantiated; it is not revised. Incomplete future Work Units from a superseded Epoch Plan
  revision that never started receive no planner. New-revision Work Units receive planners only
  when ready. Completed and observed history remains retained.
- Agent Sessions are provider-neutral. Provider credentials/tokens and provider thread ids remain
  adapter concerns, not Agent Session identity.
- Routine acceptance, integration, handoff, and continuation are policy-driven. They progress
  automatically when their eligibility/completion conditions are satisfied. Requested and observed
  facts remain separate.
- User feedback is required only when a relevant auto-continuation toggle is off at a continuation
  boundary, a flow explicitly requires feedback, or all pending development is blocked by technical
  challenges requiring human intervention.
- Epoch-level and Orchestration-level continuation are separate policies. Epoch Auto-flow can
  initiate the next ready Work Unit planner after prior executions and integration complete.
  Orchestration Auto-flow can hand an Epoch handoff to the next Epoch Planner after the Orchestrator
  processes it. Future stages such as architecture review are out of scope.
- An internal artifact is not automatically a user-facing Document. Paths are not shown directly;
  a user-facing reference may offer copy-path. User-review Documents include useful inspection
  material such as handoff notes or changed-file references. Exact future display content and
  layout remain deferred.

## Identity catalog

| Concept                         | Durable identity or fact                                                                                                                                                          | Distinct from / not durable by itself                                             |
| ------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------- |
| Orchestration                   | `orchestrationId`; owns Epochs, its Orchestration Plan context, and Orchestration-level continuation policy/history.                                                              | Dashboard position, movement label, local toggle state.                           |
| Epoch                           | `epochId`; belongs to an Orchestration and owns one Epoch Plan and its execution/history.                                                                                         | Tab, selected revision, workspace location.                                       |
| Epoch Plan                      | `epochPlanId`; the logical plan for one Epoch, stable across its revisions.                                                                                                       | A visible card or a planner/session id.                                           |
| Epoch Plan Revision             | `epochPlanRevisionId`; belongs to one Epoch Plan and declares that revision's future Work Unit scope, dependencies, and gates.                                                    | Selected/active display state.                                                    |
| Epoch Planner                   | A semantic role responsible for estimating, revising, and managing the Epoch Plan. Its observed activity/session may be separately referenced without becoming the Plan identity. | Epoch Plan, Epoch Plan Revision, or Agent Session identity.                       |
| Work Unit                       | `workUnitId`; the single planned responsibility and acceptance unit. A revision-specific scoped definition/membership links it to an Epoch Plan Revision.                         | A graph node, lane, fixture id, or a separately planned work slice.               |
| Work Unit planner/execution     | An observed execution record, if retained, belongs to one Work Unit and records its fixed ready scope once instantiated. Its attempts/returns remain history of that Work Unit.   | A revised Work Unit or a second planned responsibility.                           |
| Attempt                         | `attemptId`; belongs to one Work Unit execution, records request/launch/return observations, and identifies the fixed scope used.                                                 | Processing/idle display state.                                                    |
| Agent Session Reference         | `agentSessionRefId`; provider-neutral link to an independently identified Agent Session with a semantic role and association target.                                              | Provider thread id, credentials/token, transcript projection, availability label. |
| Review                          | `reviewId`; observed review of a Work Unit execution, attempt, Epoch Plan Revision, or Document, including result and rationale where recorded.                                   | Under-review display state.                                                       |
| Gate / feedback boundary        | `gateId` with revisioned criteria and scope; feedback requirement is part of the designed flow.                                                                                   | A map marker or inferred satisfaction.                                            |
| Policy / eligibility evaluation | Level-specific continuation policy and recorded eligibility/completion evaluation. Routine eligible progression does not require a user Decision.                                 | Toggle copy, `ready` label, or a requested transition.                            |
| User feedback                   | An explicit feedback record only at the three accepted boundaries: Auto-flow off, designed feedback flow, or all pending development technically blocked.                         | Routine acceptance, integration, handoff, or continuation.                        |
| Internal artifact               | `artifactId` and provenance/content/resolver facts where needed for technical storage and lineage.                                                                                | A user-facing Document, direct path display, or successful system opening.        |
| User-facing Document reference  | `documentRefId` when a user-useful item is exposed; links to suitable internal material without making every stored artifact a Document.                                          | Raw path or final display/layout.                                                 |
| Provenance                      | Immutable source, causal inputs, actor/session reference where applicable, and recorded time on a fact.                                                                           | A fixture's `recorded: true`, subtitle, or inferred causal link.                  |

## Relationship and cardinality map

```text
Orchestration 1 -- owns --> 1..* Epoch
Epoch 1 -- owns --> 1 Epoch Plan
Epoch Plan 1 -- has --> 1..* Epoch Plan Revision
Epoch Planner 1 -- may assess/revise --> 0..* Epoch Plan Revision
Epoch Plan Revision 1 -- assigns scoped definitions to --> 0..* Work Unit
Work Unit 1 -- has --> 0..* Work Unit execution records
Work Unit execution 1 -- has --> 0..* Attempt
Orchestration / Epoch / Epoch Planner / Work Unit execution 0..* -- links through --> 0..* Agent Session Reference --> 1 Agent Session
Gate 1 -- has --> 1..* criteria revisions; feedback records may resolve a feedback boundary
Internal artifact 0..* -- may support --> 0..* User-facing Document references
```

An Epoch Plan Revision can retain a Work Unit that has observed history, while an unstarted future
Work Unit from a superseded revision receives no planner/execution. Each Work Unit execution has
one fixed ready scope. This permits many Work Unit planners/executions for one Epoch Plan without
creating an independently planned Work Slice entity.

## Durable facts versus derived read models

| Area                             | Durable fact / recorded observation                                                                                               | Derived read model                                                         | Must not infer                                                                                                        |
| -------------------------------- | --------------------------------------------------------------------------------------------------------------------------------- | -------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------- |
| Epoch planning                   | Epoch Plan/Revision identity and lineage; scoped Work Unit membership, dependencies, gates, feedback, blockers, and provenance.   | Selected/current revision, graph, ready work, parallel lanes.              | A selected graph item or planned Work Unit has a planner or launch.                                                   |
| Work Unit execution              | Fixed ready scope, request, observed launch, attempts, returns, integration completion, and policy eligibility where each exists. | Processing/idle, attempt count, journey summary, ready status.             | Requested/planned means delivered, launched, returned, integrated, or accepted.                                       |
| Routine progression              | Eligibility/completion inputs and observed transition outcome.                                                                    | Automatic-flow status/copy.                                                | A policy toggle or eligibility result means the transition happened.                                                  |
| Feedback boundary                | Feedback requirement, received feedback, and resulting Epoch Plan revision where applicable.                                      | Waiting-for-feedback presentation.                                         | Routine flow needs user approval, or feedback changes the Orchestration Plan.                                         |
| Agent Sessions                   | Provider-neutral session/reference identity and semantic association.                                                             | Transcript, viewport, provider availability, composer draft.               | Provider thread/token is product identity or recorded transcript proves product execution.                            |
| Internal artifacts and Documents | Internal artifact provenance/resolver facts; optional user-facing Document reference and suitable source linkage.                 | Newest-first list, title, open availability, copy-path affordance, layout. | Every artifact is a Document, a row exposes a raw path, or a Document opened successfully.                            |
| Continuation                     | Separate Epoch/Orchestration policies, eligibility/completion evaluation, any required feedback, and observed initiation/handoff. | Auto-flow switches and readiness labels.                                   | Epoch policy starts a next Epoch, Orchestration policy starts a Work Unit planner, or any toggle proves a transition. |

## Scenario histories

| Scenario                       | Required distinct history                                                                                                                                                          | Prohibited shortcut                                                                                             |
| ------------------------------ | ---------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------- |
| Superseded Epoch Plan revision | Preserve completed/observed Work Unit history. Never-started incomplete future Work Units receive no planner; new-revision Work Units receive planners only when ready.            | Reassign a fixed Work Unit planner to a revised scope, or treat planned future work as started.                 |
| Ready Work Unit                | Epoch Planner determines fixed ready scope; an execution may then be requested and later observed launched.                                                                        | Ready/planned means a planner was instantiated or delivery occurred.                                            |
| Returned or idle execution     | Attempt return/process observation is retained. Review, routine completion, integration, and continuation remain separately recorded/evaluated.                                    | Idle/completed means accepted, integrated, handed off, or continued.                                            |
| Routine eligible flow          | Policy and completion/eligibility permit automatic acceptance, integration, handoff, or continuation; observed result is recorded separately.                                      | Automatic eligibility means observed progression.                                                               |
| Feedback boundary              | Pause for user feedback only when Auto-flow is off, a flow explicitly requires it, or all pending development is technically blocked; the Epoch Planner may revise the Epoch Plan. | Require a user authority Decision for ordinary progression or propagate feedback to the Orchestration Plan now. |
| Epoch continuation             | After prior Work Unit executions and integration complete, Epoch Auto-flow initiates the next ready Work Unit planner; if off, ask for feedback.                                   | Epoch toggle starts an Epoch Planner for the next Epoch.                                                        |
| Orchestration continuation     | The Epoch writes a handoff; the Orchestrator processes it; Orchestration Auto-flow hands off to the next Epoch Planner, or asks for feedback if off.                               | Epoch completion, handoff writing, or a toggle proves the next Epoch started.                                   |
| Documents and paths            | Internal storage may retain artifact/resolver facts; a useful Document reference may expose copy-path without showing raw paths.                                                   | Stored technical context, including the Orchestration Plan, is necessarily a user-review Document.              |

## Accepted choices and deferred matters

### Accepted by user Gate G1 input

- Work Unit is the product responsibility term; work slices are not a separate durable planned
  concept.
- The Epoch Plan, its revisions, and the Epoch Planner's role have the semantics above.
- Work Unit planner scope is fixed at instantiation; plan revision affects only future work that has
  not started.
- Agent Sessions are provider-neutral.
- Routine progression is policy-driven; feedback is limited to the three stated boundaries.
- Epoch and Orchestration continuation remain separate.
- Internal artifacts and user-facing Documents remain distinct; raw paths stay out of direct UI.

### Deferred, not a new semantic decision

- Exact durable field/schema and port designs.
- Exact user-facing Document content, layout, and copy-path interaction design.
- Any user-feedback back-propagation to the Orchestration Plan.
- Future extra flow stages, including architecture review.

## Rejected alternatives and supporting evidence

- **Separate planned Work Slice and Work Unit entities.** Rejected by user Gate G1 input. The
  accepted surface's lanes are recorded workflow presentation, not evidence for a second product
  responsibility identity.
- **Revise a running Work Unit planner's ready scope.** Rejected by user Gate G1 input. A revised
  Epoch Plan governs future work; observed/completed earlier history remains historical.
- **Use provider identifiers as Agent Session identity.** Rejected by the accepted provider-neutral
  Agent Session boundary and user Gate G1 input.
- **Require user Decisions for routine progression.** Rejected by user Gate G1 input. The current
  recorded continuation controls already establish that a toggle is neither eligibility nor
  execution proof; the future contract must add policy/observed separation, not fabricate approval.
- **Collapse requested and observed state.** Rejected by the accepted handoff, trajectory, and
  provisional decoder, which distinguish projection, actual launch, attempts, review, and
  acceptance. Automatic policy does not make requested work observed.
- **Treat every artifact or stored plan as a user Document.** Rejected by user Gate G1 input and the
  document-opening port, whose recorded adapter has no path and returns unsupported without opening
  a system program.
- **Reuse legacy task/run contracts.** Rejected by `src-tauri/AGENTS.md`; the legacy root handler is
  quarantined and fail-closed.

## Validation boundary

Deterministic tests can verify the contract decoders and recorded evidence fixtures/projections but
cannot prove persistence, controller integration, provider behavior, or live execution. Epoch-level
validation and the current next action are recorded in the final handoff.
