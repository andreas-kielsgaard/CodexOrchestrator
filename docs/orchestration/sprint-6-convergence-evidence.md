# Sprint 6 convergence evidence

Status: **partial**. Deterministic convergence and isolated installed-Codex Plan Builder,
Bootstrap Generator, and Epic Runner paths pass. The production user-confirmation click and the
integrated post-click chain remain a manual gate; no confirmation was bypassed.

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

## PS-WSP5: Started Sprint to one Work Slice Planner boundary

This Plan Step converges and verifies the final application-owned boundary on the required
product route `codex/operational-spine-ps-r1-wsp2-recovery`, from parent
`a1fd69bdfcc2c19d77a936eb2c8244188dfa88ac`. The accepted checkpoint sequence is:

`fe409510efd1e0de2634c635c50ae6abfaaa5d0d` → `88d536490011869e0d2be5edcec5fd4ca4d3b0e2` →
`b964509f2d979eb26e1181c796ac88c95f485647` → `8e414e4067c943e394ffaf537db93c4e452aafe9` →
`729312e12149bb43c8c67b2d1d77ad0d6d26e901` → `adf4a520d5577f3e522d0282ef225b534f4abaa6` →
`51fedb894353230ebb72f4e1c965efa643dc77cd` → `a3126c0be175fe7cf839bf5d21c9e2fed2ba923e` →
`ce16bad8f172eaa6ca813ac0c0e6947b851dcf6a` → `a1fd69bdfcc2c19d77a936eb2c8244188dfa88ac` →
one PS-WSP5 descendant.

The narrow correction is in `src-tauri/src/orchestration/sprint_runner_transition.rs`,
`src/application/orchestrations/sprintRunnerTransition.ts`, and the existing Sprint boundary in
`src/features/orchestrations/components/SprintWorkspace.tsx`. SQLite already persisted
`work_slice_planning_requests.requested_at` and `authorized_at`; the native transition query now
serializes them as `workSlicePlannerRequestedAt` and `workSlicePlannerAuthorizedAt`. The decoder
rejects a timestamp without its request, authorization without request, and planning-point
creation without durable authorization. The UI presents separate Planner request and Planner
authorization stages before the planning point, reserved correlations, Session/invocation,
Harness, launch, runtime acceptance, readiness, provider observation, and lifecycle observation.
No schema redesign or provider/agent identity was added.

Validation evidence:

- Productive Rust library path: `cargo test --manifest-path src-tauri/Cargo.toml --lib
  orchestration::bootstrap_transition::tests:: -- --test-threads=1` produced **27 passed, 1
  ignored, 1 failed**. The one failure is the unchanged accepted baseline
  `launch_accepted_epic_runner_authorizes_one_ready_sprint_runner_without_downstream_effects`,
  which still asserts `planning_ready` before required lifecycle reconciliation. The WSP1
  concurrency/reopen/no-child paths and WSP3 prepared launch/readiness/no-downstream path passed;
  the WSP2 restart/partial-effect assertions passed.
- Deterministic TypeScript/UI path: focused Vitest files passed **3 files / 9 tests** for the
  strict decoder/projection, Planner boundary UI, and Tauri transition client. TypeScript
  `tsc --noEmit` passed with exit 0.
- Frontend validation reused the saved checkout's already-installed `node_modules` only through
  isolated temporary validation roots. Product and saved `package.json` and lockfile hashes were
  identical. No package was installed, updated, or written; the temporary roots were validation
  plumbing only and are removed after validation.
- Production composition remains the real native path: `tauriOrchestrationNativeQuery` consumes
  the real native query plus `createTauriSprintRunnerTransitionClient`, which invokes
  `load_sprint_runner_transition_query`. Recorded/development composition remains explicit and
  separate; it was not used as production proof.

The evidence distinguishes eligibility/request availability, Planner request, durable Planner
authorization, planning point, reserved child correlations, Session creation, invocation
creation, Harness application, launch requested, runtime launch accepted, readiness, provider
activation observation, and lifecycle observation. Delivery and receiver activation remain
separate facts. The fake runtime and local SQLite/reopen assertions are deterministic evidence;
the transition query and UI are productive code paths; recorded/development data is not
productive evidence. No provider activation, provider lifecycle, provider MCP use, live or paid
provider behavior, human UI observation, product launch acceptance, or Slice acceptance is
claimed. The Planner Harness exposes zero MCP tools. No Planner proposal/result/settlement, Work
Unit, Handler, Implementer Session, later planning point, Sprint settlement, or downstream effect
is created or accepted. The quarantined `operational-spine-ps-r1` route and saved checkout remain
untouched; the formerly protected `8bb8` worktree remains absent and unregistered.

## OSWU-03: Accepted Work Slice plan to durable Work Units

Independent audit checkpoint: `6198cbb` -> `440cd9b` -> `0132381` -> `351e043`, with
strict native-query correction `974ee2c`. The audit used this detached-worktree chain only.

- The productive Rust path accepts only one current, valid, refinement-free, semantically
  completed, lifecycle-completed, application-accepted, materialization-ready revision. It
  creates stable Work Slice, Work Unit, and relationship identities in one immediate SQLite
  transaction. Replays, concurrent opens, and a repaired missing relationship retain those
  identities; stale current state fails closed.
- The native query, strict TypeScript decoder, composition, and bounded Sprint UI use durable
  records. Partial authorization, attempt, unit, relationship, and settlement facts remain
  distinct; only settled units project as Work Units. UI copy says planned responsibilities and
  does not claim Handler activation or execution.
- `974ee2c` rejects duplicate semantic relationships and incoherent planning-point or Sprint
  ownership in an otherwise forged native payload. It adds the duplicate-dependency regression.
- Validation: focused Rust materialization path **1 passed**; focused native-query/UI/Tauri
  suites **3 files / 26 passed**; application orchestration suite **14 files / 180 passed**;
  TypeScript passed. Broader Rust bootstrap suite observed **28 passed, 1 ignored, 1 failed**:
  the existing `launch_accepted_epic_runner_authorizes_one_ready_sprint_runner_without_downstream_effects`
  assertion still expects `planning_ready_at` before lifecycle reconciliation. Broader feature
  suite observed **8 files / 55 passed, 1 file / 1 failure**; the isolated unrelated
  `EpicPlanBuilder` suite observed **9 passed, 1 failure** in its existing product Plan prompt
  assertion. The full frontend aggregate exceeded the two-minute practical window without a
  result.
- No Handler or Implementer Session, invocation, worktree, provider process, execution attempt,
  review, Work Slice settlement, Sprint/Epic continuation, pause/restart,
  or reattachment was added. This is deterministic local evidence only; live provider,
  human-in-the-loop, and later activation/execution boundaries remain unproven.
