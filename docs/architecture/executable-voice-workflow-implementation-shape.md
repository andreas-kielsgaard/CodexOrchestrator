# Executable Voice workflow — implementation shape

Status: implemented and locally verified on 2026-10-21, including remote S21 capture and correlated playback. Changes are on `codex/voice-extension-workflow`; remote-server relay remains outside this slice.

Baseline: `552c66ab`, including the independent-device implementation and committed Voice UI refinements.

## Completion evidence

- Responsiveness experiment (2026-10-21): the normal selected design is revision 5, with ten components and 17 connections. The saved design replaces the summarizer with a configured immediate speech receipt, removes duplicate summary/completion fan-out, uses Fast mode in the Voice capability profile, and retains GPT-6.1 medium for coordination / high for delegated work. The prior design/profile were retained in `.dev/voice-deployment/before-tuning.json`; `reports/voice-tuned-design.json` records the active design. These are edits to the existing saved design, not changes to the default seed.
- Measured text-request receipt publication fell from 14.73–18.92 seconds in two earlier runs to 0.62 seconds (0.022 seconds after acceptance). The first Fast-mode-only experiment still took 17.08 seconds. The full response through coordinator/editor/output remained 49.23 seconds; no general answer-speed improvement is claimed. A JEV-classified direct follow-up published in 11.35 seconds with no new assignment (`reports/voice-direct-followup.json`). These are individual observations, not a statistical benchmark or microphone-to-audio latency measurement.
- The S21 production-service probe completed the requested reply through Bluetooth / Beoplay H95 (`reports/voice-immediate-phone-probe.txt`). A real high-reasoning worker performed a read-only gateway authentication review, returned its result with assignment/revision identity, and distinguished source findings from runtime proof (`reports/voice-deep-work-result.json`). Its resolved profile confirms GPT-6.1 / high / Fast mode.
- Android's Components tab now fits the whole saved design into one spatial map. Selection highlights incoming/outgoing connections and shows the role above the map; model, sessions and labeled connections are below, with raw evidence in Activity. Roles come from agent instructions or the configured service; positions and connections come from the selected instance. Native map taps and the accessible component picker were verified on the S21 (`reports/voice-workflow-map-final.png`, `reports/voice-workflow-map-selected.png`).
- Current release build and installed Android APK passed; 27 focused desktop Voice tests passed, including configured receipt payload isolation and preservation of normal publication metadata. The normal host remains enabled at the existing phone endpoint.
- Normal deployment: the release executable at `src-tauri/target/release/codex-orchestrator.exe` now serves the phone's existing `ws://100.105.52.29:4818/phone` connection using normal application data. A clean restart restored the selected instance and gateway without a start command (`reports/voice-normal-deployment.json`). This remains laptop-hosted; server relay is deferred.
- Android now has a read-only Workflow page with agent/service status, session links and expandable routing attempts, delivery receipts, errors and input/output previews. The authenticated `/workflow` endpoint supplies the selected instance; the page refreshes while open and retains its last data on connection failure. S21 checks verified routing expansion, recipient conversation navigation and refresh after desktop restart.
- The normal-endpoint phone probe captured live microphone audio and completed playback of the exact requested response (`reports/voice-normal-phone-probe-final.txt`). The phone's follow-up preference is now 120 seconds because the previous 30-second disconnect interrupted slower replies. Media volume was restored to zero; microphone capture was stopped and the desktop gateway left enabled. The phone sends this preference on its next connection.
- Release and Android builds passed; the APK is installed on the S21. The added phone workflow endpoint passed the focused 25-test desktop Voice suite (`reports/voice-phone-view-rust-tests.txt`); unauthenticated requests returned 401. Earlier isolated-build evidence below records the preceding implementation checks.
- Saved V4 recipes now execute agent and capability nodes through the shared OTP host. Voice uses the ordinary design editor and instance worktree selector; the seeded design has ten nodes and 23 connections.
- Transcript evaluation, input buffering, delegation/returns, progress, external observations and publication follow configured connections. The old fixed topology, private delivery loop and Voice MCP extension are removed. Shared steering retains acceptance receipts and retries expired amendments subject to task-revision supersession.
- Voice conversation state is isolated by instance; schema 67 preserves the old singleton record. The default design identity survives renaming. Device connection settings remain local.
- Core tests: 441 passed, one ignored (`reports/voice-core-final-2.txt`). Desktop Voice/schema tests: 25 passed (`reports/voice-runtime-final-2.txt`). Related UI tests: 21 passed; the final Voice-only rerun passed all four tests. Scoped ESLint, TypeScript, native build and `git diff --check` passed.
- Live graph test: removing the saved transcript connection produced one recorded input, zero evaluations, zero agent sessions and zero publications (`reports/voice-disconnected-graph.txt`). Restoring the design affects new instances only.
- Live delegation test: the configured Worker calculated 12 × 7, reported assignment revision 1, returned to the requesting Router, and Output published the result. The shared receipt tests separately cover accepted, expired and superseded steering.
- A fresh isolated copy of pre-migration data seeded and activated the final design without manual profile repair (`reports/voice-final-seed.json`). The shared editor was inspected through the owned WebView (`reports/voice-final-ui.png`).
- Final Android instrumentation used the production microphone/service, submitted a text test request, and matched the exact spoken utterance to completed playback. The S21 connected over Tailscale at `100.90.249.49:5555`; the phone speaker played “Voice workflow is connected.” (`reports/voice-phone-final-correlated.txt`, `reports/voice-final-runtime-evidence.json`). Capture/transcription were live; the test request itself was text, not a spoken test utterance. The original endpoint and muted volume were restored.
- Final desktop executable: `%LOCALAPPDATA%/CodexOrchestrator/builds/f2ad93e501f2e624/563a6d1b-4075-4a3b-8cea-b6bdcd9687ec/output/codex-orchestrator.exe`. Android application and instrumentation builds passed and were installed on the S21.

