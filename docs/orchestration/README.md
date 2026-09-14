# Epic and Sprint orchestration

Epic/Sprint is the retained system for application-managed, hierarchical agent work. Its native persistence, planning, execution, integration and settlement code remains mounted in the Epics surface. Later user direction gives this feature lower priority than Workflow, Harness and database work; its historical Sprint forecasts are not the current project roadmap.

This guide describes source at `60c3798`; historical research used `e2bfc6cb584a9ce7ada0d762f768c605a33b4160`. Historical execution results and remaining proof limits are recorded in [Validation evidence](../validation-evidence.md). For the primary configurable execution model, see [Workflows](../workflows.md).

## Scope, plans and execution identity

An **Epic** supplies overall direction across Sprints. A **Sprint** has its own intended movement, objectives and concerns. Before explicit start and repository reevaluation, its plan is a forecast of concerns. Concrete work and dependencies belong to a selected, recorded plan revision after that assessment.

A logical Sprint Plan, a plan revision, a Work Slice planning point and a Work Unit have different identities. The planning point locates a planning episode in time; the Work Unit carries the bounded responsibility selected for execution. An execution attempt stays attached to its specified scope and revision. Later planning does not silently retarget an existing attempt.

The historical statement that there is “no durable Work Slice entity” expressed a modeling choice about planned responsibility. Current persistence does retain Work Slice planning episodes, graph completion and settlement facts. It should not be read as an absence of durable slice records.

The UI composes sourced product facts and explicit relationships. It distinguishes available, pending, unavailable and unsupported information. Transcript text, a conversation title, a Harness name or a position in the flow map cannot establish that work started, was accepted or settled. Sprint objectives and concerns also remain distinct from global Epic goals. Concern links are explicit and many-to-many. Accepted or deferred concern decisions take precedence over the presentation state derived from linked Work Units; a completed unit does not silently resolve every concern it touches.

## Roles and application authority

The execution model has five roles:

| Role                  | Responsibility                                                                                                                                                |
| --------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Epic Runner           | Assess Epic direction, request the next Sprint Runner, authorize the selected Sprint's start through a separate action, and reassess returned Sprint results. |
| Sprint Runner         | Assess the selected Sprint before start, reevaluate the repository after start, request planning, and reassess returned work or continuation needs.           |
| Work Slice Planner    | Inspect current planning context and ready work, propose or refine a bounded work graph, and return planning material to the application.                     |
| Work Unit Handler     | Direct the exact Work Unit attempt, request its Implementer and independently judge returned work.                                                            |
| Work Unit Implementer | Implement the supplied specification in the attempt's isolated workspace and report its result through the supplied continuation.                             |

Plan Builder and Bootstrap are two additional pre-execution contexts. This accounts for the seven files in [the product skill catalogue](../../product/skills/) without adding two more execution roles.

The application creates and associates Sessions, supplies trusted identities, selects phase-specific Harnesses and authorizes intent-specific operations. For example, the Sprint Runner requests a Work Slice Planner; it does not create Work Units and Handler Sessions directly. The Planner returns material for application validation and materialization. The Handler reviews the Implementer; the model does not add a separate reviewer role for every level.

Tool exposure and authorization are separate checks. A tool present in a Harness does not authorize arbitrary identities, transitions or child Sessions. The role catalogue includes different phase profiles, so a role's initial activation and later continuation need not expose the same tools. Shared profile resolution and delivered configuration are explained in [Execution configuration](../execution-configuration.md).

The local orchestration MCP server binds an ephemeral loopback address for its invocation. A child-scoped bearer credential, explicit Host/Origin/authentication checks and server-side scope validation keep tool access tied to the supplied authority. Loopback reachability alone grants no permission. This boundary is implemented in `mcp.rs` and comes from decision record 0002 below.

Sprint-to-next-Work-Slice-Planner progression and Epic-to-next-Sprint-Runner progression have separate policies. For each, configured policy, current eligibility, a launch request and observed execution are distinct facts. The historical contract allowed ordinary eligible progression without creating a new user Decision for each step. This does not make the currently unsupported generic policy controllers available or bypass explicit Sprint start authorization.

This boundary came from the user's distinction between a fixed product workflow and the ad hoc Codex procedures once used to develop it. The product scaffolds its connections through MCP operations and application hooks. Retired development callback protocols are not part of this runtime contract.

## From a proposal to executable work

