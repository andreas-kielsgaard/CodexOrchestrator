# Sprint 3 convergence evidence

Status: accepted and closed after final Epic review on 2026-07-15.

## Objective

Leave Agent Session independently usable and provider- and orchestration-role neutral, with a clean
agent-access/runtime boundary and Codex CLI as the supported concrete adapter. Provider protocol,
process, and continuation details must not become Agent Session product identity or frontend
application behavior.

## Converged architecture

- `AgentSessionApplication` depends on the provider-neutral `AgentRuntime` port. Session creation
  and message commands contain no runtime selector; active composition chooses Codex CLI.
- The durable runtime binding contains only external continuation context and observed runtime
  version. Start versus resume is derived from that external context, never a local session or
  invocation ID.
- The application persists pending state before preflight, persists preflight-approved effective
  options before launch, and persists runtime events and terminal results before notifying the UI.
  Queries remain authoritative after restart or missed notifications.
- `runtime/codex` owns executable resolution, capability probing and translation, semantic option
  decisions, arguments, JSONL framing and normalization, external-context capture, process
  coordination, and protocol/process terminal reconciliation.
- Recorded and product flows share Agent Session presentation through injected clients and
  controllers. The transcript projector applies final-first presentation without parsing Codex
  protocol values.
- Older frontend Codex runtime/parsing scaffolding remains outside the active Agent Session/product
  import path. It was not expanded or retired in this Sprint.

## Capability discovery and cache policy

Codex version and start/resume help are discovered lazily during capability-dependent preflight,
not during application startup or invocation launch. The adapter translates raw observations into
semantic `supported`, `unsupported`, or `unknown` values with provenance, observation time, and
validity. Observed evidence is cached in memory for 30 minutes; fully unavailable discovery is
cached for one minute. Concurrent first use shares one discovery. Explicit refresh and invalidation
bypass cached evidence.

Unknown or unavailable evidence never becomes `unsupported`. Requested optional model or sandbox
values are omitted when support is unknown and rejected before spawn when confirmed unsupported.
Structured JSONL is required: confirmed lack of support fails preflight, while unknown evidence
allows the launch result to decide. Start and resume have separate capability surfaces. Launch
consumes the effective options already persisted by preflight and does not rediscover capabilities,
closing the preflight/launch race.

The extension protocol is: keep raw discovery inside the integration, translate only product-needed
semantics into a snapshot, record provenance and freshness, apply cached reuse plus explicit refresh
and invalidation, and add deterministic probe/cache/argument tests. This is an extension pattern,
not a speculative alternate-provider framework.

## Process and shutdown policy

The current supervisor owns direct children only. Cancellation requests immediate direct-child
termination. Application shutdown first allows a two-second natural-completion window, then
escalates termination and retains ownership through direct-child reap and terminal callback. A
reported shutdown error is logged and prevents Tauri exit so uncertain cleanup is not silently
accepted.

`ChildProcessFactory` and `SupervisedChild` are the deliberate seam for stronger platform ownership.
This Sprint makes no Windows descendant-tree cleanup claim and does not add Job Objects or general
process scheduling.

## Deterministic validation

Verified on 2026-07-15 without an agent prompt:

- focused Agent Session client, controller, projector, shared-component, recorded-harness, and Rust
  boundary checks: 58 frontend tests and 74 targeted Rust tests passed;
- full frontend suite: 77 files and 527 tests passed;
- recorded `AgentSessionScreen` live-processing harness: 9 tests passed both focused and in the full
  suite, so no isolated failure rerun was required;
- Prettier, ESLint, TypeScript, and Vite production build passed; both `index.html` and the separate
  recorded `agent-session-harness.html` were emitted;
- Rust formatting and check passed; library tests passed with 98 passed and 2 intentionally ignored;
- the normally ignored installed-Codex version/help compatibility probe passed separately and did
  not run an agent prompt;
- static scans found no `runtimeKind`, `runtime_kind`, `codex_cli`, or Codex protocol/process parsing
  in active Agent Session product/frontend application behavior; legacy task commands remain
  fail-closed;
- `git diff --check` passed after this record was formatted.

The only Work Unit correction was removal of three blank lines at the end of
`src/app/App.test.tsx`; test behavior was unchanged.

## Unsupported and unproven

- No paid/live Codex prompt ran. Successful live completion, real external-context capture and
  resume, live concurrency, live cancellation, and provider determinism remain unproven.
- MCP capability discovery, service-provider adapters, orchestration persistence/events, Plan
  Builder, initiation, execution, scheduling, automatic continuation, and multi-agent behavior are
  not implemented by Sprint 3.
- The supervisor does not prove descendant-tree ownership.
- Recorded fixtures prove recorded presentation behavior only.

## Checkout state

The shared `main` checkout was intentionally dirty before this Work Unit with accepted Sprint 1-3
work and unrelated orchestration/frontend changes, including untracked accepted modules and docs.
Convergence preserved that state: no staging, commit, reset, clean, branch switch, merge, push, or
deletion was performed.

## Epic Runner carry-forward

- Later MCP and service integrations should use integration-owned capability discovery, semantic
  snapshots with provenance and freshness, cached reuse, explicit refresh/invalidation, and
  truthful unknown/unavailable results. This record does not claim MCP discovery exists.
- The recorded `AgentSessionScreen` live-processing harness timing regression is harmless for
  Sprint 3 and did not recur in this convergence run, but remains unresolved. Do not mark it obsolete
  before detailed follow-up.
- Stronger descendant/process-tree ownership remains an available later integration path behind
  `ChildProcessFactory` and `SupervisedChild` if product risk requires it.
- Live capability/provider proof is intentionally deferred to a later Sprint.
- Unreachable frontend Codex runtime/parsing scaffolding remains a later retirement concern; do not
  treat it as an active Agent Session boundary or expand it in the meantime.

## Final Epic review

Independent review confirmed the implemented dependency direction and the evidence above. The full
frontend suite passed with 77 files and 527 tests; Rust library tests passed with 98 tests and 2
intentional ignores; ESLint, TypeScript, and the production build passed. `git diff --check` also
passed before this acceptance update. No live prompt was run.

Sprint 3 can close. The unresolved recorded timing regression, live-provider proof, stronger
descendant ownership, and retired frontend scaffolding remain explicit follow-up inputs rather than
hidden completion claims.
