# Agent Session Implementation Plan

Status: proposed execution plan

Created: 2026-07-10

Target branch family: `codex/agent-session-*`

## Objective

Rebuild the Agent Session vertical slice as a durable, independently usable Codex interaction
context with correct continuation, supervised processes, recoverable streaming, and a final-first
conversation view.

The target is complete when a user can:

1. create or open an Agent Session
2. submit text in an explicit working context
3. watch Codex processing and tool activity while it runs
4. cancel the active invocation
5. see the comprehensive final response with processing collapsed but expandable
6. close and restart the app
7. reopen the complete conversation
8. submit another message that resumes the same Codex thread

## Governing Documents

- `docs/agent-session/architecture-and-decisions.md`
- `docs/agent-session/evidence.md`

When implementation convenience conflicts with an accepted decision, update the reasoning record
explicitly rather than silently weakening the boundary.

## Delivery Principles

- Work from the reset branch; do not merge the integrated overlay wholesale.
- Recover archived code only when its responsibility and tests still fit the accepted model.
- Separate structural cleanup from behavior changes where practical.
- Every phase must leave the repository buildable and its owned tests passing.
- Persist state before treating a stream notification as authoritative.
- Test through contracts and fake process boundaries before depending on live Codex runs.
- Use the installed Codex CLI as a compatibility check, not as a unit-test dependency.
- Do not introduce task or orchestration relationships during this plan.

## Work Package Overview

| ID    | Work package                      | Primary outcome                                                      | Depends on   |
| ----- | --------------------------------- | -------------------------------------------------------------------- | ------------ |
| AS-00 | Structural and migration baseline | Reviewed module boundaries and collision-free migration path         | Reset branch |
| AS-01 | Agent Session contracts           | Stable session, invocation, event, repository, and runtime contracts | AS-00        |
| AS-02 | Durable repository and queries    | Multi-invocation SQLite persistence, list, and load                  | AS-01        |
| AS-03 | Process supervisor                | Real concurrent child-process ownership, cancel, and shutdown        | AS-01        |
| AS-04 | Codex CLI runtime adapter         | Correct start/resume identity and capability-aware arguments         | AS-01, AS-03 |
| AS-05 | Application and Tauri lifecycle   | Persist-first send/stream/complete/reconcile commands                | AS-02, AS-04 |
| AS-06 | Transcript projection and UI      | Independent reopenable conversation surface                          | AS-05        |
| AS-07 | End-to-end recovery gate          | Restart, resume, cancellation, compatibility, and cleanup proof      | AS-06        |

## Dependency Shape

```text
AS-00
  -> AS-01
       -> AS-02 -------------------+
       -> AS-03 -> AS-04 ----------+-> AS-05 -> AS-06 -> AS-07
```

Repository and process-supervisor work may proceed independently after contracts are accepted.
They should meet only through the application lifecycle in AS-05.

## AS-00: Structural and Migration Baseline

### Purpose

Create trustworthy places for the slice before adding behavior.

### Work

- Review the modular Rust archive and selectively replay responsibility-preserving moves onto the
  reset branch. Do not copy the integrated Agent Session code into the old Rust monolith.
- Establish backend homes for Agent Sessions, Codex runtime behavior, and process supervision.
- Establish a narrow frontend Agent Session feature boundary and application client boundary.
- Avoid importing the broad archived capability/compatibility-shim structure unless an actual
  consumer requires it.
- Audit the local app database and `schema_migrations` records for archived migrations `006` to
  `008`.
- Decide and execute the explicit prototype-data policy:
  - preferred during this reset: back up if desired, then reset the unshipped development database
  - if data must be retained: add an explicit forward migration that detects and transforms the
    prototype tables
- Change migration registration to use immutable explicit versions/positions rather than deriving
  durable identity only from current array order.
- Reserve archived migration IDs; do not reuse them for the new schema.

### Deliverables

- modular Rust compilation baseline
- small Agent Session module placeholders with dependency direction documented in code
- migration-ledger test covering gaps and previously applied prototype IDs
- documented local development database reset/upgrade procedure

### Exit criteria

- no behavior change to existing task paths
- frontend build/lint/tests pass
- Rust format/check/tests pass
- a database containing archived migration records can initialize without position collision
- new installs and reset development installs follow one explicit schema path

