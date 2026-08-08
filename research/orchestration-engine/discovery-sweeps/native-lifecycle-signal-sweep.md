# Signal sweep: native lifecycle

## Evidence boundary

This is a source-led inventory at commit `924036424969de293da17d0e29c67c34d1ec7c81`. It begins with native triggers, effects, listeners and feedback routes rather than with the repository's named subsystems. It records reachability and evidence boundaries; it does not recommend keeping, pruning or refactoring anything.

Source checkout: `C:\Users\user\.codex\worktrees\1ff1\Codex Orchestrator`.

## Surface accounting

- `active_app.rs:306-395` registers 66 Tauri commands in a debug build.
- Twenty-one of those commands are behind `cfg(debug_assertions)`, leaving 45 registered in release builds.
- Nine of the 45 release commands are legacy Task commands that always return the quarantine error before storage or process effects.
- One registered release command, `probe_native_profile_mcp_reporting`, has no production frontend reference.
- Two backend-to-frontend Tauri event names are emitted: `agent-session-update` and `orchestration://epic-initiation-confirmation`.
- Many operational effects never cross a Tauri command boundary. They are entered from startup reconciliation, persisted runtime notifications, agent-only loopback MCP servers, process monitors, state destruction, or the debug-only HTTP controller.

## Startup and composition signals

| Signal | Trigger | Effect and authority boundary | Feedback route | Reachability and exact artifacts |
| --- | --- | --- | --- | --- |
| Tauri application construction | Native process calls `codex_orchestrator_lib::run()` | Builds the desktop app; setup failure aborts construction. | Build error reaches `expect`; setup errors abort setup. | All desktop builds. `main.rs:1-3`, `lib.rs:806-808`, `active_app.rs:83-397`. |
| Active storage selection and migration | Tauri `setup` | Creates app data directory, opens `codex-orchestrator-active-v3.sqlite`, configures SQLite and migrates to schema version 36. This is the common durable boundary used by Agent Sessions, orchestration and Native Profiles. | Setup error; no frontend event. | All builds. `active_app.rs:85-96`; `storage.rs:4-24`. |
| Native Profile state construction | Tauri `setup` | Ensures native-profile schema and creates in-memory child/claim registries. It does **not** probe Codex or reconcile profiles at startup. | Setup error only. | All builds. `active_app.rs:97-100`; `native_profiles.rs:1570-1606`. |
| Agent runtime and notifier composition | Tauri `setup` | Creates one system Codex CLI runtime and a composite notifier whose weak transition slots are initially empty. | Later runtime notifications; no immediate frontend fact. | All builds. `active_app.rs:119-146`. |
| Agent Session startup recovery | Tauri `setup`, before orchestration transition services exist | Replays known terminal delivery or marks every still-active invocation interrupted; persists terminal state before notifying. Because transition weak slots are empty at this point, notifications reach the managed registry and Tauri event path, not bootstrap/Sprint reconciliation. | `agent-session-update`; return count is ignored except for errors. | All builds. `active_app.rs:147-153`; `agent_sessions/application/lifecycle.rs:761-843`. |
| Product execution authority composition | Tauri `setup` | Creates the application-owned execution workspace authority and Work Unit activation harness; composition alone creates no Session, Work Unit or attempt. | Setup error only. | All builds. `active_app.rs:108-118`, `159-175`. |
| Confirmation and transition wiring | Tauri `setup` | Connects persisted initiation observer, button context scheduler, Sprint transition service, Work Unit activation and the shared notification junction. These setters reject replacement registrations. | Setup error only. | All builds. `active_app.rs:180-220`; `orchestration/confirmation.rs:124-156`. |
| Bootstrap recovery | Tauri `setup`, after notification wiring | Recreates transition rows/material paths, observes durable Agent terminal facts and reconciles every persisted initiation. Reconciliation may create Sessions, launch processes and open invocation-scoped MCP servers. | Durable tables and later Agent events; no dedicated transition event. | All builds. `active_app.rs:221-223`; `orchestration/bootstrap_transition.rs:1205-1217`. |
| Sprint operational-spine recovery | Tauri `setup`, after bootstrap recovery | For every Sprint: observes terminal evidence and reconciles the Sprint; then globally reconciles Handlers, Implementer outcomes, reviews, retries, no-progress handbacks, Epic escalation receivers, accepted candidate authority, accepted integration, dependency waves, settlement and Handlers again. | Durable SQLite projections; Agent Session events for launched/terminal invocations; frontend must query transition/native read models. | All builds; high fan-out. `active_app.rs:224-226`; `orchestration/sprint_runner_transition.rs:1652-1693`. |
| File Review state selection | Tauri `setup` | Debug builds compose Git/runtime review services and make contextual production available. Release builds register the same contextual request command with an unavailable service. | Command result only. | Conditional. `active_app.rs:260-304`; `orchestration/transport.rs:21-42`, `403-461`. |
| Optional debug HTTP controller | Debug setup plus `CODEX_ORCHESTRATOR_REVIEW_CONTROLLER=enabled` | Binds loopback, writes a protected descriptor, and exposes capability-checked review operations from a dedicated thread. | HTTP results and polled operation state; no Tauri event. | Debug and environment gated. `active_app.rs:276-285`; `worktree_review/debug_controller.rs:109-167`. |

