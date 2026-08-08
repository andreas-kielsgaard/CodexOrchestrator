# Rust backend and Tauri boundary

## Executive shape

The desktop product has one Rust crate. Tauri is the composition and frontend transport edge of that crate; it is not a separate thin shell around an independently deployable backend.

```text
main.rs
  -> lib.rs::run()
    -> active_app.rs::run()
      -> Tauri service composition and IPC registration
      -> Agent Session application and Codex runtime
      -> orchestration state machines
      -> native-profile service
      -> SQLite and filesystem repositories
      -> debug-only Worktree Review composition
```

The most useful architectural boundary is therefore not simply “Tauri versus Rust.” Four distinct authority and transport surfaces exist:

1. frontend-to-Rust Tauri commands;
2. Rust-to-frontend Tauri events;
3. child-agent MCP servers and tools;
4. external Codex, Git, build and process adapters.

## Tauri-specific responsibility

### Composition root

`src-tauri/src/active_app.rs` constructs and connects the productive process:

- active SQLite database and repositories;
- Native Profile service;
- Codex CLI runtime and process supervision;
- Agent Session application and frontend notifier;
- orchestration application;
- Work Unit execution Harness;
- initiation confirmation coordinator;
- bootstrap and Sprint Runner transition services;
- managed Plan Builder service;
- debug-only Human Review and contextual File Review producer.

It also performs startup reconciliation and explicit shutdown. Agent Sessions, bootstrap transitions and Sprint Runner transitions are reconciled on startup. Managed Plan Builders, transitions and direct child processes are shut down on exit; uncertain child cleanup can prevent exit.

### Commands and events

The baseline registers 45 release commands and 21 additional debug commands. The detailed inventory is in [tauri-operations.md](tauri-operations.md).

Two release event families cross back to TypeScript:

- `agent-session-update`;
- `orchestration://epic-initiation-confirmation`.

The main-window Tauri capability grants event listen/unlisten permissions. Application operations themselves are custom commands registered by the Rust application.

### Framework-coupled files

Tauri imports and DTO/command concerns are principally concentrated in:

- `src-tauri/src/active_app.rs`;
- `src-tauri/src/agent_sessions/transport/`;
- `src-tauri/src/orchestration/transport.rs`;
- `src-tauri/src/native_profiles.rs`;
- legacy `src-tauri/src/lib.rs`;
- debug `src-tauri/src/worktree_review/`.

Most Agent Session domain/application/runtime code is framework-neutral. Orchestration transition modules are also mostly called behind Tauri state, but combine more infrastructure concerns internally.

## Cross-subsystem lifecycle bus

`ManagedPlanBuilderNotifier` is a consequential internal coupling point. Every Agent Session notification is routed to:

- the managed Plan Builder registry;
- bootstrap-transition notification handling;
- Sprint Runner transition notification handling;
- the frontend Tauri event emitter.

This makes Agent Session runtime events the shared lifecycle input for several higher-level orchestration state machines. It is a productive integration mechanism, but its name understates its current scope.

## Backend logical buckets

### Agent Sessions: layered platform component

| Layer | Artifact | Responsibility |
| --- | --- | --- |
| Domain | `agent_sessions/domain.rs` | provider-neutral identity, states, provenance, options and lifecycle invariants |
| Ports | `agent_sessions/ports.rs` | repository, runtime, capability, launch-extension and notification contracts |
| Application | `agent_sessions/application/` | create/send/cancel, application-owned invocations, launch evidence, startup reconciliation and shutdown |
| Persistence | `agent_sessions/repository/` | SQLite schema, mapping and repository |
| Tauri transport | `agent_sessions/transport/` | command DTOs, state and update event |
| Provider adapter | `runtime/codex/` | capability discovery, arguments and Codex JSONL normalization |
| Process layer | `runtime/processes/` | spawn, output collection, cancellation and direct-child supervision |

This is the clearest reusable backend platform boundary in the codebase. It keeps persistence, launch acceptance, provider activity, runtime lifecycle and semantic observations as distinct facts.

### Core orchestration application and repository

- `orchestration/domain.rs` owns the Plan Builder proposal/initiation contracts.
- `orchestration/application.rs` owns the orchestration facade and managed Plan Builder lifecycle.
- `orchestration/repository.rs` owns both writes and the large native read model.
- `orchestration/transport.rs` exposes frontend operations and confirmation events.

The repository spans more than “Plan Builder persistence”: drafts, associations, proposals, initiation, initiated Epics/Sprints, application context delivery, File Review, Git authority, Harness authoring and native-query projection all meet there.

### Bootstrap and Sprint execution: vertical feature monoliths

`bootstrap_transition.rs` owns post-confirmation preparation, Bootstrap Generator launch, material completion, retry attempts, acceptance and Epic Runner launch.

`sprint_runner_transition.rs` owns most subsequent execution:

- Epic Runner to Sprint Runner selection;
- pre-start, start and repository reevaluation;
- Work Slice planning and refinement;
- Work Unit materialization and dependency relationships;
- Handler and Implementer activation;
- outcome reporting, review and disposition;
- retries and no-progress handbacks;
- Sprint handback and Epic escalation;
- application-owned Git effects and transition projection.

Both modules combine state-machine behavior, schema and SQL, Agent Session coordination, Harness selection, embedded MCP hosting, filesystem or Git effects, query assembly and extensive tests. They are productive vertical slices and obvious future segmentation candidates at the same time.

### Focused Work Unit authority modules

