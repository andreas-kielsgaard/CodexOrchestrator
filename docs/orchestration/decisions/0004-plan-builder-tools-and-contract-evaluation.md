# 0004: Plan Builder tools and contract evaluation

Status: superseded by the single-call correction for Sprint 5 implementation.

## Initial catalog

Expose exactly one versioned semantic tool, `submit_epic_plan_proposal`, for the managed `epic_plan_builder` product context. Separately, its
product-owned, versioned Conversation Harness configuration supplies a lightweight
application-provenance prefix immediately before the first user query. It includes conditional
skill guidance but does not load or authorize skills. Normal Agent Sessions remain role-neutral.

| Tool                        | Input                                                                                    | Result                                        | Required checks                                                                                 |
| --------------------------- | ---------------------------------------------------------------------------------------- | --------------------------------------------- | ----------------------------------------------------------------------------------------------- |
| `submit_epic_plan_proposal` | `{ suggestedEpicName?, sprints: 1..20 [{ title, intendedMovement, concernSummaries }] }` | bounded persisted or idempotent-replay status | bearer, invocation-bound profile/draft scope, captured optimistic revision, derived idempotency |

The public proposal body is deliberately smaller than a full orchestration plan:
`{ suggestedEpicName?, sprints: 1..20 [{ title, intendedMovement, concernSummaries }] }`.
`concernSummaries` is required per Sprint and may be empty. Work Units, phases, risks, objectives,
acceptance gates, and broad plan aggregates are rejected with a field-path error. A successful save
returns `status: persisted`; an exact duplicate transport delivery returns `status: idempotent_replay`. Both are
terminal successes for that request and must not trigger another retry.

The application captures the authorized current proposal revision at managed invocation start and
derives a stable idempotency/correlation key from the durable Agent Invocation plus canonical
proposal payload. The invocation already authorizes one draft, profile, session association, and
actor. MCP handlers inject those values; a caller cannot select another draft through tool arguments.
The Plan Builder
prefix is delivered only when durable invocation history is empty, so restart does not reinject it.
MCP authority, durable product identity, and terminal handling remain separate orchestration and
Agent Session concerns. They are not fields or effects of the current Conversation Harness.

The save operation creates the first proposal revision or a later revision; it never creates or
initiates an Epic/Sprint/Work Unit and does not mean the proposal is user/reviewer accepted. Its
durable effect is authorized, validated, applied, persisted, and projected only. No raw CRUD,
`append_event`, materials, initiation, execution, continuation, or legacy-runtime tool is exposed.
The public catalog has no context query, raw CRUD, or caller-controlled concurrency/idempotency field.

## Errors and evaluation

Tool errors are structured, actionable, and non-secret: `unauthenticated`, `forbidden`,
`draft_not_found`, `invalid_input`, `revision_conflict`, `idempotency_conflict`, `profile_expired`,
and `internal_error`. Return `isError: true` with a stable machine code, safe user guidance, and an
operation/correlation ID; do not reveal token, filesystem, SQL, or stack details.

Golden contract cases cover zero-call discussion, one-call first proposal, one-call rebuild,
idempotent duplicate delivery, concurrent revision conflict, unauthorized invocation, hidden IDs,
malformed input, output limit, and exact Rust JSON -> TypeScript decode. Include direct MCP
initialize/list/call cases for the same authorization paths. These are deterministic evaluation;
they make no claim that a model selected either tool.

The normal evaluation budget is zero semantic calls for discussion, then exactly one submission for
a build/rebuild request. Tool-list schema, descriptions, and result payload sizes
are bounded in deterministic tests.
