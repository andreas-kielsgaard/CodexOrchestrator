---
name: review-before-merge
description: Review a completed orchestration work slice before merge or sign-off. Use from the work-slice-delegation thread when an orchestration-worker supplies a review payload, and decide whether to merge, re-prompt the worker, reconcile merge issues, or sign off without merging.
---

# Review Before Merge

## Role

Act as the review stage for a completed work slice inside the `work-slice-delegation` thread that staged the slice. The independent worker authors the review payload, but the delegation conversation performs the review as its next stage.

Do not redo the worker's implementation. Inspect evidence and decide the next step.

## Inputs

Expect a worker review payload containing:

- slice title
- worker and delegation thread ids, if known
- branch/worktree
- planner justification
- goal and acceptance criteria
- changed files
- implementation summary
- validation results
- current repository status
- risks, blockers, and unresolved questions

If available, update the delegation thread's compact relationship metadata to show the current stage as `review`. Do not store the review payload, findings, or file evidence there.

## Review Focus

Prioritize:

- correctness against acceptance criteria
- regressions or integration risks
- missing tests or weak validation
- architecture or boundary violations
- security or data risks when relevant
- whether the branch is mergeable or should be corrected

Avoid style-only comments unless they affect correctness or future maintenance.

## Decisions

Choose exactly one:

- `merge`: accepted; continue with `merge-accepted-work`.
- `re-prompt-worker`: corrections needed; produce a precise prompt for the worker thread.
- `reconcile`: merge or branch state is nontrivial; continue with `merge-reconciliation`.
- `sign-off-without-merge`: there is a well-founded reason not to merge; continue with `work-slice-reporter`.
- `human-needed`: stop and ask for intervention.

Do not combine review, merge, and reporting into one opaque response. Emit a review conclusion first.

When accepting work, include the expected repository integration route using the shared `Work Integration State` vocabulary from `../_orchestration-common/concepts.md`. Let `merge-accepted-work` or `merge-reconciliation` settle ordinary commit, cleanup, and clean-target mechanics.

When continuing to the next stage, apply the next skill's reasoning guidance. Use low or medium reasoning for clean `merge-accepted-work`, high for `merge-reconciliation`, and medium for `work-slice-reporter` unless the report must reconcile subtle blocker or merge history.

Do not start a new review, merge, reconciliation, or reporter thread for normal flow. Emit the review conclusion first, then continue the same delegation conversation with the chosen next skill. If a correction prompt must be sent to the worker and messaging is unavailable, notify the planner fork with the exact worker prompt and mark the delegation lifecycle as waiting-on-tool.

## Reasoning Guidance

Use high reasoning by default. Use medium only for very small, low-risk doc or mechanical slices. Use xhigh for high-risk architectural, data, security, or multi-branch uncertainty.

## Output Contract

Return:

- decision
- review summary
- findings with file references when applicable
- validation assessment
- expected repository integration route
- required correction prompt, merge continuation payload, reconciliation continuation payload, or reporter continuation payload
- context to carry into the next step
