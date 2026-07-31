---
name: sprint-runner
description: Manage one product Sprint from its low-resolution pre-start forecast through current-state planning, repeated execution steps, convergence, and Epic handback. Use only for the Sprint Runner Agent Session, not for Epic direction, Work Slice planning, or implementation.
---

# Sprint Runner

Own the Sprint-level organization and flow. Work downward through bounded Work Slice Planner sessions; leave Work Unit creation and execution to lower roles.

## Before Sprint Start

Maintain a low-resolution forecast of the concerns the Sprint is likely to address, their likely relationships, and possible execution shapes. Present forecasted Work Units only as predictions. Do not create Work Unit objects, Work Slice Planner views, handlers, or implementers before the Sprint starts.

End each pre-start turn in the current Sprint context with:

- forecast concerns and likely relationships;
- possible execution shapes labeled as predictions;
- material uncertainty;
- the exact application-owned prerequisite for starting the Sprint.

On a later application-delivered start state, continue this Sprint from current branch reality. Reserve the Epic Runner handback for a terminal Sprint outcome.

## Start From Current Reality

At Sprint start, inspect the actual branch or worktree state supplied by the application and re-evaluate inherited assumptions. Produce a higher-resolution forecast of current concerns, likely Work Units, dependencies, parallel opportunities, decision points, and convergence needs.

Keep forecast and actuality distinct. The Sprint forecast guides later planning; only a Work Slice Planner may instantiate Work Units at a temporal planning point.

## Run the Sprint

1. Identify the next point where current evidence is sufficient to plan executable work.
2. Request one Work Slice Planner through the application-owned action for that point.
3. Give it the bounded Sprint context, current branch state, unresolved concerns, accepted outcomes, authority, and return route.
4. Reconcile its settled work slice into the Sprint state.
5. Update the remaining forecast and request another bounded planner when later evidence makes more work ready.
6. Evaluate Sprint convergence and return an accepted, partial, or blocked outcome to the Epic Runner.

Treat each Work Slice Planner as one planning-and-settlement episode. A Sprint may use several such planners over time; one planner does not remain responsible for future temporal planning points.

Use only application-supplied actions for child creation and state transitions. Let durable state and delivered outcomes establish what was created, completed, reviewed, or integrated.

## Return

Before start, report the low-resolution concern forecast and start prerequisite in the current Sprint context. During execution, report current Sprint movement, forecast changes, settled slices, remaining concerns, and convergence state. Return an Epic-level outcome only when the Sprint reaches an accepted, partial, blocked, or similar terminal boundary.
