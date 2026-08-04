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

## HA-01: Durable initial Work Unit Handler activation

- Requested execution profile: Terra with high reasoning. Harness-confirmed profile: not exposed
  by this task harness; no runtime setting is claimed from the request alone.
- Product composition attaches an application-owned Handler activation coordinator only after the
  existing Agent Session and execution-support seams exist. It derives candidates from settled
  materialized Work Units and direct `depends_on` relationships. At this boundary no authoritative
  prerequisite-satisfaction fact exists: every plan-defined dependency therefore remains durably
  `blocked` as `prerequisite_satisfaction_not_authoritative`; Handler readiness, launch acceptance,
  provider observation, lifecycle, transcript, and silence never satisfy it. Missing initiated-
  Sprint Git authority is separately `blocked` and neither block creates a Session, invocation,
  worktree, or provider launch.
- Each eligible Work Unit has stable application-derived ordinal-0 attempt, Handler Session, and
  invocation identifiers. The coordinator records request, authorization, attempt creation,
  execution-support/worktree preparation, Session creation, invocation preparation, Harness
  binding, launch request, launch acceptance, and Handler readiness independently. Reopen and
  replay reuse those identities and only retry launch of the already-persisted pending invocation.
- The Handler package remains read-only and has no MCP tools or Implementer-request action. No
  Implementer identity, output acceptance, review, retry, settlement, continuation, pause/restart,
  provider compliance, process reattachment, or user acceptance is created or claimed.
- The initial activation persists an exact serialized `ConversationHarnessProfile` with its key and
  version. Reopen/replay reconstructs only that pinned snapshot through the narrow execution-
  support seam; a missing, malformed, or current-catalog mismatch fails closed rather than silently
  substituting a mutable profile. Callers do not supply a revision or runtime override.
- Provider activity is recorded only from the correlated invocation observation seam, stays distinct
  from launch acceptance/readiness, and is reconciled from persisted Handler activity notifications.
  The productive native query and approved Work Unit activity details distinguish blocked,
  prepared, requested, launch-accepted/ready, provider-observed, and unobserved states. Recorded
  fixtures remain development data and are not production proof.
- Correction validation: focused Rust Handler boundary tests **2/2** (305 filtered); Handler
  Harness-surface test **1/1** (304 filtered); `cargo check --manifest-path
  src-tauri/Cargo.toml`; and `git diff --check` passed. In an isolated temporary copy with
  `npm ci --ignore-scripts` (no manifest or lockfile modification in this worktree), focused
  `nativeQuery.test.ts` passed **17/17**, `npm exec tsc -- --noEmit`, ESLint, and `npm run build`
  passed. Repository-wide Rustfmt still reports pre-existing formatting drift outside this patch and
  was not applied.
- This remains a partial local checkpoint: it does not yet prove production-equivalent root launch,
  two-service concurrency, partial-stage/reopen recovery, missed provider-observation recovery, or
  catalog-drift replay with integration-level tests. No live provider, human, provider-compliance,
  lifecycle/outcome, reattachment, downstream Implementer, review, retry, settlement, or
  continuation claim is made.

### HA-01 correction: immutable revision and coordinator proof

- Handler activation no longer persists a mutable catalog JSON snapshot. New records pin the
  application-owned immutable Harness revision ID, content digest, and local commit reference;
  reopen verifies that exact evidence through `OrchestrationApplication`. A newer revision may be
  used only by a future record. Missing evidence fails closed and is never replaced by the newer
  revision. Revision ledger timestamps are normalized before both immutable manifest and SQLite
  publication so verified reopen preserves exact evidence.
- The production-equivalent `work_slice_planning_request_launches_one_prepared_planner_and_marks_readiness`
  test now drives settled Work Unit materialization, a real isolated Git worktree through the
  execution-support seam, `AgentSessionApplication`, and a deterministic recording runtime. It
  proves a missing-Git-authority root block with no execution effect, then one root Handler
  launch/acceptance/ready chain, a dependency blocked after root readiness, replay and two
  reopened coordinators without a second launch, recovery of a persisted-but-
  unnotified provider activity event, revision-A reuse after legitimate revision B publication,
  and fail-closed missing A evidence without an Implementer authorization, Session, or invocation.
- Latest focused validation: the coordinator integration test **1/1** (306 filtered), immutable
  Harness revision suite **13/13** (294 filtered), Handler surface **1/1** (306 filtered), Cargo
  check, diff check, and isolated native-query Vitest **17/17** plus TypeScript typecheck passed.

