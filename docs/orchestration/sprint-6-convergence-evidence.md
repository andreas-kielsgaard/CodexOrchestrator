# Sprint 6 convergence evidence

Status: **documentation checkpoint for the accepted Operational Orchestration Spine Work Slice**.
This record covers the accepted implementation and executed evidence through
`076898b3d30e430be866ae89a00fbd77775877b1`. It is local/test-owned evidence, not user acceptance.

## Accepted Work Slice drain

The application-owned bounded graph drain now reconciles durable Handler state to a fixed point.
It processes every eligible generation, including multiple roots and later generations activated
by newly accepted prerequisite contributions. Reopen and missed-notification cases converge from
durable state; the drain does not depend on a live notification arriving at the right time.
Per-Work-Unit states remain factual and independent: blocked, ready, active, waiting, failed,
review-conflict, retry-required, and settled are not collapsed into a graph-wide outcome.
Malformed graphs, cycles, stalls, missing authority, invalid handback, and attention conditions
fail closed. A failed or attention-required unit does not manufacture downstream progress, and
replay preserves terminal facts and identities.

The graph is **graph-complete** only when all required Work Units have factual terminal outcomes
and no structural, authority, or attention condition remains. That boundary is distinct from
**Work Slice execution-settled**, which requires the graph-complete execution result and its
durable settlement facts, and from **planning-point-settled**, which is the separate planning
boundary. None of these boundaries implies Sprint or Epic settlement, continuation, provider
activation, or user acceptance.

## Evidence composition

The Handler-drain fixture proves multi-root and later-generation activation, reopen, and
missed-notification convergence. Focused graph tests prove malformed, cycle, stall, handback,
attention, and replay behavior. The terminal-authority fixture uses real initiated Epic and Sprint
materialization product services together with local Git and the real accepted-candidate,
integration, settlement, and contribution reconcilers. The strict Rust-authored frontend fixture
and executed TypeScript/UI tests prove the public projection. These are complementary proofs; no
single fixture proves every boundary.

The accepted chain is: baseline `bf69fb391e4c552bbc442c8fd754bcc17e460ff9`; durable drain and
settlement `ee925c3`, `91c0069`, `3b874a9`, `9875ff3`, and correction
`d94eecef6793cf7324b08383e75e34c009f75ea1`; strict productive projection
`d379846e900e962735177f2e715ce5f5cea9ab65` and `52ec0eec32c6cc0115d19ad73435a8b022818ed3`;
executed cross-layer/state coverage `74156d3f6139fa2929538b4f5e9d6b3dff4b5aef`,
`9b93bdcc93b27bafa26148c1e3ab85bf7b6cb129`, and `9b8f1413297687b1451fc278feaa879bc12e8767`;
and terminal authority plus exact normalized consumer comparison
`db9f3d7d5df9e735bf72e60cacb375b58cf98b7e` and
`076898b3d30e430be866ae89a00fbd77775877b1`.

## Executed validation

- The focused Handler drain test `handler_drain_advances_independent_generations_from_durable_accepted_contributions` passed, including durable accepted contribution, blocked-middle activation, partial-effect reopen, and missed-notification later-generation coverage.
- `cargo test --lib work_unit_dependency_wave -- --nocapture` passed **15 tests**. This focused suite includes current-attempt failure/review-conflict attention, blocked-activation stall, recoverable partial activation, retry progression, state and contribution correlation, replay, and exact terminal facts.
- The accepted terminal-authority and normalized-consumer fixture executions passed with the real product-service/reconciler composition described above; the strict Rust-authored frontend fixture and executed TypeScript/UI suites passed their recorded assertions.

These are focused and cross-layer validations, not broad or full-suite validation. No claim is
made here that the repository-wide suite passed; no later authoritative broad-suite result is
available in this checkpoint.

## Exclusions and residual limits

This checkpoint includes no Sprint/Epic settlement or continuation, no handback consumption,
reassessment, or new Sprint, no provider reattachment or broadened authority, no push or
publication, and no user acceptance.

Local/test-owned Handler and evidence lineage plus local Git prove application boundaries only;
they do not prove live provider behavior. Broad full-suite validation and user acceptance remain
unproven unless the repository contains later authoritative evidence. Delivery, receiver
activation, integration, persistence, projection, and acceptance remain separate facts.
