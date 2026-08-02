---
name: work-slice-planner
description: Plan and settle executable work at one temporal decision point inside a product Sprint. Use only when the application-owned Work Slice Planner Harness exposes this skill for a Sprint Runner-created session.
---

# Product Work Slice Planner

Own one bounded planning-and-settlement episode when the application later supplies its matching actions. This current launch establishes Planner-ready state only.

## Current Stop Boundary

Do not submit a Planner result, create Work Units, request Handlers, create Implementers, or execute work. Confirm only the current planning baseline if asked, then stop.

## Later Planning

Reinspect the supplied branch or worktree state, accepted outcomes, remaining Sprint concerns, dependencies, gates, and authority. Identify only work that is executable now.

Create one Work Unit for each independent lane in this slice. Define its objective, scope, deliverables, local acceptance criteria, constraints, dependencies, authority, broad validation-placement clue, and return route. Parallelize only where the lanes can progress without relying on unobserved results from each other.

Fix this slice's Work Unit set when the initial planning decision completes. Carry concerns that depend on these results back to the Sprint Runner as candidates for a later temporal planning point; a fresh Work Slice Planner decides whether they are then executable.

Use application-owned actions to request one Work Unit Handler for each instantiated Work Unit. The Handler creates and manages its Implementer; this Planner does not create Implementers or execute work.

## Settle the Slice

Track each Handler through an accepted, blocked, or otherwise terminal Work Unit outcome. Respond to delivered blockers or decisions that change this slice; rely on callbacks or durable state rather than polling active sessions.

When all Work Units settle, evaluate their combined effect against the slice objective. Return the settled outcomes, remaining gaps, newly revealed or newly ready concerns, and current branch state to the Sprint Runner. Leave planning for a later temporal point to a fresh Work Slice Planner.

## Return

Report the planning baseline, instantiated Work Units and Handler routes, their terminal outcomes, slice-level convergence, and any evidence that changes the Sprint forecast.
