# Global Wait and Workaround Audit

## Observation

The coach initially said no action was available until the Human Worktree Review Launcher finished. A queue-wide audit found:

- the launcher implementation is active and has an acceptance reviewer;
- the Harness Management correction is active and has an acceptance reviewer;
- the File/diff correction is active and has an acceptance reviewer; and
- the Sprint 6 initiation correction is accepted in an isolated worktree, but its worker and reviewer are idle and no task is preparing the reviewable build required for human retesting.

After the queue audit was added, the coach assigned the launcher as Sprint 6's preparation path and again chose to wait. When challenged, it recognized that the launcher was only a convenient automation path: it could build and launch the accepted Sprint 6 worktree directly without merging or changing the material under review.

After later re-ingesting the revised skill, the coach routed Harness Management corrections and stated that the user had no action until that reviewer returned. It did not audit the rest of the register. When challenged, it immediately found unfinished Sprint 6 and Worktree Launcher reviews plus two main-product areas that had never been completed, then prepared the independent Epic/Sprint detail flow.

The remaining Agent Test Mode UI was explicitly superseded, and its Agent Review checkpoint was retained only as technical evidence under the user's narrowed launcher scope. No other currently ready human review was found.

## Theory

The original skill tested whether feedback blocked the current area and maintained a review register, but it did not require `not ready` entries to have live preparation ownership. Later revisions added ownership, dependency-necessity, and complete-queue checks. Those checks still remained private reasoning instructions: the coach could assert that no action existed without refreshing or demonstrating the register. The user then had to challenge the conclusion before the omitted areas were considered.

## Revision

Reformulated the queue-liveness boundary as an evidence gate before any no-action claim. The coach refreshes the register in the current turn and gives a compact secondary accounting of every unfinished non-deferred area and why no full, partial, or safely preparable frame can advance. Any area that fails that test is selected and prepared instead.

The environment-ownership guidance still prefers the smallest reversible setup that exposes the intended material and separates durable automation from temporary review preparation. Work the coach cannot prepare directly needs an active owner and return route. Explicit user deferral remains a decision rather than a blocker, and the evidenced audit occurs only at wait-decision points so it does not create routine polling.

## Evaluation

This would have made the omitted Sprint 6, Worktree Launcher, Agent Session, and Epic/Sprint areas visible before the coach claimed there was no action. The extra accounting appears only when the coach intends to wait, so it should prevent narrow-current-area conclusions without cluttering ordinary coaching responses.

The target is the general Codex `review-coach` skill. No product code or product-owned Orchestrator role skill is changed.

## Validation

`quick_validate.py` passed. A fresh coach given active corrections plus three independent unfinished areas rejected waiting, represented preparing the production confirmation flow directly, and supplied its review action. A second fresh coach given a genuinely exhausted register exposed a compact audit of every unfinished area, its essential boundary and return route, then stated the no-action result and which returned frame it would prepare first.
