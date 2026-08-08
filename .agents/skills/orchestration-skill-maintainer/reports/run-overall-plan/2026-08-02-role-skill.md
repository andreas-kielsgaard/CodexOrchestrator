# Overall Plan role skill

## Observation and theory

The three Overall Plan operations described their individual actions but did not give the owning conversation one standing contract for direction, retained state, child ownership, operation selection, or the normal Plan Slice profile. A Plan Slice cannot select its own creation settings, so Sol with high reasoning belongs with the Overall Plan reader that creates it. Repeating those boundaries inside each operation would invite drift.

## Revision

`run-overall-plan` now holds the conversation's persistent ownership, defaults Plan Slice creation to Sol with high reasoning, and routes establishment or revision, slice creation, and slice evaluation to the three operation skills. `start-plan-slice` applies and records that profile. Role-wide state and lifecycle wording was removed from those operations where appropriate.

## Evaluation

The role skill gives the reader enough context to choose an operation and configure its direct Plan Slice without introducing Slice Plan responsibilities. A read-only forward test correctly selected `start-plan-slice` after an accepted design slice. Its main risk is unnecessary simultaneous loading with every operation; concise wording limits that cost.
