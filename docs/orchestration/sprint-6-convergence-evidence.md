# Sprint 6 convergence evidence

Status: **partial**. Deterministic convergence and isolated installed-Codex Plan Builder,
Bootstrap Generator, and Epic Runner paths pass. The production user-confirmation click and the
integrated post-click chain remain a manual gate; no confirmation was bypassed.

## Epic Pause/Restart and runtime-observation convergence

- Epic control availability and target selection remain application-owned durable query state.
  They are never inferred from transcript text, provider output, or an invocation observation.
- Every durable control target now exposes only its exact source invocation and its exact
  application-owned control invocation. Both reuse the Agent Session durable observation
  projection: launch acceptance, external-context observation, provider activity/terminal
  evidence, process terminal outcome, and typed or explicitly partial MCP activity.
- A control request, selected target, cancellation request, observed interruption, control-message
  persistence, and control-invocation launch acceptance remain separate facts. A `pause work`
  message is not suspension; a `continue work` message is not useful progress. Provider receipt,
  instruction compliance, product acceptance, and work outcome remain unobserved unless another
  bounded product record proves them.
- Reopen rebuilds the correlated projection from the control row plus each matching invocation's
  durable history. Missing older normalized detail stays absent/partial; raw payload is not
  reparsed as a migration and no provider-private reattachment is attempted.
- This checkpoint has deterministic local fake-runtime and strict native-contract/UI coverage.
  It does not prove a live provider received either control message, suspended/resumed work,
  restored a provider connection, or produced a product outcome.
- **CL-1 controlled-live exchange, 2026-08-02:** one real Codex source invocation ran through
  the product Agent Session application, an isolated active-v3 SQLite database, the production
  Sprint Runner schema owner, and the Epic Pause/Restart service. The successful fresh run used
  `C:\Users\user\.codex\worktrees\89a3\Codex Orchestrator\.dev\worktree-runtime\wct1-live-20260802-2030`
  for its database, workspace, evidence, build target, and copied test-owned Codex home. Its
  durable `cl1-epic-pause-restart-evidence.json` records `passed`.

  | Stage                                                                                                                          | Durable result                                                                                                                                                  |
  | ------------------------------------------------------------------------------------------------------------------------------ | --------------------------------------------------------------------------------------------------------------------------------------------------------------- |
  | Application source request and launch acceptance                                                                               | observed: one application-origin source invocation persisted and launch-accepted                                                                                |
  | Epic correlation and Pause target selection                                                                                    | observed: one initiated Epic membership, one Pause action, and one exact source target persisted                                                                |
  | External context                                                                                                               | observed: one normalized `runtime_context_established` event persisted the Codex thread id; the live driver waited for that observation before requesting Pause |
  | Cancellation request and source lifecycle                                                                                      | observed: `cancel_requested_at`, durable `canceled` source, and process terminal; provider terminal remains absent                                              |
  | Pause message/invocation persistence and launch acceptance                                                                     | observed: one exact `pause work` invocation persisted and launch-accepted                                                                                       |
  | Restart target, message/invocation persistence, and launch acceptance                                                          | observed: one exact `continue work` invocation persisted and launch-accepted                                                                                    |
  | Control invocation terminal outcomes                                                                                           | observed: both control processes failed with exit code `1` and no JSONL terminal evidence; this is not provider receipt or compliance                           |
  | Instruction receipt/compliance, actual suspension, continuation acceptance, resumed work, useful progress, consumer acceptance | unproven; none is inferred from request/acknowledgement state                                                                                                   |
  | Reopen reconstruction                                                                                                          | observed: both correlated control actions reconstructed; both availability values were `unavailable` after settlement                                           |

  The prior same-isolated attempt persisted cancellation but timed out after 180 seconds waiting
  for the source terminal. Its supervisor waited for output readers before delivering any terminal
  outcome. The bounded correction delivers cancellation settlement after the directly owned child
  has exited. The `BlockingReader` regression demonstrates that a reader can remain open after
  direct-child exit; inherited handles in an unowned Windows descendant are a supported mechanism
  or hypothesis, not a causal claim about the installed Codex invocation. The live runs prove the
  former missing-terminal symptom and post-change source settlement with Pause progression, not
  that mechanism. After cancellation/direct-child exit, the product process reserves up to two
  reader slots per invocation and retains at most eight reader handles in a process-wide quarantine.
  It reaps finished readers before a later cancellation and at shutdown, refuses a later
  cancellation while that bounded capacity is exhausted, and reports an incomplete retained-reader
  cleanup at shutdown. Dropping an individual supervisor neither waits indefinitely nor discards
  tracked readers: it starts nonblocking background cleanup while the process-exit quarantine
  retains unresolved handles. If that cleanup worker cannot start, one process-exit supervisor
  record is retained; a full or poisoned fallback aborts rather than detaching more state. Drop
  does not claim a durable shutdown report.
  It neither kills nor reattaches to descendants. Normal exits still drain readers before terminal
  delivery. This retains no process-tree claim, provider-terminal claim, or provider-compliance
  claim. The earlier failed-Pause Restart correction remains: Restart excludes every prior control
  message from failed/interrupted source candidates.

