---
name: work-unit-review
description: Review a reported-complete Work Unit before a Planner accepts it. Use when a worker returns completion evidence and the Planner must verify the effective specification, scope containment, implementation, validation, and need for correction or user review.
---

# Work Unit Review

Review the Work Unit independently. Do not accept it from the completion report alone.

## 1. Reconstruct the effective specification

Inspect:

- the accepted Work Unit specification;
- its launch prompt;
- later prompts delivered during execution;
- prompt source metadata when available;
- accepted decision records governing the Work Unit;
- the worker report;
- the resulting repository or artifacts and validation evidence.
- Other relevant context

Treat authorized later prompts as part of the effective specification. Distinguish user, Planner or agent-session, application, and system sources.

If prompt provenance is missing or ambiguous and could change the verdict, report the evidence gap instead of inferring the source.

## 2. Review the result

Load and follow `$agent-interface-first` before choosing visible UI control as review evidence.

Determine whether:

- the objective and acceptance criteria were achieved;
- the work stayed within scope and respected non-goals;
- later prompts legitimately changed the expected outcome;
- the implementation matches the report;
- validation is sufficient;
- regressions, architectural boundary violations, or unresolved decisions remain.
- material choices match their recorded decision and stay within its containment boundary;
- any new consequential choice was returned to its authorized owner instead of being hidden as an
  obvious implementation detail.

For UI Work Units, also verify that:

- the UI is implemented in the application, not a separate presentation harness;
- a specific designed view uses an application view or tab where appropriate;
- the first implementation is minimal and supports iterative user feedback;
- unrequested variants or speculative expansion are treated as overscope.

## 3. Decide

Choose one:

- `accepted`: the Work Unit satisfies its effective specification.
- `correction-required`: return a precise correction prompt to the same worker.
- `user-review-required`: the result is ready for a product or visual decision.
- `blocked`: required evidence or authority is unavailable.

If result is accepted, continue with eligible work.

## 4. Report

Return:

- decision;
- effective-specification summary, including material later prompts and their sources;
- decision-record assessment, including unrecorded or exceeded authority;
- scope assessment;
- findings and validation assessment;
- correction prompt, user review instructions, or blocker when applicable;
- recommended launch-register update.