## Resolved implementation placement

Default recipe import remains in desktop `voice_sessions/default_design.rs`, where existing local settings and Capability Profiles are available. Portable OTP contracts and prompt assets live in core. Evaluation and buffering are separated into `evaluation.rs` and `input_buffer.rs`; `prefilter.rs` retains transcript scheduling. Context and publication have their own modules. Provider-native options use ordinary Capability Profile routes; legacy group-level fast-mode switches no longer override node execution.

The shared editor replaces the former Voice workflow panel and route inspector; operation attempts and Session Event receipts provide execution evidence. Legacy activity decoders remain. Playback diagnostics now include utterance identity and text so verification cannot confuse an acknowledgement with the requested reply. Validation combines shared fake-service graph tests with the real Voice/JEV/provider/phone flow rather than introducing a separate fake audio backend.

## Target and scope

A saved Workflow design determines Voice's agent composition, prompts, models/tools, session reuse and event connections. Starting Voice creates or resumes a Workflow instance. Editing a connection changes actual delivery. Voice supplies executable capabilities for audio, typed evaluation, buffering, task bookkeeping and publication; it does not select hidden recipients.

This slice proves that locally in the desktop application. Remote instance creation, phone relay through `orchid-server`, Linux speech setup, cross-device workflow execution and the proposed remote-control OTP follow later. Keep the existing phone protocol and local provider execution paths. Do not add a second workflow engine, external package loader, general scripting language or distributed scheduler.

## Implementation choices

- Extend ordinary Workflow recipes with agent and capability node bodies. Capabilities are declared by OTP packages; no Voice-specific node kind enters the generic engine. A typed JEV evaluation is a capability invocation with retained query evidence, not an ordinary chat turn.
- Keep Voice runtime implementations under `src-tauri/src/voice_sessions` for this slice, separated from their workflow adapter. Core owns package declarations and generic execution ports; desktop composition supplies the Voice service implementation. Moving the audio implementation to a headless host is not required to make the design executable.
- Reuse the existing draft/activate/instance-copy behavior. Design changes apply to a new instance; reconnecting the phone resumes the existing instance. Do not add version management or live graph replacement. Make this different edit behavior explicit in the UI.
- Use the existing local Workflow worktree selector for new Voice instances. Import the old working directory only if it resolves to a real registered target; otherwise require selection at instance creation. Do not fabricate repository or worktree identities.
- Retain one active phone connection per desktop process. Persist Voice state by workflow instance, so stopping one instance does not overwrite its history when another is created. Multi-phone operation is outside this slice.
- Preserve active-turn steering as an explicit agent-delivery option. Existing workflows retain their current queue behavior by default.

## Findings that determine the shape

