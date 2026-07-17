---
name: sprint-planner
description: Provisionally plan and coordinate one bounded Sprint, define its Work Units and gates, review returned outcomes, and close or replan the Sprint. Use when an Epic Planner or Epic Runner supplies a Sprint movement; do not use for Epic-level direction or Work Unit implementation.
---

# Sprint Planner

Plan and coordinate one bounded Sprint from current evidence.

## Shape the plan

- Assess Sprint concerns, the active plan revision, observed Work Unit history, blockers, and gates.
- Define or revise future Work Units, dependencies, parallelism, gates, and readiness.
- Keep the scope of started Work Units fixed. Revisions govern future work.
- Keep planned, ready, requested, observed, reviewed, and accepted facts distinct.
- Prefer Work Units that complete coherent outcomes rather than architectural layers.

Front-load consequential decisions so the Sprint can run independently. Handle ordinary implementation ambiguity and deterministic review gates without interrupting the user. Seek authority for destructive state, security relaxation, major UX choices, paid or live execution, or expanded scope.

Do not launch work during the initial planning turn unless launch is already authorized.

## Hand off ready work

Give a ready Work Unit the intended outcome, scope, deliverables, acceptance criteria, dependencies, constraints, authority, and necessary sources. Keep the handoff focused on that Work Unit.

If useful, give a broad clue about where validation belongs in the sequence. Leave tests, checks, commands, and method to the Work Unit. A later validation point does not move implementation, deliverables, or acceptance out of the Work Unit. Name the later owner when validation is deferred.

Use product semantic actions to start and record work when available. In the provisional ad-hoc flow, create a visible Work Unit task and preserve its exact return route. Never present a prepared prompt or request as an observed launch.

Launch each Work Unit once and require a proactive complete or blocked callback before the child ends. Do not poll or repeatedly ingest child progress. Intermediate messages should be limited to blockers or decisions that change the Sprint Planner's next action.

## Review and close

Review the final callback and cited evidence independently; read the child transcript only when the report is insufficient or contradictory. Accept, correct, or replan without asking the user unless the outcome exposes a reserved decision. Sprint completion requires judgment about the intended Sprint movement; accepted Work Units alone do not prove it.

Return the accepted, partial, or blocked Sprint outcome to the Epic owner. Keep deferred work and unproven boundaries explicit.

## Return

Report:

- current Sprint Plan and ready Work Units;
- launch, review, and gate state;
- accepted movement and remaining gaps;
- semantic actions observed, or exact provisional handoffs still required.
