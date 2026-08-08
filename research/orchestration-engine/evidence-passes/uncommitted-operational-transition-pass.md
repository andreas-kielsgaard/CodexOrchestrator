# Observation pass: uncommitted Work Slice Planner prelaunch transition

> **UNCOMMITTED / MOVING EVIDENCE**
>
> This document describes one dirty source worktree as observed on 2026-08-07. It is not an integrated capability claim, an accepted product boundary, or a description of the current research tip.

## Snapshot

| Item | Observed value |
| --- | --- |
| Source worktree | `C:\Users\user\.codex\worktrees\operational-spine-ps-r1\Codex Orchestrator` |
| Branch | `codex/operational-spine-ps-r1` |
| HEAD | `b964509f2d979eb26e1181c796ac88c95f485647` - `Bind planning request to Sprint Git authority` |
| Dirty scope | Five modified tracked Rust files; no untracked source files reported |
| Raw diff | 1,733 insertions, 353 deletions |
| Research tip used for comparison | `9240364` - `Bind managed sessions to ready native profiles` |
| Relationship to tip | `b964509` is the merge base; the tip is 254 commits ahead |

The raw line count substantially overstates the behavioral change. Formatting-normalized comparisons are byte-equivalent for `active_app.rs`, `bootstrap_transition.rs`, and `conversation_harness.rs`. The material implementation is concentrated in:

- `src-tauri/src/agent_sessions/application/lifecycle.rs`;
- `src-tauri/src/orchestration/sprint_runner_transition.rs`.

## Short interpretation

This is an early draft of the boundary between an authorized Work Slice Planner request and an actually launched Planner.

The prior HEAD stores an identity-free planning request and its application-derived child identities, but deliberately creates no child Agent Session. The uncommitted draft advances one step: it creates the stable child Session, persists an application-owned pending invocation, snapshots the selected Work Slice Planner Harness, and makes those effects restart-reconcilable. It still does not preflight or launch a runtime.

That direction subsequently appears in committed form at `8e414e4` (`Materialize durable Work Slice Planner prelaunch state`), then gains pinned-Harness recovery at `729312e`, launch behavior at `adf4a52`, an explicit launch boundary at `51fedb8`, and several stage-truthfulness corrections before `95fa366`. The uncommitted worktree is therefore best classified as a **precursor and alternative draft of an early cumulative slice**, but **superseded and radically incomplete** relative to `9240364`.

## Material behavior delta

### A reusable pending-invocation preparation seam

`agent_sessions/application/lifecycle.rs:285-359` adds `prepare_idempotent_application_invocation`.

The method persists an invocation without runtime preflight or launch. Its contract is narrower than the ordinary send-and-launch path:

- submitted text must be non-empty;
- an existing, available Agent Session is mandatory;
- requested runtime options come from the command or fall back to the Session;
- the invocation is application-provenance and `pending`;
- effective options, start evidence, terminal evidence, exit code and runtime diagnostics remain absent;
- replay is accepted only when Session, submitted text, provenance and requested options match exactly;
- reuse of the invocation ID for different semantics fails with a conflict.

This is a meaningful application lifecycle primitive, not Work Slice Planner-specific configuration. It separates **durable preparation authority** from **runtime launch authority**, allowing a role owner to establish an exact invocation before crossing the later launch boundary.

### The planning request now triggers prelaunch reconciliation

`sprint_runner_transition.rs:1549-1639` retains the existing identity-free MCP-facing request shape: the caller supplies `{}` and the application derives the Sprint, planning point, request fact, parent identities, Git authority, worktree route, Harness identity, child Session ID and child invocation ID.

The authority checks remain substantive:

- the caller must be the exact planning-control invocation attached to the Sprint;
- planning must already be ready and its planning-control Harness applied and launch-accepted;
- one current initiated-Sprint Git authority supplies the repository/worktree route and object/fingerprint evidence;
- an immediate SQLite transaction serializes concurrent requests;
- replay requires every derived identity and authority field to match; drift becomes an idempotency conflict.

The behavioral change is after the request transaction commits. The method now calls `reconcile_work_slice_planner` instead of returning with intent only.

### Reconciliation materializes, but does not launch, the Planner

`sprint_runner_transition.rs:1642-1713` adds open-time and per-request reconciliation. `SprintRunnerTransitionService::open` also calls the collection reconciler at line 445, so the effect can be recovered after restart.

For each current planning request, the reconciler:

1. takes the per-Sprint transition lock;
2. loads the stable planning point, Session/invocation IDs, worktree route, Harness identity, prior stage timestamps and optional Harness JSON;
3. resolves the current `WorkSlicePlanner` Harness and fails closed if its key/version differs from the request;
4. serializes the Harness and rejects a conflicting previously stored snapshot;
5. idempotently creates an application-owned Agent Session titled `Work Slice Planner`, rooted at the Git-authority worktree and carrying the Harness runtime options;
6. records `planner_session_created_at`;
7. idempotently persists a pending application invocation with a prelaunch prompt that explicitly says not to begin until a later launch boundary;
8. records `planner_invocation_created_at`, `planner_harness_applied_at`, and `planner_harness_json`.

