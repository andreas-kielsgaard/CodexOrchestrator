# Durable state and artifact ownership

This catalogue maps what the application treats as durable truth, where that truth is stored, and where a durable record is coupled to a filesystem or Git effect. It is not a database reference manual. The emphasis is on product and architectural ownership.

## State authority at a glance

| Store or artifact root | Product role | Runtime reachability | Primary implementation |
| --- | --- | --- | --- |
| `codex-orchestrator-active-v3.sqlite` | Agent Sessions, Epic planning and initiation, bootstrap, Sprint execution, Harness revisions, Git authority, native profiles | Normal application startup | `src-tauri/src/storage.rs`, feature schema modules |
| `harness-revisions/` | Immutable, content-addressed Harness configuration objects and commit manifests | Normal application composition; mutation is not yet exposed by the mounted frontend | `src-tauri/src/orchestration/conversation_harness_revision.rs` |
| `orchestration-materials/` | Approved-plan input, bootstrap manifest, Epic overview, Runner brief | Created after confirmed Epic initiation | `src-tauri/src/orchestration/bootstrap_transition.rs` |
| `execution-workspaces/` or a correlated `.co-exec/` sibling | Application-owned Handler and Implementer workspaces | Created for authorized Work Unit attempts | `src-tauri/src/orchestration/execution_support.rs` |
| target repository Git refs and objects | Accepted candidate pinning, retry baselines, serialized target integration | Later-stage orchestration execution | `accepted_candidate_authority.rs`, `accepted_integration.rs`, `sprint_runner_transition.rs` |
| `native-codex-homes/` | Application-dedicated `CODEX_HOME` profiles | Native profile settings and launches | `src-tauri/src/native_profiles.rs` |
| review runtime root | Human-review instance registry, launcher history, isolated builds, caches and authority secret | Debug application only | `src-tauri/src/worktree_review/`, `src-tauri/src/worktree_runtime/` |
| `codex-orchestrator.sqlite` | Archived task/run architecture | Legacy commands are compiled but fail closed | `src-tauri/src/lib.rs` |

## Active database lifecycle

`src-tauri/src/active_app.rs` resolves the Tauri application-data directory, creates it, and opens `codex-orchestrator-active-v3.sqlite`. `src-tauri/src/storage.rs` owns the common connection policy:

- foreign keys enabled;
- write-ahead logging;
- `FULL` synchronous commits;
- a five-second busy timeout;
- schema version `35`.

The old `codex-orchestrator.sqlite` and `codex-orchestrator-active-v2.sqlite` files are deliberately not imported. Versions 1 through 34 of the active-v3 schema are migrated in place; other non-zero versions fail closed.

Some schemas are initialized centrally when the database is created. Bootstrap and Sprint Runner services also initialize or evolve their own schemas when those services are opened. “One database” therefore does not mean “one schema owner”: schema creation and migration responsibility is distributed across the feature modules.

The production schema declarations collectively name approximately 95 current tables after the lazily initialized Sprint execution schemas are included. Temporary migration tables and test-fixture-only tables are excluded from that count.

## Logical state domains

