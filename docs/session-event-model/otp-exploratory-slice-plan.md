# Minimal exploration slice: Workflow OTP

Revision 2 — 2026-09-08. Execution update: implemented and locally validated; changes uncommitted.

The original work packages below are retained as the execution specification. The completion
register at the end records actual outcomes and deviations. See
[OTP validation](otp-exploration-validation.md) for evidence and environment limits.

## Objective and completion

Implement one local Workflow Orchestration Tool Package (OTP), one curated product API for packages, and one active execution path through those contracts. Replace the affected Workflow designs directly. Do not retain old executors, recipe translators, MCP aliases, or compatibility adapters.

The package owns continuation/handoff behavior, the completed-invocation reaction, and destination-session selection. The product owns observation, routing, definitions, session storage, actual dispatch, and recorded outcomes.

The slice is complete when the product imports the package; its tools are exposed through the managed Harness; the designer configures declared inputs/outputs; continuation, handoff and completion events route through package behavior; new/exact-session operations work; and the affected predecessor paths are removed. Evidence must include focused automated checks, a live managed-Codex exercise and a short visible designer walkthrough.

## Current evidence and revision history

- Worktree: C:/Users/user/.codex/worktrees/workflow-continuation-files.
- Branch: codex/workflow-continuation-files.
- Inspected HEAD: f893e615feb0caf3f68ca5a74461af33fb8dfa22.
- Implementation commit: 5eaf5e4; subsequent f893e61 committed the live exercise and its result report.
- Before this revision, the only untracked file was this planning document. The previously dirty implementation/test work is committed.
- The committed report docs/session-event-model/workflow-continuation-live-validation-2026-09-08.md records six real Codex invocations across four nodes, three continuation calls with delivery counts 2/1/0, and seven persisted create/edit records. The retained verified-results.json was also inspected and reports passed.
- That run used real provider processes with an in-process proxy host. Native child-sidecar startup and the Tauri window were not covered. Its four-node scenario is a regression baseline, not evidence that OTPs exist.
- At planning time, no tests, provider calls or implementation edits had been performed.
  Execution subsequently completed the work below against the same HEAD, with focused tests,
  a fresh real-Codex exercise and a visible browser-component walkthrough.

Revision 1 is archived in [otp-exploratory-slice-plan.r1.md](otp-exploratory-slice-plan.r1.md). It proposed recipe normalization, compatibility adapters and an instance-specific source-session editor. Revision 2 removes those requirements. The actual source baseline is now committed and live-validated; “pending live-test source” is superseded.

## Minimal design decisions

1. One compiled, in-process package, identified as workflow. An explicit per-product import list instantiates it at startup. No external loader, package manager, hot reload, second production package or package-settings screen.
2. Three focused sections: otp_api for the public contract, otp_host for product adapters/transport, otp_packages/workflow for behavior. Package code imports only otp_api plus ordinary libraries, not concrete product services.
3. The engine observes terminal-session facts. A package-declared consumer decides whether a fact emits its completion output. MCP calls explicitly emit outputs; no raw-MCP observer routes them.
4. The package emits a data output or a node-bound SessionRequest. The engine routes data outputs and executes session requests once. Package code never launches a provider itself.
5. Source node and capability/output identity select connections. The output node binding gives a tool context to list candidates and choose new/existing sessions. This slice uses node-level source bindings; each observed event still identifies its exact source session. An exact-source-session override UI is deferred.
6. A new Workflow recipe contract, version 2, directly stores OTP bindings. Recreate exploration recipes/instances. No v1-to-v2 translator, old trigger fallback or replay compatibility. Leave stored old data intact; list only supported recipe/instance contracts and return an explicit unsupported-contract error on an old ID.
7. Use the managed MCP server name workflow, with tools trigger_workflow_continuation and handoff_to_agent. Replace workflow_handoff rather than maintaining an alias. Existing session profiles are immutable; demonstrations use new profiles/sessions with the imported tools.
8. Same product-owned runtime sessions qualify for the exposed handles. Trusted caller/session identity and node membership remain correctness checks. No new authority framework.
9. Harness file-history ownership and selection semantics remain unchanged. File parameters never establish authorship. No OTP file-history API is needed: the product resolves existing node-file inputs.

