# Efficient Long-Running Validation

## Observation and theory

Plan Steps repeatedly run costly build commands in isolated worktrees. The earlier wording required proportionate validation but did not help the executor distinguish new evidence from redundant compilation or handle a yielded long-running command without overlapping work and repeated reconsideration.

## Revision concept

Make validation evidence-driven: select the smallest command sequence that closes local acceptance, recognize compilation already performed by build-and-run commands, serialize commands sharing build locks, and let a chosen command reach a final result before deciding what follows.

## Evaluation

This should reduce redundant builds and reasoning during tool waits without weakening implementation or acceptance. Additional commands remain appropriate when they cover a distinct target, configuration, integration boundary, risk, or unavailable gate.