| Domain | Durable facts | Principal tables | Schema owner |
| --- | --- | --- | --- |
| Agent Sessions | session identity, invocation request and lifecycle, launch acceptance, raw/normalized runtime events, diagnostics | `agent_sessions`, `agent_session_invocations`, `agent_session_invocation_launch_acceptances`, `agent_session_runtime_events`, `agent_session_invocation_diagnostics` | `agent_sessions/repository/schema.rs` |
| Plan Builder draft | draft lifecycle, Session association, capability assignment, proposal commands, provenance, immutable revisions and results | `epic_planning_drafts`, `planning_draft_lifecycle_events`, `planning_draft_agent_session_associations`, `planning_draft_profile_assignments`, `capability_profiles`, `proposal_*`, `effect_provenance` | `orchestration/repository.rs` |
| Epic initiation | confirmation command/result/event/provenance chain, exact proposal snapshot, initiated Epic and Sprint facts | `epic_initiation_*`, `epic_initiations`, `initiated_planning_drafts`, `initiated_sprints` | `orchestration/repository.rs` |
| Plan Builder continuation | one-shot, claimed and consumed application context delivery to an exact invocation | `plan_builder_context_deliveries` | `orchestration/repository.rs` |
| File Review | producer-owned review document, normalized changed-file membership, bounded serialized artifact, Git-capture authorization and linkage | `file_review_documents`, `file_review_changed_files`, `stored_file_review_artifacts`, `file_review_git_capture_authorizations`, `file_review_git_capture_documents` | `orchestration/repository.rs` |
| Initiated Sprint Git authority | verified repository/worktree identity, baseline/current objects, source and runtime correlations | `initiated_sprint_git_authorities` | `orchestration/repository.rs` and `initiated_sprint_git_authority.rs` |
| Epic bootstrap | preparation, Runner activation, completion command/result/fact chains, repeated attempts and recovery | `epic_bootstrap_transitions`, `epic_bootstrap_attempts`, `epic_bootstrap_*completion*` | `orchestration/bootstrap_transition.rs` |
| Sprint planning | Runner transition, planning episodes and requests, proposal revisions, accepted materialization | `sprint_runner_transitions`, `work_slice_planning_requests`, `work_slice_planning_episodes`, `work_slice_proposal_revisions`, `work_unit_materializations` | `orchestration/sprint_runner_transition.rs` |
| Work Unit graph | units, dependency relationships, activation intents, unit/graph execution state, settlements and attention | `work_units`, `work_unit_relationships`, `work_unit_dependency_activation_intents`, `work_unit_execution_states`, `work_slice_execution_*`, `work_unit_execution_attentions` | `sprint_runner_transition.rs`, `work_unit_dependency_wave.rs` |
| Handler/Implementer execution | Handler activation and continuation, Implementer attempts/outcomes, Handler review and decision, incomplete/no-progress/retry facts | `work_unit_handler_*`, `work_unit_implementer_*`, `work_unit_no_progress_handbacks`, `work_unit_retry_attempts` | `orchestration/sprint_runner_transition.rs` |
| Sprint-to-Epic handback | delivery, disposition, escalation receiver, reassessment, downstream request and attention | `sprint_runner_handback_*`, `epic_runner_escalation_*` | `orchestration/sprint_runner_transition.rs` |
| Attempt capability | exact role/attempt/Git authorization and opaque workspace capability grant | `execution_support_attempt_authorizations`, `execution_support_grants` | `orchestration/execution_support.rs` |
| Candidate and integration authority | accepted candidate pin, target current, attention, integration intent/evidence, Work Unit settlement, prerequisite contribution | `accepted_handler_candidates`, `sprint_target_currents`, `accepted_*attentions`, `accepted_work_unit_integrations`, `accepted_work_unit_integration_evidence`, `work_unit_settlements`, `work_unit_prerequisite_contributions` | `accepted_candidate_authority.rs`, `accepted_integration.rs` |
| Harness authoring | mutable effective working copy and idempotent commands; immutable revision lineage, local publication evidence and commands | `harness_working_copies`, `harness_working_copy_commands`, `harness_revisions`, `harness_revision_publications`, `harness_revision_commands` | `conversation_harness_working_copy.rs`, `conversation_harness_revision.rs` |
| Native profile control | profile identity/selection, readiness dimensions, attention, setup/login/canary attempts, sandbox adoption, danger authority, MCP probe lifecycle | eleven `native_codex_*` tables | `src-tauri/src/native_profiles.rs` |

## Important coupling patterns

### Database intent plus external effect

Several later orchestration stages are deliberately not SQLite-only:

- accepted candidates are pinned under `refs/codex/orchestrator/accepted/<candidate-id>`;
- retry baselines use private `refs/codex-orchestrator/retry/<stable-id>` refs;
- accepted integration creates a Git commit/tree and advances the target ref with compare-and-swap semantics;
- bootstrap writes exact prepared materials under the application-owned material root;
- execution support creates isolated, identity-correlated worktrees;
- native profile probes and canaries launch processes and may expect filesystem receipts.

The tables preserve intent, correlation, evidence and settlement around those effects. For future architecture work, those pairs should be reviewed as one consistency boundary rather than treating the SQL record or filesystem effect in isolation.

### Event history plus current projection

The design frequently keeps both append-like evidence and a current record: invocation events alongside invocation status, proposal commands/events alongside current revision, target-current rows alongside integration evidence, and attention tables alongside lifecycle rows. This supports replay and truthfulness, but increases the number of state transitions that must remain mutually consistent.

### One database, many module-owned migrations

`storage.rs` is the database bootstrapper, but it directly imports schema constants from Agent Sessions, orchestration subdomains, accepted integration, dependency waves and native profiles. The Sprint Runner and bootstrap repositories then perform additional lazy initialization and compatibility evolution. The resulting boundary is application-wide persistence with feature-local schema authorship, not a fully centralized storage layer.

