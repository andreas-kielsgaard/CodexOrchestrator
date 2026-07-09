# Agent Session Recovery Plan

Status: approved direction, not yet implemented

Created: 2026-07-10

Working branch: `codex/agent-session-reset`

## Purpose

This directory is the durable planning record for rebuilding the Agent Session vertical slice on
the cleaned workspace baseline.

An Agent Session is a durable interaction context for doing work through a text interface. The
first runtime is Codex CLI. The visible conversation is a projection of that context rather than
the complete technical record.

This plan supersedes Agent Session implementation assumptions found in the archived integrated
overlay. It does not adopt the old task dashboard or orchestration models as prerequisites.

## High-Level Plan

1. Establish a reviewed structural baseline without merging either archive wholesale.
2. Define stable Agent Session, runtime binding, invocation, and event contracts.
3. Build SQLite persistence and restart-safe session queries before launching Codex.
4. Replace `CLISessionMaster` with a real Rust process supervisor that owns child processes.
5. Implement a Codex CLI runtime adapter that resumes the external Codex thread identity.
6. Add a recoverable Tauri command and event boundary over persisted invocation state.
7. Rebuild the conversation UI as a projection: live work is visible, completed work is collapsed,
   and the final response is prominent.
8. Prove continuation, cancellation, restart recovery, and CLI compatibility with boundary tests.

The first completion target is deliberately narrow:

> Create or open a session, send text, watch Codex work, see the final response, restart the app,
> reopen the session, and continue the same Codex thread.

## Documents

- [Architecture and decisions](./architecture-and-decisions.md): product meaning, ownership
  boundaries, data model, and important decisions.
- [Evidence record](./evidence.md): findings from the archived implementation and local Codex CLI
  verification that justify the changes.
- [Implementation plan](./implementation-plan.md): phased work, dependencies, acceptance criteria,
  validation, and stop conditions.
- [Execution ledger](./execution-ledger.md): work-thread ownership, dependency gates, integration
  state, commit references, and validation outcomes.
- [Prototype database procedure](./prototype-database.md): read-only audit, non-destructive reset,
  and retained-data upgrade rules for archived migration records.

## Scope Boundaries

Included now:

- durable Agent Session identity and history
- one active invocation per session
- Codex CLI start and resume
- streamed runtime events with durable recovery
- actual process cancellation and application shutdown handling
- final-first conversation presentation with expandable execution details
- optional working directory and runtime configuration actually used

Explicitly deferred:

- task, goal, repo, and orchestration relationships
- session branching and inherited-context visibility policies
- API prompt-cache management
- generalized multi-provider UI
- process scheduling, quotas, and prioritization beyond safe concurrent supervision
- attachments and file upload
- context-library and pruning systems
- orchestration-specific conversation views

The deferred features may later relate to Agent Sessions. They must not be required for the first
slice to function.

## Source References

The previous work is preserved for inspection:

- `codex/archive-main-overlay-20260709` — integrated Agent Session and orchestration overlay
- `codex/archive-frontend-refactor-75d0-20260709` — frontend and modular-backend cleanup archive
- `codex/archive-tauri-refactor-e95b-20260709` — isolated Rust/Tauri modularization archive

These branches are evidence and selective source material. They are not merge targets by default.
