# Observation pass: Agent runtime comparison

## Evidence frame

This is a behavior-led static trace of three concrete paths:

1. an ordinary Agent Session send;
2. a managed Epic Plan Builder discussion or plan request;
3. a Work Unit Handler activation that leads to an Implementer launch.

The inspection checkout was `codex/orchestration-engine-research` at `b28137b66d79121d740267831fc22bf8cdbcbb40`. The checkout is intentionally a research branch. Product code was not changed by this pass.

Two divergent lines were known during checkout preparation: `codex/product-decision-correction-authority` at `82d9351` and `codex/final-epic-settlement` at `8965191`. Both share merge base `e3bde2ca` with this checkout. This pass describes the selected `b28137b` product baseline; it does not claim that the two divergent lines are incorporated here. The files central to these three paths predate the latest Native Profile commits on the selected branch, although later commits have continued to harden the common runtime and Work Unit paths. Examples in the local history include `203ad8c Make original Implementer route productive`, `c022dae Fix bounded implementer candidate and reporting runtime`, `6d4fbf7 fix: release handler transition lock before launch`, and `6588275 fix: gate implementer reporting on candidate evidence`.

This is code evidence, not a packaged-app or controlled-live observation. Where a statement is inferred from composition rather than demonstrated by a running product, it is identified as such.

**Supersession note:** this pass intentionally preserves the `b28137b` launch behavior it observed. Same-day descendant `9240364` later connected selected, ready Native Profile identity to the shared Agent Session application. The role/runtime comparison remains useful, but its “Native Profile relationship” section is historical rather than the current research tip. See [Native and managed runtime authority](native-and-managed-runtime-pass.md).

## Comparative orientation

| Behavior | Ordinary Agent Session | Managed Epic Plan Builder | Work Unit Handler / Implementer |
| --- | --- | --- | --- |
| Immediate initiator | User submits the shared composer | User discusses through the shared composer, or presses the product-owned plan action | Backend reconciliation acts on durable Work Slice and dependency facts; the Handler later requests the Implementer through MCP |
| Frontend mutation command | `send_agent_session_message` | `send_managed_plan_builder_message` or `request_managed_plan_builder_action` | None for Handler or Implementer launch |
| Invocation input provenance | `user` | Discussion is `user`; the plan action is `application` | `application` throughout the original Handler, Handler action continuation, Implementer, reporting continuation, and review continuation |
| Session and invocation identities | New IDs allocated by Agent Session application unless an existing Session is selected | Session is ordinary Agent Session identity; invocation is preallocated by managed service | Stable IDs derived from Work Unit or attempt identity, then persisted and replayed idempotently |
| Runtime configuration owner | Caller-selected model/sandbox plus durable Session working directory | `epic_plan_builder` Harness fixes read-only sandbox, approval policy, prompt prefix, skill guidance, and MCP exposure | Immutable Handler/Implementer Harness revisions plus execution-support authority fix sandbox, prompt, worktree, and per-continuation MCP exposure |
| MCP | None added by this path | A fresh required loopback MCP endpoint on every managed invocation | No MCP on original Handler or original Implementer; narrowly scoped MCP endpoints exist only on later action/reporting/review continuations |
| Write boundary | Whatever the requested effective sandbox permits | Read-only | Handler read-only; Implementer workspace-write in an application-created isolated Git worktree |
| Durable semantic state beyond transcript | None specific to ordinary conversation | Draft association, proposal revisions, command/result/provenance, and pending initiation context | Materialization, dependency eligibility, activation stages, execution-support grants, outcome/evidence, review, retry, integration, and settlement facts |
| Frontend observation | Shared Agent Session event plus durable Session reload; active Session also polls | Same Agent Session event/reload, plus explicit native-query refresh for proposal state | Activation state comes from native orchestration snapshots; embedded Agent Session state would use the shared Session event path if a Session reference were projected |
| Meaning of process terminal | Agent invocation terminal fact | Does not itself prove proposal persistence or initiation | Does not itself authorize the next semantic transition; each transition checks bound MCP facts, evidence, lifecycle, and durable correlations |

The main architectural finding is not three runtimes. There is one `AgentSessionApplication` and one `CodexCliRuntime` in productive composition. Plan Builder and Work Unit behavior is created by pre-persisted application facts, invocation provenance, and an optional `RuntimeLaunchExtension` that adds prompt/configuration/MCP inputs. Work Unit execution adds a second authority plane—execution-support and Git worktree verification—before the common Agent Session launch seam is reached.

## Trace 1: ordinary Agent Session

### User interaction and frontend control

`src/features/agentSessions/AgentSessionWorkspace.tsx` (`AgentSessionWorkspace`) renders `ConversationViewport` with an always-present composer target for the selected or new Session. The controller is `src/features/agentSessions/useAgentSessionController.ts` (`useAgentSessionController`). Its `sendText` callback:

1. waits for the client subscription bridge to be ready;
2. calls `AgentSessionClient.sendMessage` with visible text, optional current Session ID, and new-Session title/working-directory values;
3. accepts only `{sessionId, invocationId}` as acknowledgement;
4. records those identities locally;
5. reloads the selected durable Session and refreshes the Session list.