- Checkpoint validation: WCT-1 process supervision Rust **15 passed**; focused Epic control Rust
  **8 passed, 1 live ignored**, plus the isolated controlled-live exchange **1 passed**. The
  retained-reader correction adds focused process coverage for bounded retention, reaping,
  cancellation refusal, and shutdown behavior; its first count was invalidated by independent
  review and is superseded by the correction commit's deterministic **17 passed** result. Agent
  Session application cancellation/diagnostic coverage is **16 passed**. The ordinary parallel
  `cargo test runtime::processes::tests --lib` command passed **17/17** three times; only the
  three global-quarantine regressions coordinate with one another. The later
  strict nested-observation decoder correction reran the frontend native-
  contract, Tauri transport, production-composition, and control UI at **4 files / 30 tests
  passed**. The later terminal-status and signed-exit-code correction reran the same set at **4
  files / 36 tests passed**; TypeScript plus production Vite build passed. No Rust changed in that
  later correction, so its prior Rust evidence remains separate. Touched Rust `rustfmt --check`,
  touched-file Prettier, and `git diff --check` passed. The existing Rust dead-code warning in
  `ProcessSupervisor::system` remains a non-failure.

## Independent corrections

- User-authored managed Plan Builder queries retain `user` transcript provenance, while the
  product Plan/Rebuild action retains `application` provenance. An optional confirmed-initiation
  prefix remains application context and does not rewrite either action's submitted text or
  provenance. The prior blanket application provenance kept the real `Plan Epic` control disabled;
  the later blanket user correction briefly mislabeled the product Plan/Rebuild prompt.
- Proposal saves start an immediate SQLite transaction. This removes the WAL deferred-read to
  writer-upgrade race reproduced live as `record applied proposal command: database is locked`.
  A 30-second busy-timeout experiment did not remove the first failed call and was reverted.
- Bootstrap and Runner `launchedAt` facts now require the durable provider-neutral launch-
  acceptance marker. Persisted-but-unaccepted invocations remain conservatively unlaunched.

## Deterministic proof

- Rust library, serial: **168 passed, 6 intentionally ignored, 0 failed**.
- Focused after corrections: Plan Builder application **9 passed, 3 live ignored**; transition
  **17 passed, 1 live ignored**; orchestration repository **15 passed**; launch-acceptance crash
  windows **2 passed**.
- Installed CLI help/capability probe: **1 passed**.
- The earlier frontend serial aggregate passed **87 files / 601 tests**. After the provenance
  correction added one test, the current isolated-worker aggregate passed **87 files / 602
  tests**; the four focused provenance/UI files passed **51/51**. A forced shared single-fork run
  reproduces pre-existing cross-file DOM cleanup noise. TypeScript, ESLint, production Vite build,
  and current Tauri release build passed. Existing React `act`, Node SQLite experimental, Rust
  dead-code, and Tauri bundle-identifier warnings are non-failures.
- Rustfmt, the touched evidence file's Prettier check, and `git diff --check` passed. The repo-wide
  Prettier check still reports **39 pre-existing files** outside this unit's changes.
- Schema-v7 migration/reopen, native-v2 Rust/TypeScript goldens, strict transition-v2 decoding and
  canonical composition, confirmation controller/modal, one-shot context crash windows, Bootstrap
  retry/cross-attempt/path containment, and exact downstream launch counts pass in the aggregate.

## Installed-Codex proof

- Fresh ordinary discussion: **1 user-origin invocation, 0 proposals, 0 semantic calls**.
- Fresh build plus managed resume/rebuild: **2 user-origin invocations, 2 proposal revisions,
  2 `submit_epic_plan_proposal` calls, 0 initiation calls**; reopen projected both revisions.
- Failed evidence was retained during investigation: one rebuild run stopped at one proposal after
  the safe SQLite `internal_error`; the busy-timeout experiment reached two proposals only after
  **3** submit calls including a failed retry. The immediate-transaction correction produced the
  final exact **2/2** result.
- One isolated, test-owned already-confirmed initiation used the production specialized service,
  production MCP adapter, and real Codex. It observed **2 completed role invocations**, exactly
  **1** `complete_epic_bootstrap` call, **1** semantic fact, **2** application-written material
  files with matching paths/sizes/SHA-256 values, **1** same-attempt accepted inventory, and
  exactly **1** launch-accepted read-only Runner. That earlier proof predated the downstream
  activation boundary; the current Runner exposes only the bounded Sprint Runner request action.