No runtime adapter call, runtime preflight, MCP server injection, provider activity, or launch acceptance occurs in this draft. The read model explicitly projects `work_slice_planner_launch_accepted_at: None` at line 892.

## State and authority seams exposed by the draft

| Seam | Durable fact in this snapshot | What it does **not** prove |
| --- | --- | --- |
| Planning request | Exact parent planning-control identity plus derived planning point/request IDs | Planner creation or execution |
| Sprint Git authority | Repository/worktree identity, baseline/current object IDs and source fingerprint copied into the request | That the worktree remains current later |
| Planner configuration binding | Harness key/version and serialized Harness JSON | Runtime application of prompt, CLI arguments or MCP tools |
| Child Session materialization | Stable Session exists at the authority-selected worktree | Provider readiness or process launch |
| Child invocation materialization | Stable application-owned invocation exists in `pending` state | Preflight, launch acceptance, receiver activation or semantic work |
| Reconciliation | Request/open replay can recreate missing durable child effects idempotently | Completion of any later workflow stage |

This is a strong example of application functionality and configuration becoming one lifecycle contract. The application chooses a code-defined Conversation Harness profile; the request pins its identity and JSON; the Harness supplies runtime options; the orchestration service owns when that configuration becomes durable. The draft does not yet supply the Harness's MCP/runtime injection, so the stored `planner_harness_applied_at` name is stronger than the observable runtime fact. At most it proves the Harness was selected and attached to prelaunch materialization.

## MCP, Tauri, and frontend implications

The existing `request_work_slice_planner` MCP tool remains the entry point. Its external input remains identity-free and its response continues to report either `work_slice_planner_authorized` or launch acceptance, with guidance that launch is not provider activation or downstream acceptance. Because launch acceptance is always absent here, the response stays at `authorized` even though a Session and pending invocation now exist.

No new Tauri command, frontend operation, or frontend component is introduced by this diff. `active_app.rs` is formatting-only after normalization. The change is below the frontend boundary: an already-existing MCP action now produces additional durable Agent Session state. Any UI that interprets `downstream_not_started` or the presence of child IDs needs care:

- `downstream_not_started` becomes false as soon as the planning request fact exists, even though the child invocation is only pending;
- child IDs already existed as planned identities before materialization, so their presence alone does not distinguish intent from creation;
- the newly projected `work_slice_planner_invocation_created_at` is the useful discriminator in this draft;
- launch acceptance remains a separate and absent fact.

## Embedded validation and contradictory evidence

### What the retained tests establish

The existing test `work_slice_planning_request_is_identity_free_durable_and_has_no_child_effects` in `bootstrap_transition.rs` still contains useful evidence for the prior boundary:

- the MCP input cannot forge a Sprint ID;
- a foreign planning-control invocation is rejected;
- missing Git authority is rejected;
- concurrent requests converge on one request, planning point, child Session ID and invocation ID;
- Git-authority ambiguity and later route drift fail closed;
- no Work Unit/execution tables exist at this stage.

### Why it does not validate this uncommitted snapshot

The same test still asserts that `work_slice_planner_session_created_at` and `work_slice_planner_harness_applied_at` are absent and that no matching Agent Session or invocation exists. Those expectations were not updated when the new reconciler was added.

A focused run used an isolated Cargo target outside the source worktree:

```text
cargo test --manifest-path src-tauri/Cargo.toml \
  work_slice_planning_request_is_identity_free_durable_and_has_no_child_effects \
  --lib -- --nocapture
```

Result: **1 failed, 0 passed, 270 filtered out**. The first failure is at `bootstrap_transition.rs:4512`: the test expected `work_slice_planner_session_created_at == None`, but the implementation had recorded a timestamp.

An earlier command used `--exact` without the fully qualified module name and ran zero tests; it is not counted as validation.

The compile succeeded apart from an unrelated dead-code warning. This proves the draft builds far enough to run the focused test, but the retained acceptance evidence and the implementation are not converged.

At committed descendant `8e414e4`, the test is renamed to `work_slice_planning_request_materializes_one_prelaunch_planner_without_runtime_effects` and expects:

- one created Planner Session;
- one application-owned pending Planner invocation;
- creation, invocation and Harness timestamps;
- no runtime request and no launch acceptance;
- stable facts after reopen.

That descendant test better expresses the intended prelaunch boundary than the dirty worktree's retained test.

## Internal inconsistencies and incompleteness markers

These are reasons to preserve the snapshot as research evidence but not treat it as shippable product state:

