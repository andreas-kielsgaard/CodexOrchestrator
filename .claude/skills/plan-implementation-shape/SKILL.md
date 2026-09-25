---
name: plan-implementation-shape
description: Plan the shape of an implementation, optimizing for ownership, reuse, readability, and technical debt. Use only when explicitly requested; not for general development discussion or routine task planning.
disable-model-invocation: true
---

# Plan Implementation Shape

Before implementation, explore how to achieve the agreed functional target while leaving the affected codebase clearer, more reusable, and easier to work on. Choose the implementation's shape with these qualities as objectives. Seek the greatest worthwhile improvement within the work's scope.

A clean, minimal implementation of the agreed functionality, covering the happy flow, is usually sufficient. Usually you should avoid inventing extra bells and whistles or machinery not already discussed, that premptively try to safeguard hypothetical future problems.

Imagine the resulting codebase from the perspective of a future agent: where would it look, what would it need to understand, and which responsibilities could it change independently?

Do not assume that existing implementation shapes are correct - consider the codebase critically and take it upon yourself to refactor if it makes the codebase easier to discover and work with for future operators (within the scope of the task). E.g. splitting apart monoliths or other technical-debt-maintenance tasks.

## Discover before settling on a shape

Trace the relevant implementation, its callers, and the distribution of responsibilities before committing to an approach. Build a sufficiently complete view of the change surface to avoid designing around the first convenient place to add functionality.

Assess existing arrangements critically. Their presence does not establish that they remain suitable for the work now required.

In the discovery process you should also explore already-used external plugins / modules and potentially useful plugins modules. It is preferred to consume external libraries / modules / tools / etc rather than remaking standard functionality. If an external option no longer serves requirements you may decide to replace or modify with a custom implementation. 

## Consider what deserves a different home

Useful clues include:

- A responsibility has grown enough to deserve its own file or feature area.
- Shared behavior has several implementations that could drift apart.
- A file or object mixes responsibilities that need to evolve independently.
- A reusable component needs clearer expectations, interfaces, or placement so consumers can discover and use it correctly.
- A replacement makes existing code or representations redundant.
- An existing boundary already fits and can be reused directly.

These are considerations for judgment, not a checklist of changes to make. Ground abstractions in needs revealed by this task. Clear ownership and understandable consumption matter more than the number of files or layers.

Consider the improvement across existing consumers as well as new components: which callers should adopt shared behavior, which responsibilities remain local, and what competing code can disappear?

Adjacent changes can belong when the task reveals a concrete benefit to reuse, consistency, or maintainability. Explain that connection. Where a discovery creates a material scope or architectural choice, surface the decision, recommend an answer, and state the plan's assumption so the user can disagree without needing to respond when it is appropriate.

Pose questions and other prompts for user input in your response, not in plan files. Files serve agents and review; record the plan, decisions, and assumptions there.

## Make the proposed change reviewable

Describe the intended ownership and consumption boundaries alongside concrete files and objects to retain, adapt, create, extract, move, or remove. Explain how those changes improve the codebase and how the affected consumers fit the resulting structure.

Use enough detail to reveal the actual scope, with a presentation suited to the work. Keep language suggestive rather than definitive. 

In your response, group decision outputs in this order: agent-made choices the user has not addressed; decisions discussed but left inconclusive; decisions the user approved or prescribed. Explain your reasoning for new choices, making the points most likely to need the user's attention easy to find.