## Persisted-event and feedback signals

| Signal | Trigger | Effect and durable/external boundary | Feedback route | Reachability and exact artifacts |
| --- | --- | --- | --- | --- |
| Runtime output event | Codex stdout/stderr reader calls the runtime update sink | Serializes per invocation, appends the event to SQLite, and may persist an external runtime context binding. Persistence precedes notification. | Composite notifier, then `agent-session-update`; frontend listener reloads authoritative Session state. | Any launched Agent invocation. `agent_sessions/application/update_sink.rs:82-173`; `agent_sessions/transport/mod.rs:17,57-65`; `src/infrastructure/agentSessions/tauriAgentSessionClient.ts:20,44-48`. |
| Runtime terminal outcome | Process monitor settles a Codex child | Finishes the durable invocation, removes its update lane, then notifies. Duplicate late terminal delivery is idempotent. | Composite notifier and `agent-session-update`. | Any launched Agent invocation. `agent_sessions/application/update_sink.rs:174-218`. |
| Composite Agent notification junction | Any persisted Agent event or terminal fact, including startup recovery | On terminal: removes managed Plan Builder server; dispatches bootstrap callback; dispatches Sprint callback; finally emits the Tauri event. All branches are attempted and errors are combined. Runtime callbacks can synchronously launch another invocation and re-enter this junction. | Tauri event after orchestration callbacks; errors return to the runtime delivery path and become diagnostics. | All builds; high fan-out. `active_app.rs:9-79`; `agent_sessions/application/update_sink.rs:220-245`. |
| Epic initiation confirmation event | Button Tauri command or Plan Builder MCP tool creates/resolves an in-memory request | Publishes Requested/UserConfirmed/UserRejected and, after a confirmed durable initiation, Applied/Persisted/Projected. Pending request identity is in memory; the initiated Epic is durable. | `orchestration://epic-initiation-confirmation`; frontend listener plus command/MCP result. | All builds. `orchestration/confirmation.rs:14-44,158-229,307-390`; `orchestration/transport.rs:90-104`; `src/infrastructure/orchestrations/tauriEpicInitiationConfirmation.ts:51-55`. |
| Bootstrap terminal notification | Composite notifier receives a correlated terminal invocation | Records lifecycle, removes the invocation-scoped MCP server, forwards Epic Runner terminal to Sprint service, and reconciles the initiation. | Durable transition query; Agent event is the wake-up signal. | Only correlated terminal IDs. `orchestration/bootstrap_transition.rs:1219-1240`. |
| Sprint runtime notification | Composite notifier receives persisted runtime events or terminal facts | Correlates invocation identities to operational roles, records provider/lifecycle evidence, stops role MCP servers and enters targeted reconciliation. | Durable Sprint/native query; original Agent event. | Only known Sprint operational identities. `orchestration/sprint_runner_transition.rs:1330-1518` and role correlation queries below it. |
| Native Profile completion observation | A Native Profile query or later action invokes reconciliation | Polls stored login/setup/full-access children, terminalizes attempts, validates receipts/deadlines/identity and updates readiness/attention. No watcher publishes completion spontaneously. | Synchronous returned profile/query DTO; caller must query again. | Native Profile settings or launch-authority consumption. `native_profiles.rs:1608-1623,3259-3655`; `src/infrastructure/nativeProfiles/nativeProfileClient.ts:469-502`. |

