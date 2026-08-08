# Plan Slice role skill

## Observation and theory

The four Slice Plan operations did not provide one durable contract connecting planning, task launch, result evaluation, combined return, and Plan Step profile selection. The generic request to record model and reasoning did not explain that model follows task ambiguity while reasoning follows context scope and blast radius. A Plan Step cannot select its own creation settings, so this policy belongs with the Slice Plan reader that creates it.

## Revision

`run-plan-slice` now owns slice state, Plan Step relationships, operation selection, concurrent ready work, return timing, and Plan Step profile selection. Model choice follows task ambiguity: Luna for low ambiguity and Terra for high. Reasoning independently rises from low to high with context uncertainty, breadth, and potential blast radius. Planning format, launch mechanics, evaluation, and completion evidence remain in their operation skills.

## Evaluation

The role skill clarifies end-to-end slice ownership without reviving separate reviewer, settlement, or record roles. A read-only forward test correctly selected `maintain-slice-plan`, produced the expected planning concerns, and deferred launch to `start-plan-steps`. It leaves task-specific judgment with the operations and Plan Steps.
