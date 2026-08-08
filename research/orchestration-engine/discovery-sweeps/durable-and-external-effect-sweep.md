# Signal sweep: durable and external effects

## Evidence frame

- Source: research checkout `codex/orchestration-engine-research` at `9240364`.
- Direction: begin at writes, processes, files, listeners and notifications, then work backward to their triggers.
- Method: file-level pattern discovery followed by call-site inspection. Pattern matches are orientation signals, not automatic capability classifications.
- Validation: `cargo test --manifest-path src-tauri/Cargo.toml conversation_harness_revision --lib` passed 13 tests, with 491 filtered out. The first local build took 2m26s and emitted one unrelated dead-code warning.

## Initial effect concentration

The first sweep looked for SQL mutation, child-process creation, filesystem effects, loopback/network behavior and notification/event behavior. Files appearing in several columns deserve traversal because they coordinate different consistency domains; the count does not imply product importance.

| Artifact | Signals found together |
| --- | --- |
| `orchestration/bootstrap_transition.rs` | SQL, process, filesystem, loopback/network, notification |
| `orchestration/mcp.rs` | SQL, process/thread, loopback/network, notification |
| `orchestration/application.rs` | SQL, filesystem, loopback/network, notification |
| `native_profiles.rs` | SQL, process, filesystem, loopback/network |
| `orchestration/sprint_runner_transition.rs` | SQL, process/thread, loopback/network |
| `orchestration/file_review_originating_entry.rs` | SQL, Git process, filesystem |
| `orchestration/execution_support.rs` | SQL, Git process, filesystem |
| `orchestration/accepted_integration.rs` | SQL, Git process, filesystem |
| `worktree_review/service.rs` | SQL, process/thread, filesystem |
| `worktree_runtime/planning.rs` | process planning, filesystem, local ports |
| `orchestration/conversation_harness_revision.rs` | SQL coordination plus application-owned filesystem evidence |
| `orchestration/accepted_candidate_authority.rs` | SQL plus Git process/ref effects |
| `orchestration/confirmation.rs` | SQL plus frontend notification |
| `storage.rs` | SQL bootstrap plus filesystem roots |
| legacy `lib.rs` | SQL, process and filesystem implementation behind the release quarantine guard |

This immediately presents several different kinds of cross-domain coordination: user confirmation, agent capability transport, provider launch, Git authority, immutable configuration publication, review tooling and compatibility retention. They should not be treated as one architectural bucket merely because they all perform external effects.

## First selected traversal: immutable Harness revision publication

This signal was selected because it connects configuration, application behavior, SQLite, filesystem evidence and agent launch while having no equivalent visible authoring operation.

### Trigger

`SprintRunnerTransitionService` asks `WorkUnitExecutionHarnessService` for a current effective revision at concrete role transitions:

- original Handler activation;
- Handler action continuation;
- Handler outcome review;
- original or retry Implementer activation;
- Implementer reporting continuation.

The relevant calls are in `sprint_runner_transition.rs` at the Handler, Implementer, reporting and review reconciliation sites. There is no direct user or agent tool named “publish Harness revision.” Publication is an application-side consequence of preparing the role boundary.

### Configuration becomes durable behavior

`work_unit_execution_harness.rs` first loads the verified revision history for `work_unit_handler` or `work_unit_implementer`. If the necessary baseline or stage-specific successor does not exist, it:

1. constructs a code-authored effective configuration from `conversation_harness.rs`;
2. saves or advances a mutable working copy in SQLite;
3. requests publication of the exact current draft with an expected predecessor and idempotency key;
4. reloads the winning revision on a concurrent publication conflict;
5. converts the verified revision back into a runtime profile;
6. checks that the role-specific MCP tool set and other expected fields have not drifted;
7. stores revision ID, configuration digest and repository commit reference with the Work Unit transition facts.

The source configuration is therefore not passive seed data. A lifecycle transition can author and publish a new immutable runtime variant automatically.

### Two coordinated stores

The release composition opens `SqliteOrchestrationRepository` with an application-owned repository root at `<app-data>/harness-revisions/`. Publication spans:

