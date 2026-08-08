# Code artifact map

This map describes the principal implementation regions in the initial operational baseline. Sibling-line additions are recorded separately and linked where they materially extend the product.

## Product frontend

| Area | Responsibility | Reachability and observations |
| --- | --- | --- |
| `src/main.tsx` | Browser entry | Always mounts `ApplicationRoot` |
| `src/app/` | Product shell, surface selection, orchestration loading, initiation confirmation, presentation adaptation | Normal product entry; sibling Product Decisions line substantially extends navigation and inspection |
| `src/bootstrap/productApplicationComposition.ts` | Constructs production frontend clients and supported/unsupported controllers | Product authority boundary; useful source of what is actually wired |
| `src/application/agentSessions/` | Browser-safe Agent Session contracts | Productive and relatively focused |
| `src/application/orchestrations/` | Durable-query decoding, product read-model composition, control contracts, compatibility projections | Productive core mixed with recorded/compatibility models; `nativeQuery.ts` is 3,279 lines and `productReadModelComposer.ts` is 1,366 lines |
| `src/features/agentSessions/` | Standalone and embedded session selection, transcript, composition, processing and diagnostics | Productive reusable conversation system |
| `src/features/orchestrations/` | Plan Builder, Epic/Sprint/Work Slice/Work Unit presentation and controls | Productive; contains both current durable projections and recorded workflow geometry |
| `src/features/conversationHarnesses/` | Harness inspection and a full management editor | Product uses read-only inspection; mutation behavior is supplied only by a recorded development source |
| `src/features/fileReview/` | File and diff presentation | Product display is reusable; producing routes differ by build and branch |
| `src/features/nativeProfiles/` | Technical Settings for native Codex profiles | Productive on the operational/native-profile line |
| `src/infrastructure/` | Tauri adapters plus retained local/legacy implementations | Mixed: current Tauri clients coexist with earlier SQLite, Git, Codex, validation, and local runtime infrastructure |

## Development and review frontend

| Area | Responsibility | Initial classification |
| --- | --- | --- |
| `src/dev/agentSessions/` and `agent-session-harness.html` | Deterministic Agent Session scenarios | Development harness; secondary Vite entry is included in builds even though Tauri opens the main entry |
| `src/dev/orchestrationSection/` | Recorded orchestration data, transcripts, proposals, adjuncts, and workflow hypotheses | Development/compatibility evidence; some models remain imported by productive presentation code |
| `src/dev/conversationHarnesses/` | Recorded mutable Harness Management source and identity catalogue | Development implementation of product-shaped management behavior |
| `src/dev/fileReview/` | Recorded working-tree, staged, range, generated, and application-owned review variants | Development and historical review fixtures |
| `src/dev/worktreeRuntime/` | Projects worktree-runtime environment into a development UI source | Development-only; no mounted product consumer found |
| `offline-review/` | Screenshots, checklists, static review pages, and earlier exploration evidence | Historical/review evidence, not runtime authority |
| `orchestration-monitoring/` | Monitoring records and reports | Operational evidence; inspect separately from product code |
| `review-tools/app-inspector/` | Background launch, status, WebView interaction, capture, comparison, and waiting tools | Developer/reviewer CLI tooling; operational line contains later owned-WebView additions |

## Rust application

