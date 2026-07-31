# Waiting Turn Boundary

## Observation

Delegation actor `019faaa6-dcf8-7ce2-8d77-2a416d68bf39` received a revised acceptance condition while worker `019faaa7-b68d-7aa3-983f-5c93557ff1cd` was implementing it. The actor acknowledged the condition, then kept its turn active and emitted another waiting status despite having no review payload or other actionable input. The worker was still performing the correction and had an explicit callback route.

## Theory

The shared lifecycle concept says a waiting stage ends the agent turn, but `work-slice-delegation` only directed the reader to shared concepts when its prompt was unclear. Its role wording said to remain the slice coordinator and its stage guidance named waiting states without stating the immediate turn boundary. The consuming session could therefore interpret persistent coordination ownership as active waiting.

## Revision

Reformulated persistent ownership as coordination across callbacks. Added a role-local boundary: after a required request is delivered and progress depends on another actor, tool, or human, record the exact waiting stage and end the turn. Resume only when a callback or new message supplies actionable input.

## Evaluation

This separates workflow ownership from agent activity without changing the orchestration topology, callback contract, or review responsibility. It should stop token-burning waits and repeated status messages while preserving the same delegation actor for later review and reporting.

The target is the product-owned `work-slice-delegation` skill. No product code or running session was changed.

## Validation

`quick_validate.py` passed. A fresh delegation actor given an active worker, a delivered clarification, and no review payload recorded `waiting on worker correction`, ended the turn, preserved ownership, and avoided polling or duplicate messages.