1. **Plan Builder discussion and proposal.** Discussion can remain ordinary conversation. When asked to structure or revise a plan, the agent submits proposal material through `submit_epic_plan_proposal`. Persisting that material does not initiate an Epic or make the proposal accepted.
2. **Explicit initiation confirmation.** `request_epic_initiation` requests the application's confirmation flow. Confirmation is a separate authority from proposal submission. The shared controller owns the confirmation interaction and resulting transition.
3. **Bootstrap material.** The read-only Bootstrap agent returns material through `complete_epic_bootstrap`; the application verifies the prepared bytes and owns file writes. Material, invocation lifecycle and acceptance must correlate to the same attempt. A terminal response alone is insufficient.
4. **Sprint assessment and start.** The Epic Runner requests a Sprint Runner. Pre-start assessment, explicit start authorization and started repository reevaluation are separate recorded transitions. A launch request or accepted launch does not itself prove provider activity or completed assessment.
5. **Planning and materialization.** The Sprint Runner requests a Work Slice Planner after the required start, reevaluation and lifecycle facts. The application validates returned planning material and records the selected revision and executable graph before initiating eligible Work Units.
6. **Handler and Implementer execution.** The Handler requests the exact Implementer attempt with application-supplied scope, baseline and specification. The application records execution and reporting evidence before Handler judgment. A retry, captured diff or agent claim of success is not an accepted integration candidate.

User-authored discussion retains User provenance. Fixed Plan/Rebuild prompts are Application material; launch-only button context is not rewritten as original user text. Persisted invocation correlation supports later inspection. An authenticated application effect can be recorded before terminal invocation observation, so these timestamps and facts remain separate.

## Acceptance, integration and settlement

Handler judgment and an accepted candidate precede application-owned Git integration. The candidate identifies the exact attempt, commit/tree and baseline. Integration pins the target authority, reserves an intent, creates the integration object, advances the expected reference and reconciles runtime and database state. A changed target or incompatible result requires recorded attention; it cannot be silently treated as successful integration.

The accepted integration policy creates a correlated single-parent integration commit and preserves its evidence across reconciliation. Work Unit settlement and contributions to individual dependency edges are separate durable records. Dependent work becomes eligible from those contributions, rather than from a provider's terminal message or a graph node's visual position.

Higher-level completion remains layered:

| Recorded fact                                                   | What still remains distinct                                                        |
| --------------------------------------------------------------- | ---------------------------------------------------------------------------------- |
| Work Unit integrated and settled                                | Contributions to dependents and completion of the selected graph.                  |
| Work Slice graph drained                                        | Planning-point execution settlement and Sprint continuation assessment.            |
| Sprint continuation decision or locally persisted upward result | Delivery, receiver launch acceptance, provider activation and Epic reassessment.   |
| Epic terminal readiness                                         | Settlement request, authorization, evidence and the durable settlement itself.     |
| Product settlement                                              | Human acceptance of the product, a release, or publication to a remote repository. |

Sprint continuation can remain `continuing`, require `attention`, or be `settled`. Epic settlement can remain unresolved with a recorded reason and resumption fact. Local exhaustion or dependency waiting is not automatically a blocked Epic.

These records support specific reconciliation paths. They are not a general promise of exactly-once distributed execution or complete restart recovery for every possible interruption.

## Current capability boundaries

The product composition connects native Plan Builder, initiation, Bootstrap, Sprint transition and orchestration query clients. It still injects unsupported generic artifact-access and Sprint/Epic automatic-continuation-policy controllers. Those particular unavailable controls do not erase the implemented continuation and settlement runtime. Technical artifacts are not automatically user-facing Documents. The generic orchestration artifact contract separately represents resolving a reference, opening its content and copying a path; success at one does not prove the others. This distinction does not grant the scoped File Review viewer arbitrary path access.

Scoped contextual File Review has client, command and loader infrastructure, but the active application installs an unavailable source service. Fresh originating Sprint-context review requests can therefore return `not_ready`. Recorded review demonstrations and this infrastructure do not establish an available production source; see [File Review](../file-review.md).

Recorded compositions remain useful examples of views and relationships. They do not prove that a fresh native Epic completes the whole flow. Historical deterministic tests, isolated live invocations, integration checks and human review are kept separately in [Validation evidence](../validation-evidence.md).

## Source map

