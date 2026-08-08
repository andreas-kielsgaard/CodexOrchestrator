# Plan Slice task brief and role separation

## Observation and theory

The handoff to Plan Slice `019fc21d-1e60-7762-94be-6d4c46543329` contained both its product task and instructions for how to perform the Plan Slice role. It prescribed planning and coordination behavior already owned by `run-plan-slice`, making the parent an additional role-definition source.

`start-plan-slice` listed the task context to supply but did not explicitly stop the starter from repeating or customizing the child role contract.

## Revision

The handoff now contains only task-specific objective, accepted baseline and evidence, decisions, constraints, authority, completion boundary, artifact locations, work route, validation outcome boundary, and callback address. It invokes `run-plan-slice` for planning, Plan Step handling, profiles, validation method, evaluation, reporting, callback action, waiting, tools, and similar role mechanics.

The callback behavior itself is stated in `run-plan-slice`, so it need not be recreated in each handoff.

The starter also translates relevant parent reasoning into a few broad clues rather than a derived procedure or dense rule set. Exact instructions remain available for genuine hard task boundaries.

## Evaluation

This leaves the Overall Plan responsible for defining the bounded movement while making the Plan Slice skill the single role contract. Task-specific validation boundaries remain available without letting the parent choose commands, checks, or the child's method.

An initial forward test still repeated directions to choose validation commands, report exact fields, and return to the callback. The wording was tightened so validation is only an outcome boundary and the callback id only an address; actions attached to them remain in `run-plan-slice`.

A fresh test then produced only the `run-plan-slice` invocation and the Slice objective, present baseline, task constraints, authority, artifact routes, acceptance, validation boundary, change route, and callback address. It added no planning, Plan Step, reporting, waiting, or callback-action instructions.

The later clue-focused test also omitted parent-only solution hypotheses silently rather than explaining their status to the child.
