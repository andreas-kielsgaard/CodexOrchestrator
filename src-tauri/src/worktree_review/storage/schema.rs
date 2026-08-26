use super::{sql_error, StorageResult};
use rusqlite::{Connection, TransactionBehavior};

const SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS worktree_review_schema_migrations (
  version INTEGER PRIMARY KEY,
  applied_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS review_repositories (
  repository_id TEXT PRIMARY KEY,
  label TEXT NOT NULL,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS worktree_review_selection (
  singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
  repository_id TEXT NOT NULL,
  repository_root TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS review_branches (
  repository_id TEXT NOT NULL REFERENCES review_repositories(repository_id) ON DELETE CASCADE,
  full_ref TEXT NOT NULL CHECK (full_ref LIKE 'refs/heads/%'),
  observed_tip TEXT NOT NULL,
  observed_at TEXT NOT NULL,
  PRIMARY KEY(repository_id, full_ref)
);

CREATE TABLE IF NOT EXISTS review_worktree_associations (
  association_id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL,
  full_branch_ref TEXT NOT NULL,
  worktree_id TEXT NOT NULL,
  observed_location TEXT NOT NULL,
  provenance TEXT NOT NULL CHECK (provenance IN (
    'git_attached_branch', 'user_associated_detached', 'product_created', 'legacy_unverified'
  )),
  baseline_kind TEXT NOT NULL CHECK (baseline_kind IN (
    'created_at_object', 'observed_at_association', 'user_supplied', 'legacy_unknown'
  )),
  baseline_object_id TEXT,
  observed_head TEXT NOT NULL,
  commits_ahead INTEGER NOT NULL CHECK (commits_ahead >= 0),
  commits_behind INTEGER NOT NULL CHECK (commits_behind >= 0),
  staged_paths INTEGER NOT NULL CHECK (staged_paths >= 0),
  unstaged_paths INTEGER NOT NULL CHECK (unstaged_paths >= 0),
  untracked_paths INTEGER NOT NULL CHECK (untracked_paths >= 0),
  detached_head INTEGER NOT NULL CHECK (detached_head IN (0, 1)),
  branch_reachable INTEGER NOT NULL CHECK (branch_reachable IN (0, 1)),
  observed_at TEXT NOT NULL,
  lifecycle TEXT NOT NULL CHECK (lifecycle IN (
    'active', 'missing', 'moved', 'foreign_repository', 'branch_mismatch', 'unverified',
    'disassociated'
  )),
  associated_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  UNIQUE(repository_id, worktree_id, full_branch_ref),
  FOREIGN KEY(repository_id, full_branch_ref)
    REFERENCES review_branches(repository_id, full_ref) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS idx_review_associations_branch
  ON review_worktree_associations(repository_id, full_branch_ref, lifecycle);

CREATE TABLE IF NOT EXISTS review_workspaces (
  workspace_id TEXT PRIMARY KEY,
  repository_id TEXT NOT NULL REFERENCES review_repositories(repository_id) ON DELETE RESTRICT,
  worktree_id TEXT NOT NULL,
  observed_location TEXT NOT NULL,
  ownership_json TEXT NOT NULL,
  lifecycle TEXT NOT NULL CHECK (lifecycle IN (
    'ready', 'missing', 'removal_pending', 'removed', 'unverified'
  )),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS review_builds (
  build_id TEXT PRIMARY KEY,
  name TEXT NOT NULL,
  repository_id TEXT NOT NULL,
  full_branch_ref TEXT NOT NULL,
  workspace_id TEXT NOT NULL REFERENCES review_workspaces(workspace_id) ON DELETE RESTRICT,
  source_binding_json TEXT NOT NULL,
  retention_key TEXT NOT NULL,
  current_artifact_set_id TEXT,
  lifecycle TEXT NOT NULL CHECK (lifecycle IN (
    'active', 'superseded', 'cleanup_pending', 'cleaned', 'unverified_legacy'
  )),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  FOREIGN KEY(repository_id, full_branch_ref)
    REFERENCES review_branches(repository_id, full_ref) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS idx_review_builds_logical_source
  ON review_builds(repository_id, full_branch_ref, retention_key, created_at DESC);

CREATE TABLE IF NOT EXISTS review_operation_attempts (
  attempt_id TEXT PRIMARY KEY,
  build_id TEXT NOT NULL REFERENCES review_builds(build_id) ON DELETE RESTRICT,
  operation_kind TEXT NOT NULL CHECK (operation_kind IN (
    'materialize_source', 'build'
  )),
  execution_state TEXT NOT NULL CHECK (execution_state IN (
    'pending', 'running', 'completed', 'interrupted'
  )),
  verdict TEXT NOT NULL CHECK (verdict IN ('unknown', 'passed', 'failed', 'cancelled')),
  active_stage TEXT,
  failure_stage TEXT,
  failure_category TEXT,
  failure_message TEXT,
  requested_at TEXT NOT NULL,
  started_at TEXT,
  completed_at TEXT,
  CHECK (
    (verdict = 'failed' AND failure_stage IS NOT NULL
      AND failure_category IS NOT NULL AND failure_message IS NOT NULL) OR
    (verdict <> 'failed' AND failure_stage IS NULL
      AND failure_category IS NULL AND failure_message IS NULL)
  )
);
CREATE INDEX IF NOT EXISTS idx_review_attempts_build
  ON review_operation_attempts(build_id, requested_at DESC);

CREATE TABLE IF NOT EXISTS review_build_attentions (
  attention_id TEXT PRIMARY KEY,
  build_id TEXT NOT NULL REFERENCES review_builds(build_id) ON DELETE RESTRICT,
  category TEXT NOT NULL CHECK (category IN ('cleanup_coordination')),
  summary TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  resolved_at TEXT
);
CREATE INDEX IF NOT EXISTS idx_review_build_attentions_active
  ON review_build_attentions(build_id, recorded_at DESC)
  WHERE resolved_at IS NULL;

CREATE TABLE IF NOT EXISTS verified_artifact_sets (
  artifact_set_id TEXT PRIMARY KEY,
  build_id TEXT NOT NULL REFERENCES review_builds(build_id) ON DELETE RESTRICT,
  attempt_id TEXT NOT NULL UNIQUE REFERENCES review_operation_attempts(attempt_id) ON DELETE RESTRICT,
  storage_key TEXT NOT NULL UNIQUE,
  manifest_hash TEXT NOT NULL,
  verified_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS verified_artifact_files (
  artifact_set_id TEXT NOT NULL REFERENCES verified_artifact_sets(artifact_set_id) ON DELETE CASCADE,
  ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
  relative_path TEXT NOT NULL,
  content_hash TEXT NOT NULL,
  bytes INTEGER NOT NULL CHECK (bytes >= 0),
  PRIMARY KEY(artifact_set_id, ordinal),
  UNIQUE(artifact_set_id, relative_path)
);

CREATE TABLE IF NOT EXISTS review_cleanup_jobs (
  cleanup_job_id TEXT PRIMARY KEY,
  build_id TEXT NOT NULL REFERENCES review_builds(build_id) ON DELETE RESTRICT,
  trigger TEXT NOT NULL CHECK (trigger IN (
    'retention_policy', 'manual_request', 'reconciliation', 'legacy_migration'
  )),
  eligibility TEXT NOT NULL CHECK (eligibility IN (
    'eligible', 'build_running', 'retention_protected', 'borrowed_resource', 'already_cleaned',
    'source_unverified'
  )),
  state TEXT NOT NULL CHECK (state IN (
    'planned', 'running', 'completed', 'attention_required', 'not_eligible'
  )),
  created_at TEXT NOT NULL,
  started_at TEXT,
  settled_at TEXT
);

CREATE TABLE IF NOT EXISTS review_cleanup_resources (
  cleanup_job_id TEXT NOT NULL REFERENCES review_cleanup_jobs(cleanup_job_id) ON DELETE CASCADE,
  resource_id TEXT NOT NULL,
  ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
  resource_json TEXT NOT NULL,
  PRIMARY KEY(cleanup_job_id, resource_id),
  UNIQUE(cleanup_job_id, ordinal)
);

CREATE TABLE IF NOT EXISTS review_cleanup_effects (
  cleanup_job_id TEXT NOT NULL,
  resource_id TEXT NOT NULL,
  disposition TEXT NOT NULL CHECK (disposition IN (
    'pending', 'removed', 'already_absent', 'retained', 'failed'
  )),
  detail TEXT,
  recorded_at TEXT NOT NULL,
  PRIMARY KEY(cleanup_job_id, resource_id),
  FOREIGN KEY(cleanup_job_id, resource_id)
    REFERENCES review_cleanup_resources(cleanup_job_id, resource_id) ON DELETE CASCADE
);

CREATE TABLE IF NOT EXISTS review_cleanup_receipts (
  cleanup_job_id TEXT PRIMARY KEY REFERENCES review_cleanup_jobs(cleanup_job_id) ON DELETE RESTRICT,
  build_id TEXT NOT NULL REFERENCES review_builds(build_id) ON DELETE RESTRICT,
  completed_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS worktree_review_product_settings (
  singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
  retention_policy_json TEXT NOT NULL,
  updated_at TEXT NOT NULL
);
"#;

pub(super) fn initialize(connection: &mut Connection) -> StorageResult<()> {
    let transaction = connection
        .transaction_with_behavior(TransactionBehavior::Immediate)
        .map_err(sql_error("serialize durable worktree review migrations"))?;
    initialize_locked(&transaction)?;
    transaction
        .commit()
        .map_err(sql_error("commit durable worktree review migrations"))
}

fn initialize_locked(connection: &Connection) -> StorageResult<()> {
    connection
        .execute_batch(SCHEMA)
        .map_err(sql_error("initialize durable worktree review schema"))?;
    connection
        .execute(
            "INSERT OR IGNORE INTO worktree_review_schema_migrations(version, applied_at)
             VALUES (1, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            [],
        )
        .map_err(sql_error("record durable worktree review schema"))?;
    let mut columns = connection
        .prepare("PRAGMA table_info(review_builds)")
        .map_err(sql_error("inspect durable review build schema"))?;
    let column_names = columns
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(sql_error("query durable review build columns"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read durable review build columns"))?;
    drop(columns);
    if !column_names.iter().any(|column| column == "name") {
        connection
            .execute(
                "ALTER TABLE review_builds ADD COLUMN name TEXT NOT NULL DEFAULT 'Unverified legacy build'",
                [],
            )
            .map_err(sql_error("add durable review build name"))?;
    }
    connection
        .execute(
            "INSERT OR IGNORE INTO worktree_review_schema_migrations(version, applied_at)
             VALUES (2, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            [],
        )
        .map_err(sql_error("record durable review build name schema"))?;
    connection
        .execute(
            "INSERT OR IGNORE INTO worktree_review_schema_migrations(version, applied_at)
             VALUES (3, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            [],
        )
        .map_err(sql_error("record durable build attention schema"))?;
    connection
        .execute(
            "DELETE FROM review_operation_attempts
             WHERE operation_kind IN ('launch', 'stop', 'recover')",
            [],
        )
        .map_err(sql_error("retire legacy review lifecycle attempts"))?;
    connection
        .execute(
            "UPDATE review_operation_attempts
             SET active_stage = 'interruption_reconciliation'
             WHERE active_stage = 'recovery'",
            [],
        )
        .map_err(sql_error("rename review interruption reconciliation stage"))?;
    connection
        .execute(
            "INSERT OR IGNORE INTO worktree_review_schema_migrations(version, applied_at)
             VALUES (4, strftime('%Y-%m-%dT%H:%M:%fZ', 'now'))",
            [],
        )
        .map_err(sql_error("record minimal review operation schema"))?;
    Ok(())
}
