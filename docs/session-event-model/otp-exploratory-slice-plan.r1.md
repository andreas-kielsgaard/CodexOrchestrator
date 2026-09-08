# Exploratory slice: local Workflow OTP

Revision 1 — 2026-09-08. Planning only; implementation has not started.

## Objective and completion

Introduce one locally instantiated Orchestration Tool Package (OTP), the base Workflow OTP, and a small, explicit product API it can use. Move the existing continuation, handoff, invocation-completed reaction, and agent-delivery policy behind that boundary.

Completion means a configured product imports the package; its declared capabilities appear in MCP exposure and Workflow authoring; the engine routes configured events through package tooling; and the package can request a fresh session or prompt an exact session belonging to a bound node. Existing recipes and continuation/file-input behavior must still work. Deliver code, focused automated evidence, one live managed-runtime exercise, and a visible designer walkthrough when this plan is implemented.

This is an exploration of the boundary using existing behavior. It does not implement the full discussion/overview/plan/implementation/revision scenario or a general package platform.

## Evidence baseline

- Source task: `Workflow Scenario 1:` (`01a07a6b-a27c-73d2-adaa-096e42487016`).
- Worktree: `C:\Users\user\.codex\worktrees\workflow-continuation-files`.
- Branch: `codex/workflow-continuation-files`; HEAD: `5eaf5e4d43d90c7a15fb57ea11ee6740e53b11b9`.
- Initial inspection found the former dirty implementation committed as `feat: add workflow continuation and session file inputs`. At 15:15 CEST the source task had added new dirty validation work:
  - Modified `src-tauri/src/agent_sessions/application/tests/repair_tests.rs`: registers the live-test module; Git content hash `3073cf574b4fc4cc70075ea2511517bd4828d346`.
  - Untracked `src-tauri/src/agent_sessions/application/tests/repair_tests/live_continuation.rs`: ignored, feature-gated real Codex exercise; Git content hash `b89aa69ee84134525112cf3fdbeffad629bb118e`.
- That exercise covers approval by prompt, continuation fan-out, later clarification/revision, and file provenance. Its source is evidence of intended validation, not a passing run. No tests or provider calls were run for this planning pass.
- At 15:19 CEST, HEAD and the modified registration file were unchanged; the live exercise's hash had changed to `f693194bbb89ead99b18fdb95b558c96ba34311b`. It now accepts `WORKFLOW_LIVE_CODEX_PROGRAM` and describes approval as an instruction. The package boundary and planned sequence are unaffected; its eventual run result remains an entry-time evidence check.
- This plan is an additional document, outside that implementation baseline. Refresh HEAD, dirty contents, and source-task results before implementation; preserve the source task's changes.

Current code already has explicit continuation emission with trusted source context, zero/multiple matching connections, structured offered fields, multiple managed Workflow MCP tools, and durable harness file-change history. Those are not missing infrastructure.

Key source evidence:

| Boundary | Current source and implication |
| --- | --- |
| Capability declaration | `src-tauri/src/workflows/trigger_capabilities.rs` and `authoring_transport.rs`: continuation metadata/field validation are centralized but hardcoded to one capability. |
| MCP and observation | `workflows/mcp.rs`, `workflows/event_sources.rs`, `active_app.rs`: tool implementations explicitly route; the product observes completed invocations. |
| Recipe execution | `workflows/compiled_plan.rs`, `compiler.rs`, `execution.rs`, `instances.rs`: persisted instance recipe, source/destination connections, materialization and delivery attempts exist. |
| Session operations | `session_events/mod.rs`, `ports.rs`, and `agent_sessions/session_event_adapter.rs`: exact targeting and create-on-missing exist; always-create is not currently a first-class dispatch operation. |
| Harness exposure | `harness_engine/domain.rs`, `proxy.rs`: multiple participating tool names and trusted invocation preparation already exist. Existing sessions retain pinned profiles. |
| Inputs and evidence | `workflows/prompt_content.rs`, `file_inputs.rs`, `agent_sessions/repository/file_history.rs`, `docs/session-event-model/workflow-continuation.md`, continuation Rust/UI tests. |

Paths in this table are under `src-tauri/src/` unless explicitly rooted elsewhere.

## Concerns and decisions

