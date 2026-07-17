# Sprint 5 extension and gates

Status: authoritative operating map for the accepted Sprint 5 product slice. The minimum durable
initiation/restart path is current; broad material generation, artifacts, execution, scheduling,
runners, and continuation remain deferred. It adds no runtime behavior or live authorization.

The accepted initiation boundary is recorded in [decision 0005](decisions/0005-sprint-5-initiation-scope-revision.md).
MCP correction/integration and external MCP investigation are deferred to Epic Planner authority;
they are not Sprint 5 evidence. Tool-specific, Harness, provider/live-proof, and MCP declarative
drift are deferred and must be updated only by their owning authority.

## Ownership and extension procedure

| Concern                                      | Current home                                                                                    | Extend by                                                                                                                                                 |
| -------------------------------------------- | ----------------------------------------------------------------------------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------- |
| Durable domain, commands, events, provenance | `src-tauri/src/orchestration/domain.rs`, `application.rs`, `repository.rs`                      | Add an application command and validated domain result before repository/event/provenance writes.                                                         |
| Native query and TypeScript decode           | Rust `repository.rs`; `src/application/orchestrations/nativeQuery.ts`                           | Version the contract, emit Rust golden fixtures, then add strict TypeScript decode/projection coverage.                                                   |
| MCP transport and semantic tools             | `src-tauri/src/orchestration/mcp.rs`                                                            | Add a semantic handler and schema; never expose SQL/CRUD or let a handler append events.                                                                  |
| Exposure and authorization                   | capability-profile tables/repository plus `mcp.rs`                                              | Add the tool to the profile exposure list and recheck profile, scope, expiry, preconditions, and idempotency server-side.                                 |
| Role-specific managed launch                 | `application.rs`, `transport.rs`, `src/application/orchestrations/managedPlanBuilderSession.ts` | Add an explicit role service/command and opt-in `RuntimeLaunchExtension`; do not wrap all Agent Session launches.                                         |
| Conversation Harness initial prefix          | `src-tauri/src/orchestration/conversation_harness_catalog.json` and its orchestration adapter   | Add or version a product-context configuration entry; keep Agent Session delivery neutral. Do not add tools, authority, lifecycle, or compression fields. |
| Codex Orchestrator skills                    | `.agents/skills/`                                                                               | Add concise canonical metadata and preserve relative shared references. Repository metadata proves discovery inputs, not live child selection.            |
| Future external services                     | a new infrastructure adapter behind an application port                                         | Preserve domain IDs and provenance; make availability/refresh/credential lifecycle explicit and add deterministic adapter tests before any live proof.    |

For a new semantic tool or role/profile, complete this sequence: define the contract and safe
description/input schema; review it for scope and secret leakage; add profile exposure and
server-side authorization; implement the application command/event/provenance and persistence;
version native query/projection if its durable result is read; add direct deterministic MCP,
decoder, and task/protocol evaluation; compose the role through an explicit managed service; then
separately authorize live proof. Tool visibility is not authorization, and deterministic proof is
not model selection.

Conversation Harness currently means only the product-supplied initial prompt prefix. The prefix
may give concise skill guidance, but it neither embeds `SKILL.md` nor makes a skill active. Codex
CLI receives repository-local skill metadata from `.agents/skills`; deterministic inspection can
prove that the metadata is present and valid from the repository working directory. Only a
separately authorized live managed child can prove actual discovery and model selection.

## Fact vocabulary

| Fact       | Meaning                                                       | Does not mean                                       |
| ---------- | ------------------------------------------------------------- | --------------------------------------------------- |
| requested  | An intent/command was asked for.                              | authorized or applied.                              |
| authorized | Policy, scope, and preconditions permitted it.                | received or persisted.                              |
| received   | The transport accepted a request.                             | valid or applied.                                   |
| applied    | The application command produced its intended domain effect.  | durable/readable.                                   |
| persisted  | The effect/event/provenance committed.                        | projected or reviewed.                              |
| projected  | A native/read-model projection represents the persisted fact. | observed runtime behavior.                          |
| observed   | A separate runtime or external outcome was recorded.          | reviewed or accepted.                               |
| reviewed   | A review result was recorded.                                 | responsibility accepted unless that result says so. |
| accepted   | The relevant explicit acceptance fact was recorded.           | later initiation/execution/continuation.            |

## Historical/deferred real-flow gate

This is retained boundary guidance, not an active Sprint 5 gate. Sprint 5 closure does not await a
real-flow run, paid/live proof, or MCP correction. Those questions are deferred to Epic Planner
authority.

The prerequisite UI flow is the accepted conversation-primary Epic Plan Builder: product composition
loads the native query, opens the managed Plan Builder session, and shows the persisted proposal
through its normal read path. The new segment to review is a submitted Plan Builder message ->
role-specific Tauri command -> short-lived localhost MCP endpoint/child credential -> one
`submit_epic_plan_proposal` call -> persisted proposal/provenance -> native-query refresh/projection.

Historical guidance for any later owner was to name the endpoint and credential lifecycles, retain
deterministic versus observed evidence separately, and keep installed-client behavior,
model-tool selection, paid/provider behavior, live end-to-end results, and restart recovery
explicitly unproven until separately authorized. No such later run is a Sprint 5 requirement.

## Historical/deferred G3 UI/UX review inputs

WU-S5-17 UI is accepted. No UI/G3 retest or additional authorization decision is required for
Sprint 5. The following material is retained as historical review context only.

The accepted component path is `App` -> `ApplicationRoot` -> `OrchestrationSection` ->
`EpicPlanBuilder`, with the shared Agent Session viewport/composer and the native
`ManagedPlanBuilderSessionClient`; native query reads enter through product composition. Sprint 5
technically changed the source from an unavailable/recorded proposal boundary to the native
pre-initiation query and added the managed-message command. It did not redesign this UI.

No safe screenshot is offered here: capturing the new segment would require a live Codex prompt.
For a local non-live review, start the app with its normal product composition, inspect the Plan
Builder route and its unavailable/empty durable state, and run the focused component/client tests
listed in the convergence record. The historical review questions concerned role/status clarity,
provenance detail, unavailable/live-boundary copy, and a separate paid/live gate. They are not open
Sprint 5 acceptance questions. Layout, tool naming, and visual redesign remain outside this record.

## Historical/deferred G4 revision notes

G1-D3's child-scoped configuration and provenance estimate is now implemented for the one-tool
Plan Builder slice. G1-D4's estimate became a fact: role-specific opt-in extension and managed
command were required; global wrapping was rejected. The current lock is exactly
`submit_epic_plan_proposal` under the managed Epic Planning profile.
Expansion remains possible through versioned contracts, explicit profile/role services, and new
application adapters. Capability uncertainty remains at the installed client/model/live-provider
boundary. A live gate would have been required before any later real-flow or paid/live claim; it is
deferred to Epic Planner authority and is not a Sprint 5 closure condition.

## Deferred conversation-tool observations

Future Agent Session UI should represent application-owned/product MCP calls as concise timeline
rows with tool name and status, expandable to sanitized parameters, result/error, timing, and safe
correlation/provenance identifiers. It must never display bearer tokens, credentials, or unredacted
sensitive fields, and must distinguish requested, received, applied, and persisted outcomes. This
requires a durable or safely projected tool-call observation contract separate from transcript prose.
Context copying and long-context navigation remain deferred alongside this work; no UI or runtime
implementation is authorized by this note.
