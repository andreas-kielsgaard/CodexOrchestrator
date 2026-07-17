# WU-S6-01 evidence

Status: implementation complete for the managed Plan Builder, role harness, runtime-enforcement,
and confirmation-coordinator boundaries. Popup integration and later roles remain separate Work
Units.

## Deterministic proof

- Rust library after review correction: `cargo test --lib` — **137 passed, 3 intentionally
  ignored**.
- Focused review-correction runs: confirmation **5 passed**; MCP **7 passed**.
- Installed CLI help probe: **1 passed** against `codex-cli 0.144.0`.
- Frontend serial aggregate: **83 files / 581 tests passed**.
- `npx tsc --noEmit`, lint, production build, `cargo check`, Rustfmt, touched-file Prettier, and
  `git diff --check`: passed. Existing Rust dead-code, React `act`, and Node SQLite experimental
  warnings remain.

The deterministic matrix covers the two-tool list/schema, no-input initiation request, managed
child configuration, server-derived session scope, forbidden and cross-session denial, proposal
restart/idempotency/conflict behavior, required confirmation, rejection and confirmation state
ordering, three harness profiles, read-only start/resume argument assembly, and unknown/unsupported
sandbox rejection.

The review-correction cases additionally prove atomic concurrent replay registration, rollback and
visible retry after initial notification failure, prompt waiter completion after rejection or
confirmation notification failure, and persisted-effect reconciliation without a second initiation
command when later notification fails.

## Bounded live Codex proof

Two installed-Codex proposal-only probes ran after deterministic proof. The first discovered and
called `submit_epic_plan_proposal` once with the correct typed payload but received the safe
`internal_error` result and persisted nothing. The immediate rerun completed and durably projected
exactly one proposal. Neither requested initiation.

This proves one real managed Codex/MCP/persistence happy path and preserves the first transient
failure as a stability risk. Live resume, live initiation confirmation, popup behavior, Bootstrap
Generator creation/material completion, Epic Runner launch, and any product Sprint start remain
unproven or out of scope.

The confirmation defects cannot explain the first live `internal_error`: that probe called only
proposal submission and never requested initiation. No new live evidence changes the recorded risk.
