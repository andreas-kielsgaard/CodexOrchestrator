# Terminal worker notification

## Observation

After worker task `019f6b37-d80d-7e61-8cbd-7e618cefc6ce` delivered its review payload and handled one requested correction, planner task `019fb99b-46ad-71b2-b18a-6bfdd995e654` later sent it an informational integration notice. The notice reactivated the worker even though review, integration, reporting, and record settlement were already owned elsewhere.

## Reader and theory

The relevant reader is the planner-owned ad-hoc `work-slice-delegation` actor. Its skill correctly routed corrections back to the worker but did not state where non-actionable downstream disposition belongs. That gap encouraged a courteous lifecycle notice to an agent session with no remaining action. Its shared-concept link also pointed to the removed `_orchestration-common/concepts.md`, weakening access to current ownership and reporting guidance.

## Revision and evaluation

Treat the worker's completion notification as transfer of active disposition to the delegation actor. Keep later review, integration, reporting, and record settlement inside delegation, planner, and record routes. Reactivate the worker only for correction, clarification, or similar worker-owned action.

This closes the observed token-burning notification path without hiding actionable feedback or requiring the worker to wait. The wording remains adaptable to analogous worker-owned follow-up and does not change future product-role skills.

## Applied

`work-slice-delegation` now states the downstream audience boundary directly and resolves its ownership, liveness, work-route, context, reporting, reasoning, and thread-naming references to the current shared concept files. No product code or future product-role skill changed.
