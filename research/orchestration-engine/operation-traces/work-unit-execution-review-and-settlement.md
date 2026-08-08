# Operation trace: Work Unit execution, review and settlement

This trace follows the productive later-stage path from an accepted Work Slice plan to a settled Work Unit. It is intentionally explicit about claims, evidence, lifecycle observations and settlement because the implementation treats them as different facts.

## Participating components

| Responsibility | Principal artifact |
| --- | --- |
| Sprint execution coordinator and MCP endpoints | `src-tauri/src/orchestration/sprint_runner_transition.rs` |
| Exact role/attempt capability construction | `work_unit_execution_harness.rs` |
| Authorized isolated workspaces | `execution_support.rs` |
| Agent Session lifecycle | `agent_sessions/application/` |
| Harness configuration and immutable revisions | `conversation_harness.rs`, `conversation_harness_*`, catalogue JSON |
| Candidate pinning | `accepted_candidate_authority.rs` |
| Target integration and Work Unit settlement | `accepted_integration.rs` |
| Dependency activation and graph settlement | `work_unit_dependency_wave.rs` |
| Frontend projection | orchestration native query and Work Unit/Sprint views |

## Lifecycle

### 1. Accepted planning becomes an executable graph

`complete_work_slice_planning` validates the current planning episode and accepted proposal revision. The application materializes:

- one `work_unit_materializations` record;
- ordered `work_units`;
- explicit `work_unit_relationships`.

`reconcile_work_unit_dependency_wave` evaluates exact graph edges and settled prerequisite contributions. Eligibility and activation intent are recorded in one immediate SQLite transaction before any Handler effect occurs. Missing prerequisite contributions remain a waiting condition; graph corruption becomes structured attention.

### 2. The application activates a Handler

For an eligible Work Unit, the Sprint transition service resolves:

- initiated Sprint Git authority;
- an exact application-owned attempt authorization;
- an isolated execution workspace;
- the original pinned Handler Harness revision;
- deterministic Session and invocation identities.

The original Handler revision is deliberately actionless. After it completes, the application prepares a same-Session Handler action continuation using a later immutable Harness revision that exposes only:

- `request_work_unit_implementer`.

The MCP call records a request; its success response explicitly does not claim an Implementer outcome or later acceptance.

### 3. The application launches the Implementer

`request_work_unit_implementer_inner` revalidates the exact live Handler continuation and durable correlations. The Work Unit Harness service resolves the Implementer role against the same authorized attempt and workspace authority.

The original Implementer Harness is actionless. It receives the fixed Work Unit specification in the isolated workspace and performs the code change. The application records preparation, Harness binding, launch request, launch acceptance and ready state separately.

### 4. Completed code work becomes a candidate

Agent Session completion alone is insufficient. Reconciliation subsequently:

- verifies the original Implementer invocation completed;
- seals/commits the exact authorized workspace;
- reloads the pinned original Harness revision;
- rebinds the exact Session and invocation correlation;
- requires a non-empty changed-file manifest and comparison;
- verifies bounded evidence content for each changed file.

Absent changes, revalidation failure and evidence unavailability are recorded as distinct failure reasons.

### 5. A separate reporting continuation submits claims

The application publishes or reloads a later immutable Implementer reporting revision and starts a deterministic continuation in the same Session. Its only tools are:

- `submit_implementation_outcome`;
- `complete_implementation_outcome`.

The first tool accepts a `ReviewPending` summary and validation statement. These are stored as agent claims with a payload fingerprint. The second tool first captures application-owned file evidence, then records semantic completion only if the claims are valid and evidence is ready.

Tool success still does not mean application acceptance, Handler acceptance, integration or Work Unit settlement.

### 6. The application accepts the bounded reporting package

Reconciliation observes the reporting invocation terminal lifecycle separately. A valid accepted package requires the exact original attempt, reporting invocation, semantic completion, captured evidence and expected Harness revision. The application then marks the outcome ready for Handler review.

### 7. A separate Handler review continuation judges the outcome

The application constructs a read-only review payload from:

- submitted summary and validation statement;
- changed-file manifest;
- comparison fingerprint;
- evidence-content fingerprints.

It persists the exact delivered payload and launches a deterministic continuation in the original Handler Session with a third pinned Handler revision. Its tools are:

- `read_handler_review_evidence`;
- `accept_implementation_outcome`;
- `return_implementation_outcome`.

