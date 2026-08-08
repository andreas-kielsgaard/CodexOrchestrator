# Callback and waiting boundary

## Observation and theory

The observed planner continued calling `wait_threads` after its implementation task and activation were already evidenced. It repeated this behavior after re-reading the first revision. The bounded-confirmation exception and instruction to end the turn still allowed the reader to treat an incomplete slice and pending callback as reasons to keep the turn active.

## Revision

`start-plan-steps` permits one bounded confirmation only while the task route or activation remains unproven and explicitly skips it when that evidence exists. Once launch and delivery are evidenced, the operation completes. The reader continues work that can proceed now or records the pending callback briefly and ends the turn without waiting for it.

## Evaluation

This preserves truthful launch confirmation without turning the Plan Slice conversation into a monitor. It does not prevent targeted inspection when a returned report is insufficient or contradictory, and it does not equate callback delivery with receiver activation. A fresh read-only role test with evidenced route, activation, and delivery ended the turn without using task tools.
