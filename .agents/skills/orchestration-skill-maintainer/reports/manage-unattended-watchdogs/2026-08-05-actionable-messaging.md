# Actionable watchdog messaging

## Observation

The earlier watchdog generally avoided messaging healthy workers, but it sent repeated recovery prompts to the planner when `systemError`, unloaded state, or missing receiver-activation evidence suggested possible inactivity. Several prompts reconstructed the remaining work rather than providing the short neutral recovery now intended. Routine heartbeat results also reported progress even when no intervention was needed.

## Reader and cause

The reader is a dedicated control task with automation, compact task-inspection, and task-messaging tools. It monitors existing ownership and does not perform project work.

The skill distinguished recovery recipients and limited direct child recovery, but it did not make recipient action the universal prerequisite for a task message. Its “report only changed state” wording governed the watchdog result and could be read as permission to notify the ownership chain whenever state changed. The ordinary-inactivity route likewise did not explicitly account for a parent that already had the result or had no action available.

## Revision

Reformulated the complete messaging boundary around one clue: send a task message only when it enables a current action by that recipient. Healthy progress, normal waiting, non-actionable state changes, and already-delivered callbacks remain quiet. The watchdog may still record observations in its own result. Initial unattended context is sent only when the runner needs it, and one acknowledged boundary remains sufficient until the route or authority changes.

The parent-recovery wording now requires unfinished responsibility needing disposition and keeps the route quiet when the parent already has the result or no action is ready.

## Evaluation

This should reduce informational nudges and repeated recovery prompts while preserving the operations that justify contact: evidenced interruption recovery, inactive-child disposition, undelivered settlement, failed recovery, or similar actionable conditions. The examples are non-exhaustive, and the action gate leaves the watchdog room to handle unforeseen failures without treating ordinary progress as a communication event.
