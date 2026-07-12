# Agent Session Evidence Record

Audit date: 2026-07-09 to 2026-07-10

Primary source: `codex/archive-main-overlay-20260709`

Reset baseline: `codex/agent-session-reset`

## Purpose

This document records the evidence behind the Agent Session recovery plan. It is intended to keep
future implementation work from rediscovering or accidentally reintroducing the same failures.

The archived implementation was inspected without merging it into the reset branch.

## Evidence Summary

| ID   | Finding                                                     | Consequence                                                                    | Planned response                                                          |
| ---- | ----------------------------------------------------------- | ------------------------------------------------------------------------------ | ------------------------------------------------------------------------- |
| E-01 | Local Agent Session ID and Codex thread ID are conflated    | Resume targets the wrong identity                                              | Stable local ID plus separate runtime binding                             |
| E-02 | The configured “durable” frontend store is in memory        | Session history disappears on app restart                                      | SQLite-backed session/invocation/event repository                         |
| E-03 | Backend reload selects only the latest CLI log              | Earlier turns cannot be reconstructed                                          | Query full ordered invocation/event history                               |
| E-04 | UI emits an unsupported CLI flag                            | Current live send fails before execution                                       | Capability-aware Codex option mapping                                     |
| E-05 | Closing does not terminate the child process                | Closed sessions can continue and reappear                                      | Real process supervisor plus explicit cancel                              |
| E-06 | Stream events are the only completion path                  | Lost events can hang or truncate the UI                                        | Persist-first events and query reconciliation                             |
| E-07 | Session state is derived from CLI snapshot state            | Session and invocation lifecycles are confused                                 | Separate session and invocation records/statuses                          |
| E-08 | CLI master/distributor own wrappers, not processes          | Names imply lifecycle guarantees that do not exist                             | Replace master with backend process supervisor; defer distribution policy |
| E-09 | Codex parsing and presentation are interleaved              | Provider protocol, domain reduction, and UI policy cannot evolve independently | Split adapter normalization from transcript projection                    |
| E-10 | Agent Sessions are gated by the task-oriented app shell     | Task startup failure can make the core session surface inaccessible            | Independent feature/client composition                                    |
| E-11 | Tests use fake runtime identities and fake CLI parsing      | Green tests validate wiring while missing live failures                        | Add contract, persistence, process, and restart tests                     |
| E-12 | Archived migration IDs may already exist in local databases | Reusing ordinal positions can collide with prototype databases                 | Audit/reset explicitly and use immutable non-colliding versions           |

## Recovery Verification

The reset implementation resolves the archived findings through independently testable
boundaries:

| Evidence   | Implemented proof                                                                                                                                                              |
| ---------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------ |
| E-01       | Domain serialization and application lifecycle tests keep the stable local ID separate from the captured external context ID and resume the latter.                            |
| E-02, E-03 | The SQLite repository close/reopen test retains two invocations, ordered normalized/raw events, diagnostics, and complete session history.                                     |
| E-04       | Codex argument tests are capability-aware, and the ignored installed-CLI help probe passes explicitly against the locally installed CLI without running an agent.              |
| E-05, E-08 | Supervisor tests cover concurrent sessions, duplicate-session rejection, cancellation, cancellation failure, shutdown, reader/wait failure, reap, and registry cleanup.        |
| E-06       | Application tests prove persist-before-notify ordering, missed-notification reload, bounded delivery diagnostics, idempotent completion, and startup interruption.             |
| E-07       | Session availability and invocation lifecycle are separate domain and repository invariants.                                                                                   |
| E-09       | The Codex protocol adapter normalizes and retains provider data; a pure frontend projector independently applies live and final-first presentation policy.                     |
| E-10       | The app defaults to Agent Sessions and does not initialize the legacy task dashboard until that secondary surface is selected.                                                 |
| E-11       | Contract, repository, supervisor, fixture, application, transport, controller, projection, component, and shell tests now cover the actual boundaries.                         |
| E-12       | Migration `009` has an immutable position after reserved prototype IDs; tests cover clean, archived-ledger, recognized-prototype quarantine, and unrecognized-table rejection. |

### AS-07 gate results

Verified on 2026-07-10:

- frontend build, lint, formatting, and all 325 tests pass
- Rust formatting and check pass
- Rust tests pass: 74 library tests, one capability integration test, and one normally ignored
  installed-Codex compatibility probe executed separately and passed