| Concern | Uncertainty, coupling, and exposure | Resolution and evidence |
| --- | --- | --- |
| C1: Product/package ownership | Medium ambiguity, broad context and high rework risk if packages receive concrete engine services. | OTP-01/03: a real Workflow package works through only the curated API and a fake host. Start additive; remove migrated hardcoding at integration. |
| C2: Routing and compatibility | High complexity: compiler, event observer, package outputs and durable attempts must agree. Duplicate execution or changed recipe meaning has broad runtime impact. | OTP-04: old/new recipe equivalence, correlation and source-scoping tests. Preserve old snapshots; normalize references without rewriting them. |
| C3: Session commands | Well-defined intent, medium implementation uncertainty: fresh creation differs from missing-target creation; exact delivery must preserve initial prompts and records. | OTP-02: actual repository plus fake runtime proves fresh/exact behavior. Keep the extension local to dispatch. |
| C4: Import and discoverability | Medium ambiguity: package declaration must feed both MCP and designer, while pinned profiles remain authoritative. | OTP-01/05: configured import, catalogue parity, missing-package errors, save/reload and profile exposure evidence. No hot-reload design. |
| C5: Existing input semantics | Low definition uncertainty, medium regression risk at prompt construction. | OTP-04/06: preserve structured fields and harness-only historical attribution, including unavailable historical sessions. File-history ownership stays in the product. |
| C6: Moving baseline and live proof | Source task remains active; live test and installed-runtime outcomes are unverified here. | Entry gate and OTP-06: recapture source state; distinguish test code, passing fake-runtime tests, live provider results and visible UI results. |

## Chosen boundary

1. **Local import.** Use a compiled, in-process package factory selected by an explicit per-product `OtpHostConfiguration` import list. Initially import only `workflow@1`. The registry belongs to that product instance. New executable packages can require a rebuild; no external loader or new settings UI is needed.
2. **One obvious API.** Create `src-tauri/src/otp_api/` for descriptors, invocation context, value types and typed handles. Put implementations/adapters in `otp_host/`, and package code in `otp_packages/workflow/`. Package code depends on the API, not repositories, providers, Tauri state or `WorkflowExecutionService`. `active_app.rs` composes these; `lib.rs` receives only module wiring.
3. **Central observation and routing.** The engine observes session facts and supplies declared consumers with events for their configured source node/session. The package owns the invocation-completed reaction. It does not register its own runtime listener. MCP tools explicitly emit their declared outputs; merely observing arbitrary MCP traffic does not make it a trigger.
4. **One delivery path.** A connectable session output is a typed delivery request. The package selects new/existing and builds the request; the engine consumes it and performs the effect once through the session dispatch service. There is no direct package launch followed by a second automatic launch from its output.
5. **Compatibility.** Preserve `workflow_handoff`, `handoff_to_agent`, `trigger_workflow_continuation`, current fields, defaults and pinned session profiles. Existing recipes normalize to the base package bindings at compilation; new bindings record package/capability version references in the instance recipe. No silent fallback to a second runtime implementation when a package is unavailable.
6. **Authority remains shallow.** Product-owned runtime sessions qualify. The host supplies instance/node/session/invocation identity and checks that an exact session belongs to its bound node. Import adds available tools; existing Capability/Node Profile rules still govern exposure. No permission matrix or security subsystem is added.

The first API contains only:

| Handle | Offered operation/data |
| --- | --- |
| Invocation context | Trusted instance, source node/session/invocation/occurrence, tool binding, resolved input values and bound output node references. |
| Workflow definitions | Read a bound node or connection from the instance's pinned recipe, including its prompt and configuration references. |
| Node sessions | List available sessions for that node: identity, running state, creation/last-addressed ordering, and existing creation provenance needed by current selectors. No transcript/history API. |
| Session request construction | Request a new session for the bound node with prompt, or request a prompt to an exact session for that node. Constructors produce the session output; only host consumption performs the action. |
| Declared output emission | Emit the configured capability's output with schema-declared values; obtain routing/delivery outcome from the host. |

The API does not expose a raw event bus, arbitrary repository/service access, or a new file-history query handle. Node-file inputs continue to resolve through the existing product input service before package invocation.

## Contract to prove first

`managed MCP call / observed session fact -> engine -> declared OTP consumer -> declared output -> configured connection -> OTP prompt action -> SessionRequest -> engine session dispatch`