- mutable working-copy and immutable revision lineage in `codex-orchestrator-active-v3.sqlite`;
- normalized configuration objects addressed by SHA-256 digest;
- JSON revision manifests addressed by generated revision ID;
- an SQLite publication ledger and idempotent command/result record.

`repository_commit_ref` uses the local contract `harness-revision-commit-v1/<revision-id>`. It is not a Git commit or ref despite the name.

The command contains no repository path or configuration bytes. The repository loads the exact current draft, normalizes it, writes immutable local evidence, verifies it, and only then inserts and commits the SQLite revision/publication/command rows.

### Failure and retention behavior

- If the local repository write fails, no SQLite publication is committed.
- If a later SQLite step or commit fails, verified object or manifest files can remain as unpublished local orphans.
- Existing immutable paths replay only when their bytes match exactly; conflicting bytes fail as invalid evidence.
- Reads require the SQLite record, publication ledger, manifest, object digest and decoded complete configuration to agree.
- Missing or tampered local material is returned as invalid evidence, never silently reconstructed from the working copy.
- No production cleanup or garbage-collection path for unpublished Harness objects was found in this sweep.

The focused 13-test set proves these storage and replay contracts deterministically, including the deliberate local-orphan case.

### Visible product relationship

The application facade exposes save, create and verified-read methods only inside Rust. No Tauri command exposes working-copy save or revision publication, and no agent MCP tool exposes it. Non-test production callers are concentrated in `WorkUnitExecutionHarnessService`.

The mounted Harness inspection experience instead loads a compiled Plan Builder profile and reports many management capabilities as unavailable. Consequently:

- productive Work Unit execution can automatically create and consume immutable Harness revisions;
- the generic visible Harness Management model does not author or activate those revisions;
- “application user” creation provenance in this path names application-owned role preparation, not a demonstrated frontend user action;
- frontend inspectability and runtime configuration authority are materially different surfaces.

### Exact artifact chain

```text
sprint_runner_transition.rs
  -> WorkUnitExecutionHarnessService::current_*_revision
  -> conversation_harness.rs code-authored configuration
  -> OrchestrationApplication save/create facade
  -> SqliteOrchestrationRepository
       -> conversation_harness_working_copy.rs validation/state
       -> conversation_harness_revision.rs normalization and immutable files
       -> harness working-copy/revision/publication/command tables
  -> verified pinned profile
  -> role transition stores revision ID + digest + local commit reference
  -> runtime launch extension receives effective role policy
```

## Other raw signals retained for traversal

The reverse sweep also found these effects without yet forcing them into one model:

- accepted Handler candidates cause automatic Git object creation, serialized ref advancement, worktree convergence, database current-target advancement, settlement and prerequisite-contribution writes;
- execution support creates isolated Git worktrees and stores exact role/attempt authority before Handler or Implementer launch;
- File Review production runs a separately hardened Git environment and stores opaque review artifacts;
- native profile readiness coordinates profile SQL, filesystem identity, strict child environments, canary receipts and a separate MCP listener;
- confirmation persists the user decision and publishes a Tauri event to a waiting frontend/agent boundary;
- debug Human Review coordinates SQLite registries, build output, process ownership, readiness files and proof-control routes;
- the quarantined legacy implementation still contains SQL, Codex, Git and validation effects, but its release command guard prevents them from being reached.

These are candidate traversals, not yet a completeness claim.

## Questions opened by the sweep

- Is automatic application publication the intended long-term Harness authoring model, or a bootstrap mechanism for a future governed authoring surface?
- Should unpublished immutable Harness objects be retained as forensic evidence, garbage-collected, or prevented through a different commit protocol?
- Which term should distinguish the local content-addressed `repository_commit_ref` from actual Git authority elsewhere in the product?
- How should a user inspect the effective pinned Work Unit revision when the visible inspector is Plan Builder-specific?
- Are application-authored stage variants expected to remain code functions, or become data governed by the same revision model they create?
- Which other cross-store effects have similarly strict replay contracts but weak product observability?
