# WU-S6-02 evidence

Status: implementation complete for the durable post-confirmation Bootstrap Generator to Epic
Runner transition. No product Sprint action is exposed or started.

## Deterministic proof

- Focused transition suite: **15 passed**.
- Focused active-storage/migration suite: **8 passed**.
- Rust library aggregate: **154 passed, 3 intentionally ignored**.
- `cargo check --lib`, Rustfmt check, and `git diff --check`: passed.

The production-service tests use the active SQLite repositories, provider-neutral Agent Session
application boundary, recorded runtime, and production Streamable HTTP MCP adapter. They prove:

- deterministic contained preparation and exact approved-plan/manifest bytes before session
  creation;
- one read-only Bootstrap Generator with only `complete_epic_bootstrap` and no caller-supplied
  Epic, session, invocation, or path authority;
- bounded application-written materials with authoritative paths, SHA-256 inventory, command,
  result, and semantic fact;
- semantic-first and lifecycle-first callback order, lifecycle-only and semantic-only no-launch,
  unsuccessful lifecycle no-launch, duplicate replay, conflicting/invalid/foreign calls, path
  escape, and prepared-byte identity mismatch;
- real Agent Session startup reconciliation of an active Bootstrap invocation to `interrupted`,
  deterministic creation and launch of the next attempt, preservation of prior attempt facts, and
  same-attempt semantic/lifecycle gating before exactly one accepted inventory and Runner launch;
- restart after a persisted semantic fact but before terminal observation, exact-byte replay on the
  recovered attempt, conflict rejection, retry-boundary idempotency, no cross-attempt fact/lifecycle
  mixing, blocked failed/canceled/completed-without-fact attempts, and a three-attempt startup
  interruption ceiling;
- recovery before Bootstrap launch, after Bootstrap send acknowledgement, with Runner created
  before launch, after Runner launch acknowledgement, and after a post-persistence callback
  failure, without duplicate durable sessions, invocations, material facts, or launches;
- v1/v2/v3 active-storage migration, including exact migration of the v3 single Bootstrap fact to
  attempt zero, and deterministic rejection of an existing Windows output reparse target before
  read or acceptance;
- exactly one Bootstrap Generator launch on the no-crash path, exactly one fresh attempt per
  eligible startup interruption, and exactly one Epic Runner launch after acceptance, with the
  Runner read-only, no MCP product action, and an explicit stop before product Sprint creation.

## Boundaries

The evidence is deterministic and non-live. It does not claim confirmation-popup UI integration,
live Codex role behavior, process reattachment to an invocation interrupted by an operating-system
or provider crash, race-proof filesystem containment between metadata inspection and file open, or
any product Sprint start. Interrupted active work is no longer stuck: the application starts a
fresh bounded attempt. WU-S6-01's process-memory confirmation-registration risk and first-probe
`internal_error` remain unchanged.
