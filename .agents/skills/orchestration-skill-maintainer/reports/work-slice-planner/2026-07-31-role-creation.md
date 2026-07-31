# Work Slice Planner Role Creation

## Trigger and gap

The clarified workflow identifies a distinct decision-point agent between Sprint Runner and Work Unit Handler. Existing `sprint-planner` and `orchestration-next-work-planner` are ad-hoc workflow roles with different parents, lifecycle, and record routing, so neither is a product definition for this reader.

## Reader and revision

The new skill addresses one Work Slice Planner session supplied with current Sprint and branch state by a Sprint Runner. It identifies work executable now, instantiates one Work Unit and Handler per parallel lane, manages those Handlers through terminal outcomes, evaluates slice convergence, and returns to the Sprint Runner.

## Evaluation

Using a bounded planning-and-settlement episode resolves the current uncertainty conservatively: a later temporal point receives a fresh planner instead of silently extending the first planner's planning scope. The skill does not claim a current product harness.

Forward testing exposed one remaining ambiguity: the Planner tried to instantiate a dependent second wave after the first Handler returned. The revised skill fixes the Work Unit set at the opening decision and returns newly ready concerns to the Sprint Runner for a fresh Planner.

## Validation

`quick_validate.py` passed. A fresh Planner instantiated only independent A/B lanes and their Handlers, created no Implementers, fixed that Work Unit set, and returned dependent C as a later planning candidate.
