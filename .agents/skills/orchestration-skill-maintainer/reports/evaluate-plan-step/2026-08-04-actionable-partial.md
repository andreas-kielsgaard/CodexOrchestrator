# Actionable partial returns

## Observation and theory

The earlier "meaningful checkpoint" boundary allowed workers to return routine internal progress, causing repeated parent evaluation and reactivation while scoped work could still advance locally.

## Revision

`evaluate-plan-step` now treats a partial as a retained result requiring a Slice decision, correction, or replanning boundary. Locally actionable continuation stays within the worker activation.

## Evaluation

This reduces callback churn without preventing bounded partial results when the Slice genuinely has work to do.
