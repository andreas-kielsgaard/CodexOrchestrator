# Durable Worktree Review correction

Status: implemented candidate; publication, integration, and acceptance remain separate.

Baseline: `c998c34645847b618ff08354988a219fd00d2d9e` on
`codex/worktree-review-runtime`.

Correction branch: `codex/worktree-review-durable-architecture`.

## Objective

Make Worktree Review a complete product capability in every build profile while replacing the old
launcher/runtime split with a branch-first implementation whose responsibilities are explicit.
Favor the smallest happy-flow capability: choose a branch and exact worktree or commit, compile in
a physical worktree, retain the output in AppData, and open it.

The correction is complete when:

- repository, branch, physical worktree, source selection, build workspace, attempt, output, and
  cleanup records have distinct typed identities;
- every registered Worktree Review command is available in development and release;
- a branch is selected before an exact associated worktree, with multiple worktrees per branch or
  commit remaining distinct;
- direct, worktree-snapshot, and branch-commit sources are explicit;
- Git worktree creation is preceded by a durable workspace plan;
- build output is retained in shared product AppData and is never copied into a source worktree;
- a built application can be opened, preferring an existing window for the exact executable;
- no process status, stop, recovery, parent-child build relation, or launcher authority model is
  introduced;
- a reviewed application exposes the same Worktree Review product as any other application;
- the active build's exact source worktree is identified and excluded from direct/snapshot rebuild
  choices, while all other worktrees remain available; and
- default branch reads show current direction and tip facts without loading history.

## Product decisions

- Worktree Review is profile-independent. Proof-only or partial alternate implementations are not
  a product boundary and are removed.
- Select repository, branch, then exact worktree. Detached checkouts participate only after the
  user associates them with a branch.
- Explicit Create Worktree and Create Build use the same provisioning and association concepts.
  Create Build may create its disclosed checkout automatically.
- A source binding records what triggered a build. It is not a continuing guarantee about the
  source worktree or the quality of the resulting application.
- Opening a build requires retained output whose executable still exists. Open asks the operating
  system to focus an existing window for that exact executable when possible; otherwise it starts
  the executable. Closing belongs to the application itself.
- Builds and opened applications are independent. There is no owner instance, recursive-launcher
  restriction, status registry, stop operation, or recovery operation.
- Product-created build worktrees are retained. Borrowed worktrees are never removed by build
  retention cleanup.
- Retained build outputs, build logs, settings, cleanup ledgers, and receipts live in a shared
  Worktree Review AppData root. No output is copied back to a source worktree.
- Existing ambiguous legacy data is not assigned invented source or ownership authority.
- Branch application metadata has no current decision value and is not part of the model or UI.
- Historical branch search is outside this correction.

## Architecture

```text
React Worktree Review
  -> unconditional Tauri commands
  -> branch/build/cleanup application services
  -> Worktree Review domain and focused SQLite repositories
  -> repository context + reusable physical-worktree application

ReviewBuildExecutor
  -> PhysicalWorktreeApplication
       capture_virtual_commit(worktree root, exact HEAD)
       materialize_checkout(repository, target, exact commit, attachment)
       build(worktree root, attempt root, dependency cache, binary)
       open(build result, environment overlay)
```

`worktree_application` is a reusable domain outside Worktree Review. It deliberately contains only:

- stable virtual-commit capture without changing the user's index, refs, or checkout;
- exact physical checkout materialization and idempotent adoption;
- dependency preparation and compilation into an explicit attempt root;
- a bounded generic launch environment; and
- focus-existing-window-or-launch behavior.

It does not know about review builds, branches, associations, AppData layout, retention, cleanup,
process health, stop, recovery, or authority. Worktree Review supplies those feature-specific facts
and consumes the component.

Worktree Review owns:

- `WorktreeAssociation`: exact repository, branch, physical worktree, baseline, current observed Git
  state, provenance, and availability;
- `ReviewBuild`: immutable trigger-time source binding, workspace, retention key, and current output
  pointer;
- `ReviewOperationAttempt`: materialization/build execution state, verdict, failure, and timestamps;
- retained-output location and executable facts used to avoid presenting missing output as
  openable; and
- cleanup jobs, effects, receipts, and attention records.

An available output means only that its durable record and contained executable currently exist. It
makes no source-quality or runtime-quality claim.

## Responsibility consolidation

- `RepositoryContext` owns hardened, bounded, read-only Git facts.
- `PhysicalWorktreeApplication` is the single physical capture/checkout/build/open effect boundary
  used by Worktree Review, orchestration worktree creation, and task worktree creation.
- `association_observer` is the single constructor for observed association state, used by branch
  association, explicit creation, and build-created managed worktrees.
- `SourceMaterializationService` accepts the minimal source request, records the server-observed
  source receipt, and turns a durable workspace plan into an exact checkout through the reusable
  boundary.
- `ReviewBuildCoordinator` persists build/materialization intent before effects, coordinates build
  output, presents results, and performs stateless Open.