## AS-01: Agent Session Contracts

### Purpose

Make the product and runtime boundaries executable before choosing implementations.

### Work

- Define pure records for:
  - `AgentSession`
  - runtime binding
  - `AgentInvocation`
  - `AgentRuntimeEvent`
- Define lifecycle invariants:
  - local session ID never changes
  - external runtime identity is nullable and separately updated
  - one active invocation per session
  - invocation terminal state does not close the session
  - event sequence is monotonic per invocation
- Define repository commands and queries without SQLite types.
- Define a provider-neutral runtime port from the proven current needs:
  - start or resume an invocation
  - stream normalized/raw events
  - cancel by invocation ID
- Keep Codex-only argument and JSONL types out of these contracts.
- Define serializable frontend DTOs separately from internal records where necessary.

### Deliverables

- domain/application contract modules
- invariant tests
- fake repository and fake runtime adapter for application tests
- a short dependency contract in the relevant module README or module documentation

### Exit criteria

- contracts can represent first prompt, later resume, failure, cancellation, and interruption
- contracts preserve raw provider data without exposing Codex types to the domain
- no React, Tauri, SQLite, or process imports cross into the domain/application records
- no task, repo, goal, or orchestration ID is required

## AS-02: Durable Repository and Queries

### Purpose

Make app restart and complete-history reload work before runtime execution is introduced.

### Work

- Add collision-free SQLite migrations for sessions, invocations, and ordered runtime events.
- Keep session metadata separate from latest-invocation command details.
- Implement transactional operations for:
  - create/load/list session
  - create pending invocation with submitted user input
  - append ordered runtime events
  - set external runtime identity without changing local ID
  - complete/fail/cancel/interrupt invocation idempotently
- Implement full session loading ordered by invocation and event sequence.
- Add compact session summaries for a future/rebuilt session list.
- Preserve raw payloads and parser/normalizer version where practical.
- Decide how large stdout/stderr payloads are stored without introducing premature pruning.

### Deliverables

- SQLite schema and repository
- repository contract tests against in-memory SQLite
- list/load query DTOs
- multi-invocation rehydration tests using a newly opened database connection

### Exit criteria

- two or more invocations survive database close/reopen with original ordering
- local and external identities survive independently
- partial writes roll back or leave an explicitly recoverable pending/interrupted record
- unknown runtime events round-trip without data loss
- loading never synthesizes a conversation from only the latest log

## AS-03: Real Process Supervisor

### Purpose

Replace handler leasing with actual ownership of runtime processes and prepare for concurrent Agent
Sessions.

### Work

- Implement the supervisor in Rust/Tauri-managed state where child processes are created.
- Key active process entries by invocation ID and associate them with session ID.
- Own child handles plus output-reader tasks/threads.
- Support multiple active processes for different sessions.
- Reject or serialize a second active invocation for the same session.
- Expose explicit start, cancel, process-exit, and shutdown operations.
- Define cancellation semantics and terminal outcome mapping.
- Ensure process entries are removed exactly once after terminal handling.
- On app shutdown, terminate or deliberately detach according to the accepted policy; the preferred
  first-slice policy is terminate and mark interrupted if completion cannot be recorded.
- Do not add scheduling, queues, priorities, or generalized resource distribution.

### Deliverables

- `ProcessSupervisor` implementation
- fake child-process boundary for deterministic tests
- concurrency, cancel, spawn-failure, reader-failure, and shutdown tests

### Exit criteria

- two different sessions can own active fake processes concurrently
- the same session cannot accidentally own two active invocations
- cancel reaches the actual child boundary and produces one terminal outcome
- shutdown leaves no supervisor-owned child unaccounted for
- no TypeScript class claims to own a process it cannot control

## AS-04: Codex CLI Runtime Adapter

### Purpose

Implement the current provider truthfully behind the runtime port.

### Work

- Build new-session arguments with `codex exec --json`.
- Build continuation arguments with `codex exec resume` using the external Codex thread ID.
- Capture `thread.started` and return/update it as runtime binding data.
- Reuse and extend the typed JSONL parser, preserving unknown events and raw data.
- Normalize Codex events into runtime-event kinds needed by the application and transcript
  projector.
