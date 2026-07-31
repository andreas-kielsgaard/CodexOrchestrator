# Feedback realization continuity

## Observation

The coach presented an accepted Agent Sessions checkpoint whose navigation crossed into an older detail implementation. The navigation and corrected detail work had been accepted separately, while their required consolidation was left for later without an assigned owner or return route. Prior feedback resumed only after the user encountered the stale route.

## Theory

The skill tracked ownership by review area but did not distinguish isolated technical acceptance from realization of feedback across the user-facing route. This allowed a component checkpoint to appear complete while the integration needed for the requested outcome became inactive.

## Revision

Reformulated `Maintain Review Ownership` so actionable feedback remains open until represented where the user must judge it. Remaining integration or consolidation must be assigned with a return route before an intermediate checkpoint is presented. Such checkpoints remain available when their omissions are explicit and their review boundary is trustworthy.

## Evaluation

This should preserve useful temporary checkpoints without allowing them to silently replace the requested outcome. A first forward test kept the outcome open and bounded the review away from stale material, but still presented the checkpoint while describing consolidation as unassigned. Requiring assignment before presentation closed that ambiguity: a second fresh-agent test assigned implementation and acceptance routes before offering the bounded checkpoint. The revision adds no prescribed implementation or validation method and does not require components to merge before they can be reviewed.

## Scope

The target is the general Codex `review-coach` skill used by the coaching agent session. No product-owned Orchestrator role skill or product code requires revision for this behavior.