| Current owner | Source finding | Consequence |
| --- | --- | --- |
| `src-tauri/src/voice_sessions/workflow.rs` | `EDGES`, `targets`, `sessions` and `evidence` encode topology, role bindings and inferred connection evidence. | Replace executable topology with a normal recipe; evidence must come from actual workflow attempts. |
| `application.rs`, `pipeline.rs` | `start` and `ensure_pipeline` directly create Router, Summary, Digest, Output and JEV sessions. Prompts and permissions depend on role names. | Workflow instances own agent birth and membership; capabilities replace role-name authorization. |
| `prefilter.rs`, `observations.rs`, `delegation.rs`, `tasks.rs` | Evaluation, buffering, worker selection, return routing and steering are intertwined. Several recipients bypass `EDGES`. | Split deterministic state operations from output emission; migrate all delivery paths, not just the visible graph. |
| Core `workflows/authoring.rs`, `compiled_plan.rs`, `compiler.rs` | Every node has session-creation fields; producers cannot be actions; destinations must produce session requests/stops. | Add capability bindings and permit declared capability outputs/actions without fake sessions. |
| Core `otp_api`, `otp_host/workflow.rs` | A source contains session/invocation IDs; action input loses structured payload when it does not use prompts. | Represent capability sources and carry structured payload independently of rendered prompt inputs. |
| Core `workflows/execution.rs`, `event_sources.rs` | General graph compilation also selects an entry node; event-driven calls compile with no entry choice. | Separate graph validation/compilation from user-entry selection so multiple entry nodes do not break runtime events. |
| Core `agent_sessions/session_event_adapter.rs` | Workflow delivery uses `deliver_profiled_message`; Voice uses `deliver_or_steer_product`. | Adapt shared delivery rather than retaining Voice's separate observation-delivery loop. |
| Core `workflows/instances.rs` | Instances already retain a recipe copy, session attempts and real worktree targets. | Reuse this persistence and identity model. |
| Desktop `active_app.rs` | Voice is composed/restored before Workflow execution exists; both receive session notifications. | Reorder composition and remove duplicate routing of migrated sessions. |
| `VoiceBehaviorEditor`, `WorkflowNodeEditor`, `otpPresentation` | Voice edits separate settings; ordinary editors assume agents and session-producing actions. | Use one recipe draft and extend shared editors for capability nodes. |

## 1. Recipe, compiler and catalogue ownership

Adapt `crates/orchid-core/src/workflows/authoring.rs` and `compiled_plan.rs` to give each node a common identity/name/position and a tagged body:

- `agent`: existing Capability Profile, Node Profile, initial prompt, identity and notifications.
- `capability`: an OTP node-capability reference and typed configuration. No provider/session-creation fields are required by the generic contract.

Add `NodeCapabilityDescriptor` to `otp_api/contract.rs`: package-local identity, configuration schema, and the permitted action/output references for that node type. Extend `PackageDescriptor` with a default-empty list. Existing Workflow and Job Agent packages remain valid. Connections continue using `OutputRef` and `CapabilityRef`; node capabilities constrain which references are valid at each endpoint. Agent destinations retain their existing action configuration; capability configuration belongs to the capability node. Capability connections forward the structured output unchanged; existing prompt-input selection remains available for agent deliveries. No transformation language is needed for the default Voice design.

Extend `compiler.rs` to validate node/action compatibility, output existence and configuration, including capability action outputs used as downstream triggers. Stop requiring every action to return a session request. Compile the graph without selecting an entry; choose an entry only in `dispatch_node_user_request`. Session completion and service events address their source node directly.

Extract existing recipe migrations into `workflows/recipe_migrations.rs` as V4 is introduced. Decode V1–V3 nodes as agent nodes. Update stored instance decoding, authoring transport, TypeScript DTOs, fixtures and definition import/export validation together. Keep workflow synchronization on its existing definition path; portable recipes contain no phone tokens, audio paths or provider credentials.

## 2. Execution and OTP host boundary

Split `otp_host/workflow.rs` into a `workflow/` module with host access, routing, action dispatch and session delivery owners. Preserve public entry points through `workflow/mod.rs`. Add `workflows/capabilities.rs` for registered local capability runtimes and service-event ingress.

Core owns a small capability-runtime port: execute a bound action with instance/node identity, validated configuration and structured payload; return declared outputs or accepted asynchronous work; report later outputs with the original occurrence identity. A delivery-result callback carries the attempt/request identity and actual session receipt back to the capability that prepared an assignment. Desktop registers the Voice implementation. A package receives this through a bound `OtpHost` service handle, not repositories or the workflow service itself. Resolve bindings from the instance, never from caller-supplied device/session claims.

