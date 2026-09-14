# Architecture Notes

Updated: 2026-09-14

This document describes the current code architecture. It should explain where new work belongs and
which boundaries should stay intact.

## Execution Configuration and Session Event target

The active overhaul of mixed Harness and generic Workflow Role concepts is documented in
[`docs/session-event-model/`](session-event-model/README.md). That guide describes the working target,
recommended implementation shape, UI mapping and migration sequence. The rest of this document
continues to describe the wider application architecture and contains earlier current-state and
legacy context that has not all been rewritten around the new model.

The concise target dependency direction is:

```text
Runtime integration -> Execution Configuration
Workflow authoring -> Session Event definitions
Session Event runtime -> Agent Session ports
Agent Sessions -> runtime execution
Identity -> independent presentation and assignment
```

Use the target guide for Capability Profile, Node Profile, Session Profile, Session Event and
Workflow-node work. Treat older Harness/Role material as historical unless a currently active
feature explicitly still depends on it.

## Runtime Shape

- Desktop shell: Tauri v2.
- UI: React, TypeScript, Vite.
- Product composition mounts Orchestration, Workflow, Agent Sessions, Harness Management,
  Worktree Review, contextual File Review, Product Decisions, and Native Profile settings through
  explicit application clients.
- The core Agent Session lifecycle is Rust-first: durable records, SQLite history, application
  coordination, Codex protocol handling, and process supervision live behind Tauri commands.
- Current product capabilities share one managed ActiveDatabase where cross-feature configuration
  or runtime facts require it. Worktree Review keeps its build, source, receipt, and cleanup facts
  in a separate AppData database.

## Active product composition

`src-tauri/src/active_app.rs` is the composition root. It opens the managed ActiveDatabase once,
constructs the current product applications, registers their Tauri states and commands, and owns
shutdown ordering. `src/bootstrap/productApplicationComposition.ts` is the matching browser-side
composition root; `src/app/App.tsx` only selects and renders already-constructed feature clients.

The shared repository catalog is a product capability rather than Worktree Review state. It stores
registered local repositories and disclosure provenance in ActiveDatabase, then derives branch and
worktree availability from live standard Git reads. Workflow and Worktree Review consume this one
catalog. Optional Codex and GitHub discovery add registration candidates but do not define Git
identity.

Worktree Review remains independently responsible for selected review context, worktree/source
evidence, build attempts, retained outputs, and cleanup receipts. Its reusable
`worktree_application` dependency owns only physical capture, checkout, compilation, and opening.

## Boundary Rules

- React should consume application/domain facades, not parse Git, open SQLite, or execute Codex.
- Shared Rust Git facts stay under `src-tauri/src/repository_context/`.
- Product database composition stays under `src-tauri/src/product_database/` and `src-tauri/src/storage.rs`; capability modules own their schemas and repositories.
- Lifecycle state changes stay in application services, not UI components.
- Agent Session execution enters through the provider-neutral `AgentRuntime` port and its current
  Codex-specific adapter; Codex credentials remain owned by Codex.
- Agent provider processes are owned through `ProcessSupervisor`. Worktree Review compilation and
  opening enter through `worktree_application`, which has its own bounded process-effect contract.
- Persist raw runtime output before deriving transcript presentation. Agent Sessions store
  ordered raw runtime events.

## Agent Session Vertical Slice

Agent Sessions are a first-class product surface, independent from the legacy task dashboard. The
responsibility flow is:

```text
React Agent Session screen
  -> TypeScript AgentSessionClient
    -> Tauri commands and persisted update event
      -> Rust AgentSessionApplication
        -> SQLite AgentSessionRepository
        -> CodexCliRuntime
          -> ProcessSupervisor
```

The Rust backend is authoritative. It persists a submitted invocation before launch, persists each
ordered runtime event before notifying the WebView, separately captures the external Codex context
ID, and persists terminal state idempotently. The frontend projects durable records into a
conversation: live work is open, completed work is collapsed, and the final response remains
prominent. Reload and short-interval active reconciliation repair missed transient events.
Startup opens and reconciles durable history without executing Codex capability probes. Provider
resolution and absence therefore affect an invocation, not access to stored sessions.

Primary module map:

- `src/application/agentSessions/`: serializable client contract and DTOs
- `src/infrastructure/agentSessions/`: Tauri client and persisted update subscription
- `src/features/agentSessions/`: controller, transcript projector, and focused UI components
- `src-tauri/src/agent_sessions/`: domain, ports, repository, lifecycle, and Tauri transport
- `src-tauri/src/runtime/codex/`: Codex command capability mapping and JSONL normalization
- `src-tauri/src/runtime/processes/`: direct-child ownership, streaming, cancellation, and shutdown

The Agent Session lifecycle is independent of the retired Task/TaskRun model.

## Application and infrastructure boundaries

TypeScript contracts live under `src/application/`; feature-specific Tauri adapters live under
`src/infrastructure/`. Rust capability modules own domain rules, application services, and durable
repositories. `product_database` assembles the current schemas over the managed `ActiveDatabase`.

`active_app.rs` registers Agent Session, Workflow, Harness, Native Profile, Product Decision,
repository-catalog, and Worktree Review commands. Worktree Review commands are available in both
debug and release builds. Repository discovery and branch-history commands keep blocking work off
the UI-facing async executor.

Agent Session notifications use `agent-session://persisted-update`. Event listen/unlisten
permissions are scoped to the main window. Notifications identify Sessions and invocations;
durable repository records remain authoritative after missed events or restart.

## Retired task implementation

The original Task/TaskRun/dashboard implementation, its Tauri commands, and its isolated tests
have been removed. Current startup opens `codex-orchestrator-active-v3.sqlite` and leaves the old
`codex-orchestrator.sqlite` and active-v2 files untouched. The old migration registry is no longer
part of the application. Current Agent Session schemas and active-v3 migration support remain.

## UI Layer

Location: `src/app/`, `src/features/`, `src/main.tsx`, `src/styles.css`

The app shell switches among composed product surfaces and does not construct their domain
services. Agent Session state is owned by its feature controller, Workflow owns its definition and
instance UI, and Worktree Review owns its branch/build flow. The shared repository selector remains
an injected application boundary rather than Workflow or Worktree Review component logic.

## Testing And Verification

The reliable verification set today is:

- `npm run lint`
- `npm run format:check`
- `npm run test`
- `npm run build`

Rust/Cargo verification is environment-dependent. When Rust is installed, run the Rust
format/check/test/build checks in `src-tauri/` plus `npm run build:tauri`. The installed Codex help
compatibility probe is intentionally ignored by the ordinary suite and should be executed
explicitly for Agent Session release verification.
