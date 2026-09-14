# Project evolution

The project began as a custom UI and workflows above Codex, with visible progress and context. It evolved from a Task dashboard into durable Agent Sessions, a fixed Epic/Sprint execution model, and configurable Workflows and Harnesses. The documents that accompanied these stages mixed product design, temporary development coordination and test evidence. This account preserves the transitions and useful lessons; the maintained subject guides describe current behavior.

Research source reference: `e2bfc6cb584a9ce7ada0d762f768c605a33b4160`, reconciled with `60c3798` during the September 14, 2026 documentation consolidation. This is a source checkpoint, not a new release or complete acceptance run.

## From the first dashboard to native Sessions

The July 1 request distinguished the product from the worker process used to build it. The user wanted bounded development steps, reviewed results and an overview of visible context: user messages, assistant messages and documents. It was not a requirement to expose hidden model reasoning. The early roadmap, active-task map, worker logs and handoffs existed to coordinate that development.

The first dashboard separated execution state from attention requiring the user. Its Task/TaskRun model accumulated repository registration, Git observation, persistence and captured provider execution. Rust/Tauri progressively replaced the earlier Node-oriented execution boundary; Windows Rust/MSVC blockers were resolved during July. Their old failure notes are not current setup requirements.

The current [Agent Session](agent-session/README.md) is durable interaction context with its own invocation and runtime evidence. Rich Codex interaction uses app-server. The old Task implementation and commands were removed by the separately scoped cleanup at `ac18781`. The original Task roadmap and `codex exec` limitations do not describe the current primary interface. The [retirement record](architecture/legacy-task-retirement-plan.md) preserves its source and verification boundaries.

The early implementation work left useful ownership distinctions. Repository observations did not invent a default branch or equate unknown dirtiness with clean. Application intent survived refreshes; explicit clearing differed from omitted values. Provider output, terminal detection, persistence failures and validation outcomes were recorded separately. Optional post-run capture could fail after successful Codex execution; that failure did not rewrite the provider result. These ideas remain relevant in [Architecture](architecture.md), [Agent Sessions](agent-session/README.md) and [Worktree Review](worktree-review.md), although their original schemas and adapters have changed.

**Original sources:** **Plan Codex UI workflows**, task `019f1f79-0e23-7e13-9518-d31dfe843dc9`, user turn `019f1f7b-1847-7a52-9ca5-89af57f7268b` item 1 and turn `019f1f84-c3d7-7b10-aa33-34b91646ee9d` items 12, 19 and 22. The initial implementation trail remains at `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:docs/task-logs/` and `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:docs/handoffs/`. Worker logs 003–010 cover Git semantics, 011–025 persistence, 026–039 captured execution, and 040–051 the native first loop.

## Fixed orchestration and configurable workflows

Recorded Orchestration/Epoch views helped discover a more precise Epic/Sprint model. Plans, revisions, planning points, Work Units and execution attempts acquired separate identities. Native state, trusted operations and read-model composition then replaced the assumption that recorded views alone represented implemented execution.

The user made the product boundary explicit: the application scaffolds role connections and mediates intent-specific actions. The five execution roles include a Work Slice Planner between Sprint Runner and Work Unit execution. Product skills belong to their own catalogue; development-agent callback procedures do not become product behavior. The current implementation includes accepted integration and higher settlement, beyond several historical forecasts that stopped at earlier boundaries. [Epic and Sprint orchestration](orchestration/README.md) explains these contracts and their supersession together.

Later work centers on [Workflows](workflows.md), reusable [execution configuration](execution-configuration.md), Sessions and database ownership. The user subsequently described Epic/Sprint as somewhat deprecated. That direction lowers its priority while its code remains mounted; neither an old “future Sprint” list nor the existence of runtime code establishes a current commitment to expand it.

Recorded and productive compositions continue to have different evidentiary roles. A recorded UI can establish how a view reads and navigates. Native persistence establishes an implemented storage boundary. An isolated provider invocation can establish a particular integration. None alone proves whole-flow usability, publication or human acceptance. [Validation evidence](validation-evidence.md) preserves decisive checkpoints with those limits rather than treating the latest appended report as universal completion.

**Original sources:** direct user messages in **Review agent session view merge**, task `019f48bb-85b0-7451-bf2c-5483a36a18ff`, turn `019fc0e5-df4a-7711-b986-95eb7923142f` item 1245 and turn `019fc0f8-ec18-7bf0-8ae2-30d54641b9cf` item 1248. The later priority statement is task `01a0395c-f32c-78c0-8da1-521197812474`, raw JSONL line 9802. Superseded forecasts remain retrievable at `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:docs/orchestration/future-sprint-trajectory.md`.

## What the retired development procedures taught

The original worker process accumulated 50 task logs and 11 handoffs. Requiring each successor to reload every earlier packet increased intake cost, while routing small work through more roles sometimes obscured the actual task. Later skill catalogues added Overall Plan, Slice and Step roles, review/demo helpers and maintenance reports. These were development procedures, not the application's five-role product model.

The August corrections retired that ad hoc framework. The user said it obstructed normal work, requested deletion of the role skills and named global helpers, then retained the standalone slice-planning format while removing task-handoff prescriptions. Copying the old callbacks into a new guide would recreate the very process that was removed.

The remaining useful lessons are about the work rather than a mandatory protocol:

