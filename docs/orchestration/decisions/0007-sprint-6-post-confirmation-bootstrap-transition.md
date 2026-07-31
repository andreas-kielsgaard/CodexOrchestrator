# 0007: Sprint 6 post-confirmation bootstrap transition

Status: accepted basis for WU-S6-02 implementation. Extends 0005 and 0006 without changing the
meaning of durable Epic initiation or `orchestration-native-query/v2`.

## Decision

- `initiated` remains only the accepted durable proposal/initiation fact. The application records
  separate preparation, Bootstrap Generator session-created and launched, semantic-completion,
  lifecycle-observed, material-accepted, Epic Runner session-created and launched facts.
- One focused post-confirmation service consumes persisted initiation callbacks and reconciles all
  persisted initiations at startup. It derives stable transition, session, invocation, directory,
  and material identities from the initiation and exact approved proposal snapshot.
- Preparation precedes Agent Session creation. The application creates one contained Epic
  directory and writes the approved-plan and transition-manifest inputs. Existing paths are reused
  only when their identity and bytes match.
- The Bootstrap Generator runs read-only. Its invocation-scoped MCP server exposes only
  `complete_epic_bootstrap`; the registered application context derives Epic, session, invocation,
  and destination authority. The bounded typed content is validated and written by the
  application to the exact prepared destinations.
- One stable Bootstrap Generator role/session owns durable ordinal attempts. Each attempt has a
  deterministic invocation identity and preserves its launch, lifecycle, semantic fact, retry,
  and acceptance state. An interrupted attempt is never erased or relabeled.
- Semantic completion persists one attempt-bound command, result, fact, validated inventory,
  authoritative paths, and SHA-256 hashes. Agent Session completion alone has no material effect.
- Material acceptance requires a semantic fact and observed successful terminal lifecycle from
  the same attempt. Either may arrive first. Facts and lifecycle observations from different
  attempts never combine.
- Startup interruption automatically creates or reuses the next deterministic attempt, up to
  three total attempts. Failed, canceled, and completed-without-semantic-fact attempts are blocked
  and do not loop. A third interrupted attempt is blocked at the retry limit.
- A retry may create a new unaccepted fact only when its material bytes match the existing exact
  destinations. Conflicting bytes fail closed. Prior facts remain historical; one accepted
  attempt and inventory gate one separate read-only Epic Runner Agent Session.
- Agent Session creation and application-originated sends accept stable application identities.
  Replays reuse matching durable sessions and invocations; conflicting semantics fail closed.
- Status is exposed separately as `epic-bootstrap-transition-query/v2`, including current and
  prior attempts plus retryable, retried, blocked, and accepted truth. This avoids adding fields to
  the strict native-v2 proposal/initiation contract; WU-S6-03 may consume the new query.

## Recovery and boundary

Startup replays every incomplete stage from durable facts and filesystem verification. Active
Agent Invocations are durably observed as interrupted by the Agent Session application; the
transition then advances through a fresh bounded attempt rather than provider-process
reattachment. Output destinations that are links or reparse points are rejected before read or
write. This is deterministic link hardening, not a claim of race-proof filesystem containment.

The service never infers effects from transcripts, session idleness, or manual handoff text. This
unit stops after the Epic Runner launch is durably acknowledged; it does not create or start a
product Sprint, Work Slice Planner, Sprint Runner, Work Unit, execution, review, or adaptive-planning
action.