### HA-01 correction: approved Work Unit activity projection

- The approved `WorkSlicePlannerBoundary` now receives only the already-composed current Work Unit
  presentation view. It renders Handler activity from an explicit typed activation projection;
  JSX does not query Tauri, parse display prose, or reconstruct activation state. With no typed
  activation it keeps the materialization-only statement, “No Handler activation is recorded.”
- The native-query projection states Handler launch acceptance and application Handler readiness
  together, while provider activity remains a separate observation. It also keeps eligible,
  prepared, launch-requested, and dependency-blocked states distinct. A blocked dependent shows
  the durable `prerequisite_satisfaction_not_authoritative` reason; provider observation states no
  provider lifecycle, outcome, or acceptance.
- The integrated planner-boundary regression starts from a decoded native query, composes the
  product read model, projects the Sprint workspace, and renders a ready root plus a blocked
  dependent. It asserts the distinct wording and no Implementer, Handler-review, implementation
  output, retry, application-acceptance, or continuation claim.
- Validation for this correction: focused Rust coordinator integration **1/1** (306 filtered),
  immutable Harness revision **13/13** (294 filtered), execution-Harness **2/2** (305 filtered),
  and `cargo check --manifest-path src-tauri/Cargo.toml` passed. In an isolated temporary
  dependency root provisioned with `npm ci --ignore-scripts` and without changing this worktree’s
  manifest or lockfile, focused native-query plus Sprint Workspace Vitest passed **2 files / 22
  tests**, TypeScript passed, ESLint passed, and `npm run build` passed. Focused Prettier then
  passed for the four changed TypeScript/React files.
- `rustfmt --edition 2021` was intentionally limited to the five HA-02-named Rust files:
  `agent_sessions/application/lifecycle.rs`, `agent_sessions/application/tests.rs`,
  `orchestration/conversation_harness.rs`, `orchestration/repository/tests.rs`, and
  `orchestration/work_unit_execution_harness.rs` (**189 additions / 78 deletions**, formatting
  only). Broader mixed baseline/candidate drift was not reformatted. `git diff --check` passed.
  This evidence remains local and deterministic; no live-provider compliance, lifecycle/outcome,
  human observation, process reattachment, Implementer activity, or downstream result is claimed.

### HA-01 correction: typed Handler activity state

- `NativeMaterializedWorkUnitV1.handlerActivation` now maps to an explicit typed Work Unit
  presentation field through native composition, the product read model, and the Sprint workspace
  projection. The field represents only `blocked` with its exact durable reason, or `eligible`
  with one of `eligible_not_prepared`, `invocation_prepared`, `launch_requested`,
  `launch_accepted`, or `handler_ready`, plus a separate invocation-correlated provider-activity
  observation boolean. It intentionally omits runtime routes, authority, and immutable binding
  control values.
- The Planner boundary uses only that typed field to decide whether activity exists and which
  wording to render. Display `details` remains readable context but cannot create, suppress, or
  advance Handler state. The regression carries decoded native data through composition and
  workspace projection, then proves a no-activation Work Unit whose title, specification, and
  display details say “Handler” renders no Handler activity. It also proves ready/provider-observed,
  dependency-blocked, eligible, prepared, and launch-requested state wording, with no Implementer
  or downstream claim.
- Latest validation: isolated focused native-query plus Sprint Workspace Vitest **2 files / 22
  tests**, TypeScript, ESLint, production frontend build, and focused Prettier all passed. `cargo
  check --manifest-path src-tauri/Cargo.toml`, named-file `rustfmt --check`, and `git diff --check`
  passed; Cargo check retains only the existing dead-code warnings. The prior deterministic Rust
  coordinator and immutable-Harness regressions remain passing evidence and were not changed by
  this TypeScript-only state projection correction. No live/provider, human, Implementer, review,
  outcome, settlement, or continuation claim is added.

### HA-01 correction: launch acceptance before Handler readiness

- Native display detail now recognizes `launchAcceptedAt` before `launchRequestedAt` when
  `handlerReadyAt` is absent. The resulting copy is “Handler launch accepted; application Handler
  readiness is not yet recorded.” Provider activity remains a separate observation and the copy
  makes no provider lifecycle, outcome, compliance, or application-acceptance claim.
