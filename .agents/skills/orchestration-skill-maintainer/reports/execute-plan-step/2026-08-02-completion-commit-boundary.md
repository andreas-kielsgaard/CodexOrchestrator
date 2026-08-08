# Plan Step completion commit boundary

## Observation and theory

The ad-hoc Plan Step skills required implementation, validation, and a callback but assigned no commit responsibility. Handoffs could also prohibit staging and committing. Completed work therefore remained only as mutable worktree state. In the observed flow, accepted Slice 1 changes existed only in its worktree, contributing to the later misuse of a conversation fork to preserve that state for Slice 2.

## Revision

A Plan Step that changes repository state commits only its attributable changes after validation and before reporting completion. Its launch handoff supplies that bounded stage-and-commit authority. The completion return includes the commit identity, message, and post-commit state.

The commit is a candidate checkpoint rather than parent acceptance. No-change steps create no empty commit. Corrections receive their own commits. Work that cannot be isolated and committed safely returns blocked rather than complete, and concurrently started steps require routes that preserve commit attribution.

## Evaluation

This produces durable, reviewable boundaries between Plan Steps without authorizing push, merge, rebase, or unrelated integration. It preserves intentionally dirty unrelated state and lets evaluation distinguish committed work from accepted work.

A completion test with two attributable files and one unrelated dirty file correctly required selective staging, a step-scoped commit, and post-commit status evidence. Because the test withheld Git authority, the reader returned blocked rather than claiming completion or inventing a commit. A separate evaluation test rejected a changed-but-uncommitted return and kept its dependent gate closed pending a bounded commit correction.
