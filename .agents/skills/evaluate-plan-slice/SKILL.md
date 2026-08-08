---
name: evaluate-plan-slice
description: Evaluate a returned Plan Slice against an ad-hoc Initiative's Overall Plan. Use in the Overall Plan conversation when a slice reports completion, a blocker, a reserved decision, or evidence that may change direction.
---

# Evaluate Plan Slice

Judge what the returned slice changes at Initiative level. Do not redo its detailed planning or implementation.

## Evaluate the return

Inspect the report and cited artifacts far enough to determine:

- whether the intended movement and exit condition were met;
- whether the accepted result coheres with earlier slices and the broader product;
- whether a repository-changing result is clean, committed, and published at the exact reported remote Slice ref;
- which claims are evidenced and which remain unproven;
- whether residual work belongs inside this slice or changes the Overall Plan; and
- whether a reserved human decision is now required.

Accept the slice, send a bounded correction to its existing conversation, request the necessary decision, or revise direction. Return missing or mismatched publication evidence to the same Slice owner for correction. Treat remote publication, Overall Plan acceptance, and canonical integration as separate facts. Treat message delivery and receiver activation as separate facts when the harness distinguishes them.

When accepting a Slice, disposition its retirement manifest in the same decision turn. Use `retire-plan-slice` for routes whose retained state may now be reclaimed. Preserve routes with a concrete continuation, evidence, ownership, or safety reason; incomplete retirement evidence does not reopen otherwise accepted product work.

When acceptance unlocks another artifact-producing Slice, complete eligible generated-state reclamation before launching it.

When the returned movement will combine with earlier accepted Slices, judge the new composition boundary separately from each Slice's local acceptance. If their interaction introduces an uncovered risk, retain downstream movement behind a convergence Slice with its own conversation and acceptance boundary. Keep implementation and repository integration inside that Slice rather than performing it in the Overall Plan conversation.

Retain one compact profile-calibration judgment from the returned evidence and host-evidenced settings: whether the Slice completed successfully, whether its model or reasoning plausibly affected the result, and the stronger cause when it did not. Briefing, scope, ownership, environment, validation seams, build duration, and interruption are non-exhaustive alternatives. Include successful Slices in calibration and treat elapsed time or correction count as evidence only when linked to reasoning behavior. Change later selections only after repeated same-direction evidence.

## Return the disposition

State the accepted movement, required correction or decision, remaining directional implications, and unproven boundaries. Avoid retaining the slice's routine implementation detail in the Overall Plan conversation.