### First API and package contents

| API surface | Narrow contract |
| --- | --- |
| Invocation context | Product/Workflow instance, source node/session/invocation/occurrence, current tool/binding, resolved inputs and output-node bindings, supplied by the host. |
| Definition reads | Read a bound node or connection from the pinned instance recipe, including its prompt and configuration references. |
| Node-session read | List available sessions for a node: ID, running state, creation/last-addressed ordering and existing created-by references. No transcripts or general search. |
| Session requests | NewSession(node binding, prompt) and PromptSession(node binding, exact session ID, prompt). Constructors produce requests; only host execution has effects. |
| Output emission | Emit a declared output with JSON values. Host attaches source context, routes matching connections and returns delivery outcome. |

The Workflow package contains exactly four behaviors:

- trigger_workflow_continuation: optional outputFiles array; emit continuation, including the host-provided sourceNode field.
- handoff_to_agent: filePaths array and promptText; publish declared fields and emit handoff. Remove string-flattening and old participation-protocol baggage.
- on_invocation_completed: consume normalized terminal facts, emit only for completed invocations; offer final output and source identity.
- prompt_agent: consume configured prompt inputs and node context; apply a fresh-session mode or the existing selection rules; emit explicit new/exact session requests.

Both MCP outputs route only connections configured for that capability/output from the calling node. Zero/multiple connections are valid; handoff need not retain the former “zero matches is an error” quirk. Neither action closes a node or implements approval.

The minimal configuration editor renders the field types these declarations actually need: text, enums and their required grouping/visibility. Do not implement a general JSON Schema form engine. Node bindings and file-input controls remain product controls.

## Concerns, uncertainty and gates

| Concern | Assessment and coupling | Resolution |
| --- | --- | --- |
| C1: Real package boundary | Medium design uncertainty; API mistakes cause broad rework. Additive files are easy to revise before integration. | OTP-01/03: execute real package handlers with a fake API host; no engine imports. |
| C2: Caller context and routing | High coupling across proxy, host, compiler and attempts; duplicate dispatch has runtime impact. | OTP-01/04: one managed-proxy call produces one recorded route per matching connection. Preserve source/occurrence correlation. |
| C3: New versus existing sessions | Low ambiguity, medium shared-dispatch risk. Existing code only creates on missing targets. | OTP-02: real storage/fake runtime proves explicit fresh creation and exact delivery, including initialization and failure outcomes. |
| C4: Clean removal | Broad reference context, mechanical deletions; storage and shared canvas helpers are the principal risks. | OTP-04/05: move shared helpers first, remove predecessor registrations/consumers, compile and inspect remaining references. No user-data deletion. |
| C5: Offered data and provenance | Defined behavior; medium risk where prompt construction changes. | OTP-04/06: typed arrays/objects survive; historical node-file associations and output-path separation still pass. |
| C6: Evidence | Live provider behavior was observed before OTP; new native/UI behavior remains unproven. | OTP-06: repeat live proof on the new route and inspect the actual designer; identify environment gaps separately. |

Before implementation: recheck HEAD/status for drift. Before parallel work: an executable API/consumer contract and shared binding DTOs must exist. Before retiring each old path: its replacement consumer must pass locally; the finished slice contains no toggle or dual executor. These are engineering evidence gates, not additional approval requirements.

## File-level work packages

Unless prefixed with src/ or docs/, paths below are under src-tauri/src/. New file names are the proposed layout.

### OTP-01 — Package import and generic managed MCP entry

Outcome: an imported package supplies tool metadata and receives typed arguments plus trusted context. Addresses C1/C2. No new Workflow effects yet.

**Create**

| Files | Responsibility |
| --- | --- |
| otp_api/mod.rs, contract.rs, handles.rs | Package/tool/output descriptors, typed context and scoped handle traits, data outputs and session requests. Declare only the three required entrypoint forms: MCP, session-event consumer, routed action. |
| otp_host/mod.rs | Product-owned registry, explicit import configuration, package lookup and per-invocation host construction. |
| otp_host/mcp.rs | One managed MCP adapter: tools/list and tools/call derive from imports. Reuse the working HTTP/listener mechanics from workflows/mcp.rs without its tool-specific dispatch or Legacy enum. |

