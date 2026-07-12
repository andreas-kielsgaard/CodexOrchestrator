# Agent Session Recovery Plan

Status: recovery baseline implemented; automated gate complete; successful live-session exercise
pending available Codex usage

Created: 2026-07-10

Working branch: `codex/agent-session-reset`

## Purpose

This directory is the durable reasoning and implementation record for the Agent Session vertical
slice rebuilt on the cleaned workspace baseline.

An Agent Session is a durable interaction context for doing work through a text interface. The
first runtime is Codex CLI. The visible conversation is a projection of that context rather than
the complete technical record.

This plan supersedes Agent Session implementation assumptions found in the archived integrated
overlay. It does not adopt the old task dashboard or orchestration models as prerequisites.

## Implemented Baseline

1. The structural baseline was recovered selectively without merging either archive wholesale.
2. Stable Agent Session, runtime binding, invocation, event, repository, and runtime contracts are
   implemented in Rust with a browser-safe TypeScript client contract.
3. SQLite persistence and restart-safe ordered history queries exist before runtime launch.
4. A real Rust supervisor owns direct child processes, output readers, cancellation, and shutdown.
5. The Codex-specific adapter starts and resumes the separate external Codex thread identity.
6. The application lifecycle persists before notification and reconciles missed events by query.
7. The independent UI shows live work, collapses completed processing, and keeps the final response
   prominent with safe Markdown rendering.
8. Boundary tests prove continuation, persistence, cancellation, restart recovery, migration
   compatibility, and installed CLI help compatibility.

The implemented completion target remains deliberately narrow:

> Create or open a session, send text, watch Codex work, see the final response, restart the app,
> reopen the session, and continue the same Codex thread.

The last manual gate is one successful disposable live session through the desktop app. Automated
and desktop startup checks are complete, including the Tauri event-listener permission required for
live updates. A live launch, failure presentation, and restart/reopen were observed on 2026-07-12,
but Codex rejected the invocation at its usage limit before a provider thread could be established.

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