- the desktop app starts directly on the Agent Session surface with the legacy task surface idle
- the first desktop launch exposed a missing Tauri event-listener permission; the main-window
  capability was added, covered by an integration test, rebuilt, and verified by restarting the app
  without the permission error

One successful-session claim remains deliberately open. On 2026-07-12 the desktop app launched a
real Codex process, displayed live Working state and streamed technical activity, durably recorded
Codex's usage-limit failure, and reopened the same failed turn after an app restart. Because Codex
rejected the turn before establishing an external thread, successful final-response collapse,
second-prompt resume, concurrent live sessions, and live cancellation remain unobserved. The
automated boundaries for those paths pass, but this document does not promote them to live-observed
evidence.

The live failure also showed that raw stderr bytes dominated the technical disclosure. Commit
`0f3d46d` preserves those bytes for inspection, normalizes decoded text for new events, decodes the
already persisted legacy shape in the transcript projector, and nests raw payloads one level
deeper. After that correction, all 326 frontend tests and 75 Rust library tests pass, along with the
capability integration test, build, lint, formatting, and Rust check.

## Detailed Findings

### E-01: Wrong continuation identity

Relevant archived files:

- `src-tauri/src/lib.rs`
- `src/application/agentSession.ts`
- `src/application/agentCLISessionInterface.ts`
- `src/application/agentSessionRouter.test.ts`

Rust creates `session_id` before starting Codex. `persist_agent_session_run` later extracts the
actual Codex `thread_id` and stores it as `codex_session_id`, but `StartAgentSessionCommandResult`
returns only `session_id`.

The frontend renames its pending Agent Session to that returned local ID. Later prompts pass the
same ID to `codex exec resume`.

The router test makes the mismatch explicit: fake output contains `thread-42`, while the expected
command resumes `agent-session-42`.

Conclusion: this is a contract failure, not an isolated argument-building bug.

### E-02 and E-03: False durability and incomplete reload

Relevant archived files:

- `src/application/agentSessionStore.ts`
- `src/application/agentSessionRouter.ts`
- `src-tauri/src/lib.rs`, `AGENT_SESSION_SCHEMA`
- `src-tauri/src/lib.rs`, `persist_agent_session_run`
- `src-tauri/src/lib.rs`, `load_agent_session_record`

`createAgentSessionRouter` constructs `InMemoryAgentSessionDurableStore`. The production React
session history therefore exists only for the life of the frontend process.

Rust separately writes one mutable `agent_sessions` row and whole-run stdout/stderr blobs to
`agent_session_cli_logs`. The loader selects the newest log with `ORDER BY created_at DESC LIMIT 1`.
`AgentSession.reloadStoredSession` then synthesizes one turn from that result.

Additional storage issues:

- a session is not persisted before the process starts
- the session row stores the latest invocation's command/arguments rather than durable session
  metadata
- `cwd` is always inserted as `NULL`
- process signal is not persisted
- the session upsert and log insert are not one explicit transaction
- persistence failure rewrites a successful runtime result to `failed`, conflating execution and
  storage outcomes
- there is no list/query surface for reopening sessions

Conclusion: the raw logs are useful evidence, but the schema does not model an interaction
context.

### E-04: Current CLI incompatibility

Local verification on 2026-07-10:

```text
codex-cli 0.144.0
```

`codex exec --help` and `codex exec resume --help` confirm support for JSON output, model, sandbox,
and resume. The archived UI always emits:

```text
--reasoning-effort <value>
```

The installed CLI responds:

```text
error: unexpected argument '--reasoning-effort' found
```

The archived `AgentSessionPage` constructs raw argument arrays directly. Its fallback runtime data
also forces a model and reasoning choice when runtime discovery fails.

Conclusion: UI controls need semantic options and the Codex adapter needs version/capability-aware
mapping. The minimal restored slice should allow Codex configuration defaults instead of forcing
unverified values.

### E-05: No real close or cancellation

Relevant archived files:

- `src/application/cliInstanceHandler.ts`
- `src/application/agentCLISessionInterface.ts`
- `src/application/agentSession.ts`
- `src/application/agentSessionRouter.ts`

`CLIInstanceHandler.close` can call an optional runner close method. The runtime Agent Session
runner does not provide one. It therefore cannot terminate Codex.

Closing while a launch is active marks local state closed and removes a router entry while the
process and original launch promise continue. Completion can subsequently update the record and
register the session again.

