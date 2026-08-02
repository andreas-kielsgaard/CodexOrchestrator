use super::conversation_harness_revision::{
    decode_verified_configuration, normalized_configuration_envelope, revision_id,
    validate_create_command as validate_harness_revision_command, validate_revision,
    CreateHarnessRevisionCommand, CreateHarnessRevisionResult, HarnessRevision,
    HarnessRevisionCommitManifest, HarnessRevisionCreationProvenance, HarnessRevisionError,
    HarnessRevisionHistoryOutcome, HarnessRevisionProvenanceKind, HarnessRevisionReadOutcome,
    LocalHarnessRevisionRepository, LocalHarnessRevisionRepositoryError,
};
use super::conversation_harness_working_copy::{
    validate_command as validate_harness_working_copy_command, validate_harness_key,
    validate_working_copy, HarnessEditorKind, HarnessEffectiveConfigurationEnvelope,
    HarnessWorkingCopy, HarnessWorkingCopyEditor, HarnessWorkingCopyError,
    SaveHarnessWorkingCopyCommand, SaveHarnessWorkingCopyResult,
    HARNESS_EFFECTIVE_CONFIGURATION_V1,
};
use super::domain::{
    CapabilityProfileId, EffectProvenanceId, EpicPlanningDraftId, PlanBuilderProposal,
    PlanningDraftAgentSessionAssociationId, ProposalCommandId, ProposalEventId, ProposalResultId,
    ProposalRevisionId, SaveEpicPlanProposalCommand, SaveProposalError, SaveProposalResult,
    NATIVE_QUERY_VERSION,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction, TransactionBehavior};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use uuid::Uuid;

pub(crate) const ORCHESTRATION_SCHEMA: &str = r#"
CREATE TABLE epic_planning_drafts (
  id TEXT PRIMARY KEY,
  title TEXT,
  status TEXT NOT NULL CHECK (status IN ('active', 'canceled')),
  created_at TEXT NOT NULL,
  updated_at TEXT NOT NULL,
  canceled_at TEXT
);
CREATE TABLE planning_draft_lifecycle_events (
  id TEXT PRIMARY KEY,
  draft_id TEXT NOT NULL,
  event_kind TEXT NOT NULL CHECK (event_kind IN ('draft_begun', 'draft_title_updated', 'draft_canceled')),
  idempotency_key TEXT NOT NULL UNIQUE,
  actor_id TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT
);
CREATE TABLE capability_profiles (
  id TEXT PRIMARY KEY,
  status TEXT NOT NULL CHECK (status IN ('active', 'disabled', 'expired')),
  created_at TEXT NOT NULL
);
CREATE TABLE planning_draft_agent_session_associations (
  id TEXT PRIMARY KEY,
  draft_id TEXT NOT NULL,
  agent_session_id TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  associated_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT,
  FOREIGN KEY (agent_session_id) REFERENCES agent_sessions(id) ON DELETE RESTRICT
);
CREATE TABLE planning_draft_profile_assignments (
  draft_id TEXT NOT NULL,
  capability_profile_id TEXT NOT NULL,
  agent_session_association_id TEXT NOT NULL,
  expires_at TEXT NOT NULL,
  assigned_at TEXT NOT NULL,
  PRIMARY KEY (draft_id, capability_profile_id, agent_session_association_id),
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT,
  FOREIGN KEY (capability_profile_id) REFERENCES capability_profiles(id) ON DELETE RESTRICT,
  FOREIGN KEY (agent_session_association_id) REFERENCES planning_draft_agent_session_associations(id) ON DELETE RESTRICT
);
CREATE TABLE proposal_commands (
  id TEXT PRIMARY KEY,
  idempotency_key TEXT NOT NULL UNIQUE,
  draft_id TEXT NOT NULL,
  capability_profile_id TEXT NOT NULL,
  agent_session_association_id TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  expected_revision_token TEXT,
  proposal_json TEXT NOT NULL CHECK (json_valid(proposal_json)),
  payload_fingerprint TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT
);
CREATE TABLE effect_provenance (
  id TEXT PRIMARY KEY,
  source_kind TEXT NOT NULL CHECK (source_kind = 'managed_plan_builder'),
  recorded_at TEXT NOT NULL,
  actor_id TEXT NOT NULL,
  agent_session_association_id TEXT NOT NULL,
  capability_profile_id TEXT NOT NULL,
  causal_command_id TEXT NOT NULL UNIQUE,
  causal_result_id TEXT NOT NULL UNIQUE,
  FOREIGN KEY (causal_command_id) REFERENCES proposal_commands(id) ON DELETE RESTRICT
);
CREATE TABLE proposal_revisions (
  id TEXT PRIMARY KEY,
  draft_id TEXT NOT NULL,
  parent_revision_id TEXT,
  revision_token TEXT NOT NULL UNIQUE,
  proposal_json TEXT NOT NULL CHECK (json_valid(proposal_json)),
  command_id TEXT NOT NULL UNIQUE,
  provenance_id TEXT NOT NULL UNIQUE,
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT,
  FOREIGN KEY (parent_revision_id) REFERENCES proposal_revisions(id) ON DELETE RESTRICT,
  FOREIGN KEY (command_id) REFERENCES proposal_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (provenance_id) REFERENCES effect_provenance(id) ON DELETE RESTRICT
);
CREATE INDEX proposal_revisions_by_draft ON proposal_revisions(draft_id, recorded_at, id);
CREATE TABLE proposal_events (
  id TEXT PRIMARY KEY,
  draft_id TEXT NOT NULL,
  revision_id TEXT NOT NULL,
  command_id TEXT NOT NULL UNIQUE,
  provenance_id TEXT NOT NULL,
  event_kind TEXT NOT NULL CHECK (event_kind = 'proposal_saved'),
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT,
  FOREIGN KEY (revision_id) REFERENCES proposal_revisions(id) ON DELETE RESTRICT,
  FOREIGN KEY (command_id) REFERENCES proposal_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (provenance_id) REFERENCES effect_provenance(id) ON DELETE RESTRICT
);
CREATE INDEX proposal_events_by_draft ON proposal_events(draft_id, recorded_at, id);
CREATE TABLE proposal_command_results (
  id TEXT PRIMARY KEY,
  command_id TEXT NOT NULL UNIQUE,
  revision_id TEXT NOT NULL UNIQUE,
  event_id TEXT NOT NULL UNIQUE,
  provenance_id TEXT NOT NULL UNIQUE,
  created_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES proposal_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (revision_id) REFERENCES proposal_revisions(id) ON DELETE RESTRICT,
  FOREIGN KEY (event_id) REFERENCES proposal_events(id) ON DELETE RESTRICT,
  FOREIGN KEY (provenance_id) REFERENCES effect_provenance(id) ON DELETE RESTRICT
);
"#;

/// Additive active-v3 migration. A snapshot is the exact proposal bytes consumed by initiation;
/// it deliberately is not a generated document or filesystem artifact.
pub(crate) const ORCHESTRATION_INITIATION_SCHEMA: &str = r#"
CREATE TABLE epic_initiation_commands (
  id TEXT PRIMARY KEY, idempotency_key TEXT NOT NULL UNIQUE, draft_id TEXT NOT NULL,
  expected_revision_token TEXT NOT NULL, actor_id TEXT NOT NULL, payload_fingerprint TEXT NOT NULL,
  recorded_at TEXT NOT NULL, FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT
);
CREATE TABLE epic_initiation_results (
  id TEXT PRIMARY KEY, command_id TEXT NOT NULL UNIQUE, recorded_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES epic_initiation_commands(id) ON DELETE RESTRICT
);
CREATE TABLE epic_initiation_events (
  id TEXT PRIMARY KEY, command_id TEXT NOT NULL UNIQUE, result_id TEXT NOT NULL UNIQUE, recorded_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES epic_initiation_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (result_id) REFERENCES epic_initiation_results(id) ON DELETE RESTRICT
);
CREATE TABLE epic_initiation_provenance (
  id TEXT PRIMARY KEY, command_id TEXT NOT NULL UNIQUE, result_id TEXT NOT NULL UNIQUE, event_id TEXT NOT NULL UNIQUE, recorded_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES epic_initiation_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (result_id) REFERENCES epic_initiation_results(id) ON DELETE RESTRICT,
  FOREIGN KEY (event_id) REFERENCES epic_initiation_events(id) ON DELETE RESTRICT
);
CREATE TABLE epic_initiation_material_snapshots (
  id TEXT PRIMARY KEY, draft_id TEXT NOT NULL, proposal_revision_id TEXT NOT NULL,
  version INTEGER NOT NULL CHECK (version = 1), proposal_json TEXT NOT NULL CHECK (json_valid(proposal_json)),
  content_hash TEXT NOT NULL, recorded_at TEXT NOT NULL,
  UNIQUE(draft_id, proposal_revision_id), FOREIGN KEY (proposal_revision_id) REFERENCES proposal_revisions(id) ON DELETE RESTRICT
);
CREATE TABLE epic_initiations (
  id TEXT PRIMARY KEY, command_id TEXT NOT NULL UNIQUE, result_id TEXT NOT NULL UNIQUE, event_id TEXT NOT NULL UNIQUE,
  provenance_id TEXT NOT NULL UNIQUE, draft_id TEXT NOT NULL UNIQUE, proposal_revision_id TEXT NOT NULL UNIQUE,
  material_snapshot_id TEXT NOT NULL UNIQUE, epic_id TEXT NOT NULL UNIQUE, recorded_at TEXT NOT NULL,
  FOREIGN KEY (command_id) REFERENCES epic_initiation_commands(id) ON DELETE RESTRICT,
  FOREIGN KEY (proposal_revision_id) REFERENCES proposal_revisions(id) ON DELETE RESTRICT,
  FOREIGN KEY (material_snapshot_id) REFERENCES epic_initiation_material_snapshots(id) ON DELETE RESTRICT
);
CREATE TABLE initiated_planning_drafts (
  draft_id TEXT PRIMARY KEY, initiation_id TEXT NOT NULL UNIQUE, initiated_at TEXT NOT NULL,
  FOREIGN KEY (draft_id) REFERENCES epic_planning_drafts(id) ON DELETE RESTRICT,
  FOREIGN KEY (initiation_id) REFERENCES epic_initiations(id) ON DELETE RESTRICT
);
CREATE TABLE initiated_sprints (
  id TEXT PRIMARY KEY, epic_id TEXT NOT NULL, ordinal INTEGER NOT NULL, title TEXT NOT NULL,
  intended_movement TEXT NOT NULL, concern_summaries_json TEXT NOT NULL CHECK (json_valid(concern_summaries_json)),
  sprint_plan_id TEXT NOT NULL UNIQUE, sprint_plan_revision_id TEXT NOT NULL UNIQUE,
  UNIQUE(epic_id, ordinal), FOREIGN KEY (epic_id) REFERENCES epic_initiations(epic_id) ON DELETE RESTRICT
);
"#;

/// Durable one-shot application context scheduled only by a confirmed button initiation.
pub(crate) const PLAN_BUILDER_CONTEXT_DELIVERY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS plan_builder_context_deliveries (
  id TEXT PRIMARY KEY,
  initiation_id TEXT NOT NULL UNIQUE,
  epic_id TEXT NOT NULL,
  agent_session_id TEXT NOT NULL,
  source_kind TEXT NOT NULL CHECK (source_kind = 'button_initiation'),
  requested_at TEXT NOT NULL,
  pending_at TEXT NOT NULL,
  delivery_claim_id TEXT,
  delivery_claimed_at TEXT,
  target_invocation_id TEXT UNIQUE,
  delivered_to_invocation_id TEXT UNIQUE,
  delivered_at TEXT,
  consumed_at TEXT,
  FOREIGN KEY (initiation_id) REFERENCES epic_initiations(id) ON DELETE RESTRICT,
  FOREIGN KEY (agent_session_id) REFERENCES agent_sessions(id) ON DELETE RESTRICT,
  FOREIGN KEY (delivered_to_invocation_id) REFERENCES agent_session_invocations(id) ON DELETE RESTRICT,
  CHECK ((delivery_claim_id IS NULL) = (delivery_claimed_at IS NULL)),
  CHECK ((delivery_claim_id IS NULL) = (target_invocation_id IS NULL)),
  CHECK ((delivered_to_invocation_id IS NULL) = (delivered_at IS NULL)),
  CHECK ((delivered_at IS NULL) = (consumed_at IS NULL))
);
CREATE INDEX IF NOT EXISTS plan_builder_context_pending_by_session
  ON plan_builder_context_deliveries(agent_session_id, pending_at)
  WHERE consumed_at IS NULL;
"#;

pub(crate) const PLAN_BUILDER_CONTEXT_RECONCILIATION_SCHEMA: &str = r#"
ALTER TABLE plan_builder_context_deliveries ADD COLUMN target_invocation_id TEXT;
CREATE UNIQUE INDEX plan_builder_context_target_invocation
  ON plan_builder_context_deliveries(target_invocation_id)
  WHERE target_invocation_id IS NOT NULL;
"#;