## Non-SQL durable artifacts

### Harness revision repository

Despite using “commit” terminology, `harness-revisions/` is not a Git repository. It is an application-owned content-addressed store:

- configuration objects live under `objects/sha256/<prefix>/<digest>`;
- immutable commit manifests live under `commits/<revision-id>.json`;
- SQLite stores the revision lineage, digest and local commit reference.

The compiled Harness catalogue remains a separate source of effective configuration. This creates three configuration representations to compare later: compiled catalogue JSON, code-authored revisions, and durable working-copy/revision records.

### Bootstrap materials

Confirmed initiation prepares a contained Epic directory with an exact approved-plan JSON input and transition manifest. Completion adds the Epic overview and Runner brief. SQLite stores paths, hashes, inventory and transition facts; the files are the actual material passed onward.

### Execution workspaces and Git authority

Execution workspaces are derived from a durable attempt authorization and initiated Sprint Git authority. The implementation validates canonical paths, symlink boundaries, baseline object identity and workspace fingerprints. A workspace is therefore not merely a temp directory; it is one half of a capability contract whose other half is stored in SQLite.

### Native profile homes

Application-dedicated profiles live under `native-codex-homes/`; registered profiles may point elsewhere. The database stores canonical path and filesystem identity separately from readiness, authorization and observed attempts. Files in `CODEX_HOME` remain provider-owned input/state and should not be mistaken for application-owned proof.

## Debug review persistence

Debug builds compose a separate review runtime root. It contains:

- `registry.sqlite`: Worktree Runtime instances, port leases, commands and observations;
- `launcher.sqlite`: review sessions and review history;
- `authority.secret`: local authority token for review-runtime operations;
- `instances/`: isolated instance roots and application data;
- `shared-cache/npm/`: keyed shared Node cache;
- instance-local Cargo home/target, credentials, temp and build outputs.

These stores are operational developer tooling. They are not part of the normal release composition and should be analyzed separately from product orchestration durability even though debug File Review uses the runtime as a verified Git-comparison source.

## Legacy and quarantine state

`src-tauri/src/lib.rs` still contains the original task/run database and commands using `codex-orchestrator.sqlite`. The active application registers those commands but rejects them through the legacy availability guard. Agent Session schema code also recognizes and renames an older prototype `agent_sessions`/`agent_session_cli_logs` shape to `archived_prototype_*_008` before installing the current schema.

This gives two different retirement strategies in the same crate:

- old task functionality remains compiled behind fail-closed commands and an untouched legacy database name;
- an incompatible Agent Session prototype is detected and quarantined inside a database before the current schema is installed.

## Architectural reading

- Durable identity, provenance, idempotency and attention are first-class product concepts, not incidental implementation details.
- The state model distinguishes request, launch acceptance, runtime/provider observation, semantic completion and downstream settlement. That distinction should remain visible in future product language.
- The active database is a shared coordination substrate for several state machines. Module boundaries in Rust do not currently translate into persistence ownership boundaries.
- Harness configuration is simultaneously compiled configuration, durable authored state and immutable local artifact storage. It is a prime candidate for an explicit configuration architecture review.
- Debug review infrastructure has its own strong durability model and process authority. Its proximity to product File Review makes the release/debug boundary especially important.

## Useful future visualizations

The collected shape naturally supports:

- an authority map connecting SQL intent to filesystem, process and Git effects;
- a lifecycle ribbon showing request, acceptance, observation, completion and settlement as separate facts;
- a data-domain map of the 95-table active store grouped by owning state machine;
- a configuration provenance diagram for compiled Harness catalogue, code revisions, working copies and immutable objects;
- a storage boundary diagram separating product state, provider-owned profile state, target-repository state and debug-review state.

These are presentation opportunities, not constraints on the ongoing inventory.

## Open questions

- Should schema lifecycle become centrally versioned even if repository access remains feature-owned?
- Which current tables are part of the intended long-lived product domain, and which encode temporary proof gates or migration stages?
- Is the local Harness revision repository intended to replace the compiled catalogue, complement it, or preserve authored history only?
- Which bootstrap and execution-workspace artifacts have retention or cleanup policy, and where is that policy expressed?
- Should debug File Review keep depending on Worktree Review as its Git evidence provider in the long term?
- Is there an intended migration or archival policy for `codex-orchestrator.sqlite`, active-v2, and quarantined prototype tables?