The controller does not treat the acknowledgement as completion. `reconcileUpdate` accepts an update only when it belongs to the selected Session and its invocation is already in `invocationIdsRef`, then reloads the whole Session. A very fast event arriving before acknowledgement can therefore fail the local invocation filter, but the post-acknowledgement reload closes that gap. While a Session is active, a 1.5-second interval also reloads it. That polling is an explicit second reconciliation route for missed notifications and reopen behavior.

`src/infrastructure/agentSessions/tauriAgentSessionClient.ts` owns one shared `ensureUpdateBridge`. It installs a Tauri listener for `agent-session-update` and waits for that listener before invoking `send_agent_session_message`. Reads use `list_agent_sessions` and `load_agent_session`. The frontend does not construct transcript or lifecycle facts from event payloads; it treats the event as a reason to re-read durable state.

### Tauri boundary and application behavior

The relevant commands are in `src-tauri/src/agent_sessions/transport/mod.rs`:

- `create_agent_session`;
- `list_agent_sessions`;
- `load_agent_session`;
- `send_agent_session_message`;
- `cancel_agent_invocation`.

`TauriAgentSessionNotifier` emits `agent-session-update`. These are registered in the productive `tauri::generate_handler!` list in `src-tauri/src/active_app.rs` (`run`).

`src-tauri/src/agent_sessions/application/lifecycle.rs` (`AgentSessionApplication`) is the real launch boundary. `send_message` delegates to `send_message_with_launch_extension(command, None)`, so an ordinary send has no orchestration prompt/config/environment/MCP extension. `send_message_with_provenance` then performs the consequential sequence:

1. create or reload the Session and repair its runtime binding if necessary;
2. persist a pending invocation before provider preflight;
3. choose `Start` or `Resume` from the durable external runtime context, not from transcript content;
4. call runtime `preflight_invocation` for semantic capability resolution and effective options;
5. terminalize the durable invocation if preflight fails;
6. persist running state and effective options before process launch;
7. construct `RuntimeInvocationRequest` and `PersistedRuntimeUpdateSink`;
8. call the common runtime start/resume seam;
9. persist launch acceptance only after the runtime accepts launch.

The ordinary path uses `AgentInvocationInputProvenance::User`. Its working directory comes from the durable Session. The initial caller can request model and sandbox, but cannot add CLI arguments, environment values, or an initial prompt prefix through this Tauri command.

### Runtime, persistence, and events

`src-tauri/src/runtime/codex/runtime.rs` (`CodexCliRuntime`) discovers/caches Codex CLI capabilities in `preflight_invocation`, builds effective CLI arguments, records sanitized launch provenance in `record_launch_provenance`, and starts the supervised child. The process inherits the product process environment because the ordinary launch extension is absent. JSONL stdout/stderr is normalized into runtime updates; provider terminal evidence and process terminal evidence are deliberately separate.

`src-tauri/src/agent_sessions/application/update_sink.rs` (`PersistedRuntimeUpdateSink`) serializes updates per invocation, assigns sequence, persists the runtime event first, updates the durable external context binding when one is established, and only then notifies. A terminal invocation is also persisted before its terminal notification. Notification failure becomes a diagnostic rather than recursively manufacturing another notification.

The common Agent Session schema is in `src-tauri/src/agent_sessions/repository/schema.rs`:

- `agent_sessions`;
- `agent_session_invocations`;
- `agent_session_invocation_launch_acceptances`;
- `agent_session_runtime_events`;
- `agent_session_invocation_diagnostics`.

Repository operations are implemented in `src-tauri/src/agent_sessions/repository/mod.rs`. Startup reconciliation in `AgentSessionApplication::reconcile_startup` terminalizes orphaned active invocations as interrupted unless terminal delivery evidence already exists.

### What “ordinary” means in the current product

This path is productive application functionality, not a test seam. It is intentionally provider-neutral at the Agent Session application layer and Codex-specific at `CodexCliRuntime`.

It is also materially less governed than the orchestration paths. No Conversation Harness is loaded. No application-specific MCP server is started. No orchestration association is checked. The same generic command accepts any valid persisted Session ID; the Agent Session tables do not carry a Session ownership/type discriminator that the generic transport checks.

The last point matters for managed Sessions. If a Plan Builder, Handler, or Implementer Session becomes selectable in the standalone Agent Sessions surface, the generic composer can submit an ordinary user-provenance continuation to that Session. That continuation would not pass through the managed Plan Builder service or Work Unit transition service and would receive no corresponding Harness/MCP launch extension. Productive Work Unit embeddings are explicitly read-only, which reduces the immediate affordance, but `WorkUnitDetailWorkspace` also offers “Open in Agent Sessions” when it has a Session reference. This is a code-demonstrated capability boundary; whether every managed Session is currently reachable through productive navigation needs live UI confirmation.

### Native Profile relationship

`src/bootstrap/productApplicationComposition.ts` constructs `nativeProfileApplicationConsumer`, and `src/app/App.tsx` declares the prop. The current `App` implementation does not destructure or call it. `src/infrastructure/nativeProfiles/nativeProfileConsumer.ts` only resolves a profile query. Productive `active_app.rs` still creates the shared runtime as `CodexCliRuntime::system("codex", None)`.

Therefore the three paths in this pass do not currently show an application-to-Agent-Session bridge that applies the selected Native Profile `CODEX_HOME` or execution mode. They launch the system `codex` command in the product process environment. This is a static composition finding, not a claim about what credentials happen to be available in that environment.