Adapt `otp_api/contract.rs`, `handles.rs`, `otp_host/mod.rs` and `otp_host/mcp.rs`:

- Distinguish agent-invocation and capability-event sources, with migrations for stored attempt contexts. Agent tools still require a current authorized invocation; service events do not fabricate one.
- Carry the original structured payload in `ToolInput::Action`, whether or not the action builds a prompt.
- Allow declared capability events and data outputs from action completion. Route both through the same connection resolver and attempt store.
- Extend results with structured operation/receipt data needed by Voice tools and capability invocation status. Keep session requests and cancellation on their existing product-owned path.
- Record the concrete source/output, connection, destination and service result on `WorkflowEventAttempt`. Service completion uses the bound capability node as source, not the upstream agent that initiated it.

When a capability event creates a Session Event, use the existing `SessionEventSource::ApplicationEvent` with its occurrence reference. Keep the original upstream agent identity as causal metadata; do not relabel service input as a direct user request.

Reuse current occurrence/attempt identities and SQLite records. Asynchronous JEV work runs outside state locks and emits its completion after state is saved. Deliver queued outputs after releasing service locks; do not recursively hold Voice state while invoking another capability. This is local execution plumbing, not a new durable scheduler or exactly-once guarantee.

### Preserve steering in the shared delivery path

Add a defaulted delivery mode to `workflow.prompt_agent` and session delivery requests: ordinary queue or steer-active-otherwise-queue. Extend `session_events` request/receipt/outcome types and `AgentSessionEventAdapter` to call the existing `deliver_or_steer_product` for the latter, retaining the real input/interaction ID and acceptance state. Update remote `sessionEvent` serialization and historical record decoding as affected consumers, without adding new routing behavior.

Move receipt reconciliation for queued/accepted/expired steering into the shared delivery owner. Preserve Voice's task-revision supersession check as a Voice task policy, expressed through the recorded assignment correlation rather than a second delivery queue. Never present steering submission as a completed invocation. Existing workflows remain on queue mode unless their design opts in.

## 3. Voice package and runtime adapter

Create `crates/orchid-core/src/otp_packages/voice/` for capability/tool declarations, configuration types and the default design builder. Move the default prompt assets from `src-tauri/resources/voice-sessions/` into this package; leave the transcription script with the speech implementation. Register the package in `otp_packages/mod.rs`. Create `src-tauri/src/voice_sessions/workflow_runtime.rs` to implement the runtime port and translate Voice state/events into those declarations.

Expose node capabilities for transcript input, typed input evaluation, request buffering, speech publication and external-work observation. Keep service algorithms behind these boundaries:

| Existing owner | Assignment |
| --- | --- |
| `continuous_audio.rs`, `speech.rs`, `transcript.rs`, `gateway.rs` | Retain decoding, transcription, sentence reconciliation, capture and playback. Emit transcript/window and playback events to the bound instance instead of calling the next reasoning stage. Keep scheduling/timing mechanics here. |
| `prefilter.rs` | Extract `evaluation.rs` for JEV request construction/invocation/evidence, and `input_buffer.rs` for applying answers and releasing requests. Evaluation emits typed answers; buffer consumes them and emits `work_ready` or `reaction_ready`, never destination IDs. |
| `query_schedule.rs`, `query_inspection.rs` | Retain timing and applied-query evidence. Bind them to instance/capability node and retain exact JEV session/invocation references for inspection. |
| `pipeline.rs`, `application.rs` publication/context methods | Move context reads and speech state operations into focused `context.rs` and `publication.rs`. Publication is an explicit configured action; its output can feed back to other nodes through connections. |
| `tasks.rs`, `delegation.rs` | Retain task IDs, revisions, parent assignments and outcome checks. Replace direct session creation and recipient selection with a Voice assignment action that returns standard Workflow session requests for the configured destination agent node. |
| `observations.rs` | Retain observation of explicitly selected external work and task-status bookkeeping. Emit structured observations/outcomes; remove role-based recipient selection and the private delivery pump. Ordinary workflow-agent completions use the existing workflow event source. |

JEV configuration stores model and question instructions in the evaluation node. Initially retain the typed relevance/completion/reaction/continuation answer vocabulary required by the buffer; do not add arbitrary user code or a question-template language. The existing JEV provider APIs and query inspector remain authoritative for inference and evidence.