Conclusion: close-view/archive and cancel-invocation are different product operations. A process
supervisor must hold the child handle for cancellation to be real.

### E-06: Non-recoverable stream transport

Relevant archived files:

- `src/infrastructure/tauriCommands.ts`
- `src-tauri/src/lib.rs`, `start_agent_session_streaming`
- `src-tauri/src/lib.rs`, `spawn_agent_session_stream_reader`

The Tauri client correctly subscribes before invoking startup, and events are correlated with a
stream ID. This technique should be retained.

However:

- completion exists only as a Tauri event
- there is no timeout or invocation-status query
- emission failures are ignored
- line-reader errors are silently truncated
- `outputWasStreamed: true` prevents the full completion stdout from repairing a missed output event
- the invocation is not durably recorded before process launch

Conclusion: the stream is suitable for responsiveness, not authority. Persisted invocation state
must support reconciliation.

### E-07: Session status is actually invocation status

Relevant archived files:

- `src/application/agentSessionStore.ts`
- `src/application/agentSession.ts`

`AgentSessionRecord.status` uses `CLIInstanceSnapshot.status`. After every response the session is
marked completed or failed, then returns to running on the next input.

Conclusion: terminal process state belongs to an invocation. A session remains available after a
completed response.

### E-08: Process abstractions do not own processes

Relevant archived files:

- `src/application/cliInstanceHandler.ts`
- `src/application/cliSessionDistributor.ts`
- `src/application/cliSessionMaster.ts`

`CLIInstanceHandler.open` delegates a new command run. It has no PID or child handle.
`CLISessionDistributor` leases handler objects. `CLISessionMaster` adds a second lease map. None of
them instantiates, cancels, or shuts down the operating-system process used by the production Rust
runtime.

Conclusion: the conceptual desire for process ownership is valid, but it must exist where the
process exists. `CLISessionMaster` should be replaced by a Rust process supervisor. General process
distribution remains future policy.

### E-09: Protocol reduction and presentation are coupled

Relevant archived files:

- `src/application/agentSessionOutputFormatter.ts`
- `src/infrastructure/codex/jsonlEvents.ts`

The formatter imports an infrastructure Codex parser into the application layer and then performs
all of the following:

- protocol parsing
- event reduction
- metadata extraction
- active-turn tracking
- context/usage labeling
- final-response selection
- expansion-state handling
- UI DTO construction

The intended completed presentation is valid: live processing is shown, then grouped behind an
expandable completed disclosure while the final response remains visible. That behavior should be
preserved and tested as presentation policy.

Conclusion: keep the behavior, split its responsibilities. The Codex adapter normalizes provider
events; the transcript projector applies final-first display policy.

### E-10: Task-shell dependency

Relevant archived file:

- `src/app/AppRoot.tsx`

The root constructs and loads the Open Tasks controller regardless of active feature. Its initial
view is Tasks, and the loading branch can prevent the navigation shell from exposing Agent Sessions.

Conclusion: Agent Sessions require an independent startup/query path. Task and orchestration data
may later link to sessions but must not gate them.

### E-11: Test coverage proves the wrong boundaries

Archived Agent Session tests cover:

- fake argument construction
- in-memory store append/update behavior
- fake Tauri event correlation
- one formatted completed response
- router wiring

Rust has two focused Agent Session tests, both for argument construction.

Missing boundary tests include:

- first-turn Codex thread capture followed by correct resume
- multi-invocation persistence and process restart
- list/load session queries
- real process cancellation
- application shutdown reconciliation
- missed stream event recovery
- duplicate completion idempotency
- current CLI option compatibility
- working-directory propagation
- persistence failure independent from runtime outcome
- final-first transcript projection during and after execution

The archived overlay previously passed frontend build, lint, tests, Cargo check, and Cargo tests;
format checking still reported failures. Those results demonstrate internal consistency, not a
working Agent Session lifecycle.

### E-12: Prototype migration continuity

Relevant archived and reset files:

- reset `src-tauri/src/lib.rs` migration setup
- archived `src-tauri/src/lib.rs`, migrations `006` through `008`
- archived modular `src-tauri/src/schema/migrations.rs`

The archived overlay introduced orchestration migrations `006` and `007` and Agent Session
migration `008`. A local app database used during prototype development may already contain those
IDs and ordinal positions, even though the reset source only declares migrations through `005`.

The migration table enforces unique positions derived from array order. Adding a new sixth
migration at ordinal position five can collide with a previously applied prototype migration.