## Always-registered Tauri command signals

All rows below are registered in both debug and release builds unless a row says otherwise. A "frontend adapter" establishes code reachability, not that a particular view is mounted.

### Metadata and quarantined legacy surface

| Command signal | Trigger and effect | Boundary and feedback | Reachability and artifacts |
| --- | --- | --- | --- |
| `app_metadata` | Returns three static strings. | No durable/external effect; direct result. | Frontend caller. `lib.rs:701-708`; `src/infrastructure/tauriCommands.ts:32`. |
| `load_open_task_dashboard` | Immediately fails `ensure_legacy_tasks_available`. | Does not open the dormant legacy database; rejected promise. | Registered and frontend adapter exists. `lib.rs:710-715,810-812`; `src/infrastructure/tauriCommands.ts:37`. |
| `register_task_worktree` | Same quarantine guard before worktree/database mutation. | No effect; rejected promise. | `lib.rs:716-727`; frontend `tauriCommands.ts:41`. |
| `register_task_repo` | Same quarantine guard before repo/database mutation. | No effect; rejected promise. | `lib.rs:728-739`; frontend `tauriCommands.ts:45`. |
| `discover_task_repos` | Same quarantine guard before filesystem/Git discovery. | No effect; rejected promise. | `lib.rs:740-747`; frontend `tauriCommands.ts:49`. |
| `create_open_task` | Same quarantine guard before task insertion. | No effect; rejected promise. | `lib.rs:748-759`; frontend `tauriCommands.ts:53`. |
| `update_open_task` | Same quarantine guard before update. | No effect; rejected promise. | `lib.rs:760-772`; frontend `tauriCommands.ts:60`. |
| `archive_open_task` | Same quarantine guard before archive mutation. | No effect; rejected promise. | `lib.rs:773-781`; frontend `tauriCommands.ts:64`. |
| `load_task_run_detail` | Same quarantine guard before read. | No effect; rejected promise. | `lib.rs:782-787`; frontend `tauriCommands.ts:87`. |
| `start_codex_task_run` | Same quarantine guard before Codex, Git diff, validation, artifact or event operations. | No process or durable effect; rejected promise. | Registration materially overstates behavior. `lib.rs:788-803`; frontend `tauriCommands.ts:75`. |

### Agent Session command signals

| Command signal | Trigger and effect | Boundary and feedback | Reachability and artifacts |
| --- | --- | --- | --- |
| `create_agent_session` | Creates an available Session in active SQLite; no process. | Durable Session; direct DTO. | Frontend caller. `agent_sessions/transport/mod.rs:68-77`; application `lifecycle.rs:180-233`; frontend `tauriAgentSessionClient.ts:61`. |
| `list_agent_sessions` | Reads durable summaries. | Read-only; direct list. | Frontend caller. `transport/mod.rs:79-89`; frontend line 65. |
| `load_agent_session` | Reads one Session, invocations, events and diagnostics. | Read-only; direct details. | Frontend caller and event-refresh target. `transport/mod.rs:91-101`; frontend line 57. |
| `send_agent_session_message` | Creates or reuses a Session, persists a pending invocation, requires the selected Native Profile launch authority, preflights Codex, records running/launch acceptance, and starts or resumes a supervised child. Failure after persistence becomes durable failed evidence. | SQLite plus external Codex child; direct IDs, then streamed Agent events. | Frontend caller. `transport/mod.rs:103-113`; `application/lifecycle.rs:515-704`; frontend lines 87-90. |
| `cancel_agent_invocation` | If active, requests exact supervised child termination; otherwise replays current durable state. | External termination followed by durable terminal callback; direct DTO plus Agent event. | Frontend caller. `transport/mod.rs:115-124`; `application/lifecycle.rs:733-759`; frontend lines 93-94. |

### Planning, confirmation and read-model command signals

