---
name: merge-reconciliation
description: Reconcile nontrivial merge conflicts, stale branch state, failed post-merge validation, or branch/worktree inconsistencies after review-before-merge or merge-accepted-work. Use when a clean mechanical merge is not enough and the result must still feed work-slice reporting.
---

# Merge Reconciliation

## Role

Handle merge and branch state problems that exceed `merge-accepted-work`. Preserve accepted intent while resolving conflicts, stale state, or validation fallout.

Run this as a continuation after review or merge inside the work-slice-delegation thread.

Use the shared `Work Integration State` vocabulary in `../_orchestration-common/concepts.md` when describing the repository result.

## Inputs

Expect:

- accepted review decision or merge escalation
- source and target branches
- conflict or state description
- worker summary and changed files
- validation failures, if any
- expected repository integration route, if review or merge supplied one
- constraints about what may be changed

If available, update the delegation thread's compact relationship metadata to show the current stage as `reconciliation`. Do not store conflict details, semantic decisions, or validation evidence there.

## Reconciliation Rules

1. Identify the exact conflict or state problem.
2. Separate mechanical conflicts from semantic conflicts.
3. Resolve only within the accepted work's intent.
4. If the conflict changes product behavior, architecture, or acceptance criteria, stop and request human or review-stage input.
5. Settle ordinary repository mechanics when the reconciliation makes that safe: commit, cleanup, clean-target confirmation, or accepted in-place state.
6. If repository state is cross-slice or strategically ambiguous, classify it as `planner-needed` and explain the exact state the next planner should evaluate.
7. Validate after reconciliation when feasible.
8. Prepare the `work-slice-reporter` continuation payload with the reconciliation outcome and requested reasoning `high` when the report must preserve subtle blocker, conflict, or validation decisions.

Do not bury unresolved semantic decisions inside a merge commit.

## Reasoning Guidance

Use medium reasoning for ordinary conflicts. Use high reasoning for semantic conflicts, validation regressions, or multi-branch state. Use xhigh only when the merge changes architectural direction or threatens data/correctness.

## Output Contract

Return:

- reconciliation status
- conflicts found
- resolutions applied
- unresolved decisions or human needs
- repository integration state
- validation results
- reporter continuation payload with merge/reconciliation outcome and requested reporter reasoning