**Edit**

- harness_engine/domain.rs: replace workflow_tool_names and workflow_prepare_url with generic managed caller-context participation. Remove the workflowToolName alias/deserializer and old snapshot-compatibility test.
- harness_engine/proxy.rs: send trusted session/runtime/invocation metadata to the OTP host separately from tool arguments. Delete the /prepare round trip, _workflowInvocation injection, x-workflow-* headers, handoff-warning insertion and their helper branches. Keep ordinary proxy access filtering, streaming and lifecycle.
- harness_engine/service.rs and sidecar.rs: update descriptor/transport fixtures and caller-context registration tests. The existing session identity/current-invocation binding is sufficient; do not expose legacy Workflow identity fields as OTP authority.
- orchestration/mcp.rs and orchestration/application.rs: mechanically update their ManagedMcpUpstreamDescriptor construction. Their tools do not become OTPs in this slice.
- active_app.rs and lib.rs: import construction, generic host registration and minimal module declarations. Derive the available managed tool set from imports instead of hardcoded Workflow constants.

**Local acceptance:** import/omit behavior; tools/list matches the package descriptor; typed arguments arrive unchanged; proxy caller context is independently supplied; non-OTP MCP traffic is unaffected. No capability-specific tool name tests inside the proxy implementation.

Dependencies: entry baseline check. Freeze this small contract with a fake consumer before OTP-02/03 and UI contract work split.

### OTP-02 — Scoped node/session handles and one dispatcher

Outcome: the host can execute a fresh-session or exact-session request using current session machinery. Addresses C3.

**Create** otp_host/workflow.rs: adapters for pinned node/connection reads, available node-session summaries, output emission and session-request translation. Pass a scoped adapter into each handler; avoid a service locator or a registry/engine ownership cycle.

**Edit**

- session_events/domain.rs: introduce an explicit new-session target at a logical address. Keep exact/logical addressing for other current Session Event consumers; do not expose the whole generic command model to packages.
- session_events/mod.rs and addressing.rs: handle deliberate creation directly, then use the common initialization/dispatch/recording path. Do not manufacture “no candidates” to activate create-on-missing.
- agent_sessions/session_event_adapter.rs: adapt only where required for new/exact operations; preserve profile resolution, chosen worktree, persisted logical address and active-invocation rejection.
- session_events/tests.rs: fresh/exact, initialization and failed-delivery assertions.
- src/application/sessionEvents/contracts.ts, src/features/sessionEvents/TargetSelectionEditor.tsx and EventGroupInspector.tsx: represent/render the actual recorded new-session target. Update exhaustive presentation/default handling where these types are consumed.

A package data output can use the existing application-event identity in the generic delivery record, with the full OTP source/binding/payload recorded in the Workflow attempt. No OTP-specific trigger hierarchy is necessary inside the generic Session Event kernel.

**Local acceptance:** create two sessions for a node; prompt one exact ID; request a third; verify node membership, initialization once, stored outcomes and unchanged busy-session behavior. A request receipt must not cause a second launch. Test with SQLite and a fake runtime.

Dependencies: OTP-01. No queues, interrupts, retries or new lifecycle states.

### OTP-03 — Base Workflow package behavior

Outcome: the required behaviors are implemented entirely through the API. Addresses C1/C5.

**Create**

- otp_packages/mod.rs: the explicit local factory catalogue.
- otp_packages/workflow/mod.rs: package declaration and required handle list.
- otp_packages/workflow/tools.rs: continuation, handoff and completed-invocation handlers.
- otp_packages/workflow/prompt_agent.rs: prompt assembly and session selection from supplied inputs/configuration and node-session summaries.

Move metadata and behavior from workflows/trigger_capabilities.rs, workflows/mcp.rs and the completion-specific part of workflows/event_sources.rs. Move the Workflow selection policy out of connection-to-generic-target compilation. The package owns the selection configuration schema, including fresh versus select, and the already-supported ordering/cardinality/filter choices.

The host resolves file/node-file inputs and supplies provenance-bearing prompt contributions; the package assembles the delivery prompt. The host applies the destination node's initial prompt only when it actually creates a session.

