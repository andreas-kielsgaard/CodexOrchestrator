# Watchdog inspection-outage recovery

## Observation

After the Overall Plan accepted convergence at 12:07, watchdog task inspection was unavailable on every scheduled run from 12:22 through 14:10. The watchdog reported the outage but did not contact the stable Overall Plan owner. Inspection recovered at 14:25, after a user prompt had already reactivated that owner, and three materialized Slice routes then began moving.

The failure recurred overnight on August 6. PIP-02 stopped at 04:31 after launching its isolated application, returning only a progress update and no disposition callback. Inspection failed from 04:44 onward. At 05:00 the watchdog attempted the stable-owner handoff, but messaging and delivery verification were unavailable. Later runs treated that unverified attempt as the one allowed handoff and suppressed every retry through at least 08:20. The Overall Plan did not move again until the user's 08:29 prompt.

The user further clarified that failure to find the stored handler should trigger rediscovery from currently visible tasks. A stale or unresolved address does not establish that the owner is unavailable.

## Cause

The first formulation lacked a stable-owner fallback. The revised formulation added one, but described it as once per outage without saying that only evidenced delivery consumes that allowance. The watchdog conflated an attempted send with delivery and converted a transient messaging outage into a permanent recovery suppression.

## Revision

Before classifying an outage, resolve an unavailable stored route from the harness-visible task roster using project, role/objective, routing, and ownership evidence. Require a unique match. After consecutive failures across both routes, when the last evidenced route still requires future movement, send one compact outage handoff to the stable control owner. Failed, unavailable, or unverified sends remain pending and are retried on later scheduled runs. Evidenced delivery consumes the one-handoff allowance; unknown receiver activation does not authorize duplication. Keep child state, assignments, and continuation judgments with their existing owners.

The active automation receives the same boundary. No child route, project scope, or product skill changes.

## Evaluation

This should recover a root conversation when stored routes, inspection, and messaging fail at different times without reopening noisy monitoring or speculative child steering. Unique visible-owner evidence permits route repair; ambiguity remains explicit. One failed check remains non-actionable, and one evidenced delivery remains the quieting boundary.