- Classify terminal results using both JSONL terminal events and process exit.
- Pass explicit working directory to the child process.
- Accept semantic settings and map only supported options.
- Allow Codex configuration defaults when runtime capability data is unavailable.
- Keep runtime-info discovery independent so one failed probe does not invalidate all settings.
- Add an installed-CLI compatibility check for argument construction without running a paid/live
  agent invocation.

### Deliverables

- Codex CLI Agent runtime adapter
- argument-builder and event-normalizer tests using recorded fixtures
- fixture containing first-turn `thread.started` and a subsequent resume
- capability/option mapping tests for the supported local CLI surface

### Exit criteria

- first invocation captures a Codex thread ID without changing the Agent Session ID
- second invocation uses that Codex thread ID for resume
- unsupported settings are omitted or rejected before spawn with a useful error
- live processing, intermediate agent messages, tool activity, final response, and terminal events
  remain distinguishable
- malformed or unknown events are preserved and diagnosed without discarding unrelated valid events

## AS-05: Application and Tauri Lifecycle

### Purpose

Join persistence, the runtime adapter, and process supervision into a recoverable vertical backend.

### Work

- Implement send-message orchestration:
  1. load/create the session
  2. transactionally persist the input and pending invocation
  3. acknowledge session and invocation IDs
  4. start the runtime through the supervisor
  5. persist every ordered event before emitting its update
  6. update runtime binding from provider identity events
  7. persist terminal invocation outcome idempotently
- Add list-session and load-session queries.
- Add cancel-invocation command.
- Correlate all updates by invocation ID, not ephemeral frontend-only stream identity.
- Preserve listener-before-start subscription behavior on the frontend client.
- Add query reconciliation after reconnect, listener failure, or missing completion notification.
- On startup, mark database invocations left in pending/running state as interrupted unless a
  supervised process can be proven active.
- Keep runtime outcome and persistence/notification diagnostics separate.

### Deliverables

- narrow Tauri Agent Session command/query client
- Rust command/query handlers
- application lifecycle tests with fake repository/runtime/supervisor
- Tauri transport tests for acknowledgement, updates, missed events, and reload repair

### Exit criteria

- a missed stream update is recovered by loading the session
- a missed completion event does not leave the UI permanently waiting
- repeated completion handling does not duplicate events or corrupt status
- persistence failure is reported without rewriting a known Codex success as a runtime failure
- cancel and startup interruption produce durable terminal records

## AS-06: Transcript Projection and Agent Session UI

### Purpose

Restore the core interaction window over trustworthy backend state.

### Work

- Implement a pure transcript projector over loaded invocations and runtime events.
- Preserve the intentional presentation policy:
  - while running, show processing/tool/intermediate activity as it arrives
  - when complete, collapse processing into an expandable disclosure
  - keep the final comprehensive agent response visible
- Keep all collapsed details available after reload.
- Build a small feature controller for:
  - list/open session
  - draft input
  - send and cancel
  - stream subscription
  - reload reconciliation
  - expansion state
- Split views into a session list/selector, transcript, processing disclosure, and composer only as
  needed for clarity.
- Make Agent Sessions reachable without successful Task dashboard initialization.
- Support an explicit optional working directory.
- Initially omit inert upload controls and unverified context-size claims.
- Initially use Codex defaults or only capability-confirmed runtime settings.
- Keep raw diagnostics available through a secondary technical disclosure.

### Deliverables

- Agent Session feature controller
- transcript projector with fixture tests
- independently mountable Agent Session screen
- UI tests for live, completed, failed, canceled, interrupted, empty, and reloaded sessions

### Exit criteria

- the final response is the default completed view
- processing details can be expanded before and after app restart
- running activity remains visible without being mistaken for the final response
- previous user inputs and responses remain ordered across several invocations
- Task or orchestration client failure does not block the Agent Session surface
- close-view does not masquerade as process cancellation

## AS-07: End-to-End Recovery Gate

### Verification method and current status

The primary recovery proof is now deterministic: Rust lifecycle/repository/supervisor/runtime and
transport tests plus the separate recorded browser harness. The harness drives application DTO
scenarios and validates live-to-completed presentation, durable restart-style remount, correlation,
diagnostics, outcomes, and long content without Tauri IPC, normal app data, Codex JSONL, or
focus-sensitive automation. It complements rather than replaces tests at the real Tauri client and
transport boundary.