- A focused regression decodes the native query, composes the product read model, projects the
  Sprint workspace, and renders `WorkUnitDetailWorkspace`. It proves the typed stage is
  `launch_accepted`, the detail shows the accepted/not-ready boundary, and the false “acceptance is
  not yet recorded” wording is absent. Existing typed Planner-boundary coverage remains intact.
- Validation: isolated focused Vitest **3 files / 23 tests**, TypeScript, ESLint, production
  frontend build, and focused Prettier passed. `git diff --check` passed. This TypeScript-only
  correction changed no Rust source, so no Rust suite was rerun. No live/provider, human,
  Implementer, review, outcome, settlement, or continuation claim is added.

## HI-03: Work Unit Handler to Implementer activation audit and convergence

The independent audit covered the accepted baseline
`a5e0abdef85a1937696a7926637ee8972ea342ec` through the supplied Slice head
`b5edee236a1a6d1ac40c09103eadb3204a7f9d63`. Bounded audit corrections are checkpointed in
`547aafea71c276b3f454b16348d61fbb0d48c71c` (`Audit Handler Implementer convergence`). This
checkpoint is a candidate returned for Slice evaluation; it is not Slice acceptance, merge,
publication, receiver activation, or user acceptance.

The corrected boundary preserves the original immutable, read-only, actionless Handler revision.
Only the later application-owned same-Session continuation carries the zero-input
`request_work_unit_implementer` action. The public action rederives and verifies the stable Handler
attempt, original Handler Session and invocation, action invocation, immutable action revision,
active application provenance, original terminal state, eligibility, phase prerequisites, and
absence of block or failure before recording any Implementer effect. Wrong bearer, Host, Origin,
original invocation, Session, revision digest, inactive or terminal action, stale block or failure,
missing readiness prerequisite, and dependency-ineligible state are denied before the Implementer
request or execution-support grant.

The Implementer reuses the original Handler attempt and exact isolated worktree. Handler and
Implementer receive distinct role-bound capabilities; both Handler invocations remain `ReadOnly`,
while only the Implementer is `WorkspaceWrite` for that workspace. One pinned Implementer Harness
revision, Session, and invocation are reused across duplicate requests, reopen, partial recovery,
and provider-observation recovery. Public action authorization remains limited to the pending or
running action invocation, while restart reconciliation uses only an already-persisted correlated
Implementer request. A terminal action therefore exposes no recreated MCP server and cannot be
called, but its recorded request can still finish reconciliation without a replacement identity or
runtime launch.

Request, authorization, execution support, worktree readiness, Session creation, invocation
preparation, Harness binding, launch request, launch acceptance, provider activity observation, and
application readiness remain separate durable facts. Retryable preterminal failure retains the
same identities. Terminal non-acceptance for either the Handler action continuation or Implementer
records an exact failure reason, leaves readiness absent, and does not relaunch on reopen. The Rust
native projection and strict TypeScript decoder now fail closed on foreign correlations, missing
phase prerequisites, blocked state with later effects, failure combined with readiness, or reused
action/Implementer invocation identity. The Work Unit detail renders these as factual activity only,
including durable failure wording, and contains no activation control.

Validation evidence for the audited checkpoint:

- Production-equivalent Rust coordinator regression:

  ```text
  cargo test --manifest-path src-tauri/Cargo.toml --lib orchestration::bootstrap_transition::tests::work_slice_planning_request_launches_one_prepared_planner_and_marks_readiness -- --exact --test-threads=1
  ```

  It passed **1/1** with **315 filtered** and exercises the scoped MCP
  transport, action/context denials, stable attempt/worktree and role authority, duplicate and
  concurrent requests, restart/partial drains, immutable revision reuse, provider observation,
  terminal action and Implementer failure/no-relaunch, native projection, the blocked dependent,
  unchanged upstream state, and absence of forbidden downstream tables.
- Native repository suite passed **26/26** with **290 filtered**, including the new malformed or
  foreign activation fail-closed regression. Execution-support suite passed **10/10** with **305
  filtered**. Conversation Harness suite passed **6/6** with **309 filtered**, and Work Unit
  execution-Harness suite passed **2/2** with **313 filtered**.
- Application orchestration plus the two Work Unit/Sprint activity UI files passed **16 files / 186
  tests**. `cargo check --manifest-path src-tauri/Cargo.toml`, focused ESLint, TypeScript
  `tsc --noEmit`, and the production Vite build passed. Focused Prettier and `git diff --check`
  passed. Only the existing Rust dead-code warnings and temporary Git line-ending notices were
  observed.

