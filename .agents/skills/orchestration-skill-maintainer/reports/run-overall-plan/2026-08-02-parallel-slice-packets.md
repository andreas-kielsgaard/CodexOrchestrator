# Parallel Plan Slice packets

## Observation and theory

The original role spoke of “the next” Slice and active ownership in the singular, so it did not reliably scan the complete forecast for concurrent movement.

After the first ready-packet revision, Overall Plan task `019fc106-1222-7f52-a1ad-9189481658e8` correctly reorganized the Initiative into four parallel lanes. It then ended with three lanes “planned but not launched.” A second user prompt was required before it started them. The role said to start authorized Slices but did not make clear that existing execution authority survives replanning or that launch follows the plan in the same turn. Plan presentation therefore became an accidental approval boundary.

## Revision

The role uses the ready Slice packet to consider every forecast movement and start every authorized member that is operationally eligible. It now carries existing execution authority through planning revisions and continues from required plan presentation directly into `start-plan-slice` in the same turn. A material scope change or concrete authority, dependency, ownership, work-route, or decision gate is the reason to stop at planning.

## Evaluation

This closes the planning-to-action gap without making readiness itself universal authorization. It preserves the role's existing checks for authority, dependencies, ownership, work routes, shared surfaces, and convergence, so it should not force unsafe or unauthorized starts.

The earlier ready-packet forward test considered the complete five-Slice forecast rather than selecting only its first row and identified two simultaneously ready independent movements. In the follow-up test, an already-authorized replanning turn carried authority forward and routed all three newly ready Slices in the same turn without duplicating the active Slice or asking for confirmation.
