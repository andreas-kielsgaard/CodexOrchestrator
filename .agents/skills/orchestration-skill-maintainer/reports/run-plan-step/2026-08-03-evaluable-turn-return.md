# Plan Step evaluable turn return

## Observation and theory

HI-01 twice ended a completed agent turn while describing itself as in progress and sent no callback. The first omission left its Slice and Overall Plan idle for almost twenty-three hours; the same ending recurred immediately after recovery. The initial revision added an explicit partial disposition. HI-01 then returned one valid partial, was continued by its Slice, and again ended with a final in-progress update, an uncommitted file, an allegedly running compile, and no callback. The reader treated the final response as a progress channel despite the return contract.

## Revision

`run-plan-step` now defines an activation loop and a distinct return boundary. Scoped work and started operations remain inside the loop. A partial requires a retained result that gives the Slice an actionable decision, correction, or replanning boundary; a command, check, commit, or internal phase remains local while the assignment can advance. The role also retains the canonical routing header and re-reads its exact skill after compaction.

## Evaluation

This gives the non-polling Slice a return for every ended activation without turning routine checkpoints into parent work. The actionable-partial threshold is more selective than the earlier "meaningful checkpoint" wording while preserving real blockers, decisions, corrections, and replanning. The canonical source state addresses stale worktree skill resolution without adding product concepts.