No implementation output, Handler review, acceptance, return, retry attempt, settlement,
integration, dependent activation, later planning point, Sprint continuation, or Epic continuation
is created by this boundary. The deterministic `RecordingRuntime`, local SQLite, isolated Git
worktree, and local MCP server prove application behavior and correlations only. They do not prove
live external-provider launch behavior, provider-private activation or compliance, provider result
quality, receiver evaluation, or user acceptance.

## IO-04: Implementer outcome submission and Handler-review readiness

The accepted baseline `76b5b2f416f36c8a93de10e4edbdc9f737318855` converged through the
independently corrected candidate `0d686299e6ed793d224ffdc8fbd44f6a904f3376`. This section is
only the final candidate record returned for Plan Slice evaluation; it does not imply merge, push,
publication, release, Handler review, overall-plan acceptance, or user acceptance.

- The original immutable Implementer activation remains actionless. Only a later immutable
  reporting continuation in the same Session may expose the identity-free
  `submit_implementation_outcome { outcome: review_pending, summary, validationStatement }` and
  zero-input `complete_implementation_outcome`. The application requires whitelist discovery and
  `Available` policy for both tools. Independent audit correction `0d686299` closes the prior gap
  where matching tool names alone were sufficient.
- Every correlation is application-derived and revalidated: the exact Work Unit and stable
  attempt, its isolated writable worktree and execution-support grant, the Implementer Session,
  original `Completed` Implementer invocation, stable reporting invocation, and pinned reporting
  revision ID, configuration digest, and repository commit reference. Agent-supplied identities do
  not create or replace any of these records.
- Reporting request, preparation, Harness binding, launch request, launch acceptance, reporting
  readiness, outcome submission and validation, evidence readiness, semantic completion, terminal
  lifecycle observation and status, application acceptance, and Handler-review readiness are
  separate durable facts. The Implementer summary and validation statement remain claims, not
  evidence.
- The application owns File Review capture and later revalidation of the changed-file manifest,
  comparison and its fingerprint, and each evidence reference and content fingerprint. Acceptance
  requires the exact valid `review_pending` semantic payload and fingerprint plus exact revalidated
  evidence state, semantic completion by the stable reporting invocation, and that exact
  invocation's observed `Completed` reporting lifecycle. Reporting lifecycles observed as `Failed`,
  `Canceled`, or `Interrupted` cannot produce application acceptance or Handler-review readiness.
- Local/fake-only regressions prove exact duplicate and concurrent retry, divergent payload
  rejection, evidence and payload drift rejection, missed-notification reopen reconciliation, and
  reserved-row startup safety without replacement Work Unit, attempt, Session, original invocation,
  or reporting invocation identities.
- The Rust native query, strict TypeScript decoder and read models, approved Work Unit detail
  activity UI, and product Implementer skill agree on this boundary. The UI renders **Ready for
  Handler review** only from `handler_review_ready_at`; it explicitly records no Handler judgment
  and does not treat readiness as implementation approval or Work Unit acceptance.

Accepted validation evidence:

- `cargo test -p codex-orchestrator implementer_reporting -- --nocapture`: **6 passed, 0 failed
  (319 filtered)**.
- `cargo test -p codex-orchestrator implementer_outcome_projection -- --nocapture`: **2 passed, 0
  failed (323 filtered)**.
- `npm test -- nativeQuery.test.ts sprintWorkspacePresentation.test.ts
  WorkUnitDetailWorkspace.implementerOutcome.test.tsx`: **3 files, 24 passed**.
- `npm run build`: `tsc --noEmit` plus Vite passed; **2056 modules transformed**.
- `cargo check -p codex-orchestrator`: passed with **7 pre-existing dead-code warnings**.
- `cargo test -p codex-orchestrator --lib -- --nocapture`: **316 passed, 1 baseline-existing and
  noncausal failure, 8 ignored**.
  The sole failure,
  `orchestration::bootstrap_transition::tests::launch_accepted_epic_runner_authorizes_one_ready_sprint_runner_without_downstream_effects`,
  is the `planning_ready_at.is_some()` assertion at candidate line 2551 and baseline line 2549. It
  reproduces in isolation and predates the accepted baseline, so this aggregate is not recorded as
  an accepted green gate.
- `git diff --check`: passed.

