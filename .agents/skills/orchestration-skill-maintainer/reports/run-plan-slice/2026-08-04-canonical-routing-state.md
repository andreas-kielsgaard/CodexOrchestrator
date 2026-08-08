# Canonical routing state

## Observation and theory

An active Slice used a current launch skill but later behaved like an older worktree copy after compaction. The launch prompt had no compact, retained identity for the role skill, catalogue, task route, and callback route.

## Revision

`run-plan-slice` now reads those values from the launch routing header, retains them as distinct state, resolves operations only from that catalogue, and re-reads the exact role skill after compaction.

## Evaluation

This gives the Slice an actionable recovery source without copying skills into its product worktree. The guarantee is prompt-carried because current Codex exposes no typed instruction-source metadata.