**Local acceptance:** API-only fake-host tests exercise optional continuation output files, handoff JSON fields, completed-only consumption, current selector rules, fresh/exact requests, and no implicit approval or closure. No repository/provider imports. No additional production tools or event categories.

Dependencies: OTP-01; can overlap OTP-02. Their real convergence belongs to OTP-04.

### OTP-04 — OTP recipe execution and backend retirement

Outcome: every affected Workflow route uses declared OTP bindings and one dispatcher. Addresses C2/C4/C5.

**Rewrite/edit**

| Files | Required change |
| --- | --- |
| workflows/authoring.rs, compiled_plan.rs | Replace WorkflowConnectionTrigger and engine-owned target policy with capability/output references, action reference/configuration and node bindings. Use recipe contractVersion 2. Prompt inputs become declared output field, node-files or explicit file content, plus fixed prompt text. |
| workflows/compiler.rs | Compile an OTP execution plan, not one executable SessionEventDefinition per edge. Validate declarations/bindings through the imported registry. User entry also reaches the same prompt-agent action; no second Workflow selection implementation. |
| workflows/authoring_service.rs, authoring_transport.rs | Registry-backed configuration validation and catalogue. Return the new compiled-plan DTO. Remove list_workflow_trigger_capabilities and replace it with the imported-capability query. |
| workflows/event_sources.rs | Retain centralized observation/normalization and routing. Delete trigger_continuation, hardcoded matches_source and the conversion of package outputs into synthetic raw MCP observations. Route normalized facts into declared consumers and their outputs into configured actions. |
| workflows/execution.rs, execution_transport.rs | Execute the compiled tool plan and consume typed session requests. Retain real user-node messaging, instance targeting and delivery records. Replace dispatch_compiled_occurrence as needed; do not keep it as an alternate old edge executor. |
| workflows/instances.rs | Record source occurrence, capability/output, connection/action, selected target requests, and resulting delivery references/outcomes. Keep current attempt-before-dispatch behavior without adding replay infrastructure. |
| workflows/prompt_content.rs | Resolve ordered input values directly. Remove the positional “definition zero is user entry, subsequent definitions correspond to connections” rewrite and trigger-field JSON hidden inside reference IDs. Preserve current file reads and node-file resolver. |
| workflows/authoring_repository.rs, instances.rs | Read/write the new contract only; filter unsupported versions when listing and reject explicitly requested older records. No translation or deletion of saved user data. |
| active_app.rs | One event observer/Workflow executor and OTP MCP registration. Remove the old WorkflowApplication state, notifier callback and workflows::transport command registrations. |

**Delete the replaced backend**

- workflows/trigger_capabilities.rs and workflows/mcp.rs.
- The old Workflow stack: workflows/application.rs, domain.rs, repository.rs, transport.rs, node_sessions.rs and legacy_node_configuration.rs.
- harness_engine/workflow_adapter.rs.
- In harness_engine/service.rs: old WorkflowSessionHarnessBinder implementation, old compile_plan based on WorkflowHarnessConfig, and its dead fixtures. Retain SessionProfileHarnessAuthority and general managed MCP behavior.
- In harness_engine/catalog_service.rs: materialize_workflow_harness and its predecessor-only tests.
- In workflows/mod.rs and harness_engine/mod.rs: the deleted module registrations.
- In workflows/instance_domain.rs: remove old EffectiveRecipe/activation/session-summary records; retain the four repository/branch/worktree target types used by the current engine.
- In storage.rs: remove obsolete Workflow repository schema setup, presence checks, migration branches and tests that exist only for the deleted stack. Keep other product migrations, generic Harness storage and current recipe/session stores. Do not drop existing user tables as part of startup.

This deletion set is specifically the duplicated Workflow implementation and its callers. It does not include the separate Epic/Sprint orchestration product, generic Session Event functionality, or the archived task/run code in lib.rs.

**First integration gate:** one managed MCP continuation -> imported handler -> configured prompt action -> persisted destination delivery with a fake provider. Then exercise handoff and completed-invocation output. Finish removing predecessor paths after their replacement checks pass.