- The focused relevant test fails against the new behavior.
- The method comment still says the request "persists intent only" and that child creation/Harness application belong to later steps, while the method immediately invokes those effects.
- `planner_harness_applied_at` is recorded without runtime Harness/MCP injection.
- Previously loaded stage timestamps are discarded; idempotency comes from child repository semantics and `COALESCE`, not an explicit stage machine.
- Harness recovery depends on the current catalog profile matching the stored key/version/JSON. It cannot reconstruct a historical profile after the catalog changes. Commit `729312e` replaces this with pinned-snapshot recovery.
- No launch-requested, launch-accepted, ready, provider-active, lifecycle or semantic-result boundary exists for the Planner in this draft.
- The planning point and child identities are derived only from the Sprint and `planning_episode` is fixed to `1`; the later repeated temporal planning model is absent.
- No Work Slice proposal, acceptance, Work Unit materialization, Handler/Implementer route, review, retry, integration, dependency wave, handback, escalation or graph-completion behavior exists here.
- Native Profile selection, readiness and managed-session binding - the subject of `9240364` - are entirely later work.

## Historical placement

The first relevant descendants of `b964509` are unusually informative because they show the seam being corrected in small increments:

| Commit | What it adds or corrects beyond this dirty draft |
| --- | --- |
| `8e414e4` | Commits the durable prelaunch materialization boundary and converges the test expectation |
| `729312e` | Recovers the exact pinned Harness snapshot rather than depending only on the mutable catalog |
| `adf4a52` | Adds launch of the prepared invocation and launch evidence handling |
| `51fedb8` | Reframes the prompt and behavior as the actual Planner launch boundary |
| `a3126c0` | Projects the launch boundary |
| `ce16bad` | Corrects Planner stage disclosure |
| `a1fd69b` | Uses durable Planner creation evidence |
| `95fa366` | Converges request-stage behavior |
| `6198cbb` | Begins Work Slice plan acceptance |
| `9240364` | Much later tip after the operational spine and native-profile binding work |

The dirty worktree is not a parallel alternative to the full tip. Its HEAD is the direct ancestor immediately before this sequence, and its material ideas were incorporated, corrected and extended by descendants. The exact dirty implementation was not simply committed: the first descendant already differs in recovery structure, stage helper use and test contract.

## Product and architecture reading

From a product-owner perspective, the draft isolates a useful promise boundary: "the product has prepared the exact Planner work context" is distinct from "a Planner is running." That distinction supports truthful waiting, recovery and oversight states.

From a product-architecture perspective, it reveals three authorities that should remain separately inspectable:

1. the Sprint's current planning-control authority;
2. the Git/worktree authority for the planning episode;
3. the role configuration pinned to the child Session/invocation.

From a developer perspective, the reusable lifecycle method is the durable primitive; the Sprint transition service is the role-specific orchestrator. The key technical liabilities in this draft are the mutable-profile recovery dependency, stage names that outrun their evidence, and tests that still encode the superseded boundary.

From a designer perspective, the state should not collapse into a single "started" badge. A later visualization can distinguish at least: request authorized -> Session created -> invocation prepared -> launch requested -> launch accepted -> provider active -> Planner ready -> semantic outcome. This snapshot directly evidences only the first three.

## Evidence index

| Artifact | Material evidence |
| --- | --- |
| `src-tauri/src/agent_sessions/application/lifecycle.rs:285-359` | Reusable pending application-invocation preparation and exact replay contract |
| `src-tauri/src/orchestration/sprint_runner_transition.rs:115-139` | Planning request schema, stage fields and Harness JSON |
| `src-tauri/src/orchestration/sprint_runner_transition.rs:445` | Open-time Planner reconciliation |
| `src-tauri/src/orchestration/sprint_runner_transition.rs:845-893` | Read-model projection; launch acceptance remains hardcoded absent |
| `src-tauri/src/orchestration/sprint_runner_transition.rs:1549-1639` | Identity-free request, Git authority snapshot, concurrency and replay boundary |
| `src-tauri/src/orchestration/sprint_runner_transition.rs:1642-1713` | Prelaunch Session/invocation/Harness materialization |
| `src-tauri/src/orchestration/bootstrap_transition.rs:4382-4618` | Retained prior-boundary test and conflicting assertions |
| `8e414e4` through `95fa366` | Immediate committed evolution of the same seam |
| `9240364` | Comparison tip, 254 commits beyond the dirty worktree HEAD |

## Confidence and limits

High confidence:

- exact dirty file set, HEAD and ancestry relationship;
- formatting-only classification of three files after `rustfmt` normalization;
- durable pending-invocation and Planner materialization behavior;
- absence of runtime launch in this draft;
- focused test failure and its first contradictory assertion;
- precursor/superseded/incomplete classification.

Not claimed:

- that any uncommitted artifact was accepted, shipped or exercised through a packaged application;
- that the whole test suite passes or fails beyond the focused test;
- that file modification timestamps represent authorship order;
- that the current branch will retain this dirty state.
