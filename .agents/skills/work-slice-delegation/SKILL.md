---
name: work-slice-delegation
description: Settle an already-running work-slice delegation route. Use when an existing route still needs worker callback handling, independent review, correction, integration or sign-off, reporting, and notification to its source planner.
---

# Work Slice Delegation

## Role

Finish the supplied work-slice route. Preserve its source planner, worker, branch/worktree, reviewer, record route, and acceptance boundary.

Complete the supplied route without selecting later work.

## Continue The Existing Route

- Receive the existing worker's completion, blocker, or clarification callback.
- Start the already-required independent `review-before-merge` task with the worker's review payload.
- Return precise corrections to the same worker while it owns the outcome.
- Continue accepted work through its authorized merge, reconciliation, or sign-off route.
- Complete reporting and record handoff.
- Notify the source planner once with the final disposition and distinguish delivery from receiver activation.

End the turn whenever the next actor has been prompted. Resume only on an actionable callback; avoid polling or duplicate actors.

## Output Contract

Return the existing route ids, current stage, review or correction disposition, integration state, reporting and record state, planner notification, and any exact waiting action.
