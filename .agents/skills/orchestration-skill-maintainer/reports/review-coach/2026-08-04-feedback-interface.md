# Feedback interface and review closure

## Observation and theory

Review Coach delegated Initiative ingress correctly, but its pending-batch wording was conditional on the user first stating that correction should wait. In observed HIL use, that left ordinary multi-turn review responses vulnerable to premature dispatch.

## Revision

The coach now treats every response inside an open coherent review slice as provisional. It retains one pending batch until user closure, with early dispatch available only on the user's explicit request. Closed Initiative feedback still uses `$route-initiative-feedback`.

## Evaluation

This makes closure the stable default without preventing intentional early handoff. It reduces fragmented implementation requests and keeps the reviewed material coherent for both the user and receiving workflow.
