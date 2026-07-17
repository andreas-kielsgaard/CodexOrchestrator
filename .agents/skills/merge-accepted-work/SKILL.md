---
name: merge-accepted-work
description: Merge a clean work slice after review-before-merge accepts it. Use as a stage continuation in the work-slice-delegation thread when review decided to merge, conflicts are expected to be trivial or absent, and the merge result should be reported separately before the work-slice report is prepared.
---

# Merge Accepted Work

## Role

Perform the narrow merge step for accepted work. Keep the operation mechanical and visible. Do not re-review the implementation unless new merge state changes the risk.

Run this as a continuation after `review-before-merge` inside the work-slice-delegation thread.

Use the shared `Work Integration State` vocabulary in `../_orchestration-common/concepts.md` when describing the repository result.

## Inputs

Expect:

- accepted review decision
- source branch/worktree
- target branch
- validation expectations after merge
- expected repository integration route from review
- known risks or files to watch
- reporter-stage destination or record/reporting route

If available, update the delegation thread's compact relationship metadata to show the current stage as `merge`. Do not store merge details, validation evidence, or branch history there.

## Merge Rules

1. Confirm the review accepted the work.
2. Inspect branch cleanliness and target/source identity.
3. Merge using the project's normal method.
4. If conflicts or nontrivial reconciliation appear, stop this skill and use `merge-reconciliation`.
5. Settle ordinary repository mechanics for the accepted slice: commit, cleanup, clean-target confirmation, or accepted in-place state when appropriate for the route.
6. If repository state is cross-slice or strategically ambiguous, classify it as `planner-needed` and explain the exact state the next planner should evaluate.
7. Run requested post-merge validation when feasible.
8. Prepare the `work-slice-reporter` continuation payload with requested reasoning `medium` unless the merge exposed subtle blocker or validation history.

Do not silently resolve substantive conflicts under this skill.

## Reasoning Guidance

Use low reasoning for clean mechanical merges. Use medium when post-merge validation or branch state needs judgment. Escalate to `merge-reconciliation` rather than using high reasoning here for complex conflicts.

## Output Contract

Return:

- merge status
- source and target
- commands or actions performed
- repository integration state
- post-merge validation
- conflicts or reasons for escalation
- reporter continuation payload with merge outcome and requested reporter reasoning