Conclusion: the first structural work must inspect/reset the development database deliberately and
make migration versions explicit and immutable. New Agent Session schema work must not silently
reuse an archived ID or position.

## Additional Scope Findings

- The file-upload control records a file count but does not send file contents or paths.
- `AgentSessionPage` does not provide an explicit working-directory selection and does not pass a
  `cwd` for its normal send path.
- The “context size” label is derived from token-usage fields and does not represent a durable
  interaction-context size.
- The concrete `CodexRuntimeInfoProvider` class is not the production composition path; the Tauri
  runtime-info command implements a parallel probing path.
- `agentSessionHandler.ts` and `application/commands/runtimeCommandClient.ts` are compatibility
  re-export shims rather than durable concepts.
- The archived integrated overlay changes 194 files and combines Agent Session work with task,
  orchestration, Storybook, UI, and backend changes. It is unsuitable as a single merge unit.

## Evidence-Preserving Assets

Useful assets to recover selectively:

- typed Codex JSONL parsing and unknown-event preservation
- listener-before-start Tauri stream handshake
- raw output capture
- Markdown final-response rendering
- live processing disclosure and final-first completed presentation
- SQLite migration transaction pattern
- the modular Rust archive as responsibility-layout reference

## Verification Harness Audit (2026-07-12)

The accumulated harness was audited against the recovery-gate boundary. The privileged live driver
is compiled only for tests; its normal production module wiring exposes no new Tauri command. It
creates only a `tempfile`-owned SQLite database and workspace, and rejects any candidate path
outside that root. Ordinary `cargo test --lib` leaves the live driver ignored. With the opt-in
environment variable absent, its selected ignored test refused before capability discovery, so it
could not launch Codex.

The driver uses `AgentSessionApplication` for all lifecycle actions. Its lower-level runtime
observer is verification-specific and records the actual `ProcessLaunchSpec`, proving that the
resume target equals the persisted external context and differs from local session/invocation IDs.
It has an explicit four-invocation budget, bounded per-polling-wait deadlines, authoritative
graceful-then-forced direct-child shutdown, failure cleanup,
hashed evidence IDs, quota/rate-limit classification without retry, and an inconclusive outcome
when durable concurrent running/cancellation cannot be established. Cleanup evidence covers only
supervised direct children; it explicitly does not claim Windows descendant-tree cleanup or
provider determinism.

The real frontend Tauri client still registers the listener before `send_agent_session_message`
and keeps acknowledgement correlation IDs. Its tests retain durable reload repair after a missed
notification. Rust transport coverage still proves persisted correlated notification DTOs. The
separate browser harness uses recorded application DTOs with opaque raw payloads, no Tauri IPC, no
normal app data, no Codex JSONL/Rust evidence fixtures, and no Playwright dependency. Vite emits a
separate harness entry from the production app.

Automated 2026-07-12 result: formatting, lint, 340 frontend tests, production build, Rust format,
check, and 79 Rust tests passed (two intentional ignored tests). The built output contained both
application and harness HTML entries. Manual responsive inspection previously passed at 1280x800,
860x800, and 390x844. The real live lifecycle was not run by this audit because no explicit opt-in
was present; successful provider completion, external-context capture/resume, live concurrency,
and cancellation therefore remain open.

## Reset Baseline Cleanup Continuation (2026-07-12)

Inspection confirmed three contradictions outside the coherent Agent Session island: a mounted
legacy task surface could launch Codex without `ProcessSupervisor`; synchronous capability probing
could stall startup before history became available; and app connections had no explicit SQLite
contention policy. The smallest cleanup was quarantine rather than task redesign or a large module
extraction.

The app now mounts only Agent Sessions. Legacy command names fail closed before database, Git,
Codex, or validation work, while migrations and isolated task-screen tests remain. Production
startup composes the runtime with unknown capabilities and never invokes the test-only capability
probe; deterministic argument coverage proves defaults still emit `exec --json` and resume uses the
persisted external context. SQLite connections use foreign keys, a five-second busy timeout, WAL
for file-backed databases, and full synchronous commits; dedicated tests verify the policy.

The full non-live matrix passed: formatting, lint, 339 frontend tests, production build, Rust
format/check, 84 Rust tests, and two intentional ignored Rust tests. `git diff --check` passed. No
live provider call ran. Successful provider completion, persisted external-context capture,
reopen/resume against that real context, concurrent live sessions, and live cancellation remain
explicitly unproven.