| Area | Responsibility | Boundary observations |
| --- | --- | --- |
| `src-tauri/src/main.rs` | Native executable entry | Delegates entirely to library `run()` |
| `src-tauri/src/active_app.rs` | Tauri composition root | Opens storage, constructs services, reconciles startup, manages state, registers commands, and coordinates shutdown |
| `src-tauri/src/agent_sessions/` | Agent domain, lifecycle, observation, repository, transport, and live proof | Clearest domain/application/repository/transport separation in the backend |
| `src-tauri/src/runtime/` | Provider-neutral runtime types, Codex CLI adapter, protocol decoding, process supervision | Productive runtime boundary; application launch extensions allow role-specific configuration without putting orchestration identity in the adapter |
| `src-tauri/src/orchestration/application.rs` | Plan proposal and managed Plan Builder application behavior | Productive; directly participates in MCP creation and SQLite-backed orchestration state |
| `src-tauri/src/orchestration/repository.rs` | Durable orchestration storage and native query projection | Major concentration point: 4,899 lines plus a separate 2,445-line test module |
| `src-tauri/src/orchestration/bootstrap_transition.rs` | Post-confirmation materials, bootstrap MCP, Epic Runner preparation and launch | About 2,039 production lines followed by more than 8,000 inline test lines |
| `src-tauri/src/orchestration/sprint_runner_transition.rs` | Epic/Sprint/Work Slice/Handler/Implementer/review/escalation lifecycle and MCP actions | Broad productive operational spine; combines state transitions, SQL, process/Git effects, Harness selection, and many MCP adapters |
| `src-tauri/src/orchestration/conversation_harness*` | Catalog profiles, working copies, immutable revisions, publications, and effective runtime configuration | Product functionality and executable agent configuration are intentionally intertwined |
| `src-tauri/src/orchestration/execution_support.rs` | Application-owned execution workspace and attempt authority | Productive supporting boundary with filesystem and Git/process responsibilities |
| `src-tauri/src/orchestration/work_unit_execution_harness.rs` | Work Unit Handler/Implementer capability packages | Productive execution packaging and immutable Harness binding |
| `src-tauri/src/orchestration/accepted_*` | Candidate authority and accepted integration | Productive Git/integration semantics with isolated proof suites |
| `src-tauri/src/orchestration/file_review_*` | Authorized Git evidence production and contextual entry | Productive mechanics; service composition is debug-only in the initial baseline |
| `src-tauri/src/orchestration/transport.rs` | Tauri DTO and command boundary for orchestration | Focused transport layer, though returned structures often originate directly in application/repository modules |
| `src-tauri/src/native_profiles.rs` | Native profile schema, service, CLI execution, readiness, safety policy, MCP probe, DTOs, and Tauri commands | 8,023-line monolith; about 4,782 production lines and 3,241 inline test lines |
| `src-tauri/src/storage.rs` | Active database opening and migration composition | Central migration sequencer; feature schemas remain distributed across modules |

## Rust debug, proof, and legacy regions

| Area | Responsibility | Initial classification |
| --- | --- | --- |
| `src-tauri/src/worktree_review/` | Discover, prepare, build, run, inspect, and control review instances | Entire module and Tauri exposure are `debug_assertions`-gated; substantial operator-grade implementation |
| `src-tauri/src/worktree_runtime/` | Rust worktree-runtime planning, execution, ownership, health, projection, and registry | Compiled under `#[allow(dead_code)]`; supports debug review and tests while overlapping the JavaScript runtime |
| `src-tauri/examples/` | Background launcher and review-controller examples | Proof/operator binaries, not the packaged application entry |
| `src-tauri/src/agent_sessions/live_smoke.rs` | Opt-in controlled-live Agent Session evidence | Test-only and environment gated |
| `src-tauri/src/lib.rs` | Crate declarations, active entry delegation, and older Tasks implementation | Roughly 4,500 lines of production-shaped legacy code remain before the main test module; nine task/run commands are registered but fail closed |

## Concentration signals

Large files are not automatically defects, but they identify boundaries worth understanding:

- Rust: `bootstrap_transition.rs`, `native_profiles.rs`, `lib.rs`, `orchestration/repository.rs`, and `sprint_runner_transition.rs`.
- Frontend: `nativeQuery.ts`, `ConversationHarnessInspector.tsx`, `recordedProductReadCompositionInput.ts`, `productReadModelComposer.ts`, `SprintWorkspace.tsx`, and `WorkUnitDetailWorkspace.tsx`.
- Styling: Harness Management, global `styles.css`, Agent Sessions, orchestration subdetail, and File Review each have large independent style surfaces.