## Trace 2: managed Epic Plan Builder

### The frontend reuses an Agent Session but changes the send boundary

`src/bootstrap/productApplicationComposition.ts` wraps `tauriAgentSessionClient` with `createTauriManagedPlanBuilderSessionClient` from `src/infrastructure/orchestrations/tauriManagedPlanBuilderSessionClient.ts`. The wrapper retains the ordinary client for load/list/subscribe/cancel, but overrides mutation:

- `sendMessage` invokes `send_managed_plan_builder_message`;
- `requestPlan` invokes `request_managed_plan_builder_action`.

`src/features/orchestrations/EpicPlanBuilder.tsx` (`EpicPlanBuilder`) uses the shared `useAgentSession` controller. Ordinary discussion therefore looks and behaves like an Agent Session, but crosses the managed Tauri command. The explicit plan action calls `requestPlan`, reloads the Session, and refreshes the proposal source even on failure. It is enabled only after a user turn and when the conversation is newer than the displayed proposal, and it is disabled while an invocation/send is active.

Proposal presentation has a separate read path. `createNativeEpicPlanProposalSource` in `src/infrastructure/orchestrations/tauriOrchestrationNativeQuery.ts` calls `load_orchestration_native_query`, decodes the full durable query, and projects the selected draft through `projectEpicPlanProposal`. `EpicPlanBuilder` refreshes that source on mount, when Session details change, and when it observes `invocation_terminal`. It does not parse a proposal from transcript text or an MCP return value.

This produces two frontend subscriptions for the Plan Builder: `useAgentSession` subscribes for Session reload, and `EpicPlanBuilder` subscribes for proposal refresh. The shared Tauri client multiplexes both through its one event bridge.

### Managed Tauri and application behavior

`src-tauri/src/orchestration/transport.rs` exposes:

- `send_managed_plan_builder_message`, which calls `ManagedPlanBuilderService::send` with user provenance;
- `request_managed_plan_builder_action`, which calls `ManagedPlanBuilderService::request_plan` with application provenance and the fixed application text “Build the epic plan based on what we have discussed”.

Both return only Agent Session and invocation acknowledgement identities. The wrapper comment explicitly states that acknowledgement is not proposal persistence evidence.

`src-tauri/src/orchestration/application.rs` (`ManagedPlanBuilderService`, especially `send_with_provenance`) serializes managed sends with `send_lock` and then:

1. loads the `epic_plan_builder` profile from the Conversation Harness catalog;
2. rejects caller model/sandbox overrides that conflict with the Harness;
3. ignores caller working-directory input and derives the repository discovery root;
4. creates an Agent Session in that root on first send, with Harness-owned read-only options;
5. bootstraps the durable planning draft, capability profile, Session association, and proposal revision precondition;
6. constructs `PlanBuilderInvocation` with those server-side identities;
7. starts the invocation-scoped MCP server before allocating/launching the Agent invocation;
8. builds a `RuntimeLaunchExtension` from Harness CLI configuration, MCP configuration arguments, bearer-token environment, and an optional prompt prefix;
9. preallocates an Agent invocation and claims any pending application-delivered Plan Builder context against that exact identity;
10. calls the common Agent Session idempotent send path with user or application provenance;
11. binds the already-running MCP endpoint to the returned Agent invocation identity;
12. retains the server in `ManagedPlanBuilderRegistry` until terminal, unless the invocation terminalized synchronously before registry insertion;
13. consumes pending context only after durable launch acceptance, otherwise releases or reconciles it.

The first normal invocation receives `harness.initial_prompt_prefix()`. Later discussion turns do not repeatedly prepend it. A pending confirmed-initiation context has precedence and is delivered as a separate application-owned prompt prefix correlated to the claimed invocation.

### Configuration that is executable behavior

The source configuration is `src-tauri/src/orchestration/conversation_harness_catalog.json`, loaded with `include_str!` and decoded by `src-tauri/src/orchestration/conversation_harness.rs` (`catalog_profile`, `ConversationHarnessProfile`). The `epic_plan_builder` entry is version 4 and declares:

- read-only sandbox;
- approval policy `never`;
- required tools `submit_epic_plan_proposal` and `request_epic_initiation`;
- repository skill guidance for `.agents/product-skills/epic-plan-builder/SKILL.md`;
- first-query context delivery;
- completion criterion `proposal_persisted_or_user_ends_discussion`.

The JSON is not merely UI metadata. `runtime_options`, `runtime_configuration_args`, and `initial_prompt_prefix` convert it into effective runtime options, `-c` CLI arguments, and model-visible prompt content. `role_discovery_root` also validates that the referenced repository skill exists and has the expected `name:` metadata before returning the repository root.

Model and reasoning are nullable, so the profile can deliberately constrain sandbox/approval/MCP/prompt while leaving model selection to the underlying effective runtime defaults.

### MCP endpoint and semantic persistence

`src-tauri/src/orchestration/mcp.rs` owns the Plan Builder server. `start_managed_invocation` binds an ephemeral `127.0.0.1` port, uses Streamable HTTP RMCP, requires a generated bearer token, restricts Host and Origin, and returns `CodexMcpInjection` for the child process. `PlanBuilderInvocation` carries draft, capability profile, association, expected proposal revision, and a synchronization boundary that waits for the later Agent invocation binding.

