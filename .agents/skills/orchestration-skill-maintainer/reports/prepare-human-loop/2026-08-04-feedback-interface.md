# Feedback interface and review closure

## Observation and theory

Prepare Human Loop initially defined where to route a human result but said to act "after the user responds." During a multi-turn review, that made each response resemble a completed contribution and caused the HIL session to dispatch feedback before the user closed the review slice.

## Revision

The skill now preserves observations from a coherent review slice as one provisional batch. A response advances that slice but does not authorize dispatch. Routing occurs when the user closes the slice or explicitly requests earlier routing; Initiative results still use `$route-initiative-feedback`.

## Evaluation

This keeps task preparation focused, prevents premature workflow churn, and preserves an explicit early-routing escape without requiring the HIL reader to infer review closure.