| Responsibility                          | Implementation                                                                                                                                                                                                                                                                             |
| --------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Product wiring and mounted UI           | [productApplicationComposition.ts](../../src/bootstrap/productApplicationComposition.ts), [App.tsx](../../src/app/App.tsx), [OrchestrationSection.tsx](../../src/features/orchestrations/OrchestrationSection.tsx)                                                                         |
| Sourced product and Sprint views        | [productReadModels.ts](../../src/application/orchestrations/productReadModels.ts), [productReadModelComposer.ts](../../src/application/orchestrations/productReadModelComposer.ts), [sprintReadModelAssembly.ts](../../src/application/orchestrations/sprintReadModelAssembly.ts)          |
| Proposal and initiation authority       | [application.rs](../../src-tauri/src/orchestration/application.rs), [confirmation.rs](../../src-tauri/src/orchestration/confirmation.rs), [bootstrap_transition.rs](../../src-tauri/src/orchestration/bootstrap_transition.rs)                                                             |
| Role phases and tool exposure           | [conversation_harness_catalog.json](../../src-tauri/src/orchestration/conversation_harness_catalog.json), [mcp.rs](../../src-tauri/src/orchestration/mcp.rs), [transport.rs](../../src-tauri/src/orchestration/transport.rs)                                                               |
| Sprint planning and Work Unit execution | [sprint_runner_transition.rs](../../src-tauri/src/orchestration/sprint_runner_transition.rs), [work_unit_execution_harness.rs](../../src-tauri/src/orchestration/work_unit_execution_harness.rs)                                                                                           |
| Candidate, integration and dependencies | [accepted_candidate_authority.rs](../../src-tauri/src/orchestration/accepted_candidate_authority.rs), [accepted_integration.rs](../../src-tauri/src/orchestration/accepted_integration.rs), [work_unit_dependency_wave.rs](../../src-tauri/src/orchestration/work_unit_dependency_wave.rs) |
| Higher settlement                       | [sprint_continuation_settlement.rs](../../src-tauri/src/orchestration/sprint_continuation_settlement.rs), [epic_settlement.rs](../../src-tauri/src/orchestration/epic_settlement.rs)                                                                                                       |

Storage ownership and current schema belong to [Active database](../architecture/active-database.md), not the superseded schema numbers in early orchestration plans.

## Decision history and provenance

The original decision records are retrievable at commit `e2bfc6cb584a9ce7ada0d762f768c605a33b4160` under `docs/orchestration/decisions/`. Exact source paths below are historical Git references, not current file links.

- **Unified Rust-owned state and trusted effects:** `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:docs/orchestration/decisions/0001-durable-state-and-native-query.md`, `0002-mcp-transport-and-access.md` and `0003-managed-codex-invocation-and-proof.md` in the same directory. Early schema and provider constraints are checkpoint-specific. The routed correction in task `019f66b9-9aab-7323-8065-4b82bcbd2c2e`, raw JSONL lines 9 and 175, explicitly separates effect-time provenance from terminal observation and rejects a separate orchestration database.
- **Proposal and initiation separation:** `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:docs/orchestration/decisions/0004-plan-builder-tools-and-contract-evaluation.md` and `0005-sprint-5-initiation-scope-revision.md` document earlier one-tool and narrower initiation checkpoints. These precede the two-tool contract in `0006-sprint-6-plan-builder-harness-and-confirmation.md`.
- **Application-owned Bootstrap writes and confirmation context:** `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:docs/orchestration/decisions/0007-sprint-6-post-confirmation-bootstrap-transition.md` supersedes the workspace-writing Bootstrap proposal in 0006. Its original stop before Epic Runner launch was later exceeded. Record `0008-sprint-6-confirmation-context-and-transition-ui.md` and routed task `019f6bd0-7db6-7931-ab21-66a2e3c70232`, raw line 174, preserve the corrected User/Application provenance distinction.
- **Five roles and the Planner boundary:** direct user messages in **Review agent session view merge**, task `019f48bb-85b0-7451-bf2c-5483a36a18ff`, turn `019fc0e5-df4a-7711-b986-95eb7923142f` item 1245 and turn `019fc0f8-ec18-7bf0-8ae2-30d54641b9cf` item 1248. These also reject speculative recovery machinery as an assumed project requirement.
- **Later direction:** direct user message in task `01a0395c-f32c-78c0-8da1-521197812474`, raw line 9802, describes Epic/Sprint as somewhat deprecated. This changes priority; current source still retains the feature.

Raw line references identify messages in the local producing task's JSONL history. They support the decisions summarized here; reading those conversations is not necessary to use this guide.