The `submit_epic_plan_proposal` tool accepts only the typed proposal and derives command ID/idempotency/provenance from the bound server-side invocation. It delegates to `OrchestrationApplication::save_epic_plan_proposal`; tool success is thus a result of durable application processing, not a transcript convention. `request_epic_initiation` goes through the confirmation coordinator and does not bypass explicit user confirmation.

The main Plan Builder persistence is in `src-tauri/src/orchestration/repository.rs`:

- `epic_planning_drafts` and lifecycle events;
- `capability_profiles` and `planning_draft_profile_assignments`;
- `planning_draft_agent_session_associations`;
- `proposal_commands`, `proposal_command_results`, `proposal_revisions`, and `proposal_events`;
- `effect_provenance`;
- `plan_builder_context_deliveries`;
- the later Epic initiation tables.

The same invocation simultaneously writes the common Agent Session tables. A Plan Builder conversation is therefore one runtime history joined to a second domain record by durable association—not a special transcript stored in an orchestration-only system.

### Event and refresh behavior

Plan Builder runtime updates take the same `PersistedRuntimeUpdateSink` route as ordinary Sessions. `ManagedPlanBuilderNotifier` in `src-tauri/src/active_app.rs` first releases managed MCP ownership on terminal, invokes bootstrap/Sprint transition observers, and only then delegates to `TauriAgentSessionNotifier` to emit the frontend event. The proposal view still re-reads `load_orchestration_native_query`; the event is never accepted as proposal state.

The notable timing boundary is that the MCP server starts before the Agent invocation ID exists. `PlanBuilderInvocation` explicitly bridges that timing through a bounded wait/bind seam. The code also anticipates the inverse race: a runtime may synchronously terminalize before `send_with_provenance` can insert the server into the registry.

## Trace 3: Work Unit Handler to Implementer

### There is no user launch command

The productive Work Unit launch is not a Tauri mutation. It begins in backend reconciliation after a Work Slice proposal has been durably accepted and materialized. `src-tauri/src/orchestration/sprint_runner_transition.rs` (`SprintRunnerTransitionService`) owns the state machine.

During product boot, `src-tauri/src/active_app.rs`:

1. opens the shared active database;
2. creates `ProductExecutionSupportState` rooted at `app_data_dir/execution-workspaces`;
3. creates the common `AgentSessionApplication` and runtime;
4. constructs `WorkUnitExecutionHarnessService` from execution support, Agent Sessions, and orchestration;
5. opens `SprintRunnerTransitionService::open_with_application_git_authority`;
6. calls `attach_work_unit_handler_activation`;
7. attaches the Sprint transition service to the composite Agent Session notifier;
8. calls transition startup reconciliation.

`attach_work_unit_handler_activation` immediately calls `reconcile_work_unit_handlers`. Later durable Agent Session notifications can re-enter the same reconciliation.

### Original Handler activation

`SprintRunnerTransitionService::reconcile_work_unit_handlers` first reconciles dependency waves and Work Slice execution settlement. For each settled materialized Work Unit it calls `reconcile_work_unit_handler`.

That method:

1. loads the application-recorded Sprint Git authority and dependency eligibility;
2. derives stable Handler attempt, Session, and invocation IDs from the Work Unit ID;
3. persists or reuses a `work_unit_handler_activations` row, including blocked/eligible state and an immutable Harness revision identity/digest/repository commit;
4. loads and validates the pinned original Handler revision;
5. authorizes an existing Handler attempt through execution support;
6. derives an opaque capability and working directory from durable authority;
7. creates a read-only application-owned Agent Session titled “Work Unit Handler”;
8. prepares an application-provenance invocation with a bounded activation prompt;
9. binds the exact Session/invocation to the capability package;
10. records distinct stages such as authorization, Session creation, invocation preparation, Harness binding, launch request, launch acceptance, provider activation, and Handler ready;
11. launches through `AgentSessionApplication::launch_prepared_application_invocation_with_launch_observation` and the same `CodexCliRuntime` used by ordinary Sessions.

The original Handler is deliberately actionless. `WorkUnitExecutionHarnessService::current_handler_revision` selects or bootstraps `initial_work_unit_handler_baseline_revision_configuration`, which removes the catalog’s `request_work_unit_implementer` tool. This means the current catalog entry is not a literal description of every Handler invocation.

Only after the original Handler invocation reaches a terminal state does `reconcile_handler_action_continuation` publish/load a second immutable Handler revision, prepare another invocation in the same Session, start a scoped MCP server, and inject the one tool `request_work_unit_implementer`. The original invocation is allowed to have completed, failed, canceled, or been interrupted; terminality releases the continuation, but it does not itself request an Implementer. The action invocation must be live when the tool is called.

The input also reveals the Handler's present product role. The original activation prompt includes the stable Work Unit ID and prohibitions, but not the Work Unit specification. The action-continuation prompt says to use the one request tool, whose input is empty and whose backend derives the specification and all authority. No Handler MCP evidence-read operation exists until the much later independent review continuation. The initial Handler pair is therefore primarily a protocol/lifecycle gate around an application-derived Implementer request, not a task-decomposition or implementation-acceptance decision maker in the inspected path.

### The Handler MCP request and Implementer launch

`WorkUnitHandlerMcp::request_work_unit_implementer` is defined inline in `src-tauri/src/orchestration/sprint_runner_transition.rs`. The input is exactly `{}`. It calls `SprintRunnerTransitionService::request_work_unit_implementer`, which reaches `request_work_unit_implementer_inner` with `require_active_action = true`.

