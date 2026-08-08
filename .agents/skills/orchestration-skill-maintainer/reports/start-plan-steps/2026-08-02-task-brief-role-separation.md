# Plan Step task brief and role separation

## Observation and theory

The PS-1 handoff in task `019fc220-a82a-7203-8b6a-87cb6b3d4076` supplied a legitimate fixed implementation specification but also repeated worker mechanics: callback behavior, polling, editing method, reporting shape, and reserved-decision handling. It also sourced `run-plan-step` from the change target's stale skill snapshot.

`start-plan-steps` previously instructed the parent to authorize and restate checkpoint and return behavior even though `run-plan-step` already owns those responsibilities.

## Revision

The assignment now contains only task-specific specification, context, authority, boundaries, acceptance evidence, artifacts, work route, profile, validation outcome boundary, and callback address. It invokes `run-plan-step` for investigation, validation method, checkpointing, reporting, callback action, waiting, tools, and similar worker mechanics.

The starter still selects independent ready work, applies its planned profile, supplies an attributable route, and records evidenced launch state. Those are launch responsibilities rather than worker instructions.

Before launch, it passes a few broad clues rather than the parent's procedure or dense rule set. It refines prescriptive wording that does not protect a genuine hard boundary.

## Evaluation

The revision keeps a Plan Step sufficiently specified while allowing it to discover and execute the solution under one worker-role contract. It removes repeated role prose without weakening task scope, authority, acceptance, or routing.

A fresh test produced only the `run-plan-step` invocation and the fixed objective, artifact routes, task authority, acceptance, validation boundary, selected profile, change route, and callback address. It added no investigation, command, checkpoint, reporting, waiting, or callback-action instructions.

The later clue-focused test also omitted parent-only implementation and command hypotheses silently rather than adding disclaimers to the worker brief.