| Command signal | Trigger and effect | Boundary and feedback | Reachability and artifacts |
| --- | --- | --- | --- |
| `send_managed_plan_builder_message` | Starts an invocation-scoped Plan Builder MCP server, binds/reconciles the draft context and sends user-provenance text through Agent Session launch. Harness model/sandbox are fixed. | Durable draft/Session/invocation plus Codex and loopback MCP server; direct IDs then Agent events. | Frontend caller. `orchestration/transport.rs:331-355`; service `application.rs:329-574`; frontend `tauriManagedPlanBuilderSessionClient.ts:28`. |
| `request_managed_plan_builder_action` | Same managed path with application-authored "Build the epic plan based on what we have discussed" input. | Same boundaries and feedback. | Frontend caller. `transport.rs:356-369`; `application.rs:575-589`; frontend lines 34-37. |
| `reconcile_managed_plan_builder_session` | Verifies Session, creates/replays its durable draft binding and optionally applies the initial title. | SQLite; direct draft IDs. | Frontend draft lifecycle caller. `transport.rs:144-153`; `application.rs:180-211`; frontend `tauriEpicPlanningDraftLifecycle.ts:12`. |
| `load_managed_plan_builder_harness_inspection` | Compares durable binding/history with current Harness and launch evidence. | Read-only over durable state/catalog; direct inspection. | Frontend inspector caller. `transport.rs:159-168`; frontend `tauriConversationHarnessInspectorSource.ts:66`. |
| `update_epic_planning_draft_title` | Idempotently mutates the exact bound draft title. | SQLite; success/error only. | Frontend draft lifecycle caller. `transport.rs:282-293`; frontend line 33. |
| `cancel_epic_planning_draft` | Idempotently terminalizes the exact draft. | SQLite; success/error only. | Frontend draft lifecycle caller. `transport.rs:294-304`; frontend line 43. |
| `request_epic_initiation_confirmation` | Creates an in-memory Button-source confirmation request and emits Requested. It does not initiate the Epic. | In-memory registry; Tauri event plus direct request. | Frontend caller. `transport.rs:234-258`; frontend `tauriEpicInitiationConfirmation.ts:29`. |
| `resolve_epic_initiation_confirmation` | Confirms/rejects one in-memory request. Confirm performs durable initiation, invokes transition observer, checks projection and schedules Button context. | In-memory decision plus SQLite initiation; events and direct resolution/error. | Frontend caller. `transport.rs:259-281`; frontend line 40. |
| `load_orchestration_native_query` | Returns a repository snapshot; notifications/refresh are explicitly non-authoritative. | SQLite read; direct query. | Frontend caller. `transport.rs:370-378`; frontend `tauriOrchestrationNativeQuery.ts:26`. |
| `load_scoped_file_review` | Resolves an opaque reference through durable authorization and returns bounded review facts. | SQLite/read-only; direct available/unavailable shape. | Frontend contextual and scoped callers. `transport.rs:380-401`; frontend `tauriScopedFileReview.ts:43`. |
| `request_contextual_file_review` | Debug: derives a private Sprint relation, runs Git-backed production, stores review facts, then reauthorizes the opaque load. Release: returns `{status: unavailable, reason: not_ready}` because service state is deliberately absent. | Debug filesystem/Git plus SQLite; direct result only. | Frontend caller, but behavior is build-conditional. `transport.rs:403-461`; state composition `active_app.rs:260-304`; frontend `tauriContextualFileReview.ts:38`. |
| `load_epic_bootstrap_transition_query` | Reads bootstrap transitions. | SQLite read; direct query, no push refresh. | Frontend caller. `transport.rs:463-468`; frontend `tauriEpicBootstrapTransition.ts:19`. |
| `load_sprint_runner_transition_query` | Reads the operational-spine transition projection. | SQLite read; direct query, no dedicated event. | Frontend caller. `transport.rs:470-475`; frontend `tauriSprintRunnerTransition.ts:16`. |

## Native Profile command signals

Native Profile commands return a profile/query DTO and emit no Tauri event. Child completion is observed only when `load_native_profile_query` or another reconciling action runs.

