# Canonical routing state

## Observation and theory

Canonical source propagation can fail at the first child launch if the Overall Plan later resolves operations from a task worktree or loses its source after compaction.

## Revision

`run-overall-plan` now retains the absolute path from which its role was read, resolves sibling operations from that catalogue, and re-reads the same role skill after compaction.

## Evaluation

This anchors transitive routing at the conversation that launches Plan Slices. It adds no product-harness claim and treats task-worktree skill copies as non-authoritative.