The read tool revalidates the current application-owned evidence against the delivered payload. An accept tool call records a judgment but remains pending until that exact review invocation is observed `Completed`. A return records a structured incomplete disposition with code, explanation, classification and meaningful-progress fact; it does not itself launch a retry or contact an upward receiver.

### 8A. Accepted work becomes an application-owned candidate

After an accepted review is finalized, candidate authority revalidates:

- Work Unit, attempt and Handler decision correlations;
- Git repository/common-directory/worktree identity;
- baseline and candidate commit/tree;
- captured File Review evidence.

The candidate is pinned under `refs/codex/orchestrator/accepted/<candidate-id>`. Database intent precedes the compare-and-swap `git update-ref` effect, allowing restart reconciliation without treating an arbitrary matching ref as authority.

### 8B. Returned work becomes retry or handback state

Returned work is processed only after the review lifecycle is observed. Meaningful-progress and classification facts drive either a bounded retry attempt or a no-progress handback. Retry creation, baseline/ref authority, new Implementer launch and later outcome remain separately recorded stages.

### 9. Accepted integration settles the Work Unit

`reconcile_accepted_integrations` serializes target updates and coordinates multiple durable and Git stages:

1. reserve exact integration intent against the target-current version;
2. create or verify the integration commit/tree;
3. advance the target ref with compare-and-swap semantics;
4. advance the durable target-current record;
5. verify clean runtime and object relationships;
6. record immutable integration evidence;
7. insert `work_unit_settlements`;
8. insert one prerequisite contribution for each outgoing dependency edge.

Lock contention remains pending; detected drift or divergent replay becomes structured integration attention. Reconciliation can adopt its own partially completed stages after restart.

### 10. Dependency and Work Slice settlement advance

The dependency wave recomputes eligibility from canonical edges plus exact prerequisite contributions. Dependents become eligible only through those settlement facts—not through Handler readiness, provider activity, transcript content or silence.

When every canonical unit has a coherent integration settlement and every edge has its contribution, `reconcile_work_slice_execution_settlement` records three terminal levels:

- graph completion;
- Work Slice execution settlement;
- planning-point execution settlement.

Graph-wide or unit-specific terminal problems are retained as structured attention instead of being collapsed into “complete.”

## Control and evidence boundaries

| Fact | What it proves | What it does not prove |
| --- | --- | --- |
| Handler/Implementer launch accepted | process launch was accepted for the exact invocation | provider activity or semantic work |
| original Implementer completed | Agent Session lifecycle reached completed | valid code candidate or evidence |
| reporting claims submitted | agent supplied bounded summary/validation claims | truth of those claims |
| evidence captured | application can reproduce the bounded comparison/files | Handler acceptance |
| Handler accept tool called | exact review invocation recorded an accept judgment | finalized acceptance until lifecycle completes |
| candidate pinned | exact accepted commit/tree is retained under private authority | target integration |
| integration ref advanced | Git target points to the application-created integration commit | durable settlement until evidence/state are committed |
| Work Unit settled | integration evidence and Work Unit settlement are durable | dependent activation until contribution reconciliation |
| graph settled | every unit and dependency contribution is coherent | Sprint or Epic settlement on divergent later lines |

## Configuration behavior embedded in the trace

The logical roles “Handler” and “Implementer” each have multiple immutable runtime configurations:

- original actionless Handler;
- Handler action continuation;
- Handler review continuation;
- original actionless Implementer;
- Implementer reporting continuation.

Those revisions are created in Rust from the compiled role profile and stored in the Harness working-copy/revision system. The distinction is productive policy, not merely a test fixture, but it is not obvious from the ten-role static catalogue alone.

## Product/experience reading

- The user-visible Work Unit is backed by several agent invocations and two role-specific continuation Sessions, not one linear “agent did the task” event.
- Review is structurally independent from implementation claims; the frontend should avoid collapsing reporting, evidence readiness and acceptance into a single status.
- Retry and upward handback are policy outcomes of structured review, not generic failures.
- The current UI receives this through a large native projection. Future visualizations could expose the chain as a compact evidence ladder with drill-down rather than mirroring every backend row.

## Segmentation candidates to investigate

- extract MCP host/transport construction from Sprint lifecycle semantics;
- separate transition state machine from SQL repository and native-query projection;
- model Handler/Implementer continuations as explicit Harness variants rather than hidden code-generated revisions;
- centralize the common “persist, bind, request launch, observe acceptance, reconcile terminal” invocation protocol;
- preserve candidate/integration as one cross-store consistency boundary even if modules move.
