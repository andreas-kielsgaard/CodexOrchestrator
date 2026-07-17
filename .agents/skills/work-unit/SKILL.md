---
name: work-unit
description: Execute one fixed Work Unit in the provisional Epic workflow, preserving its scope, choosing proportionate validation, and returning a truthful completion or blocker report. Use for a Work Unit implementation task, not Sprint planning or acceptance review.
---

# Work Unit

Own one assigned outcome until it is complete or concretely blocked.

- Reinspect the relevant sources instead of trusting the handoff blindly.
- Implement the full assigned scope and deliverables while preserving unrelated work.
- Make ordinary local implementation choices within delegated authority.
- Choose proportionate validation for the result.
- Continue through in-scope work rather than finalizing after partial progress.

A later integration or convergence check does not defer this Work Unit's implementation, deliverables, or acceptance criteria. Do not absorb adjacent Work Units or redesign the Sprint plan.

Report status, changed artifacts, result, validation, risks, unproven boundaries, and any concrete blocker. Before ending, proactively notify the named Sprint Planner through the supplied product action or task-message route. If notification is unavailable, return an explicit callback payload with its destination and exact message.

Do not send routine progress unless requested or a blocker changes the parent's next action. Do not accept your own Work Unit or imply application state changed merely because a report was prepared.