**Local acceptance:** zero/multiple connections, source-node/instance isolation, typed payloads, repeated observed-notification handling, selected-session correctness, prompt contribution provenance and useful missing-capability failures. Compiler/build and a reference scan must show no active or retained old Workflow executor, MCP participation protocol, or trigger catalogue.

Dependencies: OTP-01/02/03. Own the shared compiler/composition edits here.

### OTP-05 — Designer on the declared contract; retire old UI

Outcome: the active designer configures the imported package directly. Addresses C4/C5.

**Create** src/features/workflowAuthoring/OtpConfigurationEditor.tsx for only the configuration field forms required by this package.

**Edit**

- src/application/workflowAuthoring/contracts.ts: replace old trigger/target DTOs with capability/output/action bindings, catalogue descriptors and the new compiled plan.
- src/infrastructure/workflowAuthoring/tauriWorkflowAuthoringClient.ts: the catalogue and compiled-plan commands.
- src/application/workflowInstances.ts and src/infrastructure/workflowInstances/tauriWorkflowInstanceClient.ts: new attempt/plan identity fields.
- src/features/workflowAuthoring/WorkflowConnectionEditor.tsx: declared trigger output, action configuration and output-node binding. Remove the raw TriggerBindingEditor fallback and Workflow-to-SessionEvent conversion helpers.
- WorkflowPromptInputsEditor.tsx: show only the selected output's declared fields; keep node-files/existence and explicit file inputs.
- WorkflowAuthoringScreen.tsx, WorkflowRunPanel.tsx, WorkflowInstancePanel.tsx, workflowAuthoringPresentation.ts and testFixtures.ts: catalogue, plan/attempt rendering and version-2 defaults.
- WorkflowCanvas.tsx: update shared utility/style imports after the moves below.
- src/app/App.tsx: remove workflowClient and the old WorkflowScreen fallback. The real bootstrap already uses workflowAuthoringClient/workflowInstanceClient; retain those.
- Relevant authoring/client/instance tests: use the new contract, rather than testing old/new equivalence.

**Move before deleting**

- src/features/workflows/editor/workflowNodeDrag.ts and its test -> src/features/workflowAuthoring/.
- The canvas styles still consumed from src/features/workflows/workflow.css -> workflowAuthoring/workflowCanvas.css; retain only styles used by the current canvas.

**Delete** the remaining src/features/workflows/, src/application/workflows/ and src/infrastructure/workflows/ files, plus src/app/App.workflows.test.tsx. Replace relevant current-product navigation coverage in the active authoring tests.

**Local acceptance:** declaration-derived options and fields, save/reload/activation, fresh versus selected-session configuration, current canvas drag behavior and node-file inputs. No raw arbitrary MCP/application-event trigger controls in the Workflow designer. No new canvas system, package-manager UI or exact-source-session override panel.

Dependencies: OTP-01 and binding DTO agreement with OTP-04. UI work can overlap backend work after that agreement; final integration must use the real catalogue.

### OTP-06 — Focused proof and implementation account

Outcome: establish what the new boundary actually supports. Addresses C1–C6.

**Edit** agent_sessions/application/tests/repair_tests.rs and its continuation_tests.rs/live_continuation.rs modules to instantiate the imported package and host, configure the new recipe, and call the workflow MCP server. Retain provider/file-event evidence; do not simulate authorship from arguments.

**Add tests** alongside otp_api, otp_host and otp_packages for their local contracts. Update existing Workflow compiler/execution and frontend authoring tests. Remove tests whose only purpose was the deleted compatibility paths; preserve generic proxy/profile/storage guarantees using current fixtures.

**Required evidence**

1. Package import/omit, catalogue/MCP parity and API-only package execution.
2. Managed-proxy-to-engine routing with fake provider and real storage: zero/fan-out, declared field typing, scoping, fresh/exact sessions, initial prompts, failure outcomes and no double dispatch.
3. Existing file-history checks: a later editor does not remove an earlier association; archived sessions remain part of history; missing/unknown attribution stays explicit; MCP paths add no history.
4. Focused Rust suites covering changed OTP, Workflow, Session Event, Harness and storage boundaries; frontend tests for changed contracts/current flows and npm run build. Use test-fast while iterating. Do not claim the unrelated full suite passed unless it was run.
5. Repeat the live exercise on the new import/route in a disposable database/workspace. Add fresh/exact selection evidence; retain actual MCP, delivered prompt and file-history artifacts.
6. A short visible designer walkthrough: select declared output, configure action/output node, save/reload, include node-file input and inspect a resulting delivery. Explicitly distinguish in-process-proxy proof from native child-sidecar proof.