### Default design and dynamic work

Seed a normal editable recipe with Summary, Router, Digest, Output and Worker agent nodes plus the capability nodes above. Names are defaults, not runtime role identifiers.

- Transcript-window output feeds evaluation; evaluated output feeds the request buffer.
- `work_ready` connects to Summary, Router and Digest; `reaction_ready` connects to Output. Disabling the shortcut is buffer configuration that emits `work_ready` instead.
- Summary and Router completion outputs follow configured connections. Progress, output requests and reaction reclassification are named tool outputs with configured recipients.
- Delegation emits an assignment request from Router or Worker. A configured `voice.assign_work` action on the Worker destination uses the existing task ledger plus `OtpHost.sessions`; it requests a new session or an exact session belonging to that node. The engine creates/delivers and returns the receipt to task bookkeeping. No direct `create_product_session_configured` path remains.
- Worker return connections use an explicit `return_to_requester` selection policy. Resolve the requester from the retained assignment, and deliver only when it belongs to the connection's destination node. Default edges cover Router and parent Worker; Digest observation is a separate connection. This preserves nested delegation without unconditional hidden recipients.
- Observations from explicitly watched ordinary sessions enter through the external-work capability. Their destinations are graph connections. They do not silently acquire workflow membership or Voice tool grants.
- Speech requests connect to the publication capability; publication/playback outputs feed any configured consumers. Preserve distinct published/sent/played evidence.

Remove hard-coded Router/Digest/Output authorization. Node tool grants plus instance membership authorize operations. Seed acknowledgement-only publication for Summary and full publication for Output through separate tools or declared grants. A renamed or additional node can receive those capabilities through its design.

Convert the current `voice_session` tool extension to OTP declarations and execution. New workflow nodes get tools through Node Profiles/Harness. Remove the second Voice definitions/dispatch map in `src-tauri/src/orchid_control/mcp.rs`; retain generic `orchid_control` discovery/work tools. Session results and active invocation authority continue to be checked by product services.

## 4. Instance state, settings and composition

Split current `VoiceSettings` into portable recipe configuration and device-local connection settings. Move prompts, model groups, notification choices, input policy and routing preferences into the appropriate recipe nodes. Seed grouped model values into individual nodes; remove the hidden group-level override after import.

Adapt `voice_sessions/repository.rs` and `domain.rs` to persist Voice runtime state keyed by Workflow instance ID, plus local connection settings and the active instance pointer. Voice owns transcripts, tasks, utterances and playback; the Workflow store owns the recipe snapshot, node/session bindings and delivery attempts. Do not duplicate the recipe in Voice state. Resolve agent sessions by workflow logical node address; JEV capability state retains its own query-session identity.

Create a one-time import in `voice_sessions/migration.rs`: copy existing settings into the default design, preserve the old singleton record as legacy read-only history, and create no replacement sessions until the user starts a new instance. Old agent histories and in-flight work remain untouched. Do not resume the old hard-coded pipeline in parallel. Update `phone_history.rs` and desktop history reads to include legacy records without interpreting them as new workflow instances.

Reorder desktop composition in `active_app.rs` and `active_app/voice_sessions.rs`: construct sessions and Workflow services, register Voice runtime bindings and OTP MCP, connect event observers, then restore a supported instance's phone connection. Use weak callbacks where required to avoid an ownership cycle. `session_notifications.rs` lets Workflow own completion routing and notification policy for migrated nodes; Voice observes only its service/task state and external subscriptions.

`start_voice_session` selects/creates a workflow instance with an explicit local worktree; reconnect resumes the selected instance. Stop disconnects capture without cancelling delegated work. Do not add automatic host choice or remote startup commands in this slice.

## 5. UI and contract consumers

Extend `src/application/workflowAuthoring`, `workflowInstances`, `otp` and the corresponding Tauri transports for tagged nodes, capability configuration, service outputs and steering receipts. Update `agent_sessions/client.rs` and persisted JSON decoders where shared delivery contracts cross the host boundary.

Reuse `WorkflowCanvas`, `WorkflowNodeEditor`, `WorkflowConnectionEditor`, trigger/action pickers and the existing authoring client. Extract an embeddable `WorkflowDesignEditor` from `WorkflowAuthoringScreen`; both ordinary Workflows and the Voice workspace consume it. Agent configuration continues using `NodeProfileEditor`. Capability configuration uses typed field descriptors and a focused Voice evaluation editor for its question instructions. Shared trigger/action menus filter by the selected node's supported contracts.

