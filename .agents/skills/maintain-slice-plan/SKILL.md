---
name: maintain-slice-plan
description: Build and revise the detailed execution plan for one bounded ad-hoc Plan Slice. Use in the conversation that owns the slice before launching Plan Steps and whenever returned evidence materially changes the plan.
---

# Maintain Slice Plan

Turn the assigned movement into a grounded execution plan.

## Establish the frame

Verify the supplied objective, current repository or worktree state, accepted decisions, relevant prior outcomes, authority, constraints, and completion or re-evaluation condition. Record a planning revision and the evidence baseline it relies on.

## Map the problem before the work

Identify the concerns that must be resolved, why each matters, the evidence that would resolve it, material coupling, and current uncertainty. Assess definition, complexity, risk, reversibility, available verification, and an appropriate work mode.

Surface choices that could materially change product behavior, scope, architecture, sequencing, acceptance, or expensive rework. Keep ordinary execution judgment inside the Plan Step that performs the work. Batch related human decisions when possible.

## Design Plan Steps

Bundle work by coherent outcome and evaluation boundary. For each projected Plan Step, state:

- a stable id, title, objective, and concerns addressed;
- scope, deliverables, acceptance criteria, and explicit boundaries;
- context, sources, authority, risks, and governing decisions;
- hard dependencies, preferred sequencing, and evaluation gates;
- broad validation-placement clues;
- selected model and reasoning under the Plan Slice profile policy; and
- the callback route.

Let the Plan Step choose its commands, tests, checks, and validation method. Deferring some validation to a named later integration step does not defer its implementation, deliverables, or local acceptance criteria.

## Show the execution shape

Distinguish concerns, projected steps, actual tasks, returned results, and accepted outcomes. Show hard dependencies, parallel lanes, shared integration surfaces, evaluation gates, and final convergence.

Use the following presentation as a model when applicable, adding or omitting sections when the slice warrants it:

1. Plan Slice frame
2. Evidence baseline and planning revision
3. Problem or concern map
4. Planning-characteristic assessment
5. Ambiguity register and decision packet
6. Concern-to-step and dependency maps
7. Sequence overview
8. Parallel lanes and gates
9. Detailed Plan Step specifications
10. Evidence and validation map
11. Risks and unresolved decisions
12. Launch register
13. Concise operational summary

The launch register is the current operational index of every projected or actual step and named gate, with status and reason. Preserve superseded projections as history.

Do not launch Plan Steps during the initial planning turn unless execution is already authorized.