| Command signal | Trigger and effect | Durable/external boundary and guards | Reachability and artifacts |
| --- | --- | --- | --- |
| `load_native_profile_query` | Revalidates every filesystem identity; reconciles external sandbox adoption, login/setup/full-access children; expires MCP probes; then returns profiles. This is not a pure read. | Filesystem/child polling plus SQLite updates; direct DTO. | Frontend settings read. `native_profiles.rs:1608-1623,4762-4767`; frontend `nativeProfileClient.ts:469`. |
| `register_native_profile` | Canonicalizes an existing directory, rejects an application-owned marker, records filesystem identity and default readiness/mode. | Existing filesystem plus SQLite. | Frontend action. `native_profiles.rs:1625-1636,4768-4774`; frontend line 488. |
| `create_dedicated_native_profile` | Creates an app-owned Codex home and marker, then records it; removes the new directory on failure. | Filesystem plus SQLite. | Frontend action. `native_profiles.rs:1638-1660,4775-4780`; frontend line 489. |
| `select_native_profile` | Requires current continuity, atomically makes one profile selected, invalidates an unselected profile's adoption evidence and reconciles its setup attempts. | SQLite plus filesystem validation. | Frontend action. `native_profiles.rs:1718-1755,4781-4787`; frontend line 490. |
| `select_native_profile_execution_mode` | Sets workspace-write or danger-full-access mode; does not authorize danger mode. | SQLite after active-profile guard. | Frontend action. `native_profiles.rs:1757-1772,4788-4796`; frontend line 491. |
| `authorize_native_profile_danger_full_access` | Requires selected active profile, selected danger mode, and exact supported CLI surface; writes filesystem-bound versioned authority. | CLI help/version probe plus SQLite authority; no launch. | Frontend action. `native_profiles.rs:1774-1809,4797-4805`; frontend line 492. |
| `revoke_native_profile_danger_full_access` | Marks current authority revoked. | SQLite; selected-active guard. | Frontend action. `native_profiles.rs:1811-1820,4806-4812`; frontend line 493. |
| `request_native_profile_login` | Persists a pending attempt, starts `codex login`, stores the child in memory and records launch acceptance/attention. | Browser-capable external Codex child plus SQLite; exact selected profile and isolated environment. | Frontend action. `native_profiles.rs:1822-1889,4813-4819`; frontend line 494. |
| `refresh_native_profile_readiness` | Reconciles login child, runs `codex login status` and records authenticated/unauthenticated. | Synchronous external CLI plus SQLite; raw auth payload is not read. | Frontend action. `native_profiles.rs:1891-1925,4820-4826`; frontend line 495. |
| `request_native_profile_sandbox_initialization` | Persists an attempt and starts supported Windows `codex sandbox setup --elevated --current-user --codex-home <profile>`. Launch/process success does not confirm UAC or readiness. | External elevated setup child plus SQLite; selected-active/surface guards. | Frontend action. `native_profiles.rs:1927-1938,3008-3257,4827-4835`; frontend line 496. |
| `confirm_native_profile_sandbox_initialization` | Human acknowledgment after a terminal-succeeded product request; marks sandbox initialized. | SQLite only; requires prior durable attempt outcome. | Frontend action. `native_profiles.rs:1940-1970,4836-4844`; frontend line 497. |
| `verify_native_profile_preprovisioned_sandbox` | Reads CLI surface and bounded `config.toml` evidence for external elevated mode; records observation and requires later confirmation. | Filesystem observation plus SQLite; does not write external config or claim UAC. | Frontend action. `native_profiles.rs:1972-2013,4845-4853`; frontend line 498. |
| `confirm_native_profile_preprovisioned_sandbox_adoption` | Revalidates exact profile, executable/version/capability/config observation; records explicit product adoption. | Filesystem/CLI observation plus SQLite; invalidates on drift. | Frontend action. `native_profiles.rs:2015-2058,4854-4862`; frontend line 499. |
| `run_native_profile_workspace_write_canary` | Requires observed initialized sandbox; starts an application-authored command in an owned probe root. Receipt, not exit alone, determines pass. | External sandbox child, temporary command/receipt files and SQLite. | Frontend action. `native_profiles.rs:2060-2081,3008-3458,4863-4869`; frontend line 500. |
| `run_native_profile_danger_full_access_canary` | Requires selected danger mode plus current explicit filesystem-bound authority; starts a bounded Codex prompt and settles only from the exact sentinel. | External unrestricted Codex child, owned sentinel and SQLite evidence. | Frontend action. `native_profiles.rs:2666-2758,4870-4878`; frontend line 501. |
| `probe_native_profile_mcp_reporting` | Creates/replays the durable pending application-owned MCP reporting request; does not dispatch it. | SQLite request/attention only; selected-active guard. | **Registered but no production frontend reference.** `native_profiles.rs:2083-2087,2192-2246,4879-4886`. |
| `reconcile_native_profile_mcp_reporting` | Claims an already-pending request, opens a token-protected loopback MCP server, runs one strict Codex exchange, accepts only the correlated tool receipt, then stops the server. It never creates a missing request. | SQLite claim/receipt plus call-local server and synchronous Codex process. | Frontend action named `probeMcp` invokes this command directly. With no pending request it returns unchanged state. `native_profiles.rs:2090-2190,4887-4895`; frontend `nativeProfileClient.ts:502`, `NativeProfileSettings.tsx:66`. |

