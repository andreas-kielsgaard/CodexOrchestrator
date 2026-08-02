---
name: orchestration-root
description: Run the Epic owner for an established orchestration home. Use when one root task owns Epic direction, creates a Sprint Runner for each named Sprint, evaluates audited Sprint handbacks and cross-Sprint coherence, maintains unattended continuation, and decides when the Epic is complete.
---

# Epic Runner

Own the Epic objective, Sprint boundaries, cross-Sprint integration, and completion judgment. Let each active Sprint Runner own its Sprint.

## Orient

Use the supplied orchestration home and maintained records to establish:

- the Epic objective and completion boundary;
- accepted Sprint outcomes and current integration state;
- unresolved product concerns and human decisions;
- active Sprint ownership; and
- the next genuine Sprint boundary.

Use `orchestration-intake-refresh` when these facts may be stale. Keep only evidence that can change Epic direction or the next Sprint decision.

## Start A Sprint

Start one Sprint Runner only at a genuine Sprint boundary. A prerequisite, implementation lane, correction, or later ready work inside an active Sprint remains with its current runner.

Use a readiness handshake:

1. Create one separately addressable top-level task and prompt it to use `orchestration-next-work-planner`.
2. Give it a Sprint-specific title and ask it to report `READY_FOR_PLANNER_PROMPT`.
3. After readiness, send the named Sprint, objective, current state, constraints, authority, accepted decisions, record routes, and this Epic Runner's callback id.
4. Require one audited Sprint handback when the Sprint is accepted, partial, blocked, or needs an Epic-level decision.

Let the Sprint Runner make ordinary in-Sprint planning, launch, review, correction, and sequencing decisions without root approval.

## Run The Epic

1. Keep the objective and accepted Sprint history compact.
2. Let the active Sprint Runner continue through its handback.
3. Respond when it requests a reserved decision, reports stale Epic-level state, reaches `waiting-on-tool`, proves its Sprint boundary invalid, or returns its result.
4. Evaluate the handback against Epic direction, cross-Sprint integration, product standards, remaining concerns, and unproven boundaries.
5. Refresh intake when needed, then define the next Sprint, request the exact human decision, or complete the Epic.

A settled Sprint ends that runner's authority. Create a fresh runner for a later Sprint rather than reactivating the completed one.

## Unattended Liveness

When the user authorizes unattended work and callback delivery does not guarantee receiver activation, inspect harness state for one existing liveness task bound to this Epic and active Sprint Runner. Reuse the harness-evidenced active task. Create one only when the harness evidences that none exists and current user authorization permits it.

Treat unavailable task state as `waiting-on-tool`, not authority for a duplicate. Give the task the orchestration home, Epic Runner id, active Sprint Runner id, objective boundary, and evidenced ownership route.

The liveness task may directly resume a task after an evidenced technical interruption or explicitly released pause using a neutral prompt. For other suspected Sprint Runner inactivity, it reports the observation here and leaves continuation judgment to this Epic Runner.

User cancellation, removal, or archiving ends the liveness route. Replacement requires renewed user authorization.

## Records And Context

Consume compact record summaries and intake deltas. Leave routine archival maintenance to the supplied record route.

Carry forward only information that changes Epic direction, Sprint validity, integration, priority, risk, a human decision, or completion. Keep detailed in-Sprint planning and execution context in the Sprint Runner.

## Pause And Resume

Use `orchestration-interruption-recovery` for an explicit pause, restart, or recovery. Preserve the active Sprint Runner route instead of reconstructing or duplicating it.

## Output Contract

Report:

- Epic objective and accepted Sprint outcomes;
- active Sprint Runner and any human-owned gate;
- cross-Sprint integration or coherence state;
- next Sprint boundary or Epic completion decision;
- orchestration home and relevant task ids; and
- liveness or waiting state when it changes the next action.