- A plan is easier to change when its concerns, concrete file ownership and expected result are visible. Historical decomposition formats do not mandate more agents or fixed role chains.
- A semantic instruction belongs to the reader and catalogue that can act on it. Product role guidance and general development guidance have different owners; an incident report is not itself a new instruction.
- Requested action, message delivery, receiver activation, durable effect and accepted outcome are different facts. The two external orchestration-monitoring reports illustrated this in development-tool experiments; they did not prove that this application's runtime performed those actions.
- Validation is useful when it names the change and evidence it covers. Repeated command lists, fixture success and old passing counts do not establish current native behavior. Visible user review also concerns the actual flow and transitions, rather than a diff alone.
- Temporary repositories, databases and Cargo outputs accumulated through repeated validation. The historical maintenance investigation could identify that cause without attributing every directory to one invocation. Ownership and reproducibility matter; old size estimates are not a present inventory or a reason to delete current material.

The two remaining local skills and seven product skills retain their own explicit scope. This history adds no operating instructions to them.

**Original sources:** **Orchestration Skill Maintenance**, task `019f6b37-d80d-7e61-8cbd-7e618cefc6ce`, direct user messages at raw lines 149, 191, 251, 287, 403, 430 and 475. Commit `cce270d0d0fbbf2cc0ae6b9b49a34de7831ac886` records removal of the ad hoc workflow skills. The report collection and shared concepts remain in Git at `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:.agents/skills/orchestration-skill-maintainer/reports/` and `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:.agents/skills/_shared-skill-concepts/`. The temporary-artifact investigation is `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:.agents/skills/orchestration-skill-maintainer/reports/execute-plan-step/2026-08-07-temporary-artifact-lifecycle.md`.

## The saved-checkout reconciliation record

The August 8 manifest compared 182 historical paths: 67 dirty tracked paths and 115 untracked paths. It classified 120 as exact integration, 38 as semantic integration, 16 as superseded, zero as excluded duplicates and eight as externally retained. These categories distinguish exact state, including expected absences; preserved meaning; and superseded guidance; line-ending differences alone were not semantic differences.

The comparison named saved checkout `b86a8ac8f3e7483214b13e75b47397ca4df35074`, PS-2 `5d8e8e069832ecf7b2f72544d857322ecc8d80d4` and PS-1 `b7f783ab51f530f32fa1efaabab01b7896f3ea7f`. Its complete per-path mapping is preserved at `ca8f6729ff6fc07d2c2853b1ae94b28098d64e47:.agents/skills/orchestration-skill-maintainer/reports/retire-plan-slice/2026-08-08-saved-checkout-reconciliation.md`, the commit correcting its line-ending classifications.

The eight external entries were demo, watchdog, human-loop, review and feedback maintenance reports outside that comparison's integration scope. All eight were present in the report collection reviewed for this consolidation. “External” was a historical ownership classification, not a current missing-file finding. The manifest also separated the seven product skills from an eighth proposed feedback-routing skill that was superseded.

This account preserves the manifest's conclusions and immutable mapping. It does not repeat the historical per-path reconciliation against today's tree or perform a new worktree audit.

## Ideas that remain proposals

Some historical ideas are useful context without being accepted requirements:

| Proposal and date                                                         | Retained question or intent                                                                                                                                                                                        |
| ------------------------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| Browser demonstration and later app-owned video, July–August 2026         | Make a declared scenario and observed flow easy to review, with recordings tied to the demonstrated build. A recording would still be evidence for that flow, not automatic acceptance.                            |
| Broader structural review after UI discovery, August 2026                 | Revisit responsibility boundaries after an interaction stabilizes. The notes deferred automated size thresholds and blanket enforcement pending more experience.                                                   |
| More parallel orchestration and architectural coherence, July–August 2026 | Improve visibility of planned versus actual work, attention and human control. This did not adopt another coordination framework or an expansion roadmap.                                                          |
| In-app annotations, August 2026                                           | Anchor feedback to a view, element, state or frame. Structured review feedback would remain distinct from authoritative product events.                                                                            |
| Automatic Product Decision compilation and application, August 2026       | Explore event triggers, reconciliation, applicability and compliance. Current explicit acceptance and `not_applied` versions resolve only part of that exploration; see [Product Decisions](product-decisions.md). |

Harness inspection, scoped File Review infrastructure and Worktree Review subsequently gained implementations, with capability-specific limits. They are described in their current guides rather than retained wholesale as future ideas. Navigation, remote-development and OTP plans have their own integration status in the [documentation index](README.md); this historical account does not promote them to main behavior.

**Historical sources:** `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:docs/product-ideas.md`, `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:docs/orchestration/post-orchestration-review-notes.md`, and `e2bfc6cb584a9ce7ada0d762f768c605a33b4160:docs/orchestration/epic-product-decisions-exploration.md`. These are dated proposals, not a newly adopted backlog.

## Reading the historical evidence

An immutable `commit:path` reference identifies the preserved Git content even when the old working-tree document has been consolidated. Task references identify original messages or routed corrections; routed reports remain attributed to their sender. The meaningful conclusion is explained here so local transcript access is optional.

The consolidation reviewed complete artifacts and selected consequential producing messages. It did not reconstruct every worker conversation, rerun every historical test or repeat every screenshot review. In particular, the Product Decision audit closeout remains unavailable, while the recovered Sprint 5 G3 attachment does resolve a server-confirmed save for that exact sample. Its broader-flow limits are preserved in [Validation evidence](validation-evidence.md). These boundaries prevent a useful history from becoming an invented acceptance record.
