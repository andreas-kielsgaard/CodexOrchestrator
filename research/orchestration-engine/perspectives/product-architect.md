# Product-architect perspective

## System shape

The product is a local orchestration platform built inside one Tauri Rust crate. Its main architectural planes are:

| Plane | Responsibility |
| --- | --- |
| Experience | React product shell, reusable Agent Session workspace, Epic/Sprint/Work Unit views, settings and contextual review |
| Frontend application | typed contracts, native-query decoding, presentation composition, navigation and capability adapters |
| Tauri transport | release/debug command registration and two event families |
| Agent platform | durable Agent Sessions, Codex adapter, process supervision and normalized runtime events |
| Orchestration control | Plan Builder, bootstrap, Sprint/Work Slice/Work Unit state machines and reconciliation |
| Agent capability | invocation-scoped MCP servers with durable semantic authorization |
| State/evidence | active SQLite store, immutable Harness objects, materials, workspaces, Git refs and review stores |
| External effects | Codex CLI, Git, filesystem and debug build/runtime tools |

## Strong boundaries already present

- Agent Sessions have explicit domain, ports, application, persistence, transport and provider adapters.
- Tauri composition is centralized in `active_app.rs`.
- Managed tools are bound to exact agent invocations rather than exposed globally.
- Initiated Sprint Git authority and execution-support capability hide raw repository paths from agents.
- Candidate pinning and accepted integration isolate Git authority from Handler judgment.
- The native query acts as a stable frontend read boundary over many internal tables.

## Architectural concentration points

### Shared database as coordination bus

Roughly 95 current tables span multiple independently reconciled state machines in one SQLite database. Schema constants are feature-local, migration ordering is partly centralized, and several services hold independent connections. The database is both persistence and an internal integration protocol.

### Agent Session notifier as lifecycle bus

One notifier forwards Session updates to Plan Builder, bootstrap, Sprint transitions and the frontend. This is a legitimate integration point whose current name and ownership no longer communicate its full role.

### Native query as projection bus

Most frontend understanding is assembled through one large Rust native query, a strict TypeScript decoder and large presentation composers. This provides a valuable anti-chattiness boundary while concentrating compatibility and evolution cost.

### Vertical transition monoliths

Bootstrap and especially Sprint Runner transition files own state machine, SQL, MCP, Harness selection, prompts, process/Git effects, reconciliation and projection. They preserve vertical correctness but obscure reusable infrastructure and bounded-context ownership.

## Configuration architecture

Harness behavior has overlapping sources:

1. compiled base catalogue;
2. Rust-authored stage variants;
3. durable working copies and immutable revisions;
4. product skill assets;
5. generated prompts;
6. MCP definitions/instructions;
7. dynamic CLI/runtime settings.

This is the clearest case where “configuration” is application logic. A future architecture should provide effective-configuration provenance per invocation, not merely choose one file as canonical.

## Authority surfaces

The backend exposes four transport/authority surfaces:

- Tauri commands: user/frontend requests and queries;
- Tauri events: Agent Session updates and initiation confirmation;
- MCP tools: managed-agent mutations;
- external adapters: Codex, Git, filesystem and review/build processes.

Each has different caller identity, trust, idempotency and lifecycle semantics. Refactoring around “API endpoints” without preserving those differences would erase important product controls.

## Product/debug/legacy segmentation

- release product: Agent Sessions and orchestration core;
- conditionally connected release code: Native Profile selection, contextual File Review production, some Harness inspection;
- debug internal product: Worktree/Human Review and proof controller;
- operator tools: app inspector, scripts and example clients;
- quarantined compatibility: legacy task/run stack;
- sibling product slices: Product Decisions and final settlement.

These are stronger architectural categories than directory location or compile inclusion alone.

## Likely architectural work areas

- shared MCP host and typed role/stage/tool registry;
- explicit Harness effective-configuration service;
- separation of transition state machines, repositories, effects and projections;
- common application-invocation protocol for prepare/bind/launch/observe/reconcile;
- defined persistence ownership and migration governance;
- explicit internal-tooling package boundary;
- environment and external-process policy framework;
- branch convergence architecture before treating one tip as complete truth.

These are hypotheses for later design, not current refactor instructions.

## Useful architecture visualizations

- context map showing Agent Platform, Orchestration, Harness, Git Authority, Native Profiles and Review Tooling;
- transport map distinguishing Tauri, events, MCP and external processes;
- authority graph linking user confirmation, durable facts, agent claims, application evidence and Git settlement;
- configuration provenance graph per invocation;
- consistency-boundary map for every SQLite-plus-external-effect operation;
- branch topology with capability overlays.

## Questions to carry forward

- Which orchestration subdomains should own their own repository interfaces or schema lifecycle?
- Is the native query one intentional product read API or accumulated projection debt?
- Can MCP hosting be centralized without creating an overprivileged global server?
- Should the new Native Profile identity hook expand into the Agent Runtime policy port, or remain a narrower preflight authority?
- What is the intended lifecycle and retention model for generated materials, workspaces, refs and evidence?
- What is the integration target for Product Decisions and final settlement now that the active Native Profile corrections are centralized on the research line but not local `main`?