**Documentation:** rewrite docs/session-event-model/workflow-continuation.md around package import and current bindings. Keep the existing dated live-validation report as historical evidence. Create docs/session-event-model/otp-exploration-validation.md with actual results, remaining coupling and whether a further slice is needed.

Dependencies: all earlier local acceptance plus OTP-04/05 integration. Live proof needs the authenticated compatible Codex executable; the prior report identifies a working bundled CLI. If runtime/UI access is unavailable, leave that specific completion condition open. No merge, release or automatic database reset belongs to this plan.

## Sequence, boundaries and status

Preferred sequence: OTP-01 contract/import -> OTP-02 host operations and OTP-03 package behavior -> OTP-04 first continuation route -> remaining routes/backend retirement -> OTP-05 real designer integration/UI retirement -> OTP-06 evidence.

Parallel opportunities after contract agreement: host operations, package handlers and catalogue-driven UI. Shared API/recipe DTOs and active_app.rs/compiler edits have one integration owner; do not split competing changes across those surfaces. No subagents are being requested or started by this plan.

Kept product infrastructure: harness file-event normalization/storage, node-file aggregation, node profiles, session identities, worktree targets, generic Session Event dispatch/storage, current canvas and agent-session UI.

Deferred: external loading/distribution, new deterministic tools, arbitrary tool/inference-event subscriptions, extra session/history APIs, source-session override UI, file-history extraction into OTP, hot reload, authority framework, full scenario authoring, approval enforcement and revision coordination.

The shared Session Event new-target change, retired storage references and prompt/attempt
identity were the main integration risks. Focused tests and the live exercise now cover these
boundaries. Breaking old Workflow recipes/tool-server exposure is intentional; existing records
remain intact. Native sidecar/window and the full unrelated test suite remain outside the
observed evidence.

## Completion register

| Step or gate | Status and evidence |
| --- | --- |
| Baseline | Confirmed branch/HEAD above; preserved the main checkout and historical report. |
| OTP-01 | Complete: local registry, declarations, generic managed MCP adapter and trusted caller context. Import/catalogue/proxy checks passed. |
| OTP-02 | Complete: bound reads, returned new/exact requests and one dispatcher. Real storage/fake-provider checks and live fresh/exact selection passed. |
| OTP-03 | Complete: exactly four behaviors in the API-only Workflow package; five package tests passed. |
| OTP-04 | Complete: recipe contract 2, central observation/routing, durable attempts and backend retirement. Unsupported data retained; no executor fallback. |
| First integration gate | Passed: managed proxy continuation reached the imported action and persisted destination delivery. Handoff and completion paths also passed. |
| OTP-05 | Complete: real catalogue-driven editor, output-node/session configuration, file inputs and retired UI removal. Save/reload/activation and visible walkthrough passed. |
| OTP-06 | Complete: 107 focused Rust tests, 17 frontend tests, production checks/build, eight-invocation real Codex exercise and implementation account. |
| Convergence | Complete locally. No pending implementation packet, user decision, merge or release. |

Execution used this conversation sequentially; no agent delegation occurred. Two unused alternate
panels were deleted instead of adapted. Shared dialog CSS also moved before old UI deletion.
Duplicate frontend Session Event target types were both updated. Obsolete event-definition
helpers were removed and production reused the shared node-address helper. The visible
walkthrough found delivery details constrained to the sidebar; the inspector now uses the main
pane, with a return control and focused regression coverage.

The visible walkthrough used actual components, the real serialized package catalogue, browser
local storage and recorded real delivery data. Live provider proof used an in-process proxy host.
These establish the requested local slice; they do not establish native Tauri command transport,
native child-sidecar startup, packaged restart, full accessibility or release acceptance.
