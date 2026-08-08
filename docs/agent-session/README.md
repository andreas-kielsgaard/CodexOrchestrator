# Agent Session Recovery Plan

Status: recovery baseline implemented; deterministic verification harness and non-live gate
complete; live Codex lifecycle proof pending an independently authorized account with available usage

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

The current Agent Session-to-runtime contract and Sprint 3 capability-discovery extension protocol
are recorded in [agent-access-boundary.md](./agent-access-boundary.md).

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
9. The production app mounts only Agent Sessions. Legacy task handlers fail closed before database
   or process work, while their migration compatibility and isolated component tests remain.
10. Startup does not probe Codex. The retained SQLite connection uses an explicit foreign-key,
    five-second busy-timeout, WAL, and full-synchronous policy.

The implemented completion target remains deliberately narrow:

> Create or open a session, send text, watch Codex work, see the final response, restart the app,
> reopen the session, and continue the same Codex thread.

The remaining provider-dependent gate is one successful disposable live lifecycle. A 2026-07-12
desktop retry observed live working state, technical streaming, durable failed terminal state, and
restart/reopen history, but Codex rejected it at its usage limit before it created a provider
thread. The deterministic harness covers the presentation and durable-reload cases without making
a provider-determinism claim.

## Verification Surfaces

- Rust unit/integration coverage exercises the repository, lifecycle, supervisor, Codex argument
  construction, persisted Tauri notifications, and the test-only live-smoke foundations.
- `agent-session-harness.html` is a separate Vite entry with recorded application DTO scenarios.
  It imports neither Tauri IPC nor normal app data, and `src/main.tsx` remains the production app
  entry point.
- The harness deliberately treats `rawPayload` as opaque fixture data; it does not reproduce Codex
  JSONL or Rust evidence records. It has no Playwright dependency. Browser checks are manual,
  lightweight inspection rather than focus-sensitive desktop automation.

### Deterministic commands

For a short implementation check, use the reduced-debug lane with the narrowest relevant filter:

```powershell
npm run test:rust:fast -- agent_sessions::
```

The filter reduces executed tests but Rust still compiles the library test harness. Use the
default-debug deterministic lane at a Slice or integration boundary:

```powershell
npm run format:check
npm run lint
npm test
npm run build
cargo fmt --manifest-path src-tauri/Cargo.toml --all -- --check
npm run test:rust:full
```

For the browser harness, run `npm run dev` and open
`http://localhost:1420/agent-session-harness.html`. The production build must contain both
`dist/index.html` and `dist/agent-session-harness.html`.

### Explicit live or paid proofs

The ignored driver can launch up to four real Codex invocations. Do not run it unless a human has
explicitly authorized the cost/quota exposure and the account has usable capacity.

First compile the feature and run only its deterministic harness checks:

```powershell
cargo test --manifest-path src-tauri/Cargo.toml --features live-tests --lib agent_sessions::live_smoke::tests::
```

Then, after explicit authorization, run the ignored live driver:

```powershell
$env:CODEX_AGENT_SESSION_LIVE_SMOKE = 'true'
$env:CODEX_AGENT_SESSION_LIVE_SMOKE_TIMEOUT_SECS = '180' # optional; per-wait limit, 1-300 seconds
cargo test --manifest-path src-tauri/Cargo.toml --features live-tests --lib agent_sessions::live_smoke::agent_session_live_smoke_driver -- --ignored --exact --nocapture
```

Live and paid proofs are absent from the default Rust test compilation and remain ignored after
enabling `live-tests`. Without `CODEX_AGENT_SESSION_LIVE_SMOKE=true`, the driver refuses before
capability discovery or any agent launch. Each polling wait defaults to 180 seconds and may be configured up to 300.
Runtime shutdown first allows supervised children a two-second grace period, then requests
termination and retains ownership until every direct child has been reaped. That authoritative
cleanup is intentionally not misrepresented as a time-bounded operation. The driver writes
its temporary database, workspace, and `agent-session-live-smoke-evidence.json` inside its owned
temporary root; that root is deleted when the test exits. With `--nocapture`, it also prints one
redacted durable report prefixed `AGENT_SESSION_LIVE_SMOKE_EVIDENCE=`. Its `passed` or `failed`
outcome reports the four-invocation budget, phase results, hashed IDs, final durable statuses,
direct-child cleanup, cancellation state, and stated limitations. A quota/rate-limit result is not
retried and does not prove provider completion, resume, concurrency, or cancellation.

Current verification status: the cleanup continuation passed the full non-live matrix on
2026-07-12 (339 frontend tests; 84 Rust tests; two intentional Rust ignores). Recorded-harness
manual responsive checks previously passed at 1280x800, 860x800, and 390x844. Live Codex lifecycle
remains pending and was not run during cleanup.

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
