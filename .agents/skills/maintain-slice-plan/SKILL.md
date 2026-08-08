---
name: maintain-slice-plan
description: Build, revise, and present the detailed execution plan for one bounded ad-hoc Plan Slice. Use in the conversation that owns the slice before launching Plan Steps, whenever returned evidence materially changes the plan, and whenever the current plan is requested for review.
---

# Maintain Slice Plan

Turn the assigned movement into a grounded, reviewable execution plan.

## Establish the frame

Verify the supplied objective, current repository or worktree state, accepted decisions, relevant prior outcomes, authority, constraints, and completion or re-evaluation condition. Record a planning revision and the evidence baseline it relies on.

## Map the problem before the work

Identify the concerns that must be resolved, why each matters, the evidence that would resolve it, material coupling, and current uncertainty. For each concern, assess definition, ambiguity, complexity, context breadth, blast radius, reversibility, available verification, and an appropriate work mode.

Surface choices that could materially change product behavior, scope, architecture, sequencing, acceptance, or expensive rework. Trace the intended evidence path through consequential product actions and resolve whether the Step may perform each one before launch or whether it is an entry gate. Keep ordinary execution judgment inside the Plan Step that performs the work. Batch related human decisions when possible.

## Design Plan Steps

Map every concern to a Plan Step, a slice-owned decision or gate, or an explicit deferral. Give independently evaluable outcomes separate Plan Steps even when they must run sequentially. Shared files or integration surfaces affect sequencing and ownership; they do not alone make the entire slice one work unit.

Bundle work by coherent outcome and evaluation boundary rather than file or profession. For each projected Plan Step, state:

- a stable id, title, objective, and concerns addressed;
- scope, deliverables, acceptance criteria, and explicit boundaries;
- context, sources, authority, risks, and governing decisions;
- hard dependencies, preferred sequencing, and evaluation gates;
- broad validation-placement clues;
- a visible profile assessment covering ambiguity, context scope and blast radius, selected model and reasoning, and why that choice is more appropriate than adjacent settings; and
- the callback route.

When an outcome crosses a producer-consumer contract, map the edge in the Slice Plan before launch: accepted and extensible variants, sequencing or correlation facts, privacy or negative-authority boundaries, and the first consumer evidence that can exercise the contract. Keep this at the level of contract behavior; pass only broad boundary clues to the Plan Step.

Make the objective, deliverables, acceptance, and genuine hard boundaries precise while leaving the solution path open. Translate useful context and risks into a few neutral clues about concerns, evidence, or surfaces to inspect. Silently omit unaccepted solution candidates and command or check suggestions from the child brief. State acceptance as outcome evidence rather than reporting requirements. Reserve prescriptive wording for hard boundaries such as authority, safety, scope, dependency, or acceptance.

Let the Plan Step choose its commands, tests, checks, and validation method. Deferring some validation to a named later integration step does not defer its implementation, deliverables, or local acceptance criteria.

Ensure the first locally required consumer evidence can actually run before launch; unavailable authority or environment is an entry gate. Place only genuinely cross-step or integration evidence later without weakening the Plan Step's local outcome.

When independent cross-layer evidence needs its own Plan Step, gate it on a coherent executable candidate. Place earlier exploration only where the unresolved concern is the contract or evidence strategy itself.

For a shared convergence or evidence document spanning several Plan Steps, assign one owner after the relevant product checkpoint stabilizes. Earlier Steps return exact evidence through their checkpoints and callbacks; keep the document with an earlier Step only when it is itself part of that Step's accepted outcome.

Derive each Plan Step profile from the concerns and context it actually owns rather than the overall Slice's difficulty. A stronger model or reasoning level is not a substitute for decomposition, clearer context, or bounded scope.

Leave ordinary inspection and acceptance of returned work to the Plan Slice conversation. Project a review or verification Plan Step only for independently required evidence or a distinct unresolved concern, and name the evidence it contributes beyond routine evaluation. A standing review after every implementation is not a planning gate.

## Map ready packets

For every projected or active Plan Step, identify hard dependencies, preferred ordering, decisions or evidence gates, shared integration surfaces, and whether its ownership and work route permit independent execution.

Group all currently eligible independent Plan Steps into the next ready packet. Show its parallel lanes, entry evidence, gates, shared surfaces, convergence point, and what completion unlocks. Keep ineligible steps visible with the exact gate holding them. When the packet contains only one step, state the dependency, shared seam, or other concrete reason parallel work is unavailable. Re-evaluate the packet after each accepted result, decision, or material baseline change.

## Maintain and present the plan

Keep the complete current Slice Plan through every revision. Present it in full when first established, when the user asks for it, and after context compaction. On other maintenance passes, present the change and its effects on concerns, steps, dependencies, gates, evidence, risks, and the launch register without repeating unaffected detail.

Retain the reasoning that made the plan reviewable: the evidence baseline, characteristic assessment, ambiguity and decision treatment, concern dispositions, decomposition rationale, detailed step specifications, validation placement, risks, and gates. Update these with actual execution evidence and superseded projections; do not replace them with current statuses.

When presenting the full plan, cover every consideration below. Combine adjacent sections when that improves clarity, but make each consideration visible or explain why it does not apply:

1. Plan Slice frame
2. Evidence baseline and planning revision
3. Problem or concern map
4. Planning-characteristic assessment
5. Ambiguity register and decision packet
6. Concern-to-step and dependency maps
7. Sequence and ready-packet overview
8. Parallel lanes, shared integration surfaces, gates, and convergence
9. Detailed Plan Step specifications
10. Evidence and validation map
11. Risks and unresolved decisions
12. Launch register and next ready packet
13. Concise operational summary

The launch register is the current operational index of every projected or actual step and named gate, with status and reason. Preserve superseded projections as history.

An internal plan update or detailed Plan Step prompt does not replace the initial full presentation. Execution authorization permits immediate launch after presenting that plan; it does not reduce planning depth. If execution is not authorized, stop after the plan. Otherwise expose a ready packet for `start-plan-steps` only after the complete current plan is presented and launchable.
