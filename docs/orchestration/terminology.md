# Orchestration product terminology

Status: current authority for active product-domain terminology.

| Term          | Current meaning                                                              |
| ------------- | ---------------------------------------------------------------------------- |
| Orchestration | The general product capability. It is not one managed endeavor.              |
| Epic          | One managed top-level endeavor within Orchestration.                         |
| Sprint        | One bounded implementation period within an Epic.                            |
| Work Unit     | The sole durable planned responsibility and acceptance unit within a Sprint. |

There is no durable Work Slice entity. Historical discovery material may use that label, but active
contracts, reads, controls, UI, fixtures, and tests use Work Unit.

Agent Session remains provider- and role-neutral. Agent Control remains the control concept.
Orchestration Event remains the capability-wide event category. Document and internal artifact
remain distinct.

The orchestration role vocabulary has exactly five names:

| Role                  | Responsibility                                                                           |
| --------------------- | ---------------------------------------------------------------------------------------- |
| Epic Runner           | Epic organization, Sprint-to-Sprint flow, and cross-Sprint integration.                  |
| Sprint Runner         | Sprint organization, start-time replanning, and implementation flow across the Sprint.   |
| Work Slice Planner    | Plans actually-ready work once at the recorded temporal planning point.                  |
| Work Unit Handler     | Creates the Implementer, reviews returns, requests corrections, and integrates the unit. |
| Work Unit Implementer | Performs the implementation for one Work Unit.                                           |

Work Slice Planner is a role, not a durable Work Slice entity. A Work Slice planning point is a
typed temporal planning record; current contracts do not imply recurring Planner reuse. Epic-level
continuation advances to the next Sprint Runner. Sprint-level continuation advances to the next
Work Slice Planner. Policy update, eligibility, continuation request, observed result, and recorded
Orchestration Event remain distinct facts.

Before Sprint start, the Sprint Runner plan may expose only a low-resolution forecast of concerns.
After an explicit start and recorded branch/repository reevaluation, it may expose concrete Work
Units, dependencies, parallel lanes, and the current Work Slice planning point. Missing authority is
unavailable, not inferred from UI state or Agent prose.

Sprint 2 is a clean terminology break. No production orchestration data exists, so no data migration,
compatibility alias, or dual schema is required. Recorded fixtures are disposable and may be rebuilt.
External skill names that use Epoch are outside this product-domain terminology authority.

## Record classification

| Record                                                 | Classification                                              |
| ------------------------------------------------------ | ----------------------------------------------------------- |
| `product-data-controller-integration-final-handoff.md` | Current accepted Sprint 1 handoff and Sprint 2 baseline.    |
| `epic-sprint-state-contracts-final-handoff.md`         | Current contract authority, reconciled to this terminology. |
| `capability-port-matrix.md`                            | Current capability/port boundary record.                    |
| `sprint-read-model-decomposition.md`                   | Current read-model decomposition record.                    |
| `product-read-model-composition.md`                    | Current product composition record.                         |
| `future-sprint-trajectory.md`                          | Current roadmap; later Sprints remain unlaunched.           |
| `post-orchestration-review-notes.md`                   | Current deferred review inputs.                             |
| `epoch-control-surface-discovery-final-handoff.md`     | Historical discovery provenance; superseded terminology.    |
| `g1-semantic-decision-dossier.md`                      | Historical Gate G1 provenance; superseded terminology.      |
| `next-epoch-overview.md`                               | Historical bounded input; resolved and superseded.          |