## Debug-only command signals

These 21 commands are omitted from the release handler by `cfg(debug_assertions)` at `active_app.rs:349-394`. Their frontend adapters still exist in ordinary TypeScript source.

| Command signals | Trigger and effect | Boundary and feedback | Exact artifacts |
| --- | --- | --- | --- |
| `list_human_review_worktrees`, `list_human_review_instances` | Read discovered sources and retained instance registry. | Git/filesystem/in-memory reads; direct results. | `worktree_review/transport.rs:49-61`; frontend `tauriHumanReviewLauncher.ts:11-12`. |
| `prepare_human_review_instance`, `build_human_review_instance`, `start_human_review_instance` | Prepare isolated material, run the build, or launch the exact review app. Tauri variants use `spawn_blocking` but await completion. | Review SQLite/filesystem/toolchain/owned child; direct terminal result. | `transport.rs:63-72,167-190,209-218`; frontend lines 14-18. |
| `human_review_operation_progress`, `list_human_review_operation_progress` | Read operation progress. | In-memory/review state read; direct result. | `transport.rs:74-87`; frontend lines 20-21. |
| `human_review_instance_detail`, `human_review_instance_comparison` | Read detailed retained evidence or Git/file comparison. | Filesystem/Git/review registry reads. | `transport.rs:89-103`; frontend lines 23-30. |
| `human_review_launcher_proof_navigation`, `human_review_launcher_detail_navigation`, `human_review_launcher_proof_presentation` | Read bounded proof/navigation artifacts. | Filesystem/read-only; optional direct result. | `transport.rs:105-124`; frontend lines 33-35. |
| `mark_worktree_build_ready` | Mutates child window title/size/visibility and atomically writes the rendered readiness marker. | Native window plus filesystem marker. Requires isolated-build environment variables. | `transport.rs:126-165`; frontend `tauriWorktreeBuild.ts:19`. |
| `worktree_review_proof_navigation` | Reads and validates a small environment-selected navigation artifact. | Filesystem read; fails outside isolated proof surface. | `debug_controller.rs:43-76`; frontend `tauriWorktreeBuild.ts:20`. |
| `worktree_build_context`, `worktree_build_detail`, `worktree_build_comparison` | Derive Git/worktree context, retained runtime detail and committed/uncommitted comparison. | Git/filesystem reads; require isolated-build environment. | `worktree_build.rs:68-72`; `detail.rs:179-210`; `comparison.rs:67-71`; frontend `tauriWorktreeBuild.ts:10-14`. |
| `status_human_review_instance`, `focus_human_review_instance`, `stop_human_review_instance`, `recover_human_review_instance` | Observe, activate, terminate exact owned process tree, or reconcile stale ownership. | Native process/window/registry effects; direct result. | Macro registration `transport.rs:192-207`; frontend `tauriHumanReviewLauncher.ts:36-39`. |

## Background listeners, threads and owned external work