Before creating anything, the method checks:

- the action continuation’s complete durable readiness chain;
- original Handler terminal lifecycle and application provenance;
- the current action invocation’s application provenance and active state;
- stable derived Work Unit/attempt/Session/invocation identities;
- the pinned Handler action revision and exact tool exposure;
- the Sprint Git authority;
- the bounded non-empty Work Unit specification.

It then persists or reuses a `work_unit_implementer_activations` row, pins the actionless Implementer revision, authorizes the Implementer attempt, and asks `WorkUnitExecutionHarnessService` for a package. The package is the bridge from orchestration identity to runtime configuration:

- `ExecutionSupportService::authorize_existing_attempt` records role-bound authority without accepting filesystem paths from the agent;
- `grant_for_role` re-derives repository/worktree context and returns only `ExecutionSupportReference { capability_ref, working_directory }`;
- Implementer resolution creates or reopens a deterministic isolated Git worktree under the product-owned execution workspace root;
- the package refuses an exact Implementer launch if the target worktree already contains `.codex`, because local project configuration would be loaded after CLI overrides and cannot be safely enumerated/neutralized by this implementation.

The service creates an application-owned Agent Session titled “Work Unit Implementer”, prepares a stable application-provenance invocation in it, binds correlation, and launches it through the common Agent Session application/runtime. The prompt is derived from the durable Work Unit specification (`work_unit_implementer_prompt`), not supplied by the Handler tool.

The original Implementer Harness is workspace-write and actionless. `package_runtime_launch_configuration` adds:

- Harness approval configuration (`approval_policy="never"`);
- `--ignore-rules`;
- `-c mcp_servers={}` to clear inherited MCP configuration;
- one exact, ephemeral `projects.'<canonical worktree>'.trust_level="trusted"` override;
- the immutable Harness context and skill guidance as an initial prompt prefix.

There is no bearer environment or orchestration MCP endpoint on this original write turn. This separation is intentional: implementation happens without a reporting/acceptance tool, and semantic reporting happens later.

### Terminal lifecycle leads to a second Implementer turn, not automatic acceptance

When `reconcile_implementer_reporting_continuations` runs and finds the original Implementer durably `Completed`, it:

1. seals and revalidates the exact already-authorized candidate through `commit_implementer_candidate`;
2. requires a non-empty changed-file manifest, comparison, capture authorization, and bounded evidence content;
3. pins a later immutable reporting revision;
4. creates a stable reporting invocation in the same Implementer Session;
5. starts a scoped MCP endpoint exposing only `submit_implementation_outcome` and `complete_implementation_outcome`;
6. launches the application-owned reporting continuation through the common runtime.

The reporting tools accept summary/validation claims but label their responses `accepted: false`. `complete_implementation_outcome` causes the application to capture the file evidence itself. `reconcile_implementer_outcome_acceptance` requires the exact semantic payload, captured evidence, semantic completion, and a matching `Completed` lifecycle before setting application acceptance and Handler-review readiness.

A third same-Handler-Session continuation then performs independent review through `read_handler_review_evidence`, `accept_implementation_outcome`, or `return_implementation_outcome`. Even those tool calls do not settle movement on their own; the exact review invocation must also be observed completed before acceptance/return movement is finalized.

Although this tail extends beyond the requested Implementer launch, it explains why the original Implementer has no MCP and why “process completed” is not an outcome in the Work Unit product model.

There is a noteworthy trigger asymmetry in the inspected baseline. `on_agent_notification` explicitly recognizes original Handler invocations, retry Implementer invocations, reporting invocations, and review invocations. It does not have an equivalent branch for the ordinal-0 original Implementer invocation in `work_unit_implementer_activations`. `reconcile_implementer_reporting_continuations` is reached by startup/full Work Unit reconciliation and by the retry-Implementer terminal branch, but not directly by the original Implementer terminal notification. Static control flow therefore indicates that the first attempt's reporting continuation may wait for restart or another full Work Unit reconciliation trigger after its original Implementer completes. The underlying durable state is replayable, but immediate same-run progression is not established by this code path. This should be confirmed with a focused live or integration trace before deciding whether it is an intentional pause boundary or a missing notification route.

### Work Unit MCP implementation

Unlike Plan Builder MCP, Work Unit endpoints are implemented inside the very large `sprint_runner_transition.rs` file. The relevant symbols are:

- `prepare_work_unit_handler_action` / `WorkUnitHandlerMcp`;
- `prepare_work_unit_implementer_reporting_action` / `WorkUnitImplementerReportingMcp`;
- `prepare_work_unit_handler_review_action` / `WorkUnitHandlerReviewMcp`;
- `start_scoped_server!`, which generates the loopback RMCP server functions.

Each server binds `127.0.0.1:0`, uses a generated bearer, restricts the exact Host and allowed Origin, and is stored in the `SprintRunnerTransitionService.mcp` registry by Agent invocation ID. Terminal notification calls `on_epic_runner_terminal`, whose name is broader/historical: it stops all scoped action servers in this registry, including Work Unit servers.

This is productive backend functionality but has no Tauri command and no public HTTP endpoint. Its only client is the launched Codex child configured through `CodexMcpInjection`.

### Persistence spans three related records