- That isolated state contained the one planned `initiated_sprints` row required by durable
  initiation, but **0 Sprint Agent Sessions, 0 Sprint actions, 0 Work Units, and 0 execution
  effects**.

## Production UI and residual boundary

The current release application loaded through real Tauri composition. It rendered the active
pre-initiation draft and the correlated transition label `Bootstrap attempt 1 running`, while
truthfully showing that no Sprint execution or Epic lifecycle had been observed. The retained
failed draft visibly exposed the provenance defect and disabled plan action before correction.

The corrected release binary built successfully, but Windows locked before it could be reinspected.
No modal confirmation was clicked. The remaining integrated proof is: unlock Windows, issue one
new user-origin discussion/build query on the real draft, open the shared initiation modal, and let
the human confirm or reject. Only after confirmation may the application demonstrate the integrated
prepared Bootstrap-to-Runner chain. Filesystem link/reparse checks remain deterministic rather than
race-proof, confirmation registrations remain process-memory state before decision, and no provider
process reattachment or atomic external-provider-processing claim is made.

## Sprint Runner activation boundary

- The Epic Runner Harness exposes only `request_next_sprint_runner`. Its semantic input is one
  approved `sprintId`; the application derives the Epic, originating Runner session and
  invocation, Harness revision, correlation, and launch authority from durable records.
- The durable `sprint-runner-transition-query/v1` projection distinguishes requested, authorized,
  session-created, Harness-applied, and launch-accepted facts. `preStartReady` means only durable
  launch acceptance; it does not claim lifecycle observation or Sprint acceptance.
- Local fake-runtime coverage created one correlated Sprint Runner and replayed the request and
  startup reconciliation without a second session or invocation. It also denied a foreign Runner
  and a non-owned Sprint and verified that no Work Slice or Work Unit tables existed.
- Historical pre-start boundary note (before PS-C): no Sprint start was then recorded. Current
  state records Sprint start authorization/persistence and repository/branch reevaluation, but
  still creates no Work Slice planning, Work Unit, or later-Sprint effect.
- Production startup reconciles the attached Sprint Runner transition service after Bootstrap
  reconciliation. Same-Sprint semantic calls use a narrow in-process transition lock, so concurrent
  replays return the one durable route instead of an internal uniqueness failure.
- The production composition strictly decodes and loads this query with the native orchestration and
  Bootstrap queries. The existing Sprint surface projects only phase-specific durable facts:
  outcome/lifecycle/acceptance, requested and persisted continuation, launch acceptance, explicitly
  unobserved or observed activation, Epic authorization wait, Sprint start, reevaluation, and
  planning-ready/downstream-not-started. Persistence is never described as receiver delivery or
  activation.

## Pre-start outcome and automatic continuation boundary

- Productive composition now persists a narrow pre-start semantic outcome separately from its
  matching terminal lifecycle. Only the same completed invocation with both facts is accepted.
- Acceptance persists one application-delivery request and one fresh Epic Runner continuation with
  a one-invocation `start_selected_sprint` MCP authority. Delivery, launch acceptance, provider
  activation, and receiver lifecycle remain separate facts; provider activation is not claimed.
- Only that fresh identity-free action persists Sprint start. The same Sprint Runner Session then
  receives one application-owned started invocation with a separate repository/branch reevaluation
  action. Its semantic record is required before `planningReady`; downstream remains unstarted.
- Deterministic/local coverage is appropriate for replay, restart, and fake launch evidence. No
  paid or live provider run, receiver-activation observation, or arbitrary provider reattachment is
  claimed by this record.
- The deterministic state-machine regression proves outcome-content delivery, a foreign-start
  denial, same-Session Sprint continuation, divergent semantic-replay denial, persisted
  repository/branch evidence, planning-ready, and absent downstream tables. The restart regression
  also proves that a durable v1 pre-start record keeps its historical revision and receives a
  separate v2 continuation invocation after its prior invocation has terminally settled.
- Productive notifier regression emits terminal events synchronously from the Agent Session runtime
  launch path for the pre-start, Epic continuation, and started reevaluation invocations. It proves
  no transition-lock re-entry deadlock and no duplicate restart effect. The fake runtime is
  deterministic evidence only; no live provider or human receiver activation is claimed.
- The checked-in current profiles are `epic_runner` v3 and `sprint_runner` v2. A persisted
  historical Epic Runner v2 request binding remains v2 and is not relabelled; fresh continuation
  context binds v3.