As of the cleanup continuation on 2026-07-12, the non-live matrix passes (339 frontend tests; 84
Rust tests; two intentional
ignored tests), the production build contains independent app and harness entries, and manual
responsive inspection passed at 1280x800, 860x800, and 390x844. The ignored live driver is an
optional provider gate, not ordinary test coverage: it requires explicit opt-in, uses an owned
temporary database/workspace, is bounded to four invocations with 1-300 second per-polling-wait
deadlines, and emits redacted evidence. Successful live-provider proof remains pending available,
separately authorized
Codex usage. The deliberate manual remainder is a live desktop/provider exercise; it is no longer
the primary proof for the deterministic recovery behavior.

### Purpose

Prove the vertical slice works under the failures that invalidated the prototype.

### Automated verification

- frontend formatting, lint, tests, and build
- Rust formatting, check, and tests
- database migration tests from clean and prototype-ledger states
- persistence close/reopen integration test
- process supervisor concurrency and cancellation tests
- Codex fixture start/resume identity test
- Tauri missed-event reconciliation test
- transcript projection live-to-completed transition test

### Manual verification

Using the installed Codex CLI and a disposable test session:

1. Create a session with an explicit working directory.
2. Send a prompt and observe live processing.
3. Confirm processing collapses and the final response remains visible.
4. Expand the processing record and inspect retained activity.
5. Restart the Tauri app.
6. Reopen the same session and confirm identical transcript/history.
7. Send a second prompt.
8. Confirm the process command resumes the captured Codex thread ID.
9. Start another session concurrently and confirm independent process ownership.
10. Cancel one active invocation and confirm durable canceled state and no remaining child process.

### Cleanup and handoff

- Remove or quarantine superseded Agent Session prototype code if any was temporarily recovered.
- Update top-level architecture documentation to point to the implemented modules.
- Record any plan deviations in the architecture/decision document.
- Preserve recorded JSONL fixtures with their observed CLI version.
- Do not begin task/orchestration relationships as part of completion cleanup.

### Final exit criteria

- every objective at the top of this document is demonstrated
- no critical behavior relies solely on transient frontend memory or Tauri event delivery
- the reset branch contains no accidental merge of the integrated overlay
- known deferred items remain documented rather than represented by inert controls or mock truth

## Validation Matrix

| Concern                        | Unit                   | Integration              | Manual                             |
| ------------------------------ | ---------------------- | ------------------------ | ---------------------------------- |
| Stable local/external identity | Contract tests         | First-run/resume fixture | Inspect second live invocation     |
| Multi-turn durability          | Repository tests       | Close/reopen database    | Restart app and reopen             |
| Process ownership              | Supervisor tests       | Cancel/shutdown handler  | Cancel live Codex                  |
| Streaming recovery             | Projector/client tests | Drop event then reload   | WebView/app restart                |
| CLI compatibility              | Argument tests         | Installed `--help` probe | One disposable live session        |
| Final-first presentation       | Projection tests       | UI component tests       | Observe live then completed turn   |
| Feature independence           | Controller tests       | App-shell test           | Open with task backend unavailable |

## Risks and Controls

### Codex CLI evolution

Control: isolate syntax and event parsing in the Codex adapter, retain raw events, record fixture CLI
versions, and avoid hardcoded fallback flags.

### Process cancellation differences across platforms

Control: keep termination behind a process boundary, test Windows behavior explicitly, and record
whether child process trees require platform-specific handling.

### Duplicate truth between frontend and backend

Control: backend persistence owns durable status; frontend state is a projection plus transient
draft/expansion state.

### Prototype database contamination

Control: handle it explicitly in AS-00. Never infer that reset source implies reset app data.

### Premature generic abstraction

Control: implement Codex-specific behavior behind a narrow runtime port and extract broader
interfaces only when a second runtime demonstrates the common shape.

## Stop Conditions

Pause and revise the plan if any of the following becomes true:

- Codex CLI cannot reliably expose or resume its thread identity.
- Tauri process ownership cannot support dependable cancellation on the target Windows runtime.
- the existing database cannot be safely reset or migrated without a user data decision.
- implementing the slice requires task or orchestration ownership rather than an optional future
  relationship.
- a proposed abstraction cannot be described without combining session, invocation, process,
  transport, and presentation responsibilities.