Adapt `VoiceSessionsScreen` to own selection of design/instance and connection controls, rather than a parallel behavior draft. `VoiceWorkflowPanel` retains stage focus and navigation but reads the editable recipe and the chosen instance's execution. Retain `VoiceActivityPanel`, `VoiceDiagnostics`, `VoiceRouteInspector` and `JevQueryInspector`, backed by actual workflow attempts and Voice service evidence. Update `WorkflowInstanceView` so capability nodes show operation state rather than an agent profile/session card.

Move reusable focus/layout behavior from `ExtensionWorkflowCanvas` into the shared workflow canvas adapter. Remove the extension-only executable-definition projection and the redundant Voice behavior/settings editors after their consumers migrate. Keep saved viewport, draft navigation and exact invocation links covered by the committed UI tests. No new visual redesign or Android viewer work is required.

## Reuse and dependency choices

Keep React Flow and ELK for the graph, Tokio for local async execution, SQLite/Serde for existing persistence, rmcp for MCP, and the current JEV and speech implementations. Replacing the Workflow engine with another orchestration framework would create competing session, identity and delivery ownership.

Use the Rust `jsonschema` library behind the existing `otp_api::validate_json` entry point rather than expanding its current partial validator for boolean, numeric and enum capability settings. Keep schemas local, compile them during catalogue construction, and retain typed Serde validation for Voice-specific semantics. Existing `schemars` can derive schemas where useful. The library supplies reusable compiled validators: [official validator API](https://docs.rs/jsonschema/latest/jsonschema/fn.validator_for.html). Check the chosen release against the repository toolchain during implementation; no dependency has been added by this plan.

## Code that becomes redundant

- Voice `EDGES`, `FlowEvent`, `targets`, role-to-session lookup and inferred edge-evidence matching.
- `ensure_pipeline`, direct creation of fixed Voice agents, role-dependent prompt injection and global model-group overrides.
- Hard-coded Summary/Router/Digest/Output publication and routing checks.
- Voice's recipient-bearing observation queue and its independent dispatch/steering loop; retain task state and service evidence only.
- The desktop Voice MCP extension's parallel tool declarations.
- Extension-only workflow definition DTOs/canvas wrapper once their sole Voice consumer uses the shared recipe editor. Retain any small legacy-history decoder needed for old route records.

## Delivery order and evidence

1. Introduce recipe/node/source contracts and migrations; keep ordinary Workflow/Job Agent behavior passing. Split host routing/dispatch and add capability execution with fake services.
2. Add the Voice package, default design and runtime adapter; connect transcript → evaluation → buffer → agent delivery, including active-turn steering.
3. Move delegation, worker returns, observations, output requests and publication onto configured connections. Remove the old topology and dispatch paths.
4. Connect design/instance editing, migrate settings/history, and reorder composition. Exercise the whole local Voice journey.

Required checks:

- Existing recipes, instances, OTP calls and replicated definitions decode; new agent and capability recipes round-trip through synchronization without device secrets.
- With deterministic fake JEV/provider/speech services, run the default design from transcript to publication and playback receipt. Remove the Digest edge and prove zero Digest delivery; redirect an output to a newly added node and prove it receives it; rename roles without changing behavior.
- Repeated transcript revisions, mixed requests, reaction rerouting and stale evaluation answers retain current buffer semantics. Removing evaluation/buffer connections prevents their downstream actions.
- Amendment steering, nested worker returns and outcome revisions preserve correct assignment correlation. No delivery reaches a recipient absent a matching connection. Publication grants are enforced by configured capability, independent of node name.
- Stop/reconnect resumes an instance without recreating its agent sessions; a new design applies to a new instance. Legacy histories remain readable. Local execution remains independent of server connectivity.
- Frontend tests cover ordinary and Voice authoring, connection changes, capability settings, saved draft/viewport, route receipts and invocation navigation. Run focused Rust tests for workflows/OTP/session events/Voice plus Rust check, frontend build and scoped lint.
- Inspect an isolated desktop instance and exercise typed text with real agents/JEV. Separately verify microphone input and phone playback before calling the local Voice migration live-validated. This does not prove remote phone relay or remote-host support.

The decisive acceptance condition is behavioral: changing the saved design changes agent creation and event delivery without editing Voice Rust code.
