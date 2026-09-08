---
name: maintain-slice-plan
description: Build, revise, and present a detailed plan for one explicitly identified bounded slice of work. Use when the user asks for a slice plan or invokes this skill; do not use for routine task planning.
---

# Maintain Slice Plan

Maintain one grounded, reviewable planning artifact for a bounded objective.

## Establish the frame

State the objective, boundaries, completion condition, current evidence and workspace state, accepted decisions, relevant prior outcomes, constraints, assumptions, risks, and uncertainties. Record the planning revision and its evidence baseline.

## Map the problem

Identify the concerns that must be resolved, why each matters, what evidence would resolve it, material coupling, and current uncertainty.

Assess each concern for definition, ambiguity, complexity, context breadth, blast radius, reversibility, available verification, and decisions that could materially change behavior, scope, architecture, sequencing, acceptance, or expensive rework. Batch related human decisions when practical.

## Define work packages

Map every concern to a coherent work package, a plan-level decision or gate, or an explicit deferral. Give independently evaluable outcomes separate packages even when they must occur sequentially. Organize packages by outcome and evaluation boundary rather than by file or profession.

For each package, state:

- stable id, title, intended outcome, and concerns addressed;
- scope, deliverables, acceptance evidence, and hard boundaries;
- relevant context, sources, constraints, risks, and governing decisions;
- dependencies, preferred sequencing, and gates; and
- required validation outcomes and where they belong.

For work crossing a producer-consumer boundary, map accepted and extensible variants, sequencing or correlation facts, privacy or negative-authority constraints, and the earliest consumer evidence that can exercise the contract.

Confirm that locally required evidence is feasible within the planned conditions. Treat unavailable environments, essential test seams, information, or decisions as gates. Later integration evidence does not defer a package's implementation, deliverables, or local acceptance.

## Map sequence and parallelism

For every package, identify hard dependencies, preferred ordering, decisions or evidence gates, shared integration surfaces, and whether it can overlap with other work.

Group compatible packages into parallel lanes. Show their entry conditions, shared surfaces, convergence points, what their completion enables, and the exact gates holding later work. When only one lane is possible, state the concrete dependency or shared constraint responsible.

Re-evaluate the sequence after each material result, decision, or baseline change.

## Maintain and present the plan

Keep the complete current plan through revisions. Present it in full when first established, when requested, and after context loss. Otherwise present the changed portion and its effects without repeating unaffected detail.

A full plan covers:

1. Objective, boundaries, and completion condition
2. Evidence baseline and planning revision
3. Concern and uncertainty map
4. Decisions, assumptions, and gates
5. Work-package and dependency map
6. Sequence, parallel lanes, shared surfaces, and convergence
7. Detailed package specifications
8. Evidence and validation placement
9. Risks, deferrals, and unresolved decisions
10. Current status and planned sequence

Preserve superseded projections and meaningful deviations as history. Finish each maintenance pass with a concise summary of the planned sequence, parallel opportunities, and remaining gates.