- `WorktreeReviewCleanupService` owns cleanup planning, effects, receipts, and restart reconciliation.
- projection modules own transport DTOs; coordinators do not duplicate presentation mapping.
- the old Human Review launcher and `worktree_runtime` implementation are deleted instead of
  remaining as a second authority path.

## Branch reads and UI

The default branch overview performs one batched `for-each-ref` query for local branch tip metadata
and direction relative to the remote default branch. It does not construct a historical view.

Branch detail presents the branch, exact worktrees, association candidates, and builds. Application
metadata is intentionally absent because it has no current decision value. The existing lazy branch
history path is unchanged; historical search is outside this correction. Git-heavy Tauri commands
run on blocking workers rather than the UI-facing async executor.

The active build context is a pair of durable build/worktree identities passed to an opened build.
It changes only source-choice affordances for that exact worktree; it does not filter repositories,
branches, other worktrees, or Worktree Review itself.

## Debug-gate handling

- Product commands, state composition, Worktree Review navigation, build, and Open are unconditional.
- No Worktree Review source contains `debug_assertions` gates.
- The separate proof controller, harness route, child-shell view, and old debug-only launcher commands
  are removed because they duplicated product behavior and concealed release gaps.
- Tests and fixtures remain tests; build profiles do not select different product capabilities.

## Durability and cleanup

One shared SQLite database under the canonical Worktree Review AppData root owns selection,
associations, workspaces, builds, attempts, retained outputs, cleanup, settings, and attentions.
SQLite uses WAL, a bounded busy timeout, and serialized migrations for multiple independent reviewed
applications.

Every created checkout has a workspace plan stored before `git worktree add`. A crash after the Git
effect therefore leaves an identifiable planned workspace rather than an unexplained product-owned
directory. Successful creation promotes that record to ready and saves the association atomically.

Automatic retention removes only eligible AppData output/log/scratch resources. It never deletes a
build worktree. Cleanup effects are contained and idempotent, and their outcomes settle into retained
receipts.

## Structural rules

- No product behavior selected by debug profile.
- No SQL outside storage adapters.
- No raw Git reads outside repository adapters and no Git mutations outside provisioning adapters.
- No catalog, inventory, query, or projector mutates Git.
- No Tauri types below transport.
- No log or filesystem presence infers a build verdict.
- No available-output presentation without a retained output record and existing contained
  executable.
- No cleanup effect without typed ownership and containment evidence.
- No build hierarchy, process registry, stop, recovery, or child-launcher authority.
- No post-build claim that the source worktree or application remains unchanged or correct.

## Execution slices

1. Establish a shared repository context, persisted selection, typed domain, and focused storage.
2. Replace the flat launcher with branch-first contracts and exact worktree/source selection.
3. Persist workspace, build, and materialization intent before checkout effects.
4. Build and retain output through the minimal reusable physical-worktree application component.
5. Add stateless Open and propagate shared AppData plus exact active build/worktree context.
6. Keep default branch reads bounded and run Git reads off the UI-facing executor without expanding
   historical search.
7. Consolidate provisioning, association observation, projections, and Git comparison ports.
8. Remove the legacy launcher/runtime/proof surfaces and all Worktree Review debug gates.
9. Validate focused domain/storage/component behavior plus debug, release, frontend, formatting, and
   diff hygiene.

## Candidate validation

- Worktree Review Rust suite: 38 passed.
- Reusable physical-worktree application suite: 11 passed.
- The focused storage suite includes concurrent first-open migration and independent database-owner
  visibility checks.
- Development and release library checks passed; Worktree Review and physical-worktree commands are
  present in the release composition. The crate still reports unrelated and focused dead-code
  warnings, but the removed legacy runtime is no longer the warning source.
- Focused frontend suite: 20 passed across Worktree Review, the native worktree adapter, local Git
  reads, and local runtime composition.
- TypeScript production build: passed, with the existing large-chunk advisory.
- Focused ESLint, Rust formatting, and Git diff checks: passed.

## Known residuals

- Cleanup reconciliation has no cross-process claimant. Effects are idempotent and fail closed, but
  two applications can race one unsettled job and one may record a storage conflict. A future change
  should add a narrow cleanup claim or mutex, not restore application lifecycle tracking.
- Failed or interrupted build-attempt scratch/output directories are not yet enrolled in the durable
  cleanup ledger; only retained superseded outputs and their logs are automatically cleaned.
- Old per-instance Worktree Review databases and artifact roots are not inferred or merged. Any
  migration or retirement needs an explicit audited operation.
- Automated validation does not replace a packaged manual smoke test of compile, launch, exact-window
  focus, and shared-state visibility across two independently opened applications.

## Safety and publication

Preserve unrelated checkouts and dirty work. Do not merge, push, publish, retire worktrees, migrate
legacy AppData, or delete retained material without separate authority. Report candidate
implementation, validation, publication, integration, and acceptance as distinct facts.