Every Handler/Implementer turn writes the common Agent Session tables. It also writes orchestration transition tables declared in `sprint_runner_transition.rs`, including:

- `work_unit_materializations` and `work_units`;
- `work_unit_handler_activations`;
- `work_unit_handler_action_continuations`;
- `work_unit_implementer_activations`;
- `work_unit_implementer_outcomes`;
- `work_unit_handler_reviews` and `work_unit_handler_decisions`;
- dependency, retry, integration, graph-completion, settlement, and attention tables.

Execution authority is separately recorded by `src-tauri/src/orchestration/execution_support.rs` in:

- `execution_support_attempt_authorizations`;
- `execution_support_grants`.

These are separate Rust repositories/services but share the one active SQLite database. The product therefore has one physical durability boundary with several logical writers and multiple connections. The transition code uses immediate transactions and explicit lock/replay handling because an Agent Session runtime can synchronously persist an update and re-enter transition reconciliation before an outer launch call returns.

## Frontend reality of the Work Unit path

### What the product read exposes

`load_orchestration_native_query` in `src-tauri/src/orchestration/transport.rs` calls `OrchestrationApplication::native_query`; `SqliteOrchestrationRepository::native_query_at` in `src-tauri/src/orchestration/repository.rs` joins the Work Unit activation/outcome/review tables into `NativeQueryV2.workUnits`. `load_sprint_runner_transition_query` separately exposes higher Sprint transition status. `createNativeQueryOrchestrationClient` loads both snapshots and sends them through `nativeQueryProductCompositionInputV2` and `composeProductOrchestrationReadModels`.

`src/features/orchestrations/components/WorkUnitDetailWorkspace.tsx` is an observation/navigation surface. It renders:

- Work Unit presentation and execution state;
- Handler/action/Implementer activation progress;
- per-attempt outcome/review/retry/integration activity;
- lifecycle links;
- slots for Work Slice Planner, Handler, and Implementer Agent Sessions.

It contains no “launch Handler” or “launch Implementer” mutation. `SharedAgentSessionPanel` uses `embeddedSessionIsWritable`; productive composition supplies `{ client: tauriAgentSessionClient }` with no `writableSessionIds`, so any embedded Work Unit Session is read-only.

### Current productive session-projection gap

The native query contains Handler and Implementer Session/invocation IDs inside activation structures, but `nativeQueryProductCompositionInputV2` currently sets:

- `events.workUnitExecutions` to `[]`;
- `events.attempts` to `[]`;
- `events.agentSessionReferences` to `[]`;
- `referenceIndex.agentSessions` only from initiated Plan Builder associations.

`SprintWorkspace.workUnitSessions` does not derive embedded Sessions from the activation structures. It looks for canonical `workspace.agentSessionReferences` targeting a Work Unit execution, optionally enriched by a recorded presentation adjunct. Product boot uses `productOrchestrationPresentationAdapter` without such an adjunct.

Consequently, the productive static composition can display durable Handler/Implementer activation text but does not currently construct the Session references needed for `WorkUnitDetailWorkspace` to embed those conversations; its slots fall back to “No recorded session.” The richer recorded adjunct exists under `src/dev/orchestrationSection/recordedPresentationAdjunct.ts`, which is development/sample presentation data, not the productive native-query composition.

This is a concrete connection gap, not evidence that the backend Sessions are absent. The backend creates them and stores their IDs; the product read currently leaves those IDs stranded inside activation details.

### Current productive freshness gap

`src/app/useOrchestrationLoad.ts` loads the orchestration client once on mount and exposes an explicit `refresh`. In `src/app/App.tsx`, the evident productive refresh call is tied to confirmed Epic initiation. There is no subscription from `agent-session-update` to `orchestrationLoad.refresh`, and no orchestration-specific update event in this path.

If a Session reference were present, its embedded `useAgentSession` controller would update that transcript from Agent Session events. The surrounding Work Unit activation/outcome/review state would remain the native-query snapshot loaded by `useOrchestrationLoad` until the surface is remounted or another explicit refresh path is invoked. This creates two freshness levels on one page: live-ish Session history and snapshot orchestration state.

## Cross-path event ordering

The productive notifier in `src-tauri/src/active_app.rs` is named `ManagedPlanBuilderNotifier`, but it is actually the composite notifier for the entire shared Agent Session application. For every persisted update it:

1. releases Plan Builder registry ownership if terminal;
2. calls the Bootstrap transition observer;
3. calls the Sprint transition observer;
4. finally emits `agent-session-update` through `TauriAgentSessionNotifier`.

Therefore a persisted Handler or Implementer update can synchronously cause orchestration reconciliation, persistence of new transition facts, startup of a continuation MCP server, and even a downstream Agent Session launch before the frontend receives the original event. Nested runtime updates can re-enter the same notifier. The code contains lock-release and synchronous-terminal handling specifically for this behavior.

This ordering is defensible for backend convergence—the frontend sees events only after application observers have had a chance to advance durable state—but it also means the Tauri event is not a simple mirror of one isolated mutation. It may arrive after a cascade. Since the frontend responds by reloading only the Agent Session named in the event, it does not automatically re-query the orchestration facts changed by that cascade.

## Application functionality mixed with configuration

The traces expose several distinct forms of “configuration,” with different operational weight:

