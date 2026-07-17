---
name: phase-completion-audit
description: Audit whether an orchestration phase or milestone is actually complete. Use when orchestration-next-work-planner or the root orchestrator suspects a phase boundary has been reached and needs evidence about completed slices, unresolved blockers, validation, documentation, records, and remaining work before moving on.
---

# Phase Completion Audit

## Role

Assess whether a phase, milestone, or first usable slice is complete enough to advance. This is conditional, not part of every orchestration loop.

The planner or root orchestrator should trigger this skill when all visible work for a phase appears done, when manual testing or review suggests a milestone may be complete, or when the next step depends on knowing whether a phase can close.

## Inputs

Expect:

- phase or milestone definition
- current high-level orchestration map
- completed slice reports
- open blockers or follow-ups
- validation and review summaries
- record-maintainer state
- product or manual-test gates

If available, create or update a compact thread-relationship `sub-agent-context` record keyed by this audit thread id for compaction recovery. Do not store audit evidence, phase facts, or verdict reasoning there.

## Audit Checks

Check:

- required work slices are complete, merged, signed off, or intentionally deferred
- review and merge outcomes are recorded
- validation appropriate to the phase has passed or has explicit residual risk
- blockers are resolved, accepted, or escalated to a human
- record root reflects the current done/missing state
- no active worker result is waiting for intake, review, merge, or reporting
- next phase entry criteria are satisfied

Do not require exhaustive historical detail. The question is whether moving forward is justified.

## Decisions

Choose one:

- `complete`: phase can close.
- `partial`: phase can advance with explicit residual risks or deferred items.
- `not-complete`: specific missing work remains.
- `human-needed`: cannot decide without human input.

## Reasoning Guidance

Use high reasoning when phase closure affects product direction, release readiness, or multi-branch sequencing. Use medium for narrow internal milestones. Avoid low reasoning unless the phase checklist is trivial.

When a root or planner starts this audit through thread tooling, request the chosen reasoning level as launch metadata and omit model overrides unless the human explicitly requested a model.

## Output Contract

Return:

- verdict
- evidence
- missing or deferred items
- unresolved blockers or human actions
- record updates needed
- recommended next orchestration action
