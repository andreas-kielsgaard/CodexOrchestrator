# Plan Step skill source boundary

## Observation and theory

The three newly launched Plan Slice tasks resolved their role and operation skills from older isolated worktrees rather than the current ad-hoc catalogue. The same worktrees contained stale `start-plan-steps` copies, so their Plan Step launches were exposed to the same source ambiguity for `run-plan-step`.

The earlier wording preferred host current resolution and allowed an explicit path only when separately evidenced. The observed host selected the child's worktree copy, showing that name resolution alone does not prove which catalogue supplied the instructions.

## Revision

`start-plan-steps` now uses the catalogue that supplied the operation as its instruction source. It resolves the sibling `run-plan-step/SKILL.md`, verifies that it is readable, and begins every Plan Step prompt with an `AD-HOC TASK ROUTING` header containing the absolute role skill, catalogue, task route, and callback task. The child is told to retain those values and re-read the exact role skill after compaction. The change route remains separate.

Self-contained assignments, authoritative artifact locations, and callback-only conversation ids remain unchanged.

## Evaluation

This closes the same instruction-drift path at the next routing boundary without copying the orchestration catalogue into product worktrees. Current Codex carries the values in the prompt rather than typed task metadata, so the boundary remains prompt-enforced. A missing current source becomes an explicit launch boundary.

A forward test with an older isolated worktree selected the current catalogue's absolute `run-plan-step/SKILL.md` path, kept it separate from the product route, and produced a self-contained fixed assignment.