| Signal | Trigger | Effect/boundary | Feedback and lifetime | Build/reachability and artifacts |
| --- | --- | --- | --- | --- |
| Codex stdout/stderr readers | Agent runtime launches a child | Reader threads convert pipes into `ProcessOutput`; JSONL protocol becomes durable events. | Runtime update sink; joined during settlement. | All builds, per invocation. `runtime/processes/monitoring.rs:35-82`; `runtime/codex/runtime.rs:56-119`. |
| Codex process monitor | Supervised child launch | One named monitor waits, handles cancellation/shutdown, reaps child and emits one terminal outcome. | Persisted terminal update; supervisor registry/condvar. | All builds, per invocation. `runtime/processes/supervisor.rs:72-175`; `monitoring.rs:105-199`. |
| Plan Builder MCP listener | Managed send/request-plan | Loopback server with bearer, Host and Origin guards exposes the role's tool set to one invocation. | Tools mutate durable planning/confirmation state; registry stops server on terminal or exit. | All builds, invocation scoped. `orchestration/mcp.rs:430-530`; `orchestration/application.rs:101-129,329-574`. |
| Bootstrap MCP listener | Bootstrap reconciliation launches generator | Loopback bearer/Host/Origin-guarded `complete_epic_bootstrap`. | Semantic materials and transition reconciliation; registry terminal/exit cleanup. | All builds, invocation scoped. `bootstrap_transition.rs:1928-2025`; registry `1120-1149`. |
| Sprint operational MCP listeners | Sprint reconciliation launches role invocations | A shared loopback server pattern exposes role-specific tool sets for pre-start, start, planning, handback, escalation, Planner, Handler, Implementer reporting and review. | Tool results persist into the operational spine; terminal notification removes server; bootstrap shutdown drains all. | All builds, invocation scoped. `sprint_runner_transition.rs:1518-1557,4460-4512` plus server macro/tool adapters. |
| Native CLI sink drains | Login/setup/full-access child spawn | Two detached sink-only threads drain raw stdout/stderr; product intentionally retains no output. | No user feedback; handles are dropped after outer child settles and streams close later. | All builds for asynchronous Native CLI children. `native_profiles.rs:746-798`. |
| Native login/setup/full-access child registries | Native commands start child | In-memory ownership maps; no watcher. Query/action reconciliation polls, times out, validates identity/receipt and persists state. | Returned query/action DTO on a later call. `NativeProfileService::drop` terminates remaining children. | All builds. `native_profiles.rs:1570-1578,3259-3655,3771-3790`. |
| Native MCP reporting listener | Reconcile command claims pending request | Call-local loopback bearer/Host-guarded MCP server; synchronous strict Codex exchange; receipt channel capacity one. | Command returns only after receipt settlement/failure and server join. | All builds, but request creation currently has no frontend caller. `native_profiles.rs:1327-1450,2090-2190`. |
| Debug review controller | Debug setup with enable environment | Dedicated Tokio/axum loopback server, protected descriptor/capability. | HTTP command results; Drop signals graceful shutdown and removes descriptor. | Debug and environment only. `debug_controller.rs:78-167`. |
| Debug controller operation worker | HTTP `begin_prepare`, `begin_build` or `begin_open` | Named background thread runs work and records terminal result/progress. | HTTP client polls operation progress; no Tauri event. | Debug controller only. `worktree_review/service.rs:553-650`; HTTP routing `debug_controller.rs:387-557`. |
| Human Review runtime child | Tauri or debug controller start operation | Builds/starts a separate isolated Tauri app under exact worktree/process authority; output pumps and Windows Job ownership live below the facade. | Direct/polled review status and retained review registry. | Debug only. Composition `worktree_review/composition.rs:10-64`; execution `worktree_runtime/execution.rs:78-132`; ownership `worktree_runtime/ownership.rs:289-510`. |

## Shutdown signals

| Signal | Trigger | Effect and authority boundary | Feedback | Exact artifacts |
| --- | --- | --- | --- | --- |
| Managed Plan Builder shutdown | Tauri `ExitRequested` | Drains active Plan Builder MCP servers. | No frontend feedback. | `active_app.rs:403-407`; `orchestration/application.rs:127-129,175-177`. |
| Bootstrap/Sprint shutdown | Same exit request | Drains bootstrap MCP servers, then attached Sprint role MCP servers. | No frontend feedback. | `active_app.rs:408-412`; `bootstrap_transition.rs:1276-1283`; `sprint_runner_transition.rs:1555-1561`. |
| Agent runtime shutdown | Same exit request | Stops accepting work, gives direct children two seconds, terminates/escalates, reaps every child and waits for terminal callbacks. If authority reports failure, Tauri exit is prevented for retry. | `stderr` diagnostic; application stays open on failure. | `active_app.rs:413-424`; `agent_sessions/application/lifecycle.rs:845-849`; `runtime/codex/runtime.rs:26,460-464`; `runtime/processes/supervisor.rs:211-285`. |
| Native Profile state destruction | Tauri state eventually drops | Best-effort terminates login, setup and full-access canary children. This is not called explicitly before Agent runtime shutdown and cannot prevent exit on failure. | None. | `native_profiles.rs:3771-3790`. |
| Debug controller destruction | Managed debug state drops | Sends graceful-shutdown signal and removes descriptor; no join handle is retained. | None. | `worktree_review/debug_controller.rs:78-90`. |
| Supervisor Drop backstop | Last process supervisor owner drops | Calls shutdown and ignores the returned error. | None. | `runtime/processes/supervisor.rs:465-469`. |