No Handler review or judgment, Work Unit accept or return, retry attempt, settlement, dependent
activation, planning settlement, Sprint or Epic continuation, Pause/Restart, authority broadening,
or process reattachment was created. No live provider, Codex, or MCP process turn, paid smoke, real
OS/process reattachment, packaged Tauri release/build, or production migration was exercised.
Live-provider compliance, production behavior, and user acceptance remain unproven.

## HR-04: Independent Handler review, acceptance, and return

The accepted baseline `d255461114ea359872ad71774143b5f12dcadf04` converged through the
independently audited candidate `9223e25f81b3e9a300fd23ffd6e71b073cda0683` via
`3efdaae`, `f7d4676`, `7548705`, `1deb186`, `96a725c`, `2341f19`, and `9223e25`. This is
an independently audited local candidate for the bounded Handler-review movement. It is not
merge, push, integration, release, production/provider observation, planning settlement, retry
activation, dependent activation, or user acceptance.

- One durably application-accepted, Handler-review-ready `review_pending` Implementer outcome
  automatically persists one application-owned delivery/review record and one stable fresh review
  invocation in the original Handler Session. That invocation uses a distinct immutable Handler
  revision, `ReadOnly` sandbox, `Never` approval, the exact attempt worktree, and only
  `read_handler_review_evidence`, `accept_implementation_outcome`, and
  `return_implementation_outcome`.
- Exact structured claims, the application-owned changed-file manifest, evidence references and
  content fingerprints, and the comparison fingerprint are delivered and revalidated. Callers
  supply neither routing identities nor unrestricted raw paths. Delivery request, delivery
  persistence, Harness binding, launch request, launch acceptance, review readiness, semantic
  judgment, lifecycle observation, conflict, final decision, implementation accepted/returned,
  `retry_required`, and settlement readiness remain separate facts.
- Only the exact live review invocation may submit one identity-free accept or bounded structured
  return judgment. Judgment remains pending until that exact invocation is durably observed
  `Completed`; `Failed`, `Canceled`, `Interrupted`, provider-terminal-without-judgment,
  transcript, silence, or files never imply a decision. Accept and return are distinct. Return
  retains code, explanation, and `retry_required_at`, creates no attempt ordinal 1, and leaves
  `settlement_ready_at` absent.
- Reopen/partial effects reuse the exact review identity and revision and do not double-launch;
  exact replay is idempotent. True concurrent exact accepts replay, while simultaneous
  accept-vs-return produces one authoritative judgment, one domain conflict, and one final
  decision without a database-lock/unavailable loser. The production native query, strict
  TypeScript decoder/read models, existing Work Unit activity UI, and Handler product skill
  project these facts without manual review buttons or later-workflow implication.
- No retry creation, Implementer relaunch, planning-point settlement, dependent activation,
  Work Slice settlement, Sprint/Epic continuation, Pause/Restart change, process reattachment,
  authority expansion, or upward continuation/callback exists.

Validation evidence for this checkpoint:

- HR-01E full Handler-review production transition test: **1 passed, 327 filtered, about 182s**.
  Typed JSON regression: **1 passed**. Relevant Implementer reporting/Harness suite: **6 passed**.
  Rust repository/native projection suite: **29 passed**.
- Independent concurrent replay/divergent-race/no-downstream-effects test: **1 passed, 329
  filtered, 58.80s**. `cargo check` passed at implementation, projection, and audit checkpoints
  with only pre-existing dead-code warnings. `git diff --check` passed and the candidate worktree
  is clean.
- HR-03 attempted to rerun the pre-existing broad Handler-review test, but managed local-server
  drain exceeded two minutes and the agent-owned process was terminated; that rerun is neither
  passed nor failed. The earlier HR-01E completed pass is the evidence. Frontend Vitest/`tsc`
  execution is unproven because this authorized clean checkout has no installed binaries and
  dependencies were not installed; static independent TypeScript/TSX export/type/fixture/render
  review found no defect.

No retry, downstream activation, settlement, continuation, provider observation, process
reattachment, authority expansion, or user acceptance is proven by this record.

## AIS-06: Accepted Work Unit integration and settlement convergence

This final documentation records the accepted Work Unit integration and settlement Slice at
clean detached checkpoint `85a1ace2ea3a1a3eca7d66e7c35806552094fb2c`. It supersedes the earlier
Handler-review wording above only for the later boundary; that wording remains historically
accurate at its older checkpoint.