| Module | Product boundary |
| --- | --- |
| `execution_support.rs` | exact attempt authorization and isolated workspace capability |
| `work_unit_execution_harness.rs` | pinned Handler/Implementer Harness resolution and launch package |
| `initiated_sprint_git_authority.rs` | durable, verified repository/worktree comparison authority |
| `accepted_candidate_authority.rs` | candidate commit/tree validation and private ref pinning |
| `accepted_integration.rs` | serialized target integration and Work Unit settlement |
| `work_unit_dependency_wave.rs` | dependency activation, execution state, graph and planning-point settlement |

These are active application components, not merely proof helpers, even though much of their evidence is exercised through deterministic tests.

### Native Profiles: integrated subsystem without internal layering

`src-tauri/src/native_profiles.rs` combines:

- policy and DTO types;
- SQLite schemas and migrations;
- canonical path/filesystem identity;
- profile registration and selection;
- authentication and sandbox processes;
- setup and full-access canaries;
- MCP reporting server and dispatch;
- Windows-specific behavior;
- Tauri command functions.

Its product responsibility is substantial, but the file does not exhibit the domain/ports/application/adapters split used by Agent Sessions.

### Worktree Review and Worktree Runtime

`worktree_runtime/` is a general isolated-instance runtime with durable registry, port leases, process ownership, source projection, builds and health checks. `worktree_review/` specializes it for developer review, comparison evidence and launcher UX.

The composition is debug-only, although some generic runtime modules compile in release. This is best treated as an internal tooling subsystem rather than a loose collection of tests.

## Release reachability distinctions

### Clearly productive

- Agent Session creation, messaging, cancellation, persistence and event updates;
- Codex runtime and process supervision;
- managed Plan Builder discussion, Build request and proposal persistence;
- initiation confirmation and durable initiation;
- bootstrap through Epic Runner activation;
- Sprint Runner, Work Slice and Work Unit execution;
- Handler/Implementer reporting and review;
- candidate acceptance, integration and dependency-wave settlement;
- native orchestration queries;
- loading an already persisted scoped File Review.

### Implemented with an incomplete or conditional product path

#### Native Profile launch selection

At current inspected tip `9240364`, the shared Agent Session application requires a selected, fully ready Native Profile, binds each Session to profile/filesystem identity and overlays `CODEX_HOME` on launch. This applies to standalone and orchestration-managed Sessions because the authority is installed on the common application.

The launch still does not consume the Native Profile `project_launch` result. Selected execution mode, strict environment/configuration projection and exact danger authorization therefore remain separate from the Agent Session sandbox/runtime request. This is connected identity with incomplete policy convergence, not the earlier disconnected state.

#### Native MCP readiness action

The backend separates creation of a pending probe from reconciliation/dispatch. The frontend calls reconciliation from its visible action and has no caller for probe creation. Without a pending request, reconciliation does not create one. Backend dispatch, terminal-history and selection-race corrections are present at the current tip, but they do not close this frontend initiation gap.

#### Contextual File Review production

`request_contextual_file_review` is release-registered, but release composition supplies an unavailable producer. The Git-comparison producer is composed in debug builds through Human Review. `load_scoped_file_review` remains able to load a previously stored opaque artifact.

### Quarantined compatibility

Most of `src-tauri/src/lib.rs` is the original task/run backend. Nine operations remain registered for IPC compatibility but fail before opening the legacy database. The code is still compiled and tested, so it should be classified as quarantined implementation—not deleted code and not productive functionality.

## External process boundaries

### Productive adapters

- Codex CLI: version/help capability probes and `codex exec [resume] --json` invocations.
- Native-profile Codex CLI: login/status, Windows sandbox setup, canaries and MCP readiness exchange.
- Git: execution workspace creation, File Review capture, candidate pinning, retry refs and accepted integration.

### Debug/operator adapters

Worktree Review/Runtime discovers or invokes Git, Node, TypeScript, Vite, Vitest, Tauri CLI, Cargo, rustc and the built application. Repository-level helper scripts and the app-inspector CLI add further operator surfaces catalogued separately.

## Branch-scoped backend slices

### Product Decisions line

The sibling Product Decisions branch adds `product_decisions.rs` and seven Tauri commands for version history, explicit acceptance and Agent Session-assisted correction. Accepted decisions currently remain `not_applied`; the branch records decision authority but does not apply it to orchestration behavior.

### Sprint/Epic settlement line

The sibling final-settlement branch adds `sprint_continuation_settlement.rs` and `epic_settlement.rs`. It models continuing/attention/settled Sprint outcomes and strict final Epic settlement without adding Tauri commands. Integration occurs through Sprint reconciliation and the existing native query.

These are created product slices on divergent lines, not capabilities in the operational baseline. They must remain visibly branch-scoped until a combined integration line exists.

## Architectural implications to test later

- The Agent Session structure is a plausible template for extracting Native Profile and transition responsibilities, but the correct seams should follow authority and lifecycle—not file size alone.
- The notifier bus, shared database and native query are three major cross-feature integration points.
- Frontend Tauri IPC is only one control plane. Agent MCP tools own most orchestration mutations after initiation.
- Command registration, frontend caller presence, release composition and complete user outcome are separate reachability facts.
- Tauri transport is reasonably localized except in the two largest historical/integrated files: `lib.rs` and `native_profiles.rs`.
- Several “backend” effects are actually coordinated transactions across SQLite, Git, filesystem and child processes; refactoring must preserve those consistency boundaries.