/// Normalized ownership and membership facts for a producer-owned File Review payload.
pub(crate) const FILE_REVIEW_FACTS_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS file_review_documents (
 document_ref_id TEXT PRIMARY KEY, epic_id TEXT NOT NULL, sprint_id TEXT NOT NULL, provenance_id TEXT NOT NULL,
 opaque_reference TEXT NOT NULL UNIQUE, title TEXT NOT NULL, summary TEXT, idempotency_key TEXT NOT NULL UNIQUE, payload_fingerprint TEXT NOT NULL, recorded_at TEXT NOT NULL,
 FOREIGN KEY (epic_id) REFERENCES epic_initiations(epic_id) ON DELETE RESTRICT, FOREIGN KEY (sprint_id) REFERENCES initiated_sprints(id) ON DELETE RESTRICT,
 FOREIGN KEY (provenance_id) REFERENCES epic_initiation_provenance(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS file_review_changed_files (
 document_ref_id TEXT NOT NULL, changed_file_reference_id TEXT NOT NULL, display_name TEXT NOT NULL,
 change_kind TEXT NOT NULL CHECK (change_kind IN ('added','modified','deleted','renamed')), previous_display_name TEXT, ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
 CHECK (change_kind = 'renamed' OR previous_display_name IS NULL),
 PRIMARY KEY (document_ref_id, changed_file_reference_id), UNIQUE (document_ref_id, ordinal),
 FOREIGN KEY (document_ref_id) REFERENCES file_review_documents(document_ref_id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS stored_file_review_artifacts (
 artifact_id TEXT PRIMARY KEY, document_ref_id TEXT NOT NULL UNIQUE,
 contract_version TEXT NOT NULL CHECK (contract_version = 'stored-file-review-artifact/v1'), payload BLOB NOT NULL,
 payload_bytes INTEGER NOT NULL CHECK (payload_bytes > 0 AND payload_bytes <= 1000000), provenance_id TEXT NOT NULL,
 FOREIGN KEY (document_ref_id) REFERENCES file_review_documents(document_ref_id) ON DELETE RESTRICT,
 FOREIGN KEY (provenance_id) REFERENCES epic_initiation_provenance(id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS file_review_git_capture_authorizations (
 capture_authorization_id TEXT PRIMARY KEY, idempotency_key TEXT NOT NULL UNIQUE, payload_fingerprint TEXT NOT NULL,
 epic_id TEXT NOT NULL, sprint_id TEXT NOT NULL, provenance_id TEXT NOT NULL,
 repository_id TEXT NOT NULL, repository_root TEXT NOT NULL, worktree_id TEXT NOT NULL, worktree_root TEXT NOT NULL,
 baseline_object_id TEXT NOT NULL, current_object_id TEXT NOT NULL, recorded_at TEXT NOT NULL,
 FOREIGN KEY (epic_id) REFERENCES epic_initiations(epic_id) ON DELETE RESTRICT,
 FOREIGN KEY (sprint_id) REFERENCES initiated_sprints(id) ON DELETE RESTRICT,
 FOREIGN KEY (provenance_id) REFERENCES epic_initiation_provenance(id) ON DELETE RESTRICT
);
"#;
pub(crate) const FILE_REVIEW_FACTS_IDEMPOTENCY_SCHEMA: &str = r#"
ALTER TABLE file_review_documents ADD COLUMN payload_fingerprint TEXT NOT NULL DEFAULT '';
"#;
pub(crate) const FILE_REVIEW_GIT_CAPTURE_AUTHORIZATION_SCHEMA: &str = r#"
ALTER TABLE file_review_changed_files RENAME TO file_review_changed_files_v9;
CREATE TABLE file_review_changed_files (
 document_ref_id TEXT NOT NULL, changed_file_reference_id TEXT NOT NULL, display_name TEXT NOT NULL,
 change_kind TEXT NOT NULL CHECK (change_kind IN ('added','modified','deleted','renamed')), previous_display_name TEXT,
 ordinal INTEGER NOT NULL CHECK (ordinal >= 0),
 CHECK (change_kind = 'renamed' OR previous_display_name IS NULL),
 PRIMARY KEY (document_ref_id, changed_file_reference_id), UNIQUE (document_ref_id, ordinal),
 FOREIGN KEY (document_ref_id) REFERENCES file_review_documents(document_ref_id) ON DELETE RESTRICT
);
INSERT INTO file_review_changed_files (document_ref_id,changed_file_reference_id,display_name,change_kind,previous_display_name,ordinal)
 SELECT document_ref_id,changed_file_reference_id,display_name,change_kind,NULL,ordinal FROM file_review_changed_files_v9;
DROP TABLE file_review_changed_files_v9;
CREATE TABLE IF NOT EXISTS file_review_git_capture_authorizations (
 capture_authorization_id TEXT PRIMARY KEY, idempotency_key TEXT NOT NULL UNIQUE, payload_fingerprint TEXT NOT NULL,
 epic_id TEXT NOT NULL, sprint_id TEXT NOT NULL, provenance_id TEXT NOT NULL,
 repository_id TEXT NOT NULL, repository_root TEXT NOT NULL, worktree_id TEXT NOT NULL, worktree_root TEXT NOT NULL,
 baseline_object_id TEXT NOT NULL, current_object_id TEXT NOT NULL, recorded_at TEXT NOT NULL,
 FOREIGN KEY (epic_id) REFERENCES epic_initiations(epic_id) ON DELETE RESTRICT,
 FOREIGN KEY (sprint_id) REFERENCES initiated_sprints(id) ON DELETE RESTRICT,
 FOREIGN KEY (provenance_id) REFERENCES epic_initiation_provenance(id) ON DELETE RESTRICT
);
"#;

/// Private durable relation between initiated orchestration ownership and verified runtime Git facts.
pub(crate) const INITIATED_SPRINT_GIT_AUTHORITY_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS initiated_sprint_git_authorities (
 authority_id TEXT PRIMARY KEY, idempotency_key TEXT NOT NULL UNIQUE, payload_fingerprint TEXT NOT NULL,
 epic_id TEXT NOT NULL, sprint_id TEXT NOT NULL, provenance_id TEXT NOT NULL,
 repository_id TEXT NOT NULL, repository_root TEXT NOT NULL, repository_common_dir TEXT NOT NULL,
 worktree_id TEXT NOT NULL, worktree_root TEXT NOT NULL,
 baseline_object_id TEXT NOT NULL, current_object_id TEXT NOT NULL,
 runtime_instance_ref TEXT NOT NULL UNIQUE, runtime_source_ref TEXT NOT NULL,
 source_fingerprint TEXT NOT NULL, recorded_at TEXT NOT NULL,
 FOREIGN KEY (epic_id) REFERENCES epic_initiations(epic_id) ON DELETE RESTRICT,
 FOREIGN KEY (sprint_id) REFERENCES initiated_sprints(id) ON DELETE RESTRICT,
 FOREIGN KEY (provenance_id) REFERENCES epic_initiation_provenance(id) ON DELETE RESTRICT
);
"#;
pub(crate) const STORED_FILE_REVIEW_ARTIFACT_V1: &str = "stored-file-review-artifact/v1";
pub(crate) const FILE_REVIEW_ARTIFACT_MAX_BYTES: usize = 1_000_000;

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileReviewChangedFileWrite {
    pub(crate) changed_file_reference_id: String,
    pub(crate) display_name: String,
    pub(crate) change_kind: String,
    pub(crate) previous_display_name: Option<String>,
}
/// Producer/control-only authority. Roots remain private and are never serialized by NativeQuery.
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileReviewGitCaptureAuthorizationWrite {
    pub(crate) capture_authorization_id: String,
    pub(crate) idempotency_key: String,
    pub(crate) epic_id: String,
    pub(crate) sprint_id: String,
    pub(crate) provenance_id: String,
    pub(crate) repository_id: String,
    pub(crate) repository_root: String,
    pub(crate) worktree_id: String,
    pub(crate) worktree_root: String,
    pub(crate) baseline_object_id: String,
    pub(crate) current_object_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct FileReviewGitCaptureAuthorization {
    pub(crate) capture_authorization_id: String,
    pub(crate) epic_id: String,
    pub(crate) sprint_id: String,
    pub(crate) provenance_id: String,
    pub(crate) repository_id: String,
    pub(crate) repository_root: String,
    pub(crate) worktree_id: String,
    pub(crate) worktree_root: String,
    pub(crate) baseline_object_id: String,
    pub(crate) current_object_id: String,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileReviewGitCaptureAuthorizationError {
    Invalid,
    Forbidden,
    Conflict,
    Unavailable,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StoreFileReviewGitCaptureAuthorizationResult {
    Stored,
    IdempotentReplay,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InitiatedSprintGitAuthorityWrite {
    pub(crate) sprint_id: String,
    pub(crate) idempotency_key: String,
    pub(crate) repository_id: String,
    pub(crate) repository_root: String,
    pub(crate) repository_common_dir: String,
    pub(crate) worktree_id: String,
    pub(crate) worktree_root: String,
    pub(crate) baseline_object_id: String,
    pub(crate) current_object_id: String,
    pub(crate) runtime_instance_ref: String,
    pub(crate) runtime_source_ref: String,
    pub(crate) source_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct InitiatedSprintGitAuthority {
    pub(crate) authority_id: String,
    pub(crate) epic_id: String,
    pub(crate) sprint_id: String,
    pub(crate) provenance_id: String,
    pub(crate) repository_id: String,
    pub(crate) repository_root: String,
    pub(crate) repository_common_dir: String,
    pub(crate) worktree_id: String,
    pub(crate) worktree_root: String,
    pub(crate) baseline_object_id: String,
    pub(crate) current_object_id: String,
    pub(crate) runtime_instance_ref: String,
    pub(crate) runtime_source_ref: String,
    pub(crate) source_fingerprint: String,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum InitiatedSprintGitAuthorityError {
    Invalid,
    Forbidden,
    Conflict,
    Unavailable,
}

#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StoreInitiatedSprintGitAuthorityResult {
    Stored { authority_id: String },
    IdempotentReplay { authority_id: String },
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) struct StoreFileReviewFacts {
    pub(crate) document_ref_id: String,
    pub(crate) epic_id: String,
    pub(crate) sprint_id: String,
    pub(crate) provenance_id: String,
    pub(crate) opaque_reference: String,
    pub(crate) title: String,
    pub(crate) summary: Option<String>,
    pub(crate) artifact_id: String,
    pub(crate) payload: Vec<u8>,
    pub(crate) idempotency_key: String,
    pub(crate) changed_files: Vec<FileReviewChangedFileWrite>,
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum FileReviewFactsError {
    Invalid,
    Forbidden,
    Conflict,
    Unavailable(String),
}
#[derive(Clone, Debug, PartialEq, Eq)]
pub(crate) enum StoreFileReviewFactsResult {
    Stored,
    IdempotentReplay,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(tag = "status", rename_all = "snake_case")]
pub(crate) enum ScopedFileReviewLoad {
    Available { document: ScopedFileReviewDocument },
    Unavailable,
    Unauthorized,
    Invalid,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ScopedFileReviewDocument {
    pub(crate) document_ref_id: String,
    pub(crate) title: String,
    pub(crate) summary: Option<String>,
    pub(crate) artifact_id: String,
    pub(crate) payload: Vec<u8>,
    pub(crate) changed_files: Vec<FileReviewChangedFileDto>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct FileReviewChangedFileDto {
    pub(crate) changed_file_reference_id: String,
    pub(crate) display_name: String,
    pub(crate) change_kind: String,
    pub(crate) previous_display_name: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PendingPlanBuilderContextDelivery {
    pub(crate) delivery_id: String,
    pub(crate) initiation_id: String,
    pub(crate) epic_id: String,
    pub(crate) claim_id: String,
    pub(crate) target_invocation_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ManagedPlanBuilderBinding {
    pub(crate) associated_at: DateTime<Utc>,
}

pub(crate) struct SqliteOrchestrationRepository {
    connection: Mutex<Connection>,
    clock: Arc<dyn OrchestrationClock>,
    harness_revisions: LocalHarnessRevisionRepository,
    #[cfg(test)]
    fail_next_context_consume: std::sync::atomic::AtomicBool,
}

pub(crate) trait OrchestrationClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

struct SystemOrchestrationClock;
impl OrchestrationClock for SystemOrchestrationClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

impl SqliteOrchestrationRepository {
    pub(crate) fn new(connection: Connection) -> Result<Self, SaveProposalError> {
        Self::new_with_clock(connection, Arc::new(SystemOrchestrationClock))
    }

    pub(crate) fn new_with_clock(
        connection: Connection,
        clock: Arc<dyn OrchestrationClock>,
    ) -> Result<Self, SaveProposalError> {
        Self::new_with_clock_and_harness_revision_repository(
            connection,
            clock,
            std::env::temp_dir().join(format!(
                "codex-orchestrator-harness-revisions-{}",
                Uuid::new_v4()
            )),
        )
    }

    pub(crate) fn new_with_clock_and_harness_revision_repository(
        connection: Connection,
        clock: Arc<dyn OrchestrationClock>,
        harness_revision_repository: PathBuf,
    ) -> Result<Self, SaveProposalError> {
        crate::storage::configure_sqlite_connection(&connection)
            .map_err(sql_error("configure orchestration database"))?;
        Ok(Self {
            connection: Mutex::new(connection),
            clock,
            harness_revisions: LocalHarnessRevisionRepository::new(harness_revision_repository),
            #[cfg(test)]
            fail_next_context_consume: std::sync::atomic::AtomicBool::new(false),
        })
    }

    pub(crate) fn open(path: impl AsRef<Path>) -> Result<Self, SaveProposalError> {
        let path = path.as_ref();
        let repository_root = path
            .parent()
            .unwrap_or_else(|| Path::new("."))
            .join(crate::storage::HARNESS_REVISION_REPOSITORY_DIRECTORY_NAME);
        Self::open_with_harness_revision_repository(path, repository_root)
    }

    pub(crate) fn open_with_harness_revision_repository(
        path: impl AsRef<Path>,
        harness_revision_repository: PathBuf,
    ) -> Result<Self, SaveProposalError> {
        let connection =
            Connection::open(path).map_err(sql_error("open orchestration database"))?;
        Self::new_with_clock_and_harness_revision_repository(
            connection,
            Arc::new(SystemOrchestrationClock),
            harness_revision_repository,
        )
    }

    #[cfg(test)]
    pub(crate) fn harness_revision_repository_root(&self) -> PathBuf {
        self.harness_revisions.root().to_path_buf()
    }

    pub(crate) fn create_planning_draft(
        &self,
        id: &EpicPlanningDraftId,
        created_at: DateTime<Utc>,
    ) -> Result<(), SaveProposalError> {
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT INTO epic_planning_drafts (id, title, status, created_at, updated_at) VALUES (?1, NULL, 'active', ?2, ?2)",
                params![id.as_str(), timestamp(created_at)],
            )
            .map_err(sql_error("create planning draft"))?;
        Ok(())
    }

    pub(crate) fn schedule_button_initiation_context(
        &self,
        initiation: &super::domain::InitiateEpicResult,
    ) -> Result<(), SaveProposalError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_error("begin button initiation context scheduling"))?;
        let (session_id, epic_id): (String, String) = transaction
            .query_row(
                "SELECT association.agent_session_id, initiation.epic_id
                 FROM epic_initiations initiation
                 JOIN planning_draft_agent_session_associations association
                   ON association.draft_id=initiation.draft_id
                 WHERE initiation.id=?1",
                params![initiation.initiation_id.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .map_err(sql_error(
                "derive managed Plan Builder session for initiation context",
            ))?;
        if epic_id != initiation.epic_id.as_str() {
            return Err(SaveProposalError::Unavailable(
                "button initiation context identity does not match durable initiation".into(),
            ));
        }
        let now = timestamp(self.clock.now());
        let delivery_id = format!("plan-builder-context-{}", initiation.initiation_id.as_str());
        transaction
            .execute(
                "INSERT OR IGNORE INTO plan_builder_context_deliveries
                 (id,initiation_id,epic_id,agent_session_id,source_kind,requested_at,pending_at)
                 VALUES (?1,?2,?3,?4,'button_initiation',?5,?5)",
                params![
                    delivery_id,
                    initiation.initiation_id.as_str(),
                    epic_id,
                    session_id,
                    now
                ],
            )
            .map_err(sql_error("record pending button initiation context"))?;
        let existing: (String, String, String) = transaction
            .query_row(
                "SELECT initiation_id,epic_id,agent_session_id
                 FROM plan_builder_context_deliveries WHERE id=?1",
                params![delivery_id],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?)),
            )
            .map_err(sql_error("verify pending button initiation context"))?;
        if existing
            != (
                initiation.initiation_id.as_str().to_string(),
                initiation.epic_id.as_str().to_string(),
                session_id,
            )
        {
            return Err(SaveProposalError::Unavailable(
                "button initiation context identity was already used for different semantics"
                    .into(),
            ));
        }
        transaction
            .commit()
            .map_err(sql_error("commit button initiation context scheduling"))
    }

    pub(crate) fn claim_pending_plan_builder_context(
        &self,
        session_id: &str,
        claim_id: &str,
        target_invocation_id: &str,
    ) -> Result<Option<PendingPlanBuilderContextDelivery>, SaveProposalError> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_error("begin Plan Builder context claim"))?;
        let unresolved_claims: i64 = transaction
            .query_row(
                "SELECT count(*) FROM plan_builder_context_deliveries
                 WHERE agent_session_id=?1 AND consumed_at IS NULL AND delivery_claim_id IS NOT NULL",
                params![session_id],
                |row| row.get(0),
            )
            .map_err(sql_error("check unresolved Plan Builder context claims"))?;
        if unresolved_claims != 0 {
            return Err(SaveProposalError::Unavailable(
                "an earlier Plan Builder context claim requires launch reconciliation".into(),
            ));
        }
        let delivery = transaction
            .query_row(
                "SELECT id,initiation_id,epic_id FROM plan_builder_context_deliveries
                 WHERE agent_session_id=?1 AND consumed_at IS NULL AND delivery_claim_id IS NULL
                 ORDER BY pending_at,id LIMIT 1",
                params![session_id],
                |row| {
                    Ok(PendingPlanBuilderContextDelivery {
                        delivery_id: row.get(0)?,
                        initiation_id: row.get(1)?,
                        epic_id: row.get(2)?,
                        claim_id: claim_id.to_string(),
                        target_invocation_id: target_invocation_id.to_string(),
                    })
                },
            )
            .optional()
            .map_err(sql_error("read pending Plan Builder context"))?;
        if let Some(delivery) = delivery.as_ref() {
            transaction
                .execute(
                    "UPDATE plan_builder_context_deliveries
                     SET delivery_claim_id=?2,delivery_claimed_at=?3,target_invocation_id=?4
                     WHERE id=?1 AND consumed_at IS NULL",
                    params![
                        delivery.delivery_id,
                        claim_id,
                        timestamp(self.clock.now()),
                        target_invocation_id
                    ],
                )
                .map_err(sql_error("claim pending Plan Builder context"))?;
        }
        transaction
            .commit()
            .map_err(sql_error("commit Plan Builder context claim"))?;
        Ok(delivery)
    }

    pub(crate) fn load_claimed_plan_builder_context(
        &self,
        session_id: &str,
    ) -> Result<Option<PendingPlanBuilderContextDelivery>, SaveProposalError> {
        let connection = self.lock()?;
        connection
            .query_row(
                "SELECT id,initiation_id,epic_id,delivery_claim_id,target_invocation_id
                 FROM plan_builder_context_deliveries
                 WHERE agent_session_id=?1 AND consumed_at IS NULL AND delivery_claim_id IS NOT NULL",
                params![session_id],
                |row| {
                    Ok(PendingPlanBuilderContextDelivery {
                        delivery_id: row.get(0)?,
                        initiation_id: row.get(1)?,
                        epic_id: row.get(2)?,
                        claim_id: row.get(3)?,
                        target_invocation_id: row.get(4)?,
                    })
                },
            )
            .optional()
            .map_err(sql_error("load claimed Plan Builder context"))
    }

    pub(crate) fn consume_plan_builder_context(
        &self,
        delivery: &PendingPlanBuilderContextDelivery,
    ) -> Result<(), SaveProposalError> {
        #[cfg(test)]
        if self
            .fail_next_context_consume
            .swap(false, std::sync::atomic::Ordering::AcqRel)
        {
            return Err(SaveProposalError::Unavailable(
                "injected Plan Builder context consume failure".into(),
            ));
        }
        let connection = self.lock()?;
        let now = timestamp(self.clock.now());
        let changed = connection
            .execute(
                "UPDATE plan_builder_context_deliveries
                 SET delivered_to_invocation_id=?3,delivered_at=?4,consumed_at=?4
                 WHERE id=?1 AND delivery_claim_id=?2 AND target_invocation_id=?3 AND consumed_at IS NULL",
                params![
                    delivery.delivery_id,
                    delivery.claim_id,
                    delivery.target_invocation_id,
                    now
                ],
            )
            .map_err(sql_error("consume Plan Builder context"))?;
        if changed != 1 {
            return Err(SaveProposalError::Unavailable(
                "pending Plan Builder context claim is stale".into(),
            ));
        }
        Ok(())
    }

    #[cfg(test)]
    pub(crate) fn fail_next_plan_builder_context_consume(&self) {
        self.fail_next_context_consume
            .store(true, std::sync::atomic::Ordering::Release);
    }

    pub(crate) fn release_plan_builder_context(
        &self,
        delivery: &PendingPlanBuilderContextDelivery,
    ) -> Result<(), SaveProposalError> {
        let connection = self.lock()?;
        connection
            .execute(
                "UPDATE plan_builder_context_deliveries
                 SET delivery_claim_id=NULL,delivery_claimed_at=NULL,target_invocation_id=NULL
                 WHERE id=?1 AND delivery_claim_id=?2 AND target_invocation_id=?3 AND consumed_at IS NULL",
                params![
                    delivery.delivery_id,
                    delivery.claim_id,
                    delivery.target_invocation_id
                ],
            )
            .map_err(sql_error("release Plan Builder context claim"))?;
        Ok(())
    }

    pub(crate) fn create_capability_profile(
        &self,
        id: &CapabilityProfileId,
        status: &str,
        created_at: DateTime<Utc>,
    ) -> Result<(), SaveProposalError> {
        if !matches!(status, "active" | "disabled" | "expired") {
            return Err(SaveProposalError::InvalidInput(
                "capability profile status is invalid".into(),
            ));
        }
        let connection = self.lock()?;
        connection
            .execute(
                "INSERT INTO capability_profiles (id, status, created_at) VALUES (?1, ?2, ?3)",
                params![id.as_str(), status, timestamp(created_at)],
            )
            .map_err(sql_error("create capability profile"))?;
        Ok(())
    }

    pub(crate) fn assign_profile(
        &self,
        draft_id: &EpicPlanningDraftId,
        profile_id: &CapabilityProfileId,
        association_id: &PlanningDraftAgentSessionAssociationId,
        expires_at: DateTime<Utc>,
        assigned_at: DateTime<Utc>,
    ) -> Result<(), SaveProposalError> {
        let connection = self.lock()?;
        connection.execute("INSERT INTO planning_draft_profile_assignments (draft_id, capability_profile_id, agent_session_association_id, expires_at, assigned_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![draft_id.as_str(), profile_id.as_str(), association_id.as_str(), timestamp(expires_at), timestamp(assigned_at)])
            .map_err(sql_error("assign capability profile"))?;
        Ok(())
    }

    pub(crate) fn associate_agent_session(
        &self,
        association_id: &PlanningDraftAgentSessionAssociationId,
        draft_id: &EpicPlanningDraftId,
        session_id: &str,
        actor_id: &str,
        associated_at: DateTime<Utc>,
    ) -> Result<(), SaveProposalError> {
        let connection = self.lock()?;
        connection.execute("INSERT INTO planning_draft_agent_session_associations (id, draft_id, agent_session_id, actor_id, associated_at) VALUES (?1, ?2, ?3, ?4, ?5)", params![association_id.as_str(), draft_id.as_str(), session_id, actor_id, timestamp(associated_at)])
            .map_err(sql_error("associate Agent Session"))?;
        Ok(())
    }

    /// Resolves the calling session's managed Plan Builder binding, or atomically creates its
    /// pre-initiation draft, profile, association, and assignment. None of these rows imply an
    /// initiated Epic.
    pub(crate) fn bootstrap_managed_plan_builder(
        &self,
        session_id: &str,
    ) -> Result<
        (
            EpicPlanningDraftId,
            CapabilityProfileId,
            PlanningDraftAgentSessionAssociationId,
        ),
        SaveProposalError,
    > {
        let profile = CapabilityProfileId::new("plan-builder-capability-profile-v1")
            .map_err(SaveProposalError::InvalidInput)?;
        let now = self.clock.now();
        let connection = self.lock()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(sql_error("begin managed Plan Builder bootstrap"))?;
        if let Some((draft, profile, association, status, _initiated)) = transaction
            .query_row(
                "SELECT assignment.draft_id, assignment.capability_profile_id, association.id, draft.status, EXISTS(SELECT 1 FROM initiated_planning_drafts initiated WHERE initiated.draft_id = draft.id) FROM planning_draft_profile_assignments assignment JOIN planning_draft_agent_session_associations association ON association.id = assignment.agent_session_association_id JOIN epic_planning_drafts draft ON draft.id = assignment.draft_id WHERE association.agent_session_id = ?1 AND association.actor_id = 'managed-plan-builder' ORDER BY association.associated_at ASC LIMIT 1",
                params![session_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, String>(3)?, row.get::<_, bool>(4)?)),
            )
            .optional()
            .map_err(sql_error("resolve managed Plan Builder binding"))?
        {
            if status == "canceled" {
                return Err(SaveProposalError::Forbidden);
            }
            transaction
                .commit()
                .map_err(sql_error("commit managed Plan Builder binding read"))?;
            return Ok((
                EpicPlanningDraftId::new(draft).map_err(SaveProposalError::InvalidInput)?,
                CapabilityProfileId::new(profile).map_err(SaveProposalError::InvalidInput)?,
                PlanningDraftAgentSessionAssociationId::new(association)
                    .map_err(SaveProposalError::InvalidInput)?,
            ));
        }
        let draft = EpicPlanningDraftId::new(format!(
            "epic-planning-draft-{}",
            uuid::Uuid::new_v4().simple()
        ))
        .map_err(SaveProposalError::InvalidInput)?;
        let association = PlanningDraftAgentSessionAssociationId::new(format!(
            "plan-builder-association-{}",
            uuid::Uuid::new_v4().simple()
        ))
        .map_err(SaveProposalError::InvalidInput)?;
        transaction
            .execute(
                "INSERT OR IGNORE INTO epic_planning_drafts (id, title, status, created_at, updated_at) VALUES (?1, NULL, 'active', ?2, ?2)",
                params![draft.as_str(), timestamp(now)],
            )
            .map_err(sql_error("bootstrap planning draft"))?;
        transaction.execute("INSERT OR IGNORE INTO planning_draft_lifecycle_events (id, draft_id, event_kind, idempotency_key, actor_id, recorded_at) VALUES (?1, ?2, 'draft_begun', ?3, 'application-user', ?4)", params![new_id("draft-event"), draft.as_str(), format!("begin:{session_id}"), timestamp(now)]).map_err(sql_error("record draft begin"))?;
        transaction.execute("INSERT OR IGNORE INTO capability_profiles (id, status, created_at) VALUES (?1, 'active', ?2)", params![profile.as_str(), timestamp(now)]).map_err(sql_error("bootstrap capability profile"))?;
        transaction.execute("INSERT OR IGNORE INTO planning_draft_agent_session_associations (id, draft_id, agent_session_id, actor_id, associated_at) VALUES (?1, ?2, ?3, 'managed-plan-builder', ?4)", params![association.as_str(), draft.as_str(), session_id, timestamp(now)]).map_err(sql_error("bootstrap Agent Session association"))?;
        transaction.execute("INSERT OR IGNORE INTO planning_draft_profile_assignments (draft_id, capability_profile_id, agent_session_association_id, expires_at, assigned_at) VALUES (?1, ?2, ?3, '2100-01-01T00:00:00.000Z', ?4)", params![draft.as_str(), profile.as_str(), association.as_str(), timestamp(now)]).map_err(sql_error("bootstrap capability assignment"))?;
        transaction
            .commit()
            .map_err(sql_error("commit managed Plan Builder bootstrap"))?;
        Ok((draft, profile, association))
    }

    pub(crate) fn load_managed_plan_builder_binding(
        &self,
        session_id: &str,
    ) -> Result<Option<ManagedPlanBuilderBinding>, SaveProposalError> {
        let connection = self.lock()?;
        let associated_at = connection
            .query_row(
                "SELECT association.associated_at
                 FROM planning_draft_agent_session_associations association
                 JOIN planning_draft_profile_assignments assignment
                   ON assignment.agent_session_association_id=association.id
                  AND assignment.draft_id=association.draft_id
                 WHERE association.agent_session_id=?1
                   AND association.actor_id='managed-plan-builder'
                 ORDER BY association.associated_at,association.id
                 LIMIT 1",
                params![session_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(sql_error("load managed Plan Builder binding"))?;
        associated_at
            .map(|value| {
                DateTime::parse_from_rfc3339(&value)
                    .map(|associated_at| associated_at.with_timezone(&Utc))
                    .map(|associated_at| ManagedPlanBuilderBinding { associated_at })
                    .map_err(|error| {
                        SaveProposalError::Unavailable(format!(
                            "load managed Plan Builder binding timestamp: {error}"
                        ))
                    })
            })
            .transpose()
    }

    pub(crate) fn update_planning_draft_title(
        &self,
        draft_id: &EpicPlanningDraftId,
        session_id: &str,
        title: Option<&str>,
        idempotency_key: &str,
    ) -> Result<(), SaveProposalError> {
        let title = title.map(str::trim).filter(|value| !value.is_empty());
        if title.is_some_and(|value| value.len() > 240) || idempotency_key.trim().is_empty() {
            return Err(SaveProposalError::InvalidInput(
                "draft title or idempotency key is invalid".into(),
            ));
        }
        let now = timestamp(self.clock.now());
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_error("begin title update"))?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT draft_id FROM planning_draft_lifecycle_events WHERE idempotency_key = ?1",
                params![idempotency_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_error("read title idempotency"))?;
        if let Some(existing) = existing {
            return if existing == draft_id.as_str() {
                Ok(())
            } else {
                Err(SaveProposalError::IdempotencyConflict)
            };
        }
        let changed = transaction.execute("UPDATE epic_planning_drafts SET title = ?1, updated_at = ?2 WHERE id = ?3 AND status = 'active' AND NOT EXISTS (SELECT 1 FROM initiated_planning_drafts WHERE draft_id = ?3) AND EXISTS (SELECT 1 FROM planning_draft_agent_session_associations WHERE draft_id = ?3 AND agent_session_id = ?4 AND actor_id = 'managed-plan-builder')", params![title, now, draft_id.as_str(), session_id]).map_err(sql_error("update draft title"))?;
        if changed == 0 {
            return Err(SaveProposalError::Forbidden);
        }
        transaction.execute("INSERT INTO planning_draft_lifecycle_events (id, draft_id, event_kind, idempotency_key, actor_id, recorded_at) VALUES (?1, ?2, 'draft_title_updated', ?3, 'application-user', ?4)", params![new_id("draft-event"), draft_id.as_str(), idempotency_key, now]).map_err(sql_error("record title update"))?;
        transaction
            .commit()
            .map_err(sql_error("commit title update"))?;
        Ok(())
    }

    pub(crate) fn cancel_planning_draft(
        &self,
        draft_id: &EpicPlanningDraftId,
        session_id: &str,
        idempotency_key: &str,
    ) -> Result<(), SaveProposalError> {
        if idempotency_key.trim().is_empty() {
            return Err(SaveProposalError::InvalidInput(
                "idempotency key is required".into(),
            ));
        }
        let now = timestamp(self.clock.now());
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction()
            .map_err(sql_error("begin draft cancellation"))?;
        let existing: Option<String> = transaction
            .query_row(
                "SELECT draft_id FROM planning_draft_lifecycle_events WHERE idempotency_key = ?1",
                params![idempotency_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(sql_error("read cancel idempotency"))?;
        if let Some(existing) = existing {
            return if existing == draft_id.as_str() {
                Ok(())
            } else {
                Err(SaveProposalError::IdempotencyConflict)
            };
        }
        let associated: bool = transaction.query_row("SELECT EXISTS(SELECT 1 FROM planning_draft_agent_session_associations WHERE draft_id = ?1 AND agent_session_id = ?2 AND actor_id = 'managed-plan-builder')", params![draft_id.as_str(), session_id], |row| row.get(0)).map_err(sql_error("authorize cancellation"))?;
        if !associated {
            return Err(SaveProposalError::Forbidden);
        }
        let changed = transaction.execute("UPDATE epic_planning_drafts SET status = 'canceled', canceled_at = COALESCE(canceled_at, ?1), updated_at = ?1 WHERE id = ?2 AND status = 'active' AND NOT EXISTS (SELECT 1 FROM initiated_planning_drafts WHERE draft_id = ?2)", params![now, draft_id.as_str()]).map_err(sql_error("cancel draft"))?;
        if changed == 0 {
            return Err(SaveProposalError::Forbidden);
        }
        transaction.execute("INSERT INTO planning_draft_lifecycle_events (id, draft_id, event_kind, idempotency_key, actor_id, recorded_at) VALUES (?1, ?2, 'draft_canceled', ?3, 'application-user', ?4)", params![new_id("draft-event"), draft_id.as_str(), idempotency_key, now]).map_err(sql_error("record cancellation"))?;
        transaction
            .commit()
            .map_err(sql_error("commit cancellation"))?;
        Ok(())
    }

    pub(crate) fn save_epic_plan_proposal(
        &self,
        command: SaveEpicPlanProposalCommand,
    ) -> Result<SaveProposalResult, SaveProposalError> {
        command
            .validate()
            .map_err(SaveProposalError::InvalidInput)?;
        let fingerprint = fingerprint(&command)?;
        let effect_time = self.clock.now();
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_error("begin proposal save"))?;
        let existing = find_command_result(&transaction, &command.idempotency_key)?;
        if let Some((stored_fingerprint, _)) = &existing {
            if stored_fingerprint != &fingerprint {
                return Err(SaveProposalError::IdempotencyConflict);
            }
        }
        let draft_exists = transaction
            .query_row(
                "SELECT 1 FROM epic_planning_drafts WHERE id = ?1 AND status = 'active' AND NOT EXISTS (SELECT 1 FROM initiated_planning_drafts WHERE draft_id = epic_planning_drafts.id)",
                params![command.epic_planning_draft_id.as_str()],
                |_| Ok(()),
            )
            .optional()
            .map_err(sql_error("read planning draft"))?
            .is_some();
        if !draft_exists {
            return Err(SaveProposalError::DraftNotFound);
        }
        let authorized = transaction.query_row(
            "SELECT 1 FROM planning_draft_profile_assignments assignment JOIN planning_draft_agent_session_associations association ON association.id = assignment.agent_session_association_id JOIN capability_profiles profile ON profile.id = assignment.capability_profile_id WHERE assignment.draft_id = ?1 AND assignment.capability_profile_id = ?2 AND assignment.agent_session_association_id = ?3 AND association.actor_id = ?4 AND association.agent_session_id = ?5 AND association.draft_id = ?1 AND assignment.expires_at >= ?6 AND profile.status = 'active'",
            params![command.epic_planning_draft_id.as_str(), command.capability_profile_id.as_str(), command.agent_session_association_id.as_str(), command.actor_id, command.agent_session_id, timestamp(effect_time)], |_| Ok(())
        ).optional().map_err(sql_error("authorize proposal save"))?.is_some();
        if !authorized {
            return Err(SaveProposalError::Forbidden);
        }
        if let Some((_, mut result)) = existing {
            transaction
                .commit()
                .map_err(sql_error("commit authorized idempotent proposal save"))?;
            result.idempotent_replay = true;
            return Ok(result);
        }
        let latest: Option<(String, String)> = transaction.query_row(
            "SELECT id, revision_token FROM proposal_revisions WHERE draft_id = ?1 ORDER BY recorded_at DESC, id DESC LIMIT 1", params![command.epic_planning_draft_id.as_str()], |row| Ok((row.get(0)?, row.get(1)?))
        ).optional().map_err(sql_error("read current proposal revision"))?;
        if latest.as_ref().map(|(_, token)| token.as_str()) != command.expected_revision.as_deref()
        {
            return Err(SaveProposalError::RevisionConflict);
        }

        let command_id = ProposalCommandId::new(new_id("proposal-command"))
            .map_err(SaveProposalError::Unavailable)?;
        let result_id = ProposalResultId::new(new_id("proposal-result"))
            .map_err(SaveProposalError::Unavailable)?;
        let revision_id = ProposalRevisionId::new(new_id("proposal-revision"))
            .map_err(SaveProposalError::Unavailable)?;
        let event_id = ProposalEventId::new(new_id("proposal-event"))
            .map_err(SaveProposalError::Unavailable)?;
        let provenance_id = EffectProvenanceId::new(new_id("effect-provenance"))
            .map_err(SaveProposalError::Unavailable)?;
        let revision_token = new_id("proposal-revision-token");
        let proposal_json = serde_json::to_string(&command.proposal)
            .map_err(|error| SaveProposalError::Unavailable(error.to_string()))?;
        transaction.execute("INSERT INTO proposal_commands (id, idempotency_key, draft_id, capability_profile_id, agent_session_association_id, actor_id, expected_revision_token, proposal_json, payload_fingerprint, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10)", params![command_id.as_str(), command.idempotency_key, command.epic_planning_draft_id.as_str(), command.capability_profile_id.as_str(), command.agent_session_association_id.as_str(), command.actor_id, command.expected_revision, proposal_json, fingerprint, timestamp(effect_time)]).map_err(sql_error("record applied proposal command"))?;
        transaction.execute("INSERT INTO effect_provenance (id, source_kind, recorded_at, actor_id, agent_session_association_id, capability_profile_id, causal_command_id, causal_result_id) VALUES (?1, 'managed_plan_builder', ?2, ?3, ?4, ?5, ?6, ?7)", params![provenance_id.as_str(), timestamp(effect_time), command.actor_id, command.agent_session_association_id.as_str(), command.capability_profile_id.as_str(), command_id.as_str(), result_id.as_str()]).map_err(sql_error("record effect provenance"))?;
        let proposal_json = serde_json::to_string(&command.proposal)
            .map_err(|error| SaveProposalError::Unavailable(error.to_string()))?;
        transaction.execute("INSERT INTO proposal_revisions (id, draft_id, parent_revision_id, revision_token, proposal_json, command_id, provenance_id, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)", params![revision_id.as_str(), command.epic_planning_draft_id.as_str(), latest.map(|(id, _)| id), revision_token, proposal_json, command_id.as_str(), provenance_id.as_str(), timestamp(effect_time)]).map_err(sql_error("record proposal revision"))?;
        transaction.execute("INSERT INTO proposal_events (id, draft_id, revision_id, command_id, provenance_id, event_kind, recorded_at) VALUES (?1, ?2, ?3, ?4, ?5, 'proposal_saved', ?6)", params![event_id.as_str(), command.epic_planning_draft_id.as_str(), revision_id.as_str(), command_id.as_str(), provenance_id.as_str(), timestamp(effect_time)]).map_err(sql_error("append proposal event"))?;
        transaction.execute("INSERT INTO proposal_command_results (id, command_id, revision_id, event_id, provenance_id, created_at) VALUES (?1, ?2, ?3, ?4, ?5, ?6)", params![result_id.as_str(), command_id.as_str(), revision_id.as_str(), event_id.as_str(), provenance_id.as_str(), timestamp(effect_time)]).map_err(sql_error("record proposal command result"))?;
        let result = SaveProposalResult {
            command_id,
            result_id,
            revision_id,
            revision_token,
            event_id,
            provenance_id,
            idempotent_replay: false,
        };
        transaction
            .commit()
            .map_err(sql_error("commit proposal save"))?;
        Ok(result)
    }

    pub(crate) fn native_query_at(
        &self,
        generated_at: DateTime<Utc>,
    ) -> Result<NativeQueryV2, String> {
        let connection = self.lock().map_err(|error| error.to_string())?;
        let mut draft_statement = connection.prepare("SELECT draft.id, draft.title, CASE WHEN initiated.draft_id IS NOT NULL THEN 'initiated' ELSE draft.status END, draft.created_at, draft.updated_at, draft.canceled_at, latest.id FROM epic_planning_drafts draft LEFT JOIN initiated_planning_drafts initiated ON initiated.draft_id = draft.id LEFT JOIN proposal_revisions latest ON latest.id = (SELECT revision.id FROM proposal_revisions revision WHERE revision.draft_id = draft.id ORDER BY revision.recorded_at DESC, revision.id DESC LIMIT 1) ORDER BY draft.created_at, draft.id").map_err(|error| error.to_string())?;
        let planning_drafts = draft_statement
            .query_map([], |row| {
                Ok(PlanningDraftDto {
                    epic_planning_draft_id: row.get(0)?,
                    title: row.get(1)?,
                    status: row.get(2)?,
                    created_at: row.get(3)?,
                    updated_at: row.get(4)?,
                    canceled_at: row.get(5)?,
                    current_proposal: match row.get::<_, Option<String>>(6)? {
                        Some(revision_id) => CurrentProposalDto::Available {
                            proposal_revision_id: revision_id,
                        },
                        None => CurrentProposalDto::Empty {},
                    },
                })
            })
            .map_err(|error| error.to_string())?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| error.to_string())?;
        let agent_session_associations = collect(&connection, "SELECT id, draft_id, agent_session_id, actor_id, associated_at FROM planning_draft_agent_session_associations ORDER BY associated_at, id", |row| Ok(AgentSessionAssociationDto { agent_session_association_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, agent_session_id: row.get(2)?, actor_id: row.get(3)?, associated_at: row.get(4)? }))?;
        let proposal_revisions = collect(&connection, "SELECT id, draft_id, parent_revision_id, revision_token, proposal_json, command_id, provenance_id, recorded_at FROM proposal_revisions ORDER BY recorded_at, id", |row| Ok(ProposalRevisionDto { proposal_revision_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, parent_proposal_revision_id: row.get(2)?, revision_token: row.get(3)?, proposal: parse_proposal_json(row.get::<_, String>(4)?)?, command_id: row.get(5)?, provenance_id: row.get(6)?, recorded_at: row.get(7)? }))?;
        let recorded_proposal_events = collect(&connection, "SELECT id, draft_id, revision_id, command_id, provenance_id, event_kind, recorded_at FROM proposal_events ORDER BY recorded_at, id", |row| Ok(RecordedProposalEventDto { proposal_event_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, proposal_revision_id: row.get(2)?, command_id: row.get(3)?, provenance_id: row.get(4)?, event_kind: row.get(5)?, recorded_at: row.get(6)? }))?;
        let provenance_links = collect(&connection, "SELECT id, source_kind, recorded_at, actor_id, agent_session_association_id, capability_profile_id, causal_command_id, causal_result_id FROM effect_provenance ORDER BY recorded_at, id", |row| Ok(ProvenanceLinkDto { provenance_id: row.get(0)?, source_kind: row.get(1)?, recorded_at: row.get(2)?, actor_id: row.get(3)?, agent_session_association_id: row.get(4)?, capability_profile_id: row.get(5)?, causal_command_id: row.get(6)?, causal_result_id: row.get(7)? }))?;
        let initiation_commands = collect(&connection, "SELECT id, draft_id, expected_revision_token, actor_id, idempotency_key, payload_fingerprint, recorded_at FROM epic_initiation_commands ORDER BY recorded_at, id", |row| Ok(InitiationCommandDto { command_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, expected_revision_token: row.get(2)?, actor_id: row.get(3)?, idempotency_key: row.get(4)?, payload_fingerprint: row.get(5)?, recorded_at: row.get(6)? }))?;
        let initiation_results = collect(&connection, "SELECT id, command_id, recorded_at FROM epic_initiation_results ORDER BY recorded_at, id", |row| Ok(InitiationResultDto { result_id: row.get(0)?, command_id: row.get(1)?, recorded_at: row.get(2)? }))?;
        let initiation_events = collect(&connection, "SELECT id, command_id, result_id, recorded_at FROM epic_initiation_events ORDER BY recorded_at, id", |row| Ok(InitiationEventDto { event_id: row.get(0)?, command_id: row.get(1)?, result_id: row.get(2)?, recorded_at: row.get(3)? }))?;
        let initiation_provenance = collect(&connection, "SELECT id, command_id, result_id, event_id, recorded_at FROM epic_initiation_provenance ORDER BY recorded_at, id", |row| Ok(InitiationProvenanceDto { provenance_id: row.get(0)?, command_id: row.get(1)?, result_id: row.get(2)?, event_id: row.get(3)?, recorded_at: row.get(4)? }))?;
        let material_snapshots = collect(&connection, "SELECT id, draft_id, proposal_revision_id, version, proposal_json, content_hash, recorded_at FROM epic_initiation_material_snapshots ORDER BY recorded_at, id", |row| Ok(MaterialSnapshotDto { material_snapshot_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, proposal_revision_id: row.get(2)?, version: row.get(3)?, proposal: parse_proposal_json(row.get::<_, String>(4)?)?, content_hash: row.get(5)?, recorded_at: row.get(6)? }))?;
        let initiated_epics = collect(&connection, "SELECT id, draft_id, proposal_revision_id, material_snapshot_id, epic_id, recorded_at, command_id, result_id, event_id, provenance_id FROM epic_initiations ORDER BY recorded_at, id", |row| Ok(InitiatedEpicDto { initiation_id: row.get(0)?, epic_planning_draft_id: row.get(1)?, proposal_revision_id: row.get(2)?, material_snapshot_id: row.get(3)?, epic_id: row.get(4)?, recorded_at: row.get(5)?, command_id: row.get(6)?, result_id: row.get(7)?, event_id: row.get(8)?, provenance_id: row.get(9)? }))?;
        let initiated_sprints = collect(&connection, "SELECT id, epic_id, ordinal, title, intended_movement, concern_summaries_json, sprint_plan_id, sprint_plan_revision_id FROM initiated_sprints ORDER BY epic_id, ordinal", |row| Ok(InitiatedSprintDto { sprint_id: row.get(0)?, epic_id: row.get(1)?, ordinal: row.get(2)?, title: row.get(3)?, intended_movement: row.get(4)?, concern_summaries: serde_json::from_str(&row.get::<_, String>(5)?).map_err(|e| to_sql_error(e.to_string()))?, sprint_plan_id: row.get(6)?, sprint_plan_revision_id: row.get(7)? }))?;
        let file_review_documents = collect(&connection, "SELECT document.document_ref_id, document.epic_id, document.sprint_id, document.provenance_id, document.title, document.summary, artifact.artifact_id FROM file_review_documents document JOIN initiated_sprints sprint ON sprint.id = document.sprint_id AND sprint.epic_id = document.epic_id JOIN epic_initiations epic ON epic.epic_id = document.epic_id AND epic.provenance_id = document.provenance_id JOIN stored_file_review_artifacts artifact ON artifact.document_ref_id = document.document_ref_id AND artifact.provenance_id = document.provenance_id AND artifact.contract_version = 'stored-file-review-artifact/v1' AND artifact.payload_bytes > 0 AND artifact.payload_bytes <= 1000000 ORDER BY document.recorded_at, document.document_ref_id", |row| Ok(FileReviewDocumentDto { document_ref_id: row.get(0)?, epic_id: row.get(1)?, sprint_id: row.get(2)?, provenance_id: row.get(3)?, title: row.get(4)?, summary: row.get(5)?, artifact_id: row.get(6)?, changed_files: Vec::new() }))?;
        let mut file_review_documents = file_review_documents;
        for document in &mut file_review_documents {
            document.changed_files = connection.prepare("SELECT changed_file_reference_id, display_name, change_kind, previous_display_name FROM file_review_changed_files WHERE document_ref_id = ?1 ORDER BY ordinal").map_err(|e| e.to_string())?.query_map(params![document.document_ref_id], |row| Ok(FileReviewChangedFileDto { changed_file_reference_id: row.get(0)?, display_name: row.get(1)?, change_kind: row.get(2)?, previous_display_name: row.get(3)? })).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        }
        // The Sprint Runner owns creation of these tables.  Repository-only pre-Sprint stores
        // remain readable as an empty materialization projection.
        let materialization_tables = connection.query_row("SELECT EXISTS(SELECT 1 FROM sqlite_master WHERE type='table' AND name='work_unit_materializations')", [], |row| row.get::<_, bool>(0)).map_err(|e| e.to_string())?;
        let work_unit_materializations = if materialization_tables { collect(&connection, "SELECT materialization_id,planning_point_id,accepted_revision_id,epic_id,sprint_id,work_slice_id,authorization_recorded_at,attempt_recorded_at,work_units_created_at,relationships_completed_at,settled_at FROM work_unit_materializations ORDER BY authorization_recorded_at,materialization_id", |row| Ok(WorkUnitMaterializationDto { materialization_id:row.get(0)?, planning_point_id:row.get(1)?, accepted_revision_id:row.get(2)?, epic_id:row.get(3)?, sprint_id:row.get(4)?, work_slice_id:row.get(5)?, authorization_recorded_at:row.get(6)?, attempt_recorded_at:row.get(7)?, work_units_created_at:row.get(8)?, relationships_completed_at:row.get(9)?, settled_at:row.get(10)? }))? } else { Vec::new() };
        let work_units = if materialization_tables { collect(&connection, "SELECT work_unit_id,materialization_id,work_slice_id,accepted_revision_id,lane_ordinal,lane_title,specification FROM work_units ORDER BY materialization_id,lane_ordinal", |row| Ok(WorkUnitDto { work_unit_id:row.get(0)?, materialization_id:row.get(1)?, work_slice_id:row.get(2)?, accepted_revision_id:row.get(3)?, lane_ordinal:row.get(4)?, lane_title:row.get(5)?, specification:row.get(6)? }))? } else { Vec::new() };
        let work_unit_relationships = if materialization_tables { collect(&connection, "SELECT relationship_id,materialization_id,relationship_kind,from_id,to_id,ordinal FROM work_unit_relationships ORDER BY materialization_id,relationship_kind,from_id,to_id", |row| Ok(WorkUnitRelationshipDto { relationship_id:row.get(0)?, materialization_id:row.get(1)?, relationship_kind:row.get(2)?, from_id:row.get(3)?, to_id:row.get(4)?, ordinal:row.get(5)? }))? } else { Vec::new() };
        Ok(NativeQueryV2 {
            contract_version: NATIVE_QUERY_VERSION,
            generated_at: timestamp(generated_at),
            planning_drafts,
            agent_session_associations,
            proposal_revisions,
            recorded_proposal_events,
            provenance_links,
            initiation_commands,
            initiation_results,
            initiation_events,
            initiation_provenance,
            material_snapshots,
            initiated_epics,
            initiated_sprints,
            file_review_documents,
            work_unit_materializations,
            work_units,
            work_unit_relationships,
        })
    }

    /// Product-owned semantic transition. It consumes the current saved proposal atomically and
    /// creates only an Epic and its ordered preparatory Sprints.
    pub(crate) fn initiate_epic(
        &self,
        command: super::domain::InitiateEpicCommand,
    ) -> Result<super::domain::InitiateEpicResult, super::domain::InitiateEpicError> {
        use super::domain::{
            EpicId, EpicInitiationId, InitiateEpicError, InitiateEpicResult, ProposalRevisionId,
        };
        command.validate()?;
        if command.actor_id != "application-user" {
            return Err(InitiateEpicError::Forbidden);
        }
        let fingerprint = format!(
            "{}:{}:{}",
            command.epic_planning_draft_id.as_str(),
            command.expected_revision_token,
            command.actor_id
        );
        let now = timestamp(self.clock.now());
        let mut connection = self.connection.lock().map_err(|_| {
            InitiateEpicError::Unavailable("orchestration database lock is poisoned".into())
        })?;
        let tx = connection
            .transaction()
            .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        let existing: Option<(String, String, String, String, String)> = tx.query_row(
            "SELECT command.payload_fingerprint, initiation.id, initiation.epic_id, initiation.proposal_revision_id, snapshot.content_hash FROM epic_initiation_commands command JOIN epic_initiations initiation ON initiation.command_id = command.id JOIN epic_initiation_material_snapshots snapshot ON snapshot.id = initiation.material_snapshot_id WHERE command.idempotency_key = ?1",
            params![command.idempotency_key], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?, r.get(4)?))).optional().map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        if let Some((stored, initiation, epic, revision, hash)) = existing {
            if stored != fingerprint {
                return Err(InitiateEpicError::IdempotencyConflict);
            }
            return Ok(InitiateEpicResult {
                initiation_id: EpicInitiationId::new(initiation)
                    .map_err(InitiateEpicError::InvalidInput)?,
                epic_id: EpicId::new(epic).map_err(InitiateEpicError::InvalidInput)?,
                proposal_revision_id: ProposalRevisionId::new(revision)
                    .map_err(InitiateEpicError::InvalidInput)?,
                material_snapshot_hash: hash,
                idempotent_replay: true,
            });
        }
        let status: Option<String> = tx
            .query_row(
                "SELECT status FROM epic_planning_drafts WHERE id=?1",
                params![command.epic_planning_draft_id.as_str()],
                |r| r.get(0),
            )
            .optional()
            .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        if tx
            .query_row(
                "SELECT 1 FROM initiated_planning_drafts WHERE draft_id=?1",
                params![command.epic_planning_draft_id.as_str()],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?
            .is_some()
        {
            return Err(InitiateEpicError::AlreadyInitiated);
        }
        match status.as_deref() {
            Some("active") => {}
            Some("canceled") => return Err(InitiateEpicError::Canceled),
            Some(_) => return Err(InitiateEpicError::AlreadyInitiated),
            None => return Err(InitiateEpicError::DraftNotFound),
        }
        let latest: Option<(String, String, String)> = tx.query_row("SELECT id, revision_token, proposal_json FROM proposal_revisions WHERE draft_id=?1 ORDER BY recorded_at DESC, id DESC LIMIT 1", params![command.epic_planning_draft_id.as_str()], |r| Ok((r.get(0)?,r.get(1)?,r.get(2)?))).optional().map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        let (revision_id, token, proposal_json) =
            latest.ok_or(InitiateEpicError::ProposalMissing)?;
        if token != command.expected_revision_token {
            return Err(InitiateEpicError::RevisionConflict);
        }
        let proposal = parse_proposal_json(proposal_json.clone())
            .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        let command_id = new_id("epic-initiation-command");
        let result_id = new_id("epic-initiation-result");
        let event_id = new_id("epic-initiation-event");
        let provenance_id = new_id("epic-initiation-provenance");
        let snapshot_id = new_id("epic-material-snapshot");
        let initiation_id = new_id("epic-initiation");
        let epic_id = new_id("epic");
        let hash = format!("{:x}", Sha256::digest(proposal_json.as_bytes()));
        tx.execute("INSERT INTO epic_initiation_commands (id,idempotency_key,draft_id,expected_revision_token,actor_id,payload_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7)", params![command_id,command.idempotency_key,command.epic_planning_draft_id.as_str(),command.expected_revision_token,command.actor_id,fingerprint,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.execute(
            "INSERT INTO epic_initiation_results (id,command_id,recorded_at) VALUES (?1,?2,?3)",
            params![result_id, command_id, now],
        )
        .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.execute("INSERT INTO epic_initiation_events (id,command_id,result_id,recorded_at) VALUES (?1,?2,?3,?4)", params![event_id,command_id,result_id,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.execute("INSERT INTO epic_initiation_provenance (id,command_id,result_id,event_id,recorded_at) VALUES (?1,?2,?3,?4,?5)", params![provenance_id,command_id,result_id,event_id,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.execute("INSERT INTO epic_initiation_material_snapshots (id,draft_id,proposal_revision_id,version,proposal_json,content_hash,recorded_at) VALUES (?1,?2,?3,1,?4,?5,?6)", params![snapshot_id,command.epic_planning_draft_id.as_str(),revision_id,proposal_json,hash,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.execute("INSERT INTO epic_initiations (id,command_id,result_id,event_id,provenance_id,draft_id,proposal_revision_id,material_snapshot_id,epic_id,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![initiation_id,command_id,result_id,event_id,provenance_id,command.epic_planning_draft_id.as_str(),revision_id,snapshot_id,epic_id,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        for (ordinal, sprint) in proposal.sprints.iter().enumerate() {
            let sprint_id = new_id("sprint");
            tx.execute("INSERT INTO initiated_sprints (id,epic_id,ordinal,title,intended_movement,concern_summaries_json,sprint_plan_id,sprint_plan_revision_id) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)", params![sprint_id, epic_id, ordinal as i64, sprint.title, sprint.intended_movement, serde_json::to_string(&sprint.concern_summaries).unwrap(), new_id("sprint-plan"), new_id("sprint-plan-revision")]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        }
        tx.execute("INSERT INTO initiated_planning_drafts (draft_id,initiation_id,initiated_at) VALUES (?1,?2,?3)", params![command.epic_planning_draft_id.as_str(),initiation_id,now]).map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        tx.commit()
            .map_err(|e| InitiateEpicError::Unavailable(e.to_string()))?;
        Ok(InitiateEpicResult {
            initiation_id: EpicInitiationId::new(initiation_id)
                .map_err(InitiateEpicError::InvalidInput)?,
            epic_id: EpicId::new(epic_id).map_err(InitiateEpicError::InvalidInput)?,
            proposal_revision_id: ProposalRevisionId::new(revision_id)
                .map_err(InitiateEpicError::InvalidInput)?,
            material_snapshot_hash: hash,
            idempotent_replay: false,
        })
    }

    pub(crate) fn native_query(&self) -> Result<NativeQueryV2, String> {
        self.native_query_at(self.clock.now())
    }

    /// Internal application command. It persists a complete dirty draft only; it does not commit,
    /// activate, bind, or apply a Harness configuration.
    pub(crate) fn save_harness_working_copy(
        &self,
        command: SaveHarnessWorkingCopyCommand,
    ) -> Result<SaveHarnessWorkingCopyResult, HarnessWorkingCopyError> {
        validate_harness_working_copy_command(&command)?;
        let fingerprint = harness_working_copy_command_fingerprint(&command)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| HarnessWorkingCopyError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| HarnessWorkingCopyError::Unavailable)?;

        let replay: Option<(String, String, i64, i64, String, String, String)> = transaction
            .query_row(
                "SELECT payload_fingerprint,harness_key,expected_current_revision,result_revision,result_json,result_digest,recorded_at FROM harness_working_copy_commands WHERE idempotency_key=?1",
                [&command.idempotency_key],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?, row.get(6)?)),
            )
            .optional()
            .map_err(|_| HarnessWorkingCopyError::Unavailable)?;
        if let Some((
            existing_fingerprint,
            harness_key,
            expected_revision,
            result_revision,
            result_json,
            result_digest,
            recorded_at,
        )) = replay
        {
            if existing_fingerprint != fingerprint {
                return Err(HarnessWorkingCopyError::Conflict);
            }
            if result_digest != format!("{:x}", Sha256::digest(result_json.as_bytes()))
                || harness_key != command.harness_key
                || expected_revision != command.expected_current_revision as i64
                || result_revision != expected_revision + 1
            {
                return Err(HarnessWorkingCopyError::InvalidStoredState);
            }
            let result = decode_harness_working_copy_result(&result_json)?;
            if result.harness_key != command.harness_key
                || result.draft_revision != command.expected_current_revision + 1
                || result.configuration != command.configuration
                || result.editor != command.editor
                || timestamp(result.saved_at) != recorded_at
            {
                return Err(HarnessWorkingCopyError::InvalidStoredState);
            }
            return Ok(SaveHarnessWorkingCopyResult::IdempotentReplay(result));
        }

        let current =
            load_harness_working_copy_from_connection(&transaction, &command.harness_key)?;
        let current_revision = current.as_ref().map_or(0, |copy| copy.draft_revision);
        if current_revision != command.expected_current_revision {
            return Err(HarnessWorkingCopyError::Conflict);
        }
        let draft_revision = current_revision
            .checked_add(1)
            .filter(|revision| *revision <= i64::MAX as u64)
            .ok_or(HarnessWorkingCopyError::Invalid)?;
        let saved_at = self.clock.now();
        let working_copy = HarnessWorkingCopy {
            harness_key: command.harness_key.clone(),
            configuration: command.configuration.clone(),
            draft_revision,
            dirty: true,
            editor: command.editor.clone(),
            saved_at,
        };
        validate_working_copy(&working_copy)?;
        let envelope = HarnessEffectiveConfigurationEnvelope {
            contract_version: HARNESS_EFFECTIVE_CONFIGURATION_V1.into(),
            configuration: command.configuration,
        };
        let configuration_json =
            serde_json::to_string(&envelope).map_err(|_| HarnessWorkingCopyError::Invalid)?;
        let saved_at_text = timestamp(saved_at);
        let working_copy_digest = harness_working_copy_digest(
            &command.harness_key,
            HARNESS_EFFECTIVE_CONFIGURATION_V1,
            &configuration_json,
            draft_revision as i64,
            1,
            command.editor.kind.as_str(),
            &command.editor.reference,
            &saved_at_text,
        );
        let result_json =
            serde_json::to_string(&working_copy).map_err(|_| HarnessWorkingCopyError::Invalid)?;
        let result_digest = format!("{:x}", Sha256::digest(result_json.as_bytes()));
        transaction
            .execute(
                "INSERT INTO harness_working_copies (harness_key,configuration_contract_version,configuration_json,working_copy_digest,draft_revision,dirty,editor_kind,editor_reference,saved_at) VALUES (?1,?2,?3,?4,?5,1,?6,?7,?8) ON CONFLICT(harness_key) DO UPDATE SET configuration_contract_version=excluded.configuration_contract_version,configuration_json=excluded.configuration_json,working_copy_digest=excluded.working_copy_digest,draft_revision=excluded.draft_revision,dirty=1,editor_kind=excluded.editor_kind,editor_reference=excluded.editor_reference,saved_at=excluded.saved_at",
                params![
                    command.harness_key,
                    HARNESS_EFFECTIVE_CONFIGURATION_V1,
                    configuration_json,
                    working_copy_digest,
                    draft_revision as i64,
                    command.editor.kind.as_str(),
                    command.editor.reference,
                    saved_at_text,
                ],
            )
            .map_err(|_| HarnessWorkingCopyError::Unavailable)?;
        transaction
            .execute(
                "INSERT INTO harness_working_copy_commands (idempotency_key,payload_fingerprint,harness_key,expected_current_revision,result_revision,result_json,result_digest,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8)",
                params![
                    command.idempotency_key,
                    fingerprint,
                    working_copy.harness_key,
                    command.expected_current_revision as i64,
                    draft_revision as i64,
                    result_json,
                    result_digest,
                    saved_at_text,
                ],
            )
            .map_err(|_| HarnessWorkingCopyError::Unavailable)?;
        transaction
            .commit()
            .map_err(|_| HarnessWorkingCopyError::Unavailable)?;
        Ok(SaveHarnessWorkingCopyResult::Stored(working_copy))
    }

    /// Private read for later Harness commit/version work. No transport or NativeQuery exposes it.
    pub(crate) fn load_harness_working_copy(
        &self,
        harness_key: &str,
    ) -> Result<Option<HarnessWorkingCopy>, HarnessWorkingCopyError> {
        validate_harness_key(harness_key)?;
        let connection = self
            .connection
            .lock()
            .map_err(|_| HarnessWorkingCopyError::Unavailable)?;
        load_harness_working_copy_from_connection(&connection, harness_key)
    }

    /// Publishes the exact current working-copy revision through the application-owned local
    /// repository. The command carries no configuration bytes or repository authority.
    pub(crate) fn create_harness_revision(
        &self,
        command: CreateHarnessRevisionCommand,
    ) -> Result<CreateHarnessRevisionResult, HarnessRevisionError> {
        validate_harness_revision_command(&command)?;
        let fingerprint = harness_revision_command_fingerprint(&command)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| HarnessRevisionError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| HarnessRevisionError::Unavailable)?;

        let replay: Option<(String, String, i64, Option<String>, String, String, String, String)> =
            transaction
                .query_row(
                    "SELECT payload_fingerprint,harness_key,expected_source_draft_revision,expected_predecessor_revision_id,result_revision_id,result_json,result_digest,recorded_at FROM harness_revision_commands WHERE idempotency_key=?1",
                    [&command.idempotency_key],
                    |row| {
                        Ok((
                            row.get(0)?,
                            row.get(1)?,
                            row.get(2)?,
                            row.get(3)?,
                            row.get(4)?,
                            row.get(5)?,
                            row.get(6)?,
                            row.get(7)?,
                        ))
                    },
                )
                .optional()
                .map_err(|_| HarnessRevisionError::Unavailable)?;
        if let Some((
            existing_fingerprint,
            harness_key,
            source_draft_revision,
            expected_predecessor,
            result_revision_id,
            result_json,
            result_digest,
            recorded_at,
        )) = replay
        {
            if existing_fingerprint != fingerprint {
                return Err(HarnessRevisionError::Conflict);
            }
            if harness_key != command.harness_key
                || source_draft_revision != command.expected_source_draft_revision as i64
                || expected_predecessor != command.expected_predecessor_revision_id
                || result_digest != format!("{:x}", Sha256::digest(result_json.as_bytes()))
            {
                return Err(HarnessRevisionError::InvalidStoredState);
            }
            let recorded: HarnessRevision = serde_json::from_str(&result_json)
                .map_err(|_| HarnessRevisionError::InvalidStoredState)?;
            validate_revision(&recorded)?;
            if recorded.revision_id != result_revision_id
                || recorded.harness_key != command.harness_key
                || recorded.source_draft_revision != command.expected_source_draft_revision
                || recorded.predecessor_revision_id != command.expected_predecessor_revision_id
                || recorded.creation_provenance != command.creation_provenance
                || timestamp(recorded.created_at) != recorded_at
            {
                return Err(HarnessRevisionError::InvalidStoredState);
            }
            let verified = load_verified_harness_revision_from_connection(
                &transaction,
                &self.harness_revisions,
                &recorded.revision_id,
            )?
            .ok_or(HarnessRevisionError::InvalidStoredState)?;
            if verified != recorded {
                return Err(HarnessRevisionError::InvalidStoredState);
            }
            return Ok(CreateHarnessRevisionResult::IdempotentReplay(verified));
        }

        let working_copy =
            load_harness_working_copy_from_connection(&transaction, &command.harness_key)?
                .ok_or(HarnessRevisionError::MissingWorkingCopy)?;
        if working_copy.draft_revision != command.expected_source_draft_revision {
            return Err(HarnessRevisionError::Conflict);
        }
        let existing = load_verified_harness_revision_history_from_connection(
            &transaction,
            &self.harness_revisions,
            &command.harness_key,
        )?;
        if existing
            .last()
            .is_some_and(|revision| revision.source_draft_revision >= working_copy.draft_revision)
        {
            return Err(HarnessRevisionError::Conflict);
        }
        let current_head = existing.last().map(|revision| revision.revision_id.clone());
        if current_head != command.expected_predecessor_revision_id {
            return Err(HarnessRevisionError::Conflict);
        }

        let (normalized_envelope, configuration_digest) =
            normalized_configuration_envelope(&working_copy.configuration)?;
        let created_at = self.clock.now();
        let revision_id = revision_id();
        let repository_commit_ref = LocalHarnessRevisionRepository::commit_reference(&revision_id);
        let revision = HarnessRevision {
            revision_id,
            harness_key: command.harness_key.clone(),
            configuration: working_copy.configuration,
            configuration_digest,
            source_draft_revision: working_copy.draft_revision,
            predecessor_revision_id: current_head,
            repository_commit_ref,
            creation_provenance: command.creation_provenance.clone(),
            created_at,
        };
        validate_revision(&revision)?;
        let manifest = HarnessRevisionCommitManifest::for_revision(&revision);
        self.harness_revisions
            .install_and_verify(&manifest, &normalized_envelope)
            .map_err(map_local_harness_revision_error)?;

        let created_at_text = timestamp(created_at);
        let result_json =
            serde_json::to_string(&revision).map_err(|_| HarnessRevisionError::Invalid)?;
        let result_digest = format!("{:x}", Sha256::digest(result_json.as_bytes()));
        transaction
            .execute(
                "INSERT INTO harness_revisions (revision_id,harness_key,configuration_contract_version,configuration_digest,source_draft_revision,predecessor_revision_id,repository_commit_ref,creation_provenance_kind,creation_provenance_reference,created_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)",
                params![
                    revision.revision_id,
                    revision.harness_key,
                    HARNESS_EFFECTIVE_CONFIGURATION_V1,
                    revision.configuration_digest,
                    revision.source_draft_revision as i64,
                    revision.predecessor_revision_id,
                    revision.repository_commit_ref,
                    revision.creation_provenance.kind.as_str(),
                    revision.creation_provenance.reference,
                    created_at_text,
                ],
            )
            .map_err(|_| HarnessRevisionError::Unavailable)?;
        transaction
            .execute(
                "INSERT INTO harness_revision_publications (revision_id,harness_key,repository_commit_ref,evidence_kind,verified_at) VALUES (?1,?2,?3,'local_commit_verified',?4)",
                params![
                    revision.revision_id,
                    revision.harness_key,
                    revision.repository_commit_ref,
                    created_at_text,
                ],
            )
            .map_err(|_| HarnessRevisionError::Unavailable)?;
        transaction
            .execute(
                "INSERT INTO harness_revision_commands (idempotency_key,payload_fingerprint,harness_key,expected_source_draft_revision,expected_predecessor_revision_id,result_revision_id,result_json,result_digest,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9)",
                params![
                    command.idempotency_key,
                    fingerprint,
                    command.harness_key,
                    command.expected_source_draft_revision as i64,
                    command.expected_predecessor_revision_id,
                    revision.revision_id,
                    result_json,
                    result_digest,
                    created_at_text,
                ],
            )
            .map_err(|_| HarnessRevisionError::Unavailable)?;
        transaction
            .commit()
            .map_err(|_| HarnessRevisionError::Unavailable)?;
        Ok(CreateHarnessRevisionResult::Published(revision))
    }

    pub(crate) fn load_harness_revision(&self, revision_id: &str) -> HarnessRevisionReadOutcome {
        let connection = match self.connection.lock() {
            Ok(connection) => connection,
            Err(_) => return HarnessRevisionReadOutcome::Unavailable,
        };
        match load_verified_harness_revision_from_connection(
            &connection,
            &self.harness_revisions,
            revision_id,
        ) {
            Ok(Some(revision)) => HarnessRevisionReadOutcome::AvailableAndVerified { revision },
            Ok(None) => HarnessRevisionReadOutcome::Missing,
            Err(HarnessRevisionError::InvalidStoredState)
            | Err(HarnessRevisionError::InvalidLocalCommitEvidence) => {
                HarnessRevisionReadOutcome::InvalidLocalCommitEvidence
            }
            Err(_) => HarnessRevisionReadOutcome::Unavailable,
        }
    }

    pub(crate) fn load_harness_revision_history(
        &self,
        harness_key: &str,
    ) -> HarnessRevisionHistoryOutcome {
        if validate_harness_key(harness_key).is_err() {
            return HarnessRevisionHistoryOutcome::Missing;
        }
        let connection = match self.connection.lock() {
            Ok(connection) => connection,
            Err(_) => return HarnessRevisionHistoryOutcome::Unavailable,
        };
        match load_verified_harness_revision_history_from_connection(
            &connection,
            &self.harness_revisions,
            harness_key,
        ) {
            Ok(revisions) if revisions.is_empty() => HarnessRevisionHistoryOutcome::Missing,
            Ok(revisions) => HarnessRevisionHistoryOutcome::AvailableAndVerified { revisions },
            Err(HarnessRevisionError::InvalidStoredState)
            | Err(HarnessRevisionError::InvalidLocalCommitEvidence) => {
                HarnessRevisionHistoryOutcome::InvalidLocalCommitEvidence
            }
            Err(_) => HarnessRevisionHistoryOutcome::Unavailable,
        }
    }

    /// Control-side write seam. There is intentionally no Tauri command for this authority.
    pub(crate) fn store_file_review_git_capture_authorization(
        &self,
        value: FileReviewGitCaptureAuthorizationWrite,
    ) -> Result<StoreFileReviewGitCaptureAuthorizationResult, FileReviewGitCaptureAuthorizationError>
    {
        validate_git_capture_authorization(&value)?;
        let fingerprint = git_capture_authorization_fingerprint(&value);
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| FileReviewGitCaptureAuthorizationError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| FileReviewGitCaptureAuthorizationError::Unavailable)?;
        let owned: Option<i64> = tx.query_row("SELECT 1 FROM initiated_sprints sprint JOIN epic_initiations epic ON epic.epic_id=sprint.epic_id AND epic.provenance_id=?3 WHERE sprint.id=?1 AND sprint.epic_id=?2", params![value.sprint_id, value.epic_id, value.provenance_id], |r| r.get(0)).optional().map_err(|_| FileReviewGitCaptureAuthorizationError::Unavailable)?;
        if owned.is_none() {
            return Err(FileReviewGitCaptureAuthorizationError::Forbidden);
        }
        let existing: Option<String> = tx.query_row("SELECT payload_fingerprint FROM file_review_git_capture_authorizations WHERE idempotency_key=?1 OR capture_authorization_id=?2", params![value.idempotency_key, value.capture_authorization_id], |r| r.get(0)).optional().map_err(|_| FileReviewGitCaptureAuthorizationError::Unavailable)?;
        if let Some(existing) = existing {
            return if existing == fingerprint {
                Ok(StoreFileReviewGitCaptureAuthorizationResult::IdempotentReplay)
            } else {
                Err(FileReviewGitCaptureAuthorizationError::Conflict)
            };
        }
        tx.execute("INSERT INTO file_review_git_capture_authorizations (capture_authorization_id,idempotency_key,payload_fingerprint,epic_id,sprint_id,provenance_id,repository_id,repository_root,worktree_id,worktree_root,baseline_object_id,current_object_id,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13)", params![value.capture_authorization_id, value.idempotency_key, fingerprint, value.epic_id, value.sprint_id, value.provenance_id, value.repository_id, value.repository_root, value.worktree_id, value.worktree_root, value.baseline_object_id, value.current_object_id, timestamp(self.clock.now())]).map_err(|_| FileReviewGitCaptureAuthorizationError::Unavailable)?;
        tx.commit()
            .map_err(|_| FileReviewGitCaptureAuthorizationError::Unavailable)?;
        Ok(StoreFileReviewGitCaptureAuthorizationResult::Stored)
    }

    /// Producer-facing capability: the opaque authorization identity is its sole input.
    pub(crate) fn load_file_review_git_capture_authorization(
        &self,
        capture_authorization_id: &str,
    ) -> Result<Option<FileReviewGitCaptureAuthorization>, FileReviewGitCaptureAuthorizationError>
    {
        if capture_authorization_id.trim().is_empty() {
            return Err(FileReviewGitCaptureAuthorizationError::Invalid);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| FileReviewGitCaptureAuthorizationError::Unavailable)?;
        connection.query_row("SELECT authorization.capture_authorization_id, authorization.epic_id, authorization.sprint_id, authorization.provenance_id, authorization.repository_id, authorization.repository_root, authorization.worktree_id, authorization.worktree_root, authorization.baseline_object_id, authorization.current_object_id FROM file_review_git_capture_authorizations authorization JOIN initiated_sprints sprint ON sprint.id=authorization.sprint_id AND sprint.epic_id=authorization.epic_id JOIN epic_initiations epic ON epic.epic_id=authorization.epic_id AND epic.provenance_id=authorization.provenance_id WHERE authorization.capture_authorization_id=?1", params![capture_authorization_id], |r| Ok(FileReviewGitCaptureAuthorization { capture_authorization_id:r.get(0)?, epic_id:r.get(1)?, sprint_id:r.get(2)?, provenance_id:r.get(3)?, repository_id:r.get(4)?, repository_root:r.get(5)?, worktree_id:r.get(6)?, worktree_root:r.get(7)?, baseline_object_id:r.get(8)?, current_object_id:r.get(9)? })).optional().map_err(|_| FileReviewGitCaptureAuthorizationError::Unavailable)
    }

    /// Application-only transition store. Runtime and Git facts are supplied by an internal port.
    pub(crate) fn store_initiated_sprint_git_authority(
        &self,
        value: InitiatedSprintGitAuthorityWrite,
    ) -> Result<StoreInitiatedSprintGitAuthorityResult, InitiatedSprintGitAuthorityError> {
        validate_initiated_sprint_git_authority(&value)?;
        let authority_id = initiated_sprint_git_authority_id(&value);
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?;
        let tx = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?;
        let ownership: Option<(String, String)> = tx
            .query_row(
                "SELECT sprint.epic_id, epic.provenance_id FROM initiated_sprints sprint JOIN epic_initiations epic ON epic.epic_id=sprint.epic_id JOIN epic_initiation_provenance provenance ON provenance.id=epic.provenance_id WHERE sprint.id=?1",
                [&value.sprint_id],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?;
        let Some((epic_id, provenance_id)) = ownership else {
            return Err(InitiatedSprintGitAuthorityError::Forbidden);
        };
        let fingerprint = initiated_sprint_git_authority_fingerprint(
            &authority_id,
            &epic_id,
            &provenance_id,
            &value,
        );
        let existing: Option<(String, String)> = tx
            .query_row(
                "SELECT authority_id,payload_fingerprint FROM initiated_sprint_git_authorities WHERE idempotency_key=?1 OR authority_id=?2 OR runtime_instance_ref=?3",
                params![value.idempotency_key, authority_id, value.runtime_instance_ref],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?;
        if let Some((existing_id, existing_fingerprint)) = existing {
            return if existing_fingerprint == fingerprint {
                Ok(StoreInitiatedSprintGitAuthorityResult::IdempotentReplay {
                    authority_id: existing_id,
                })
            } else {
                Err(InitiatedSprintGitAuthorityError::Conflict)
            };
        }
        tx.execute(
            "INSERT INTO initiated_sprint_git_authorities (authority_id,idempotency_key,payload_fingerprint,epic_id,sprint_id,provenance_id,repository_id,repository_root,repository_common_dir,worktree_id,worktree_root,baseline_object_id,current_object_id,runtime_instance_ref,runtime_source_ref,source_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12,?13,?14,?15,?16,?17)",
            params![authority_id, value.idempotency_key, fingerprint, epic_id, value.sprint_id, provenance_id, value.repository_id, value.repository_root, value.repository_common_dir, value.worktree_id, value.worktree_root, value.baseline_object_id, value.current_object_id, value.runtime_instance_ref, value.runtime_source_ref, value.source_fingerprint, timestamp(self.clock.now())],
        )
        .map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?;
        tx.commit()
            .map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?;
        Ok(StoreInitiatedSprintGitAuthorityResult::Stored { authority_id })
    }

    /// Private downstream load. The live initiated Sprint/Epic/provenance chain is reauthorized.
    pub(crate) fn load_initiated_sprint_git_authority(
        &self,
        authority_id: &str,
    ) -> Result<Option<InitiatedSprintGitAuthority>, InitiatedSprintGitAuthorityError> {
        if !bounded_application_id(authority_id) {
            return Err(InitiatedSprintGitAuthorityError::Invalid);
        }
        let connection = self
            .connection
            .lock()
            .map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?;
        let loaded = connection.query_row("SELECT authority.authority_id,authority.epic_id,authority.sprint_id,authority.provenance_id,authority.repository_id,authority.repository_root,authority.repository_common_dir,authority.worktree_id,authority.worktree_root,authority.baseline_object_id,authority.current_object_id,authority.runtime_instance_ref,authority.runtime_source_ref,authority.source_fingerprint,authority.idempotency_key,authority.payload_fingerprint FROM initiated_sprint_git_authorities authority JOIN initiated_sprints sprint ON sprint.id=authority.sprint_id AND sprint.epic_id=authority.epic_id JOIN epic_initiations epic ON epic.epic_id=authority.epic_id AND epic.provenance_id=authority.provenance_id JOIN epic_initiation_provenance provenance ON provenance.id=authority.provenance_id WHERE authority.authority_id=?1", [authority_id], |row| Ok((InitiatedSprintGitAuthority { authority_id:row.get(0)?, epic_id:row.get(1)?, sprint_id:row.get(2)?, provenance_id:row.get(3)?, repository_id:row.get(4)?, repository_root:row.get(5)?, repository_common_dir:row.get(6)?, worktree_id:row.get(7)?, worktree_root:row.get(8)?, baseline_object_id:row.get(9)?, current_object_id:row.get(10)?, runtime_instance_ref:row.get(11)?, runtime_source_ref:row.get(12)?, source_fingerprint:row.get(13)? }, row.get::<_, String>(14)?, row.get::<_, String>(15)?))).optional().map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?;
        let Some((authority, idempotency_key, fingerprint)) = loaded else {
            return Ok(None);
        };
        let write = InitiatedSprintGitAuthorityWrite {
            sprint_id: authority.sprint_id.clone(),
            idempotency_key,
            repository_id: authority.repository_id.clone(),
            repository_root: authority.repository_root.clone(),
            repository_common_dir: authority.repository_common_dir.clone(),
            worktree_id: authority.worktree_id.clone(),
            worktree_root: authority.worktree_root.clone(),
            baseline_object_id: authority.baseline_object_id.clone(),
            current_object_id: authority.current_object_id.clone(),
            runtime_instance_ref: authority.runtime_instance_ref.clone(),
            runtime_source_ref: authority.runtime_source_ref.clone(),
            source_fingerprint: authority.source_fingerprint.clone(),
        };
        validate_initiated_sprint_git_authority(&write)?;
        let expected_id = initiated_sprint_git_authority_id(&write);
        let expected_fingerprint = initiated_sprint_git_authority_fingerprint(
            &expected_id,
            &authority.epic_id,
            &authority.provenance_id,
            &write,
        );
        if authority.authority_id != expected_id || fingerprint != expected_fingerprint {
            return Err(InitiatedSprintGitAuthorityError::Forbidden);
        }
        Ok(Some(authority))
    }

    /// Application-owned context lookup. The caller supplies only the initiated Sprint identity;
    /// private runtime and Git authority remain inside the repository boundary.
    pub(crate) fn load_initiated_sprint_git_authority_for_sprint(
        &self,
        sprint_id: &str,
    ) -> Result<Option<InitiatedSprintGitAuthority>, InitiatedSprintGitAuthorityError> {
        if !bounded_application_id(sprint_id) {
            return Err(InitiatedSprintGitAuthorityError::Invalid);
        }
        let authority_ids = {
            let connection = self
                .connection
                .lock()
                .map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?;
            let mut statement = connection
                .prepare(
                    "SELECT authority_id FROM initiated_sprint_git_authorities WHERE sprint_id=?1 ORDER BY recorded_at, authority_id LIMIT 2",
                )
                .map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?;
            let authority_ids = statement
                .query_map([sprint_id], |row| row.get::<_, String>(0))
                .map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?
                .collect::<Result<Vec<_>, _>>()
                .map_err(|_| InitiatedSprintGitAuthorityError::Unavailable)?;
            authority_ids
        };
        match authority_ids.as_slice() {
            [] => Ok(None),
            [authority_id] => self.load_initiated_sprint_git_authority(authority_id),
            _ => Err(InitiatedSprintGitAuthorityError::Conflict),
        }
    }

    /// Producer-only application seam. No Tauri command accepts these facts.
    pub(crate) fn store_file_review_facts(
        &self,
        facts: StoreFileReviewFacts,
    ) -> Result<StoreFileReviewFactsResult, FileReviewFactsError> {
        validate_file_review_facts(&facts)?;
        let payload_fingerprint = file_review_fingerprint(&facts)?;
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| FileReviewFactsError::Unavailable("database lock is poisoned".into()))?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|e| FileReviewFactsError::Unavailable(e.to_string()))?;
        let ownership: Option<i64> = transaction.query_row("SELECT 1 FROM initiated_sprints sprint JOIN epic_initiations epic ON epic.epic_id = sprint.epic_id AND epic.provenance_id = ?3 WHERE sprint.id = ?1 AND sprint.epic_id = ?2", params![facts.sprint_id, facts.epic_id, facts.provenance_id], |row| row.get(0)).optional().map_err(|e| FileReviewFactsError::Unavailable(e.to_string()))?;
        if ownership.is_none() {
            return Err(FileReviewFactsError::Forbidden);
        }
        let existing: Option<String> = transaction
            .query_row(
                "SELECT payload_fingerprint FROM file_review_documents WHERE idempotency_key = ?1",
                params![facts.idempotency_key],
                |row| row.get(0),
            )
            .optional()
            .map_err(|e| FileReviewFactsError::Unavailable(e.to_string()))?;
        if let Some(existing) = existing {
            return if existing == payload_fingerprint {
                Ok(StoreFileReviewFactsResult::IdempotentReplay)
            } else {
                Err(FileReviewFactsError::Conflict)
            };
        }
        let occupied: Option<()> = transaction.query_row("SELECT 1 FROM file_review_documents WHERE document_ref_id = ?1 OR opaque_reference = ?2", params![facts.document_ref_id, facts.opaque_reference], |_| Ok(())).optional().map_err(|e| FileReviewFactsError::Unavailable(e.to_string()))?;
        if occupied.is_some() {
            return Err(FileReviewFactsError::Conflict);
        }
        let now = timestamp(self.clock.now());
        transaction.execute("INSERT INTO file_review_documents (document_ref_id,epic_id,sprint_id,provenance_id,opaque_reference,title,summary,idempotency_key,payload_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10)", params![facts.document_ref_id, facts.epic_id, facts.sprint_id, facts.provenance_id, facts.opaque_reference, facts.title, facts.summary, facts.idempotency_key, payload_fingerprint, now]).map_err(|e| FileReviewFactsError::Unavailable(e.to_string()))?;
        for (ordinal, file) in facts.changed_files.iter().enumerate() {
            transaction.execute("INSERT INTO file_review_changed_files (document_ref_id,changed_file_reference_id,display_name,change_kind,previous_display_name,ordinal) VALUES (?1,?2,?3,?4,?5,?6)", params![facts.document_ref_id, file.changed_file_reference_id, file.display_name, file.change_kind, file.previous_display_name, ordinal as i64]).map_err(|e| FileReviewFactsError::Unavailable(e.to_string()))?;
        }
        transaction.execute("INSERT INTO stored_file_review_artifacts (artifact_id,document_ref_id,contract_version,payload,payload_bytes,provenance_id) VALUES (?1,?2,?3,?4,?5,?6)", params![facts.artifact_id, facts.document_ref_id, STORED_FILE_REVIEW_ARTIFACT_V1, facts.payload, facts.payload.len() as i64, facts.provenance_id]).map_err(|e| FileReviewFactsError::Unavailable(e.to_string()))?;
        transaction
            .commit()
            .map_err(|e| FileReviewFactsError::Unavailable(e.to_string()))?;
        Ok(StoreFileReviewFactsResult::Stored)
    }

    /// Opaque references select a pre-authorized Document; every lookup rechecks durable scope.
    pub(crate) fn load_scoped_file_review(
        &self,
        opaque_reference: &str,
    ) -> Result<ScopedFileReviewLoad, String> {
        if opaque_reference.trim().is_empty() {
            return Ok(ScopedFileReviewLoad::Invalid);
        }
        let connection = self.lock().map_err(|e| e.to_string())?;
        let exists = connection
            .query_row(
                "SELECT 1 FROM file_review_documents WHERE opaque_reference = ?1",
                params![opaque_reference],
                |_| Ok(()),
            )
            .optional()
            .map_err(|e| e.to_string())?
            .is_some();
        if !exists {
            return Ok(ScopedFileReviewLoad::Unavailable);
        }
        let document: Option<(String, String, Option<String>, String, Vec<u8>, i64)> = connection.query_row("SELECT document.document_ref_id, document.title, document.summary, artifact.artifact_id, artifact.payload, artifact.payload_bytes FROM file_review_documents document JOIN initiated_sprints sprint ON sprint.id = document.sprint_id AND sprint.epic_id = document.epic_id JOIN epic_initiations epic ON epic.epic_id = document.epic_id AND epic.provenance_id = document.provenance_id JOIN stored_file_review_artifacts artifact ON artifact.document_ref_id = document.document_ref_id AND artifact.provenance_id = document.provenance_id WHERE document.opaque_reference = ?1 AND artifact.contract_version = ?2", params![opaque_reference, STORED_FILE_REVIEW_ARTIFACT_V1], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?, row.get(5)?))).optional().map_err(|e| e.to_string())?;
        let Some((document_ref_id, title, summary, artifact_id, payload, payload_bytes)) = document
        else {
            return Ok(ScopedFileReviewLoad::Unauthorized);
        };
        if payload_bytes < 1
            || payload_bytes as usize != payload.len()
            || payload.len() > FILE_REVIEW_ARTIFACT_MAX_BYTES
        {
            return Ok(ScopedFileReviewLoad::Invalid);
        }
        let changed_files = connection.prepare("SELECT changed_file_reference_id, display_name, change_kind, previous_display_name FROM file_review_changed_files WHERE document_ref_id = ?1 ORDER BY ordinal").map_err(|e| e.to_string())?.query_map(params![document_ref_id], |row| Ok(FileReviewChangedFileDto { changed_file_reference_id: row.get(0)?, display_name: row.get(1)?, change_kind: row.get(2)?, previous_display_name: row.get(3)? })).map_err(|e| e.to_string())?.collect::<Result<Vec<_>, _>>().map_err(|e| e.to_string())?;
        if changed_files.is_empty() {
            return Ok(ScopedFileReviewLoad::Invalid);
        }
        Ok(ScopedFileReviewLoad::Available {
            document: ScopedFileReviewDocument {
                document_ref_id,
                title,
                summary,
                artifact_id,
                payload,
                changed_files,
            },
        })
    }

    /// Captures the authorized optimistic precondition for one managed Agent Invocation. The
    /// captured value is never exposed to the agent; save rechecks it transactionally.
    pub(crate) fn capture_plan_builder_precondition(
        &self,
        draft_id: &EpicPlanningDraftId,
        profile_id: &CapabilityProfileId,
        association_id: &PlanningDraftAgentSessionAssociationId,
        agent_session_id: &str,
        actor_id: &str,
    ) -> Result<Option<String>, SaveProposalError> {
        let connection = self.lock()?;
        let now = timestamp(self.clock.now());
        let authorized = connection.query_row("SELECT 1 FROM epic_planning_drafts draft JOIN planning_draft_profile_assignments assignment ON assignment.draft_id = draft.id JOIN planning_draft_agent_session_associations association ON association.id = assignment.agent_session_association_id JOIN capability_profiles profile ON profile.id = assignment.capability_profile_id WHERE draft.id = ?1 AND draft.status = 'active' AND assignment.capability_profile_id = ?2 AND assignment.agent_session_association_id = ?3 AND association.agent_session_id = ?4 AND association.actor_id = ?5 AND assignment.expires_at >= ?6 AND profile.status = 'active'", params![draft_id.as_str(), profile_id.as_str(), association_id.as_str(), agent_session_id, actor_id, now], |_| Ok(())).optional().map_err(sql_error("authorize managed proposal precondition"))?.is_some();
        if !authorized {
            return Err(SaveProposalError::Forbidden);
        }
        connection.query_row("SELECT revision_token FROM proposal_revisions WHERE draft_id = ?1 ORDER BY recorded_at DESC, id DESC LIMIT 1", params![draft_id.as_str()], |row| row.get(0)).optional().map_err(sql_error("capture managed proposal precondition"))
    }

    /// Derives the current initiation precondition from the registered managed Agent Session.
    /// No product identity or optimistic token is accepted from the agent tool input.
    pub(crate) fn capture_agent_initiation_precondition(
        &self,
        draft_id: &EpicPlanningDraftId,
        profile_id: &CapabilityProfileId,
        association_id: &PlanningDraftAgentSessionAssociationId,
        agent_session_id: &str,
        actor_id: &str,
    ) -> Result<String, super::domain::InitiateEpicError> {
        use super::domain::InitiateEpicError;
        let connection = self.connection.lock().map_err(|_| {
            InitiateEpicError::Unavailable("orchestration database lock is poisoned".into())
        })?;
        let now = timestamp(self.clock.now());
        let status: Option<String> = connection
            .query_row(
                "SELECT status FROM epic_planning_drafts WHERE id = ?1",
                params![draft_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| InitiateEpicError::Unavailable(error.to_string()))?;
        match status.as_deref() {
            Some("active") => {}
            Some("canceled") => return Err(InitiateEpicError::Canceled),
            Some(_) => return Err(InitiateEpicError::AlreadyInitiated),
            None => return Err(InitiateEpicError::DraftNotFound),
        }
        let authorized = connection
            .query_row(
                "SELECT 1 FROM planning_draft_profile_assignments assignment JOIN planning_draft_agent_session_associations association ON association.id = assignment.agent_session_association_id JOIN capability_profiles profile ON profile.id = assignment.capability_profile_id WHERE assignment.draft_id = ?1 AND assignment.capability_profile_id = ?2 AND assignment.agent_session_association_id = ?3 AND association.draft_id = ?1 AND association.agent_session_id = ?4 AND association.actor_id = ?5 AND assignment.expires_at >= ?6 AND profile.status = 'active'",
                params![draft_id.as_str(), profile_id.as_str(), association_id.as_str(), agent_session_id, actor_id, now],
                |_| Ok(()),
            )
            .optional()
            .map_err(|error| InitiateEpicError::Unavailable(error.to_string()))?
            .is_some();
        if !authorized {
            return Err(InitiateEpicError::Forbidden);
        }
        connection
            .query_row(
                "SELECT revision_token FROM proposal_revisions WHERE draft_id = ?1 ORDER BY recorded_at DESC, id DESC LIMIT 1",
                params![draft_id.as_str()],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| InitiateEpicError::Unavailable(error.to_string()))?
            .ok_or(InitiateEpicError::ProposalMissing)
    }

    pub(crate) fn initiation_is_projected(
        &self,
        initiation_id: &super::domain::EpicInitiationId,
    ) -> Result<bool, String> {
        Ok(self
            .native_query()?
            .initiated_epics
            .iter()
            .any(|epic| epic.initiation_id == initiation_id.as_str()))
    }

    /// Returns only the requested draft's semantic context after the same durable profile,
    /// association, actor, expiry, and active-profile checks used for mutations.
    pub(crate) fn plan_builder_context(
        &self,
        draft_id: &EpicPlanningDraftId,
        profile_id: &CapabilityProfileId,
        association_id: &PlanningDraftAgentSessionAssociationId,
        actor_id: &str,
    ) -> Result<serde_json::Value, SaveProposalError> {
        let connection = self.lock()?;
        let now = timestamp(self.clock.now());
        let exists = connection
            .query_row(
                "SELECT 1 FROM epic_planning_drafts WHERE id = ?1",
                params![draft_id.as_str()],
                |_| Ok(()),
            )
            .optional()
            .map_err(sql_error("read planning draft"))?
            .is_some();
        if !exists {
            return Err(SaveProposalError::DraftNotFound);
        }
        let authorized = connection.query_row("SELECT 1 FROM planning_draft_profile_assignments assignment JOIN planning_draft_agent_session_associations association ON association.id = assignment.agent_session_association_id JOIN capability_profiles profile ON profile.id = assignment.capability_profile_id WHERE assignment.draft_id = ?1 AND assignment.capability_profile_id = ?2 AND assignment.agent_session_association_id = ?3 AND association.actor_id = ?4 AND association.draft_id = ?1 AND assignment.expires_at >= ?5 AND profile.status = 'active'", params![draft_id.as_str(), profile_id.as_str(), association_id.as_str(), actor_id, now], |_| Ok(())).optional().map_err(sql_error("authorize planning context"))?.is_some();
        if !authorized {
            return Err(SaveProposalError::Forbidden);
        }
        let latest: Option<(String, String, String)> = connection.query_row("SELECT id, revision_token, proposal_json FROM proposal_revisions WHERE draft_id = ?1 ORDER BY recorded_at DESC, id DESC LIMIT 1", params![draft_id.as_str()], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?))).optional().map_err(sql_error("read planning context"))?;
        let proposal = latest
            .as_ref()
            .map(|(_, _, json)| serde_json::from_str::<PlanBuilderProposal>(json))
            .transpose()
            .map_err(|error| {
                SaveProposalError::Unavailable(format!("read proposal context: {error}"))
            })?;
        Ok(
            serde_json::json!({ "epicPlanningDraftId": draft_id.as_str(), "currentProposal": latest.map(|(id, token, _)| serde_json::json!({"proposalRevisionId": id, "revisionToken": token, "proposal": proposal})) }),
        )
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, SaveProposalError> {
        self.connection.lock().map_err(|_| {
            SaveProposalError::Unavailable("orchestration database lock is poisoned".into())
        })
    }
}

fn validate_file_review_facts(facts: &StoreFileReviewFacts) -> Result<(), FileReviewFactsError> {
    let non_blank = |value: &str| !value.trim().is_empty() && value.len() <= 4_000;
    if !non_blank(&facts.document_ref_id)
        || !non_blank(&facts.epic_id)
        || !non_blank(&facts.sprint_id)
        || !non_blank(&facts.provenance_id)
        || !non_blank(&facts.opaque_reference)
        || !non_blank(&facts.title)
        || !non_blank(&facts.artifact_id)
        || !non_blank(&facts.idempotency_key)
        || facts.payload.is_empty()
        || facts.payload.len() > FILE_REVIEW_ARTIFACT_MAX_BYTES
        || facts.changed_files.is_empty()
    {
        return Err(FileReviewFactsError::Invalid);
    }
    let mut ids = std::collections::HashSet::new();
    for file in &facts.changed_files {
        if !non_blank(&file.changed_file_reference_id)
            || !non_blank(&file.display_name)
            || !matches!(
                file.change_kind.as_str(),
                "added" | "modified" | "deleted" | "renamed"
            )
            || file.display_name.chars().any(|c| c.is_control())
            || file.display_name.starts_with('/')
            || file.display_name.starts_with('\\')
            || file
                .display_name
                .split(['/', '\\'])
                .any(|part| part == "..")
            || (file.change_kind == "renamed") != file.previous_display_name.is_some()
            || file
                .previous_display_name
                .as_ref()
                .is_some_and(|path| !display_safe_path(path))
            || !ids.insert(&file.changed_file_reference_id)
        {
            return Err(FileReviewFactsError::Invalid);
        }
    }
    Ok(())
}

fn display_safe_path(value: &str) -> bool {
    !value.trim().is_empty()
        && value.len() <= 4_000
        && !value.chars().any(|c| c.is_control())
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.split(['/', '\\']).any(|part| part == "..")
}

fn validate_git_capture_authorization(
    value: &FileReviewGitCaptureAuthorizationWrite,
) -> Result<(), FileReviewGitCaptureAuthorizationError> {
    let nonblank = |x: &str| !x.trim().is_empty() && x.len() <= 4_000;
    let ids = [
        &value.capture_authorization_id,
        &value.idempotency_key,
        &value.epic_id,
        &value.sprint_id,
        &value.provenance_id,
        &value.repository_id,
        &value.worktree_id,
    ];
    if !ids.iter().all(|x| nonblank(x))
        || !canonical_absolute_root(&value.repository_root)
        || !canonical_absolute_root(&value.worktree_root)
        || !git_object_id(&value.baseline_object_id)
        || !git_object_id(&value.current_object_id)
        || value
            .baseline_object_id
            .eq_ignore_ascii_case(&value.current_object_id)
    {
        return Err(FileReviewGitCaptureAuthorizationError::Invalid);
    }
    Ok(())
}
fn canonical_absolute_root(value: &str) -> bool {
    let path = std::path::Path::new(value);
    path.is_absolute()
        && !path.components().any(|part| {
            matches!(
                part,
                std::path::Component::ParentDir | std::path::Component::CurDir
            )
        })
}
fn git_object_id(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn git_capture_authorization_fingerprint(value: &FileReviewGitCaptureAuthorizationWrite) -> String {
    let mut hash = Sha256::new();
    for part in [
        &value.capture_authorization_id,
        &value.idempotency_key,
        &value.epic_id,
        &value.sprint_id,
        &value.provenance_id,
        &value.repository_id,
        &value.repository_root,
        &value.worktree_id,
        &value.worktree_root,
        &value.baseline_object_id,
        &value.current_object_id,
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn validate_initiated_sprint_git_authority(
    value: &InitiatedSprintGitAuthorityWrite,
) -> Result<(), InitiatedSprintGitAuthorityError> {
    if ![
        &value.sprint_id,
        &value.idempotency_key,
        &value.repository_id,
        &value.worktree_id,
        &value.runtime_instance_ref,
        &value.runtime_source_ref,
    ]
    .iter()
    .all(|value| bounded_application_id(value))
        || !canonical_absolute_root(&value.repository_root)
        || !canonical_absolute_root(&value.repository_common_dir)
        || !canonical_absolute_root(&value.worktree_root)
        || !git_object_id(&value.baseline_object_id)
        || !git_object_id(&value.current_object_id)
        || value
            .baseline_object_id
            .eq_ignore_ascii_case(&value.current_object_id)
        || value.source_fingerprint.len() != 64
        || !value
            .source_fingerprint
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        return Err(InitiatedSprintGitAuthorityError::Invalid);
    }
    Ok(())
}

fn bounded_application_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
}

fn initiated_sprint_git_authority_id(value: &InitiatedSprintGitAuthorityWrite) -> String {
    let mut hash = Sha256::new();
    for part in [
        b"initiated-sprint-git-authority/v1".as_slice(),
        value.sprint_id.as_bytes(),
        value.idempotency_key.as_bytes(),
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part);
    }
    format!(
        "sprint-git-authority-{}",
        &format!("{:x}", hash.finalize())[..24]
    )
}

fn initiated_sprint_git_authority_fingerprint(
    authority_id: &str,
    epic_id: &str,
    provenance_id: &str,
    value: &InitiatedSprintGitAuthorityWrite,
) -> String {
    let mut hash = Sha256::new();
    for part in [
        authority_id,
        &value.idempotency_key,
        epic_id,
        &value.sprint_id,
        provenance_id,
        &value.repository_id,
        &value.repository_root,
        &value.repository_common_dir,
        &value.worktree_id,
        &value.worktree_root,
        &value.baseline_object_id,
        &value.current_object_id,
        &value.runtime_instance_ref,
        &value.runtime_source_ref,
        &value.source_fingerprint,
    ] {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

fn file_review_fingerprint(facts: &StoreFileReviewFacts) -> Result<String, FileReviewFactsError> {
    let mut hash = Sha256::new();
    for value in [
        &facts.document_ref_id,
        &facts.epic_id,
        &facts.sprint_id,
        &facts.provenance_id,
        &facts.opaque_reference,
        &facts.title,
        &facts.artifact_id,
    ] {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    if let Some(summary) = &facts.summary {
        hash.update([1]);
        hash.update((summary.len() as u64).to_be_bytes());
        hash.update(summary.as_bytes());
    } else {
        hash.update([0]);
    }
    hash.update((facts.payload.len() as u64).to_be_bytes());
    hash.update(&facts.payload);
    for file in &facts.changed_files {
        for value in [
            &file.changed_file_reference_id,
            &file.display_name,
            &file.change_kind,
        ] {
            hash.update((value.len() as u64).to_be_bytes());
            hash.update(value.as_bytes());
        }
        if let Some(previous) = &file.previous_display_name {
            hash.update([1]);
            hash.update((previous.len() as u64).to_be_bytes());
            hash.update(previous.as_bytes());
        } else {
            hash.update([0]);
        }
    }
    Ok(format!("{:x}", hash.finalize()))
}

fn find_command_result(
    transaction: &Transaction<'_>,
    idempotency_key: &str,
) -> Result<Option<(String, SaveProposalResult)>, SaveProposalError> {
    transaction.query_row("SELECT command.payload_fingerprint, command.id, result.id, result.revision_id, revision.revision_token, result.event_id, result.provenance_id FROM proposal_commands command JOIN proposal_command_results result ON result.command_id = command.id JOIN proposal_revisions revision ON revision.id = result.revision_id WHERE command.idempotency_key = ?1", params![idempotency_key], |row| Ok((row.get(0)?, SaveProposalResult { command_id: ProposalCommandId::new(row.get::<_, String>(1)?).map_err(to_sql_error)?, result_id: ProposalResultId::new(row.get::<_, String>(2)?).map_err(to_sql_error)?, revision_id: ProposalRevisionId::new(row.get::<_, String>(3)?).map_err(to_sql_error)?, revision_token: row.get(4)?, event_id: ProposalEventId::new(row.get::<_, String>(5)?).map_err(to_sql_error)?, provenance_id: EffectProvenanceId::new(row.get::<_, String>(6)?).map_err(to_sql_error)?, idempotent_replay: false }))).optional().map_err(sql_error("read idempotent proposal command"))
}

fn collect<T>(
    connection: &Connection,
    sql: &str,
    map: impl FnMut(&rusqlite::Row<'_>) -> rusqlite::Result<T>,
) -> Result<Vec<T>, String> {
    connection
        .prepare(sql)
        .map_err(|error| error.to_string())?
        .query_map([], map)
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())
}
fn fingerprint(command: &SaveEpicPlanProposalCommand) -> Result<String, SaveProposalError> {
    serde_json::to_string(&(
        command.epic_planning_draft_id.as_str(),
        command.capability_profile_id.as_str(),
        command.agent_session_association_id.as_str(),
        &command.agent_session_id,
        &command.actor_id,
        &command.expected_revision,
        &command.proposal,
    ))
    .map_err(|error| SaveProposalError::Unavailable(error.to_string()))
}
fn harness_working_copy_command_fingerprint(
    command: &SaveHarnessWorkingCopyCommand,
) -> Result<String, HarnessWorkingCopyError> {
    let bytes = serde_json::to_vec(command).map_err(|_| HarnessWorkingCopyError::Invalid)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

fn decode_harness_working_copy_result(
    value: &str,
) -> Result<HarnessWorkingCopy, HarnessWorkingCopyError> {
    let result: HarnessWorkingCopy =
        serde_json::from_str(value).map_err(|_| HarnessWorkingCopyError::InvalidStoredState)?;
    validate_working_copy(&result)?;
    Ok(result)
}

fn load_harness_working_copy_from_connection(
    connection: &Connection,
    harness_key: &str,
) -> Result<Option<HarnessWorkingCopy>, HarnessWorkingCopyError> {
    let row: Option<(String, String, String, i64, i64, String, String, String)> = connection
        .query_row(
            "SELECT configuration_contract_version,configuration_json,working_copy_digest,draft_revision,dirty,editor_kind,editor_reference,saved_at FROM harness_working_copies WHERE harness_key=?1",
            [harness_key],
            |row| {
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    row.get(4)?,
                    row.get(5)?,
                    row.get(6)?,
                    row.get(7)?,
                ))
            },
        )
        .optional()
        .map_err(|_| HarnessWorkingCopyError::Unavailable)?;
    let Some((
        contract,
        configuration_json,
        digest,
        revision,
        dirty,
        editor_kind,
        editor_reference,
        saved_at,
    )) = row
    else {
        return Ok(None);
    };
    if digest
        != harness_working_copy_digest(
            harness_key,
            &contract,
            &configuration_json,
            revision,
            dirty,
            &editor_kind,
            &editor_reference,
            &saved_at,
        )
        || contract != HARNESS_EFFECTIVE_CONFIGURATION_V1
        || revision <= 0
        || dirty != 1
    {
        return Err(HarnessWorkingCopyError::InvalidStoredState);
    }
    let envelope: HarnessEffectiveConfigurationEnvelope = serde_json::from_str(&configuration_json)
        .map_err(|_| HarnessWorkingCopyError::InvalidStoredState)?;
    if envelope.contract_version != HARNESS_EFFECTIVE_CONFIGURATION_V1 {
        return Err(HarnessWorkingCopyError::InvalidStoredState);
    }
    let editor_kind = HarnessEditorKind::parse(&editor_kind)
        .ok_or(HarnessWorkingCopyError::InvalidStoredState)?;
    let saved_at = DateTime::parse_from_rfc3339(&saved_at)
        .map_err(|_| HarnessWorkingCopyError::InvalidStoredState)?
        .with_timezone(&Utc);
    let working_copy = HarnessWorkingCopy {
        harness_key: harness_key.into(),
        configuration: envelope.configuration,
        draft_revision: revision as u64,
        dirty: true,
        editor: HarnessWorkingCopyEditor {
            kind: editor_kind,
            reference: editor_reference,
        },
        saved_at,
    };
    validate_working_copy(&working_copy)?;
    Ok(Some(working_copy))
}

fn harness_working_copy_digest(
    harness_key: &str,
    contract: &str,
    configuration_json: &str,
    revision: i64,
    dirty: i64,
    editor_kind: &str,
    editor_reference: &str,
    saved_at: &str,
) -> String {
    let mut digest = Sha256::new();
    for part in [
        b"harness-working-copy-envelope/v1".as_slice(),
        harness_key.as_bytes(),
        contract.as_bytes(),
        configuration_json.as_bytes(),
        &revision.to_be_bytes(),
        &dirty.to_be_bytes(),
        editor_kind.as_bytes(),
        editor_reference.as_bytes(),
        saved_at.as_bytes(),
    ] {
        digest.update((part.len() as u64).to_be_bytes());
        digest.update(part);
    }
    format!("{:x}", digest.finalize())
}

fn harness_revision_command_fingerprint(
    command: &CreateHarnessRevisionCommand,
) -> Result<String, HarnessRevisionError> {
    let bytes = serde_json::to_vec(command).map_err(|_| HarnessRevisionError::Invalid)?;
    Ok(format!("{:x}", Sha256::digest(bytes)))
}

#[derive(Clone, Debug)]
struct StoredHarnessRevision {
    revision_id: String,
    harness_key: String,
    configuration_digest: String,
    source_draft_revision: u64,
    predecessor_revision_id: Option<String>,
    repository_commit_ref: String,
    creation_provenance: HarnessRevisionCreationProvenance,
    created_at: DateTime<Utc>,
}

impl StoredHarnessRevision {
    fn manifest(&self) -> HarnessRevisionCommitManifest {
        HarnessRevisionCommitManifest::from_metadata(
            self.revision_id.clone(),
            self.harness_key.clone(),
            self.configuration_digest.clone(),
            self.source_draft_revision,
            self.predecessor_revision_id.clone(),
            self.repository_commit_ref.clone(),
            self.creation_provenance.clone(),
            self.created_at,
        )
    }
}

fn load_stored_harness_revision_from_connection(
    connection: &Connection,
    revision_id: &str,
) -> Result<Option<StoredHarnessRevision>, HarnessRevisionError> {
    let mut statement = connection
        .prepare(
            "SELECT revision.revision_id,revision.harness_key,revision.configuration_contract_version,revision.configuration_digest,revision.source_draft_revision,revision.predecessor_revision_id,revision.repository_commit_ref,revision.creation_provenance_kind,revision.creation_provenance_reference,revision.created_at,publication.repository_commit_ref,publication.evidence_kind,publication.verified_at FROM harness_revisions revision LEFT JOIN harness_revision_publications publication ON publication.revision_id=revision.revision_id WHERE revision.revision_id=?1",
        )
        .map_err(|_| HarnessRevisionError::Unavailable)?;
    let mut rows = statement
        .query([revision_id])
        .map_err(|_| HarnessRevisionError::Unavailable)?;
    let Some(row) = rows.next().map_err(|_| HarnessRevisionError::Unavailable)? else {
        return Ok(None);
    };
    let decoded: Result<
        (
            String,
            String,
            String,
            String,
            i64,
            Option<String>,
            String,
            String,
            String,
            String,
            Option<String>,
            Option<String>,
            Option<String>,
        ),
        rusqlite::Error,
    > = (|| {
        Ok((
            row.get(0)?,
            row.get(1)?,
            row.get(2)?,
            row.get(3)?,
            row.get(4)?,
            row.get(5)?,
            row.get(6)?,
            row.get(7)?,
            row.get(8)?,
            row.get(9)?,
            row.get(10)?,
            row.get(11)?,
            row.get(12)?,
        ))
    })();
    let (
        revision_id,
        harness_key,
        contract,
        configuration_digest,
        source_draft_revision,
        predecessor_revision_id,
        repository_commit_ref,
        provenance_kind,
        provenance_reference,
        created_at,
        publication_ref,
        evidence_kind,
        verified_at,
    ) = decoded.map_err(|_| HarnessRevisionError::InvalidStoredState)?;
    if contract != HARNESS_EFFECTIVE_CONFIGURATION_V1
        || source_draft_revision <= 0
        || publication_ref.as_deref() != Some(repository_commit_ref.as_str())
        || evidence_kind.as_deref() != Some("local_commit_verified")
        || verified_at.as_deref() != Some(created_at.as_str())
    {
        return Err(HarnessRevisionError::InvalidStoredState);
    }
    let creation_provenance = HarnessRevisionCreationProvenance {
        kind: HarnessRevisionProvenanceKind::parse(&provenance_kind)
            .ok_or(HarnessRevisionError::InvalidStoredState)?,
        reference: provenance_reference,
    };
    let created_at = DateTime::parse_from_rfc3339(&created_at)
        .map_err(|_| HarnessRevisionError::InvalidStoredState)?
        .with_timezone(&Utc);
    Ok(Some(StoredHarnessRevision {
        revision_id,
        harness_key,
        configuration_digest,
        source_draft_revision: source_draft_revision as u64,
        predecessor_revision_id,
        repository_commit_ref,
        creation_provenance,
        created_at,
    }))
}

fn load_verified_harness_revision_from_connection(
    connection: &Connection,
    repository: &LocalHarnessRevisionRepository,
    revision_id: &str,
) -> Result<Option<HarnessRevision>, HarnessRevisionError> {
    let Some(stored) = load_stored_harness_revision_from_connection(connection, revision_id)?
    else {
        return Ok(None);
    };
    let manifest = stored.manifest();
    let envelope = repository
        .read_and_verify(&stored.repository_commit_ref, &manifest)
        .map_err(map_local_harness_revision_error)?;
    let configuration = decode_verified_configuration(&envelope, &stored.configuration_digest)?;
    let revision = HarnessRevision {
        revision_id: stored.revision_id,
        harness_key: stored.harness_key,
        configuration,
        configuration_digest: stored.configuration_digest,
        source_draft_revision: stored.source_draft_revision,
        predecessor_revision_id: stored.predecessor_revision_id,
        repository_commit_ref: stored.repository_commit_ref,
        creation_provenance: stored.creation_provenance,
        created_at: stored.created_at,
    };
    validate_revision(&revision)?;
    Ok(Some(revision))
}

fn load_verified_harness_revision_history_from_connection(
    connection: &Connection,
    repository: &LocalHarnessRevisionRepository,
    harness_key: &str,
) -> Result<Vec<HarnessRevision>, HarnessRevisionError> {
    let revision_ids = connection
        .prepare(
            "SELECT revision_id FROM harness_revisions WHERE harness_key=?1 ORDER BY source_draft_revision,revision_id",
        )
        .map_err(|_| HarnessRevisionError::Unavailable)?
        .query_map([harness_key], |row| row.get::<_, String>(0))
        .map_err(|_| HarnessRevisionError::Unavailable)?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|_| HarnessRevisionError::InvalidStoredState)?;
    let mut revisions: Vec<HarnessRevision> = Vec::with_capacity(revision_ids.len());
    for revision_id in revision_ids {
        let revision =
            load_verified_harness_revision_from_connection(connection, repository, &revision_id)?
                .ok_or(HarnessRevisionError::InvalidStoredState)?;
        if revision.harness_key != harness_key {
            return Err(HarnessRevisionError::InvalidStoredState);
        }
        if let Some(previous) = revisions.last() {
            if revision.predecessor_revision_id.as_deref() != Some(previous.revision_id.as_str())
                || revision.source_draft_revision <= previous.source_draft_revision
                || revision.created_at < previous.created_at
            {
                return Err(HarnessRevisionError::InvalidStoredState);
            }
        } else if revision.predecessor_revision_id.is_some() {
            return Err(HarnessRevisionError::InvalidStoredState);
        }
        revisions.push(revision);
    }
    Ok(revisions)
}

fn map_local_harness_revision_error(
    error: LocalHarnessRevisionRepositoryError,
) -> HarnessRevisionError {
    match error {
        LocalHarnessRevisionRepositoryError::InvalidEvidence => {
            HarnessRevisionError::InvalidLocalCommitEvidence
        }
        LocalHarnessRevisionRepositoryError::Unavailable => HarnessRevisionError::Unavailable,
    }
}

fn timestamp(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::Millis, true)
}
fn new_id(prefix: &str) -> String {
    format!("{prefix}-{}", Uuid::new_v4())
}
fn sql_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> SaveProposalError {
    move |error| SaveProposalError::Unavailable(format!("{context}: {error}"))
}
fn to_sql_error(error: String) -> rusqlite::Error {
    rusqlite::Error::FromSqlConversionFailure(
        0,
        rusqlite::types::Type::Text,
        Box::new(std::io::Error::new(std::io::ErrorKind::InvalidData, error)),
    )
}

fn parse_proposal_json(value: String) -> rusqlite::Result<PlanBuilderProposal> {
    let proposal: PlanBuilderProposal =
        serde_json::from_str(&value).map_err(|error| to_sql_error(error.to_string()))?;
    proposal.validate().map_err(to_sql_error)?;
    Ok(proposal)
}

#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeQueryV2 {
    contract_version: &'static str,
    generated_at: String,
    planning_drafts: Vec<PlanningDraftDto>,
    agent_session_associations: Vec<AgentSessionAssociationDto>,
    proposal_revisions: Vec<ProposalRevisionDto>,
    recorded_proposal_events: Vec<RecordedProposalEventDto>,
    provenance_links: Vec<ProvenanceLinkDto>,
    initiation_commands: Vec<InitiationCommandDto>,
    initiation_results: Vec<InitiationResultDto>,
    initiation_events: Vec<InitiationEventDto>,
    initiation_provenance: Vec<InitiationProvenanceDto>,
    material_snapshots: Vec<MaterialSnapshotDto>,
    initiated_epics: Vec<InitiatedEpicDto>,
    initiated_sprints: Vec<InitiatedSprintDto>,
    file_review_documents: Vec<FileReviewDocumentDto>,
    work_unit_materializations: Vec<WorkUnitMaterializationDto>,
    work_units: Vec<WorkUnitDto>,
    work_unit_relationships: Vec<WorkUnitRelationshipDto>,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct PlanningDraftDto {
    epic_planning_draft_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    title: Option<String>,
    status: String,
    created_at: String,
    updated_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    canceled_at: Option<String>,
    current_proposal: CurrentProposalDto,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct AgentSessionAssociationDto {
    agent_session_association_id: String,
    epic_planning_draft_id: String,
    agent_session_id: String,
    actor_id: String,
    associated_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
enum CurrentProposalDto {
    Empty {},
    Available { proposal_revision_id: String },
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProposalRevisionDto {
    proposal_revision_id: String,
    epic_planning_draft_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    parent_proposal_revision_id: Option<String>,
    revision_token: String,
    proposal: PlanBuilderProposal,
    command_id: String,
    provenance_id: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct RecordedProposalEventDto {
    proposal_event_id: String,
    epic_planning_draft_id: String,
    proposal_revision_id: String,
    command_id: String,
    provenance_id: String,
    event_kind: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProvenanceLinkDto {
    provenance_id: String,
    source_kind: String,
    recorded_at: String,
    actor_id: String,
    agent_session_association_id: String,
    capability_profile_id: String,
    causal_command_id: String,
    causal_result_id: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiationCommandDto {
    command_id: String,
    epic_planning_draft_id: String,
    expected_revision_token: String,
    actor_id: String,
    idempotency_key: String,
    payload_fingerprint: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiationResultDto {
    result_id: String,
    command_id: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiationEventDto {
    event_id: String,
    command_id: String,
    result_id: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiationProvenanceDto {
    provenance_id: String,
    command_id: String,
    result_id: String,
    event_id: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct MaterialSnapshotDto {
    material_snapshot_id: String,
    epic_planning_draft_id: String,
    proposal_revision_id: String,
    version: i64,
    proposal: PlanBuilderProposal,
    content_hash: String,
    recorded_at: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiatedEpicDto {
    initiation_id: String,
    epic_planning_draft_id: String,
    proposal_revision_id: String,
    material_snapshot_id: String,
    epic_id: String,
    recorded_at: String,
    command_id: String,
    result_id: String,
    event_id: String,
    provenance_id: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct InitiatedSprintDto {
    sprint_id: String,
    epic_id: String,
    ordinal: i64,
    title: String,
    intended_movement: String,
    concern_summaries: Vec<String>,
    sprint_plan_id: String,
    sprint_plan_revision_id: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkUnitMaterializationDto {
    materialization_id: String,
    planning_point_id: String,
    accepted_revision_id: String,
    epic_id: String,
    sprint_id: String,
    work_slice_id: String,
    authorization_recorded_at: String,
    attempt_recorded_at: Option<String>,
    work_units_created_at: Option<String>,
    relationships_completed_at: Option<String>,
    settled_at: Option<String>,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkUnitDto {
    work_unit_id: String,
    materialization_id: String,
    work_slice_id: String,
    accepted_revision_id: String,
    lane_ordinal: i64,
    lane_title: String,
    specification: String,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct WorkUnitRelationshipDto {
    relationship_id: String,
    materialization_id: String,
    relationship_kind: String,
    from_id: String,
    to_id: String,
    ordinal: Option<i64>,
}
#[derive(Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
struct FileReviewDocumentDto {
    document_ref_id: String,
    epic_id: String,
    sprint_id: String,
    provenance_id: String,
    title: String,
    summary: Option<String>,
    artifact_id: String,
    changed_files: Vec<FileReviewChangedFileDto>,
}

#[cfg(test)]
mod tests;
