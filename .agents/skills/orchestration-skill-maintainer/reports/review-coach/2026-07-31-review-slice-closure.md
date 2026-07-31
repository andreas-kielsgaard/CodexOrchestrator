# Review Slice Closure and Correction Dispatch

## Observation

The user reviewed one Worktree Launcher slice across several interactions and asked that implementation wait until the slice was complete. The coach accurately accumulated the requested widget, build-detail, retained-build, failure-message, observability, compatibility, and cache outcomes. At 17:52:36 the user closed the slice.

The coach then re-read skills, inspected tools and memory, and started the next Harness Management preview before sending the correction handoffs at 17:56:52. The implementation task began at 17:56:57, before the coach's final response at 17:57:16. The work therefore was initiated, but only after unrelated next-slice preparation and without a clear activation account. The user reasonably experienced the closure as having failed to start the gathered work.

## Theory

The skill explained how to route feedback once it was actionable, but it did not model feedback held pending across a multi-interaction review slice or define closure as the event that commits that batch. It also did not order correction dispatch ahead of preparing the next area. The coach could synthesize the feedback correctly while treating dispatch as one background obligation among several transition tasks.

## Revision

The complete Review Coach skill was reconsidered and its granularity and routing sections were reformulated together.

A coherent slice may now span several guided interactions. When the user wants gather-first review, the coach maintains one pending batch and continues the slice without starting correction early. Sign-off, completion, or similar closure reconciles that batch: unresolved requested outcomes become one actionable correction package unless the user explicitly accepts the material unchanged or defers it.

The coach completes the implementation handoff before preparing the next review area. It uses the host operation that starts or continues the implementation session, confirms the dispatch receipt, represents work state only at the supported evidence level, and repairs a rejected handoff in the same turn. The role-local use of `delegate-and-yield` now leaves worker execution to callbacks while allowing only the independent review queue and next user movement to finish the coach's turn.

## Evaluation

This preserves the user's deliberate gather-first review boundary while making its closure operational rather than merely conversational. It should prevent both premature implementation and the observed gap after sign-off. The ordering is narrow and does not add polling: the coach checks the dispatch result it already receives, then yields while worker and reviewer callbacks carry completion.

The trace also guards against overstating the incident. The implementation did start; the revision targets delayed ordering and ambiguous state evidence rather than inventing a missing runtime capability.

The maintained target is the general Codex `review-coach` skill used by this Orchestrator review, not a product-owned Orchestrator role skill. No product code or running agent session was changed.

## Validation

`quick_validate.py` passed after the reformulation.

Two fresh Review Coach sessions exercised the closure boundary without seeing the diagnosis. In the gather-first case, the coach consolidated all three pending outcomes, dispatched implementation plus independent acceptance review, confirmed the handoff receipts, and continued with the already-prepared independent review area. In the explicit unchanged-acceptance case, it closed the slice with zero requested outcomes and created no implementation work.