- Package descriptor: ID/version, required handle names, concrete MCP/session-event/action entrypoints, configuration inputs, and output schemas. Implement only the variants consumed in this slice.
- Base package entries: continuation MCP; existing handoff MCP; completed-invocation consumer; prompt-agent action. The consumer accepts completed invocations only, retaining today's behavior. The action owns existing selection policy and the explicit fresh-session choice.
- Delivery variants: `NewSession { prompt }` and `ExistingSession { sessionId, prompt }`, each bound to an output node. Existing cardinality `All` becomes a request for each selected session. Missing-target fail/no-op behavior remains as configured.
- Host context supplies source identity; package payload supplies declared data. Optional output files remain structured strings, not authorship claims. The designer offers schema-declared fields, not arbitrary MCP request/response inspection.
- Correlate source occurrence, connection/tool binding, output ordinal and resulting delivery. Preserve durable attempts and repeated-notification behavior. Assert a single launch path; do not promise distributed exactly-once delivery or add crash-replay machinery.
- The earliest consumer proof is one continuation call through a local managed proxy and real engine, using a fake provider, resulting in a persisted destination delivery. Establish this before migrating every trigger.

## Work packages

### OTP-01 — Importable package and public contract

**Outcome:** a product-configured local Workflow package exposes one authoritative descriptor through a small API. Addresses C1/C4.

Define `otp_api/`, product registry/import configuration in `otp_host/`, and the Workflow package factory. Register the required entrypoints and schemas; keep existing tool identities. Validate duplicate IDs, missing imports and unsupported contract versions. Use current capability metadata as the seed, not an unrelated plugin framework.

**Local acceptance:** import/omit fixtures, descriptor-to-MCP/catalogue parity, and package construction with an API-only fake host. No dynamic loading, second production OTP, permission framework or package manager.

**Dependency/gate:** baseline capture first. Freeze the small request/output contract with an executable fake consumer before OTP-02/03/05 split. Shape may change at this gate; record the change in this plan.

### OTP-02 — Node-specific session operations

**Outcome:** host adapters execute fresh/exact requests with existing session configuration, addressing and delivery records. Addresses C3.

Adapt Workflow instance definitions and `SessionDirectory` into the narrow reads. Extend the Session Event dispatch path minimally to create a fresh session even when candidates exist, without faking an empty query. Reuse real profile resolution, worktree binding, invocation dispatch and outcome persistence. New sessions receive node initialization once; existing sessions receive the new delivery text.

**Local acceptance:** with real SQLite storage and fake runtime, create two sessions on one node, prompt one exact ID, then create a third. Prove initial-prompt behavior, scope mismatch rejection, recorded outcomes and unchanged active-session rejection. Prove failed requests do not claim successful dispatch. No queues, interrupts or new session lifecycle states.

**Dependencies:** OTP-01. Can overlap OTP-03/05; owns session API adapters and the generic dispatch extension.

### OTP-03 — Workflow behavior implemented by the package

**Outcome:** required behavior runs against the API without concrete engine dependencies. Addresses C1/C4.

Move continuation and handoff argument handling/emission into the package. Declare the completed-invocation consumer and implement the prompt action's selection/prompt policy using supplied configuration, resolved inputs and node session summaries. The product retains observation, authoritative profiles and actual launches.

**Local acceptance:** fake-host tests prove empty/populated continuation arguments, handoff compatibility, completed-only consumption, current selector/default behavior, and new/exact request outputs. Calling continuation has no node closure or approval effect. The package uses no agent transcripts or raw database handles.

**Dependencies:** OTP-01. Can overlap OTP-02/05. Actual routing evidence belongs to OTP-04, not a substitute for these local tests.

### OTP-04 — One compiled route through the host

**Outcome:** active Workflow execution invokes package entrypoints and centrally consumes their outputs. Addresses C2/C5.

Wire import registry, managed MCP adapter, normalized session facts and package invocation in `active_app.rs`/`otp_host/`. Extend recipe/compiler bindings enough to identify producer capability/output, consuming action and bound node context. Keep exact source-session overrides in instance bindings, outside reusable recipes. Translate existing connections without a graph redesign. Preserve prompt contribution ordering, file-input resolution, source scoping and recorded attempts.

First prove continuation -> prompt action -> destination through the managed proxy with a fake provider. Then migrate handoff and completed-invocation routes. Remove the migrated active hardcoded paths so one event cannot execute both routes. Retain compatibility adapters for serialized recipes and isolated legacy tests, not a duplicate active executor.

