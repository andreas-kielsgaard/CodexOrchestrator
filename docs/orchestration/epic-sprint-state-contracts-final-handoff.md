# Epic and Sprint state contracts final handoff

Status: current contract authority, reconciled during Sprint 2 to `terminology.md`. The earlier Gate
G1 and discovery records remain historical provenance; their Epoch, Orchestration-instance, Work
Slice, and Planner Episode terms are superseded for active product use.

## Objective and evidence baseline

These provider-neutral contracts keep durable facts, Agent Control, artifact access, and application
read projections distinct. They do not implement persistence, product adapters, transitions, prompt
delivery, provider launch, or UI redesign.

Sprint 1 established product-neutral reads, canonical composition, same-tree product and recorded
adapters, Agent Session integration, Agent Control and artifact seams, truthful unsupported product
boot, separate policy-update and continuation-request controllers, exact command/result correlation,
and no fabricated task identity.

## Active hierarchy and identities

`Orchestration capability -> Epic -> Sprint -> Sprint Plan -> Sprint Plan Revision -> Work Unit`
is the active planning chain. A Work Unit is the sole durable planned responsibility and acceptance
unit; there is no durable Work Slice.

- An Epic is one managed endeavor and owns Sprints.
- A Sprint is one bounded implementation period and owns one logical Sprint Plan.
- A Sprint Plan keeps a stable identity across ordered Sprint Plan Revisions.
- A revision defines future Work Unit membership, dependencies, gates, and scope.
- A Work Unit execution fixes one revision-specific scope; attempts cannot retarget it.
- Superseded revisions retain requested and observed history. Unstarted superseded future work has
  no execution.
- Sprint Planner Activity is evidence about planning work, not Sprint Plan identity.

## Agent Session relationships and roles

Agent Sessions and Agent Session References remain provider-neutral. Provider thread ids,
credentials, tokens, runtime fields, and transcript projections are not product identity.

Associations include Epic Runner, Epic Plan Builder, Sprint Runner, Sprint Planner, Work Unit
planner, worker, and reviewer, with an extension form for later participant types. Epic Runner is
the entity-associated role; Orchestrator is reserved for a capability-wide coordinator.

## Agent Control and continuation authority

Agent Control commands identify their recipient Agent Session and carry authority inputs,
idempotency scope, precondition evidence, and prompt provenance. Only an exact correlated result and
its resulting Orchestration Event can prove an observed effect.

Epic-level and Sprint-level continuation have distinct policies, eligibility evaluations, requests,
and observations:

- Sprint-level continuation targets the next ready Work Unit planner.
- Epic-level continuation targets the next Sprint Planner.

Policy updates, eligibility, continuation requests, command results, and observed initiation never
collapse into one fact. Auto-flow state alone proves none of them.

The intended product path remains `Agent Control -> Agent Session -> application MCP handling ->
Orchestration Event -> UI read model`. Agent prose is never authoritative state.

## Artifacts, Documents, and reads

Internal artifacts are technical lineage/storage references. A user-facing Document is an explicit
inspectable reference. Resolution, system opening, and copy-path are separate focused operations;
raw paths occur only in a successful copy-path result.

Product read models are provider-neutral and independent from recorded compatibility shapes. The
logical Sprint Plan is not reconstructed from Sprint Planner Activities. The event root remains the
composition authority, and the product composer remains the one canonical product composition path.

| Module group                                           | Responsibility                                                                       |
| ------------------------------------------------------ | ------------------------------------------------------------------------------------ |
| `orchestrationEvents*`                                 | Capability-wide Orchestration Events, identities, and referential validation.        |
| `agentControl*`                                        | Agent Control commands, policies, eligibility, provenance, idempotency, and results. |
| `artifactAccess*`                                      | Artifact, Document, access result, and future port contracts.                        |
| `productReadModels.ts` / `productReadModelComposer.ts` | Canonical product Epic/Sprint reads.                                                 |
| `sprintReadModels.ts`                                  | Provider-neutral Sprint application reads.                                           |
| `sprintControlSurfaceCompatibility.ts`                 | Provisional recorded/discovery compatibility input.                                  |
| `sprintControlSurfaceDecoder.ts`                       | Compatibility decoding and referential validation.                                   |
| `sprintDerivedState.ts`                                | Pure Work Unit and concern presentation derivation.                                  |
| `sprintRelationshipGraph.ts`                           | Sprint Plan, revision, activity, Work Unit, gate, and dependency projection.         |
| `sprintReadModelAssembly.ts`                           | Product and recorded read-model assembly.                                            |
| `sprintControlSurface.ts`                              | Public projection facade.                                                            |
| `recordedPlanWorkflow.ts`                              | Disposable fixture geometry, never a product execution contract.                     |

## Clean-break and truthfulness boundary

No production orchestration data exists. Sprint 2 requires no migration, compatibility alias, or
dual schema. Recorded fixtures are disposable and may be rebuilt. Historical documents retain old
terms only where the old naming is part of the provenance, and each such record is marked
superseded.

No live provider call, persistence, MCP/event persistence, prompt delivery, process launch,
transition, continuation execution, filesystem opening, clipboard action, migration, or native
effect is proven by these contracts or by Sprint 2.

## Validation boundary

Sprint 2 validation must combine active-document semantic scans, explicit classification of every
historical legacy-term hit, stale-link and renamed-path scans, scoped formatting, the full frontend
suite, lint, typecheck/build, and whitespace checks. These checks can prove terminology and static
composition coherence; they cannot prove any deferred runtime behavior.

## Deferred and non-goals

- persistence models, database schemas, migrations, serialization, and recovery;
- provider adapters, prompt delivery, Agent Session launch, and supervised processes;
- transition, handoff, retry, or automatic-continuation execution;
- filesystem/path resolution, system opening, clipboard behavior, and Document UI redesign;
- Plan Builder implementation and later Sprint work;
- Rust changes or legacy task/run extraction, reuse, or expansion;
- external skill renaming.