The authoritative chain is distinct from retry-only capture. The Handler-review baseline is
`06769488643ea8272c03671c0988f0e5f7e6ef59`. Policy A selects authoritative Git integration. The
accepted-candidate authority and target-binding chain is `2453a68`, `8a69194`, `1edbbe3`; the
integration/recovery/migration chain is `36f3eeb`, `5d55978`, `5d035be`, `e140457`, `3461a1b`,
`00143a1`, `9e3110a`, `2e22390`, `9fb3e94`; productive projection and boundary correction are
`98722a4`, `ba893de`; and the independent audit corrections are `a5f102a`, `85a1ace`.
Retry-only capture retains a candidate for retry or attention and does not itself authorize,
reserve, integrate, settle, or contribute a prerequisite effect.

For an accepted candidate, the application derives target-scoped reservation and effects from
durable authority and the bound target. Under the target lock, one deterministic single-parent
integration commit is created or exactly adopted for the conflict-free accepted candidate, never
duplicated; an advanced parent delta is applied to the target; and replay reconciles the exact
candidate, target, commit, runtime, and database state. The durable sequence
keeps request, authorization, reservation, object creation, ref advancement, runtime advancement,
database advancement, evidence, terminal attention, Work Unit settlement, and each per-edge
prerequisite contribution as separate facts. A same-candidate replay reuses the settled route and
does not create a second commit or settlement. Different accepted candidates serialize on the
target, but only a conflict-free candidate receives its parent-linked integration commit, evidence,
and settlement; a conflicting candidate ends in durable terminal attention and remains unsettled.
Invalid retained lineage likewise ends in durable terminal attention and remains unsettled rather
than being silently retried.

The productive Rust, TypeScript, and UI projection exposes only safe semantic integration facts:
progress, terminal attention, success, settlement, and prerequisite contribution. Private candidate,
authority, intent, ref, integration identity, commit, tree, and repository-path values remain out
of the projection. Legacy observed integration, completion, and handoff events remain empty; their
historical/materialization settlement is separate from productive integration settlement and does
not imply receiver delivery, provider activation, Work Unit acceptance, or later workflow state.

The accepted evidence is deterministic local proof: isolated Git and temporary SQLite fixtures,
reopen/recovery, same-candidate replay, different-candidate serialization, exact CAS/runtime/DB
convergence, terminal attention, projection privacy, Work Unit settlement, and per-edge prerequisite
contribution. It does not prove a live provider effect, receiver activation, provider acceptance,
or user acceptance. The focused Cargo rerun attempted for this documentation did not complete within
the bounded 120-second command window and is therefore not recorded as a passing current gate.
Frontend Vitest, TypeScript, and ESLint execution remains unproven because this worktree has no
`node_modules`; nothing was installed. Static or syntax review is not frontend runtime proof.

Hard downstream stop: no retry creation, Implementer relaunch, dependent activation, planning-point,
Work Slice, Sprint, or Epic settlement, continuation, push, publication, provider acceptance, or
user acceptance occurred or is authorized by this record.

## DWE-01: dependent Work Unit execution-cycle convergence

The productive Handler-review terminal notification now enters the same ordered downstream drain
as startup reconciliation: finalize the completed accept-or-return decision, reconcile accepted
candidate and integration facts, then recompute dependency eligibility only when a settled
prerequisite contribution leaves a dependent absent, blocked, or not launch-accepted. Thus an
accepted root can make its exact dependent eligible without a separate manual replay. A return
cannot create an accepted candidate, integration, contribution, retry, or later attempt.

Current deterministic validation in this candidate checkout:

- Handler-review terminal notification: **1 passed, 367 filtered**.
- Concurrent exact replay and accept-vs-return race: **1 passed, 367 filtered**.
- `git diff --check` passed. The first attempted focused selector matched no test; it is not a
  passing gate. Two earlier compilation attempts timed out while building the isolated Cargo
  target; the completed reruns above are the recorded results.

The existing native-query, strict frontend read-model, and Work Unit activity surfaces were
inspected for the established safe projections, but frontend Vitest, TypeScript, and UI execution
are **unrun** here because this clean routed checkout has no `node_modules`; dependencies were not
installed. Broad Rust suites, live-provider receipt, provider activation/compliance, and human
acceptance are likewise unproven. This correction does not add retry, ordinal-1 authority,
relaunch, planning-point/Work Slice/Sprint/Epic continuation, reattachment, push, or publication.