**Local acceptance:** old/new recipe fixtures yield equivalent deliveries; zero/multiple connections and different source nodes/instances route correctly; repeat notifications do not add launches; declared fields stay typed; missing package/capability errors are actionable. Preserve original recipe/connection references in delivery evidence.

**Dependencies:** OTP-01/02/03. This is the integration owner for compiler, attempts and composition; avoid concurrent edits to those surfaces. No general deterministic executor or migration of unrelated application-event behavior.

### OTP-05 — Configure declared inputs and node outputs

**Outcome:** existing connection editing configures the imported capabilities without a continuation-specific registry. Addresses C4.

Extend authoring DTO/client/catalogue and the existing connection editor. Expose supported source event/capability, declared fields, source node/session binding, prompt action configuration and destination node binding. Default the source binding to the configured node's sessions; allow an exact session binding only within that instance/node context. Recipe templates keep node-level bindings until an instance exists. Preserve the existing node-files selector and add the explicit fresh-session choice.

Reuse current controls for these concrete schemas. Do not build arbitrary schema forms, a new canvas model or a package-management screen. Instance-specific session IDs must not leak into reusable recipe templates; place such overrides on instance bindings.

**Local acceptance:** mocked-catalogue UI tests prove import-driven options, schema field selection, save/reload, template/instance binding separation, pinned instance behavior and unavailable-capability messages. A visible walkthrough verifies the integrated flow in OTP-06.

**Dependencies:** OTP-01 for catalogue work; coordinate binding DTOs with OTP-04 before persisting them. Can overlap OTP-02/03, converges with OTP-04.

### OTP-06 — End-to-end evidence and exploration result

**Outcome:** demonstrate the boundary and document what it proves. Addresses C2/C5/C6.

Refresh and reuse the source task's live exercise after its state/results are known. Adapt it to import the package and record package/binding, source occurrence and resulting delivery identities. Add the fresh/exact session scenario. Retain output-path-versus-authorship, historical editor, scope and reopen checks. Use disposable workspaces/databases.

**Acceptance evidence:** focused OTP/Workflow/Session Event/Harness tests; existing continuation/history Rust tests and Workflow authoring tests; frontend build. Use `test:rust:fast` with relevant filters while iterating, then the affected Rust and frontend suites after integration. Inspect failures before broadening tests. Record actual commands/results at implementation time.

Run the feature-gated live Codex exercise separately from fake-runtime tests. Verify MCP exposure/call, receiver activation, persisted delivery and file-history facts separately. Capture a visible designer walkthrough for import-derived options, bindings, trigger fields and node-files inputs. A compiled live test is not a passing live run; the subprocess exercise is not desktop UI evidence.

**Dependencies/gates:** OTP-04/05 and local acceptance from every earlier package. Live completion requires an available authenticated Codex runtime; visual completion requires a runnable app/browser. If unavailable, report the exact evidence gap and leave that completion condition open. Publish an exploration conclusion: API sufficient, remaining coupling, and any required next slice. No merge/release is part of this plan.

## Sequence, deferrals, and status

1. Refresh the moving source baseline and any live-test results; preserve its dirty work.
2. Complete OTP-01 and its consumer-contract gate.
3. OTP-02 host operations, OTP-03 package behavior and OTP-05 catalogue/UI work can proceed as separate lanes after shared DTO agreement. This is a work dependency map, not an instruction to spawn agents.
4. OTP-04 integrates one continuation route first, then the other existing behaviors; OTP-05 converges on actual instance bindings.
5. OTP-06 completes automated, live and visible evidence, then revises the boundary assessment.

Shared surfaces are API types, recipe/binding DTOs, compiler and `active_app.rs`; assign one integration owner when implementation begins. Baseline drift, the first consumer proof and final runtime/UI availability are the remaining gates. No additional user product decision is required to start the planned first package.

Deferred: external OTP loading/distribution, new deterministic actions, arbitrary inference/tool-event subscriptions, richer session search/history, file-history API extraction, profile hot reload, full scenario authoring, approval machinery, revision coordination and broader security work. Existing file provenance stays harness-only; edits by later nodes retain earlier editor association. Missing/unknown attribution remains explicit.

Status: plan established, implementation packages not started. The initial clean-state observation was superseded by the two dirty validation files recorded above; future baseline changes must be recorded as evidence updates, not treated as work performed by this slice.
