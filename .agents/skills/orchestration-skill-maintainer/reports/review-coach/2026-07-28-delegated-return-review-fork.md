# Delegated Review Continuity

Status: general review-coach and delegation skills revised. Product worker lifecycle aligned. Product code and harnesses unchanged.

## Observations

In coaching session `019fa398-6f66-76d2-b7b9-38bb2b4898c9`, a worker callback first caused the main coach to perform implementation acceptance itself. After acceptance moved to reviewer `019fa873-f533-7232-b382-3fa22def41cf`, the coach ended its dispatch response without giving the user another review action.

Worker `019fa85d-ad38-7911-acde-f7a726475425` then sent its COMPLETE package to the reviewer but remained active polling for disposition.

## Theory

The original coach flow lacked separate acceptance ownership and a concrete user continuation across dispatch. The later worker brief described session addressability as “remaining available,” even though that property belongs to the harness and requires no worker action. The worker interpreted the phrase as active waiting.

## Revision

The coach creates a reviewer fork and routes worker callbacks there. Raw worker reports are routing material in the coaching session; reviewer approval returns as a compact trusted disposition.

Every successful dispatch response ends with an independently executable coaching instruction or focused question. If the current area depends on the correction, the coach moves to another independent review area unless a named dependency blocks all review work.

Worker briefs now state the complete action sequence: deliver the callback and end the turn. The reviewer sends any correction later through the harness.

## Evaluation

Implementation, acceptance, and coaching each have one active owner. The coach stays focused on the user, the user receives a next action, and the worker has no work after handoff unless a later correction arrives.

Fresh evaluators that explicitly ingested the relevant skills continued independent coaching after dispatch and ended worker activity after a COMPLETE notification without polling.
