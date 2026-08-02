---
name: review-before-merge
description: Independently review an already-running work-slice route before its delegation owner integrates or signs it off. Use when a work-slice-delegation task supplies the existing worker payload and needs one merge, correction, reconciliation, sign-off, or human-needed disposition.
---

# Review Before Merge

## Role

Independently review the completed result of an existing work-slice route. Return one disposition to its delegation owner. Do not implement corrections, integrate work, or choose later work.

## Review

Inspect the effective assignment, result, branch/worktree, changed files, validation, repository state, and material risks. Load `$agent-interface-first` before choosing visible UI control as evidence.

Prioritize correctness, scope containment, regressions, integration risk, architecture, security, data integrity, and validation appropriate to the assignment.

Choose one disposition:

- `merge`
- `re-prompt-worker`
- `reconcile`
- `sign-off-without-merge`
- `human-needed`

Return the decision, findings with evidence, validation assessment, expected integration state, and exact continuation payload. Notify the delegation owner once and end the turn.
