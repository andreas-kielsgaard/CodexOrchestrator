# Review Handoff Stop Boundary

Status: product worker and shared lifecycle concepts revised. General delegation and review-coach briefs aligned. Product code unchanged.

## Observation

Worker session `019fa85d-ad38-7911-acde-f7a726475425` completed its correction, sent the required COMPLETE package to reviewer `019fa873-f533-7232-b382-3fa22def41cf`, and then stayed active polling for disposition. It narrated review progress even though no correction had arrived.

## Theory

The launch prompt said to remain available and route corrections until disposition. From the worker's perspective, session addressability is controlled by the parent harness and requires no action. Presenting it as a worker duty left the agent searching for work and encouraged active waiting.

The worker skill also omitted a direct stop boundary after successful notification and referenced a removed `_orchestration-common/concepts.md` path.

## Revision

The worker now sends its review payload and required notification, then ends the turn. The reviewer owns the next active step. A later correction message starts a new worker turn.

The shared concepts, coach brief, and general delegation skill use the same action-based sequence without assigning session addressability to an agent. The worker's stale reference now points to the existing lifecycle, owner-liveness, and reporting concepts.

## Evaluation

This preserves correction continuity through harness messaging while giving the worker no work between report delivery and a later correction. It removes polling and token use without introducing a new lifecycle responsibility.

A fresh evaluator that explicitly ingested the worker skill ended after the COMPLETE notification, did not poll, and made no acceptance claim.