## High-fan-out junctions

These are topology flags, not disposition judgments.

1. **`active_app::setup`** constructs storage, native authority, runtime, notifier, transition graph, review variants and all Tauri state, then performs three ordered reconciliation passes. A failure anywhere prevents normal startup. `active_app.rs:84-305`.
2. **`ManagedPlanBuilderNotifier::notify`** is the synchronous junction from one durable Agent fact into managed-server cleanup, bootstrap reconciliation, Sprint reconciliation and frontend emission. `active_app.rs:9-79`.
3. **`SprintRunnerTransitionService::reconcile_startup`** can traverse and mutate nearly every operational stage and can launch multiple role Sessions/processes/MCP servers. `sprint_runner_transition.rs:1652-1693`.
4. **`PersistedRuntimeUpdateSink`** is the boundary where process output becomes ordered SQLite evidence, runtime binding, orchestration wake-up and UI event. `agent_sessions/application/update_sink.rs:82-245`.
5. **`NativeProfileService::query`** is named and exposed as a load operation but also performs continuity checks, child reconciliation, timeout settlement and durable readiness/attention changes. `native_profiles.rs:1608-1623`.
6. **`InitiationConfirmationCoordinator::resolve`** joins in-memory human decision, durable Epic initiation, bootstrap observer, projection verification, multiple frontend events and Button-only context scheduling. `orchestration/confirmation.rs:205-390`.
7. **Exit handling** drains three MCP registries and the Agent runtime through nested services; only the Agent runtime result can prevent exit, while Native Profile child cleanup is deferred to Drop. `active_app.rs:399-426`.

## Registration and behavior mismatches to retain as signals

These findings describe observable source relationships without assigning a future disposition.

- Nine legacy Task commands are registered and have frontend adapters, but every call fails at one shared quarantine guard before any dormant implementation runs. Registration is not capability evidence. `lib.rs:710-812`.
- `start_codex_task_run` is registered with extensive native Codex/Git/validation implementation behind it, but the guard makes all of that unreachable through Tauri at this snapshot. `lib.rs:788-803,1042-1390`.
- The registered `probe_native_profile_mcp_reporting` command is the only Tauri seam that creates the durable pending probe, but no production frontend file references it. The visible `probeMcp` action calls only the reconciler, which explicitly does not create a request. `native_profiles.rs:2083-2246,4879-4895`; `nativeProfileClient.ts:502`.
- `request_contextual_file_review` is registered in release builds, but release composition always supplies unavailable state and returns `not_ready`; production Git capture exists only in debug composition. `active_app.rs:260-304`; `orchestration/transport.rs:403-461`.
- Twenty-one Human Review/Worktree Build commands appear in the handler source but do not exist in release registration because each entry and its module are debug-gated. `active_app.rs:1-2,349-394`; `lib.rs:19-20`.
- `load_native_profile_query` sounds read-only but can kill invalid children, expire probes and persist readiness/attention/lifecycle updates. Its direct return, not an event, is the feedback boundary. `native_profiles.rs:1608-1623`.
- Transition queries are registered read operations, but transition progress has no dedicated frontend event. The Agent Session event can be a wake-up hint only; the durable transition queries are the authority. `orchestration/transport.rs:463-475`; `active_app.rs:29-78`.

## Reachability limits

- This sweep establishes source registration and production TypeScript references, not that every adapter is mounted in every route or exercised in a packaged build.
- Test-only threads, listeners and commands below `cfg(test)` were excluded.
- Internal MCP tools are agent-callable only while their exact invocation-scoped listener and bearer configuration are alive; they are not Tauri frontend commands.
- Native Profile readiness is not reconciled automatically during `NativeProfileService::open`; it becomes current through query/action/launch-authority calls.
- A Tauri event is a notification route, not durable authority. Both event consumers must reload or interpret durable state.
