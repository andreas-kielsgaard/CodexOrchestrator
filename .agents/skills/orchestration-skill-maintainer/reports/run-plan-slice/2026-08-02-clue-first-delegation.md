# Clue-first Plan Step delegation

## Observation and theory

The observed Slice converted its detailed reasoning into a large PS-1 packet containing required concerns, proposed architecture, evidence placement, and procedural rules. The task was bounded, but the child received much of the parent's intended solution rather than room to independently resolve it.

## Revision

The Slice keeps detailed decomposition reasoning in its own conversation. Each Plan Step receives an objective, success evidence, genuine boundaries, and a few broad clues. Prescriptive instructions are reserved for hard task boundaries; execution and ordinary judgment remain with `run-plan-step`.

## Evaluation

The change keeps Plan Steps independently evaluable without turning their specifications into prescribed implementations. Exact deliverables and acceptance remain available, so clue-first delegation does not mean vague ownership.

An initial forward test still passed a shared-mapper hypothesis, compatibility-shim hypothesis, and three exact command suggestions as clues. The wording was tightened so child clues remain neutral and those provisional choices stay in the Slice Plan.

A fresh test silently omitted the mapper, shim, and commands. It passed only the frozen-DTO objective, worktree and contract boundaries, acceptance, and neutral clues about complete propagation at each adapter boundary.
