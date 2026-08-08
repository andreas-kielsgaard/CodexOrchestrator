---
name: sprint-runner
description: Maintain one application-authorized product Sprint in its pre-start ready state. Use only when the application-owned Sprint Runner Harness exposes this skill.
---

# Product Sprint Runner

Own the Sprint pre-start forecast, started reevaluation, and application-delivered planning-control or handback-reassessment continuation.

## Before Sprint Start

Maintain a low-resolution forecast of the concerns the Sprint is likely to address, their likely relationships, and possible execution shapes. Present forecasted Work Units only as predictions. Do not create Work Unit objects, Work Slice Planner views, handlers, or implementers before the Sprint starts.

Submit the one structured pre-start outcome through the supplied application action. Its success is recorded, not accepted: matching terminal lifecycle observation is still required.

Include:

- forecast concerns and likely relationships;
- possible execution shapes labeled as predictions;
- material uncertainty;
- the exact application-owned prerequisite for starting the Sprint.

On a later application-delivered start state, reevaluate repository and branch reality through the supplied action, then stop. The application must separately observe this invocation's completed lifecycle before it may deliver planning control.

## Planning Control

Only when the application supplies the planning-control continuation may you use its single action to request one Work Slice Planner. The action has no identity or route input. Do not request a Planner before that continuation, and do not create Planner results, Work Units, Handlers, Implementers, or child sessions.

## Return

Report only the stage the application asked you to perform, then stop.

## Handback Reassessment

Read only the supplied bounded handback context. Record one next movement through the supplied action. Alternate eligible work preserves the concern. Wait only for an agent-achievable dependency owner, enabling result, and resumption path. Local exhaustion requests one upward report; it does not activate an Epic Runner or declare Sprint or Epic blockage.
