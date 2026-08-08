---
name: run-plan-slice
description: Run one Plan Slice conversation in an ad-hoc Codex Initiative. Use for the lifetime of the conversation that owns detailed slice planning, Plan Step instantiation and evaluation, combined acceptance, and return to the Overall Plan conversation.
---

# Plan Slice Conversation

Own one bounded movement from its supplied objective through a combined result. Plan, launch, and evaluate Plan Steps; leave each step's assigned work to its own conversation.

## Keep the slice state

Maintain the slice frame, evidence baseline, planning revisions, concern map, decisions, projected and actual Plan Steps, dependencies, gates, accepted outcomes, and remaining completion evidence.

Distinguish a concern, a projected step, an evidenced task, a returned result, and an accepted outcome. Preserve superseded projections and actual execution history when evidence changes the plan.

## Select Plan Step profiles

Choose the model from task ambiguity and the reasoning level independently from context scope:

- Use Luna for low-ambiguity work and Terra for high-ambiguity work.
- Use low reasoning for well-scoped, local context. Increase through medium toward high as the needed context becomes poorly defined, broad, or potentially high-blast-radius.

Luna with high reasoning and Terra with low reasoning are both valid. Record the selected model and reasoning for each projected Plan Step and keep the requested and harness-confirmed settings distinct.

## Use the operation skills

- Use `maintain-slice-plan` for the initial detailed plan and material replanning.
- Use `start-plan-steps` to instantiate each currently ready packet.
- Use `evaluate-plan-step` when a Plan Step returns or needs a slice-owned disposition.
- Use `complete-plan-slice` when all required outcomes appear dispositioned and the combined slice is ready for judgment.

Let ready independent Plan Steps proceed in parallel. Route bounded corrections back to the owning Plan Step conversation, and represent additional review, exploration, integration, verification, or similar work as a Plan Step when it is genuinely required.

Avoid polling and routine progress ingestion. Continue planning, evaluation, or other ready work while callbacks are pending. Return to the Overall Plan conversation only after combined slice evaluation or when the slice needs a decision held there.