| Form | Location | Runtime effect |
| --- | --- | --- |
| Ordinary Session request options | frontend contracts and Agent Session domain | User-selected model/sandbox become requested/effective runtime options |
| Conversation Harness catalog | `src-tauri/src/orchestration/conversation_harness_catalog.json` | Validated into sandbox, approval, prompt prefix, skill guidance, MCP requirements, and lifecycle language |
| Catalog-to-immutable-revision transforms | `conversation_harness.rs` | Produce historical/action/reporting/review variants that can differ from the current catalog profile |
| Immutable Harness working copies/revisions | `conversation_harness_working_copy.rs`, `conversation_harness_revision.rs`, orchestration repository | Pin configuration JSON, digest, repository commit, predecessor, and provenance for replay/reopen |
| Runtime launch extension | `agent_sessions/ports.rs`, assembled by orchestration services | Adds CLI config, ephemeral MCP endpoints, bearer environment, and initial prompt prefix to one invocation |
| Execution-support authority | `execution_support.rs` | Derives exact worktree and opaque evidence capability from durable Sprint Git authority |
| Tauri composition | `active_app.rs` | Chooses the one database, system Codex command, runtime, notifier ordering, services, and exposed commands |

Two subtleties are important for later architecture decisions.

First, the current catalog is not the complete executable truth for Work Units. The Handler catalog advertises the request tool, but the original Handler revision is deliberately transformed to remove it. The Implementer catalog is actionless, while a later derived immutable revision adds reporting tools. To understand a particular invocation, consumers need the pinned revision snapshot, not only `conversation_harness_catalog.json`.

Second, “skill guidance” is both prompt content and discovery validation. `ConversationHarnessProfile::initial_prompt_prefix` serializes the skill name/path/purpose/use condition into the prompt. `role_discovery_root` validates the repository file. For Work Unit packages, the `discovery_root` is stored in `WorkUnitExecutionHarnessPackage` and exposed by a getter, but the current launch code uses the execution-support working directory; no production call to the package getter was found. In practice an execution worktree of the same repository may contain the skill path, but the retained `discovery_root` field is not itself a runtime routing input in the inspected path.

## Product and architecture observations

### From a product-owner perspective

- “Agent Session,” “Plan Builder,” “Handler,” and “Implementer” are not separate providers. They are different products built on one conversation/runtime record.
- Plan Builder interaction is genuinely managed at its send boundary and has a durable structured outcome separate from conversation text.
- Work Unit execution is backend-driven. The visible Work Unit surface is an observer; an agent-to-agent MCP action, not a user button, initiates the Implementer.
- The initial Handler receives no Work Unit specification and may unlock its action continuation after any terminal status; its present role is closer to a governed request gate than a substantive task-routing judgment. Substantive Handler judgment appears later in the independent outcome review continuation.
- The Work Unit model deliberately separates request, authorization, Session creation, Harness binding, launch request, launch acceptance, provider activity, semantic reporting, application acceptance, review, integration, and settlement.
- The current productive UI does not yet connect its backend-created Handler/Implementer Session identities into the embedded Session panes, and its orchestration read is not event-refreshed.

### From a product-architecture perspective

- `RuntimeLaunchExtension` is the pivotal extension seam. It lets orchestration decorate a common Agent Session invocation without forking the runtime implementation.
- The Agent Session store is the lifecycle authority for processes and transcripts. Orchestration stores semantic authority and correlations. Execution support stores filesystem/Git capability authority.
- The composite notifier is an in-process event bus hidden behind one `AgentSessionNotifier` interface. Its name understates its scope, and its synchronous ordering couples runtime persistence to orchestration progression.
- Plan Builder MCP has a focused module; most other orchestration MCP adapters, server lifecycle, workflow state machine, SQL, and reconciliation coexist in `sprint_runner_transition.rs`. That concentration is a notable implementation boundary for later refactoring, not evidence that any part is non-productive.
- Tauri is thin for all three paths: it registers commands, owns state objects, and emits events. The substantial behavior lives in ordinary Rust application/runtime/repository services. Work Unit mutation is not exposed to Tauri at all.

### From an expert-developer perspective

- Persist-before-notify and prepared-before-launch are consistent across the paths.
- Idempotent application invocation IDs and per-stage timestamps make reopen/replay explicit for managed flows.
- MCP bearer/Host/Origin restrictions and identity-free tool inputs are real defense boundaries, not only prompt instructions.
- Work Unit runtime hardening (`--ignore-rules`, empty inherited MCP map, exact worktree trust override, refusal of local `.codex`) is tightly coupled to a specific Codex CLI behavior and version comment. It should be treated as compatibility-sensitive code.
- Ordinal-0 Implementer completion is durable but is not directly routed to reporting-continuation reconciliation by `on_agent_notification`; retry Implementers are. This asymmetry is likely to matter for uninterrupted execution.
- The generic Agent Session send accepts managed Session IDs without an orchestration ownership check. The UI’s read-only embedding is an affordance restriction, not a backend prohibition.
- Native Profile selection and the shared Agent Session launch path are presently composed alongside each other, not connected.

### From an expert-designer perspective

