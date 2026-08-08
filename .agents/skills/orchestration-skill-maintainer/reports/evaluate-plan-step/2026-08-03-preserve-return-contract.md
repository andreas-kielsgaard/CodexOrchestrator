# Preserve the Plan Step return contract

## Observation and theory

HI-01's Slice asked the worker to return only when complete or concretely blocked. The worker later found substantial scoped work remaining, did not classify it as blocked, and ended without a callback. Narrowing the return contract removed the evaluable partial route needed by a non-polling parent.

## Revision

`evaluate-plan-step` now handles partial results explicitly. Continuation and correction prompts state the required outcome while preserving complete, partial, blocked, and clarification dispositions.

## Evaluation

The Slice can still demand full completion and reject insufficient partial work. Preserving the return dispositions prevents an ended child activation from becoming invisible without weakening acceptance criteria.
