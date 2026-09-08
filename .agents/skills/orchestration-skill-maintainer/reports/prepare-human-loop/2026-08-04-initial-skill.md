# Prepare Human Loop

## Observation and theory

Autonomous development can leave an available user without useful work even when upcoming decisions, reviewable checkpoints, or preparable scenarios could benefit from human attention. Monitoring and blocker-clearing roles do not own turning that opportunity into an executable human task; Review Coach begins once a review surface and objective are ready.

## Revision concept

Create a general Codex skill for an agent that identifies the highest-value human contribution, performs reversible agent-side preparation, verifies readiness, and hands off one self-contained task while preserving existing implementation ownership.

## Evaluation

The skill covers test and review preparation, early decisions, blockers, and analogous contributions without fixing a closed taxonomy. It avoids defining a product role or workflow and hands guided review to Review Coach. The main risk is unnecessary human interruption; the usefulness test and requirement to perform agent-executable work first constrain that risk.

## Placement

The skill lives at `C:\Users\user\.codex\skills\prepare-human-loop` because its reader is a general Codex agent, not a product-owned orchestration role.