- The shared conversation viewport provides strong continuity across ordinary, Plan Builder, and embedded orchestration conversations.
- Plan Builder correctly presents structured proposal state from durable product data rather than asking the user to trust transcript wording.
- Work Unit details are designed around progressive evidence: activation, attempts, review, integration, lifecycle links, and parallel Session panes.
- Today that design promises navigable Session context that the productive read model does not populate. It can show detailed activation milestones while simultaneously saying “No recorded session,” even though the activation carries backend Session IDs.
- Without orchestration refresh on Agent events, a user could watch conversation progress while adjacent workflow labels remain stale. This is a product-state coherence issue, not simply a loading-spinner detail.

## Ambiguities and items not proven by this pass

- No packaged cold-open or live provider run was performed. Runtime launch acceptance, MCP negotiation, tool invocation, visual behavior, and restart convergence were not re-demonstrated here.
- It was not proven through the running navigation tree that every application-owned Session can be selected in the standalone Agent Sessions surface. The generic backend command and generic list/load client do not enforce an ownership distinction, so the capability exists if navigation supplies the ID.
- The exact effective inherited environment of `CodexCliRuntime::system("codex", None)` was not inspected. This pass only establishes that the selected Native Profile consumer is not wired into these launch requests.
- The two divergent post-baseline branches were located but not merged into the research checkout. Their effect on Product Decision/final-settlement features should be analyzed separately before any repository-wide “current product” claim.
- `role_discovery_root` validates the configured skill in the source checkout, while Work Unit runtime working directory comes from execution support. The execution worktree is expected to represent the repository, but this pass did not prove the exact skill-discovery result inside a live isolated worktree.
- The native Work Unit query/projection gap and refresh gap are established by static productive composition. Their visible severity should be confirmed in the packaged application with a database containing an active Work Unit.
- The initial-Implementer notification asymmetry was established by control-flow inspection, not a timed live run. Startup reconciliation should recover it, but immediate reporting launch remains unproven.

## High-value evidence index

### Shared Agent Session path

- `src/features/agentSessions/AgentSessionWorkspace.tsx` — `AgentSessionWorkspace`
- `src/features/agentSessions/useAgentSessionController.ts` — `useAgentSessionController`, `sendText`, `reconcileUpdate`
- `src/infrastructure/agentSessions/tauriAgentSessionClient.ts` — `ensureUpdateBridge`, `createTauriAgentSessionClient`
- `src-tauri/src/agent_sessions/transport/mod.rs` — Tauri commands and `TauriAgentSessionNotifier`
- `src-tauri/src/agent_sessions/application/lifecycle.rs` — `AgentSessionApplication::send_message`, `send_message_with_provenance`, `reconcile_startup`
- `src-tauri/src/agent_sessions/application/update_sink.rs` — `PersistedRuntimeUpdateSink`
- `src-tauri/src/agent_sessions/repository/schema.rs` and `repository/mod.rs` — common durable lifecycle
- `src-tauri/src/runtime/codex/runtime.rs` — `CodexCliRuntime`, `preflight_invocation`, `record_launch_provenance`

### Plan Builder path

- `src/features/orchestrations/EpicPlanBuilder.tsx` — `EpicPlanBuilder`, `requestPlan`
- `src/infrastructure/orchestrations/tauriManagedPlanBuilderSessionClient.ts` — managed client wrapper
- `src/infrastructure/orchestrations/tauriOrchestrationNativeQuery.ts` — `createNativeEpicPlanProposalSource`
- `src-tauri/src/orchestration/transport.rs` — managed send/action and native-query commands
- `src-tauri/src/orchestration/application.rs` — `ManagedPlanBuilderService::send_with_provenance`, `ManagedPlanBuilderRegistry`
- `src-tauri/src/orchestration/mcp.rs` — `PlanBuilderInvocation`, `submit_epic_plan_proposal`, `start_managed_invocation`
- `src-tauri/src/orchestration/repository.rs` — Plan Builder and proposal persistence

### Work Unit path

- `src-tauri/src/orchestration/sprint_runner_transition.rs` — `SprintRunnerTransitionService`, Handler/Implementer reconciliation, scoped MCP adapters, schemas
- `src-tauri/src/orchestration/work_unit_execution_harness.rs` — immutable revision selection, package construction, runtime configuration
- `src-tauri/src/orchestration/execution_support.rs` — role authorization, worktree/capability grant, candidate commit/evidence
- `src-tauri/src/orchestration/conversation_harness.rs` — catalog conversion and derived immutable Handler/Implementer configurations
- `src-tauri/src/orchestration/conversation_harness_catalog.json` — declarative current profiles
- `src-tauri/src/orchestration/repository.rs` — `native_query_at` and Work Unit projection
- `src/application/orchestrations/nativeQuery.ts` — `nativeQueryProductCompositionInputV2`
- `src/features/orchestrations/components/SprintWorkspace.tsx` — `workUnitSessions`
- `src/features/orchestrations/components/WorkUnitDetailWorkspace.tsx` — productive detail surface
- `src/features/orchestrations/components/SharedAgentSessionPanel.tsx` and `src/features/agentSessions/embeddedAgentSession.ts` — embedded read/write policy
- `src/app/useOrchestrationLoad.ts` and `src/app/App.tsx` — orchestration snapshot lifetime

### Product composition

- `src/bootstrap/productApplicationComposition.ts` — frontend client composition
- `src-tauri/src/active_app.rs` — database/runtime/service/Tauri/notifier composition
- `src/infrastructure/nativeProfiles/nativeProfileConsumer.ts` and `src/app/App.tsx` — currently unconsumed Native Profile application seam
