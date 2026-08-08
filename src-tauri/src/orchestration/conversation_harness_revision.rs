//! Immutable, application-owned Conversation Harness revision contracts and local repository.

use super::conversation_harness_working_copy::{
    validate_complete_configuration, validate_harness_key, HarnessEffectiveConfiguration,
    HarnessEffectiveConfigurationEnvelope, HarnessWorkingCopyError,
    HARNESS_EFFECTIVE_CONFIGURATION_V1,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    fs::{self, OpenOptions},
    io::Write,
    path::{Path, PathBuf},
};
use uuid::Uuid;

pub(crate) const HARNESS_REVISION_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS harness_revisions (
  revision_id TEXT PRIMARY KEY,
  harness_key TEXT NOT NULL,
  configuration_contract_version TEXT NOT NULL CHECK (configuration_contract_version = 'harness-effective-configuration/v1'),
  configuration_digest TEXT NOT NULL CHECK (length(configuration_digest) = 64),
  source_draft_revision INTEGER NOT NULL CHECK (source_draft_revision > 0),
  predecessor_revision_id TEXT,
  repository_commit_ref TEXT NOT NULL UNIQUE,
  creation_provenance_kind TEXT NOT NULL CHECK (creation_provenance_kind IN ('application_user', 'agent_session')),
  creation_provenance_reference TEXT NOT NULL,
  created_at TEXT NOT NULL,
  UNIQUE (harness_key, source_draft_revision),
  FOREIGN KEY (predecessor_revision_id) REFERENCES harness_revisions(revision_id) ON DELETE RESTRICT,
  CHECK (predecessor_revision_id IS NULL OR predecessor_revision_id <> revision_id)
);
CREATE UNIQUE INDEX IF NOT EXISTS harness_revision_root_by_harness
  ON harness_revisions(harness_key) WHERE predecessor_revision_id IS NULL;
CREATE UNIQUE INDEX IF NOT EXISTS harness_revision_unique_predecessor
  ON harness_revisions(harness_key, predecessor_revision_id)
  WHERE predecessor_revision_id IS NOT NULL;
CREATE INDEX IF NOT EXISTS harness_revision_history_by_harness
  ON harness_revisions(harness_key, source_draft_revision);
CREATE TABLE IF NOT EXISTS harness_revision_publications (
  revision_id TEXT PRIMARY KEY,
  harness_key TEXT NOT NULL,
  repository_commit_ref TEXT NOT NULL UNIQUE,
  evidence_kind TEXT NOT NULL CHECK (evidence_kind = 'local_commit_verified'),
  verified_at TEXT NOT NULL,
  FOREIGN KEY (revision_id) REFERENCES harness_revisions(revision_id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS harness_revision_commands (
  idempotency_key TEXT PRIMARY KEY,
  payload_fingerprint TEXT NOT NULL,
  harness_key TEXT NOT NULL,
  expected_source_draft_revision INTEGER NOT NULL CHECK (expected_source_draft_revision > 0),
  expected_predecessor_revision_id TEXT,
  result_revision_id TEXT NOT NULL UNIQUE,
  result_json TEXT NOT NULL CHECK (json_valid(result_json)),
  result_digest TEXT NOT NULL CHECK (length(result_digest) = 64),
  recorded_at TEXT NOT NULL,
  FOREIGN KEY (result_revision_id) REFERENCES harness_revisions(revision_id) ON DELETE RESTRICT
);
CREATE INDEX IF NOT EXISTS harness_revision_commands_by_harness
  ON harness_revision_commands(harness_key, expected_source_draft_revision);
"#;

pub(crate) const HARNESS_REVISION_COMMIT_CONTRACT_V1: &str = "harness-revision-commit/v1";

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum HarnessRevisionProvenanceKind {
    ApplicationUser,
    AgentSession,
}

impl HarnessRevisionProvenanceKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::ApplicationUser => "application_user",
            Self::AgentSession => "agent_session",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "application_user" => Some(Self::ApplicationUser),
            "agent_session" => Some(Self::AgentSession),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessRevisionCreationProvenance {
    pub(crate) kind: HarnessRevisionProvenanceKind,
    pub(crate) reference: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateHarnessRevisionCommand {
    pub(crate) harness_key: String,
    pub(crate) expected_source_draft_revision: u64,
    pub(crate) expected_predecessor_revision_id: Option<String>,
    pub(crate) idempotency_key: String,
    pub(crate) creation_provenance: HarnessRevisionCreationProvenance,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessRevision {
    pub(crate) revision_id: String,
    pub(crate) harness_key: String,
    pub(crate) configuration: HarnessEffectiveConfiguration,
    pub(crate) configuration_digest: String,
    pub(crate) source_draft_revision: u64,
    pub(crate) predecessor_revision_id: Option<String>,
    pub(crate) repository_commit_ref: String,
    pub(crate) creation_provenance: HarnessRevisionCreationProvenance,
    pub(crate) created_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CreateHarnessRevisionResult {
    Published(HarnessRevision),
    IdempotentReplay(HarnessRevision),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum HarnessRevisionError {
    Invalid,
    IncompleteConfiguration,
    MissingWorkingCopy,
    Conflict,
    InvalidStoredState,
    InvalidLocalCommitEvidence,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum HarnessRevisionReadOutcome {
    AvailableAndVerified { revision: HarnessRevision },
    Missing,
    InvalidLocalCommitEvidence,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "status",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum HarnessRevisionHistoryOutcome {
    AvailableAndVerified { revisions: Vec<HarnessRevision> },
    Missing,
    InvalidLocalCommitEvidence,
    Unavailable,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessRevisionCommitManifest {
    contract_version: String,
    revision_id: String,
    harness_key: String,
    configuration_contract_version: String,
    configuration_digest: String,
    source_draft_revision: u64,
    predecessor_revision_id: Option<String>,
    repository_commit_ref: String,
    creation_provenance: HarnessRevisionCreationProvenance,
    created_at: DateTime<Utc>,
}

impl HarnessRevisionCommitManifest {
    pub(crate) fn for_revision(revision: &HarnessRevision) -> Self {
        Self::from_metadata(
            revision.revision_id.clone(),
            revision.harness_key.clone(),
            revision.configuration_digest.clone(),
            revision.source_draft_revision,
            revision.predecessor_revision_id.clone(),
            revision.repository_commit_ref.clone(),
            revision.creation_provenance.clone(),
            revision.created_at,
        )
    }

    #[allow(clippy::too_many_arguments)]
    pub(crate) fn from_metadata(
        revision_id: String,
        harness_key: String,
        configuration_digest: String,
        source_draft_revision: u64,
        predecessor_revision_id: Option<String>,
        repository_commit_ref: String,
        creation_provenance: HarnessRevisionCreationProvenance,
        created_at: DateTime<Utc>,
    ) -> Self {
        Self {
            contract_version: HARNESS_REVISION_COMMIT_CONTRACT_V1.into(),
            revision_id,
            harness_key,
            configuration_contract_version: HARNESS_EFFECTIVE_CONFIGURATION_V1.into(),
            configuration_digest,
            source_draft_revision,
            predecessor_revision_id,
            repository_commit_ref,
            creation_provenance,
            created_at,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct LocalHarnessRevisionRepository {
    root: PathBuf,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum LocalHarnessRevisionRepositoryError {
    InvalidEvidence,
    Unavailable,
}

impl LocalHarnessRevisionRepository {
    pub(crate) fn new(root: PathBuf) -> Self {
        Self { root }
    }

    pub(crate) fn root(&self) -> &Path {
        &self.root
    }

    pub(crate) fn commit_reference(revision_id: &str) -> String {
        format!("{HARNESS_REVISION_COMMIT_CONTRACT_V1}/{revision_id}")
    }

    pub(crate) fn install_and_verify(
        &self,
        manifest: &HarnessRevisionCommitManifest,
        normalized_envelope: &[u8],
    ) -> Result<(), LocalHarnessRevisionRepositoryError> {
        validate_revision_id(&manifest.revision_id)
            .map_err(|_| LocalHarnessRevisionRepositoryError::InvalidEvidence)?;
        validate_digest(&manifest.configuration_digest)
            .map_err(|_| LocalHarnessRevisionRepositoryError::InvalidEvidence)?;
        if manifest.contract_version != HARNESS_REVISION_COMMIT_CONTRACT_V1
            || manifest.configuration_contract_version != HARNESS_EFFECTIVE_CONFIGURATION_V1
            || manifest.repository_commit_ref != Self::commit_reference(&manifest.revision_id)
            || sha256(normalized_envelope) != manifest.configuration_digest
        {
            return Err(LocalHarnessRevisionRepositoryError::InvalidEvidence);
        }
        let object_path = self.object_path(&manifest.configuration_digest);
        write_immutable(&object_path, normalized_envelope)?;
        let manifest_bytes = serde_json::to_vec(manifest)
            .map_err(|_| LocalHarnessRevisionRepositoryError::InvalidEvidence)?;
        let commit_path = self.commit_path(&manifest.revision_id);
        write_immutable(&commit_path, &manifest_bytes)?;
        self.read_and_verify(&manifest.repository_commit_ref, manifest)
            .map(|_| ())
    }

    pub(crate) fn read_and_verify(
        &self,
        repository_commit_ref: &str,
        expected: &HarnessRevisionCommitManifest,
    ) -> Result<Vec<u8>, LocalHarnessRevisionRepositoryError> {
        let revision_id = parse_commit_reference(repository_commit_ref)?;
        if revision_id != expected.revision_id {
            return Err(LocalHarnessRevisionRepositoryError::InvalidEvidence);
        }
        let manifest_bytes = read_evidence(&self.commit_path(revision_id))?;
        let manifest: HarnessRevisionCommitManifest = serde_json::from_slice(&manifest_bytes)
            .map_err(|_| LocalHarnessRevisionRepositoryError::InvalidEvidence)?;
        if &manifest != expected
            || manifest.contract_version != HARNESS_REVISION_COMMIT_CONTRACT_V1
            || manifest.configuration_contract_version != HARNESS_EFFECTIVE_CONFIGURATION_V1
            || manifest.repository_commit_ref != repository_commit_ref
        {
            return Err(LocalHarnessRevisionRepositoryError::InvalidEvidence);
        }
        validate_digest(&manifest.configuration_digest)
            .map_err(|_| LocalHarnessRevisionRepositoryError::InvalidEvidence)?;
        let envelope = read_evidence(&self.object_path(&manifest.configuration_digest))?;
        if sha256(&envelope) != manifest.configuration_digest {
            return Err(LocalHarnessRevisionRepositoryError::InvalidEvidence);
        }
        Ok(envelope)
    }

    fn object_path(&self, digest: &str) -> PathBuf {
        self.root
            .join("objects")
            .join("sha256")
            .join(&digest[..2])
            .join(digest)
    }

    fn commit_path(&self, revision_id: &str) -> PathBuf {
        self.root
            .join("commits")
            .join(format!("{revision_id}.json"))
    }

    #[cfg(test)]
    pub(crate) fn object_path_for_test(&self, digest: &str) -> PathBuf {
        self.object_path(digest)
    }

    #[cfg(test)]
    pub(crate) fn commit_path_for_test(&self, revision_id: &str) -> PathBuf {
        self.commit_path(revision_id)
    }
}

pub(crate) fn validate_create_command(
    command: &CreateHarnessRevisionCommand,
) -> Result<(), HarnessRevisionError> {
    validate_harness_key(&command.harness_key).map_err(|_| HarnessRevisionError::Invalid)?;
    if command.expected_source_draft_revision == 0
        || command.expected_source_draft_revision > i64::MAX as u64
        || !bounded_token(&command.idempotency_key, 240)
        || !bounded_token(&command.creation_provenance.reference, 240)
        || command
            .expected_predecessor_revision_id
            .as_deref()
            .is_some_and(|value| validate_revision_id(value).is_err())
    {
        return Err(HarnessRevisionError::Invalid);
    }
    Ok(())
}

pub(crate) fn validate_revision(revision: &HarnessRevision) -> Result<(), HarnessRevisionError> {
    validate_revision_id(&revision.revision_id)?;
    validate_harness_key(&revision.harness_key)
        .map_err(|_| HarnessRevisionError::InvalidStoredState)?;
    validate_digest(&revision.configuration_digest)?;
    if revision.source_draft_revision == 0
        || revision.source_draft_revision > i64::MAX as u64
        || revision.configuration.identity.machine_key != revision.harness_key
        || revision.repository_commit_ref
            != LocalHarnessRevisionRepository::commit_reference(&revision.revision_id)
        || !bounded_token(&revision.creation_provenance.reference, 240)
        || revision
            .predecessor_revision_id
            .as_deref()
            .is_some_and(|value| validate_revision_id(value).is_err())
    {
        return Err(HarnessRevisionError::InvalidStoredState);
    }
    validate_complete_configuration(&revision.configuration)
        .map_err(|_| HarnessRevisionError::InvalidStoredState)
}

pub(crate) fn normalized_configuration_envelope(
    configuration: &HarnessEffectiveConfiguration,
) -> Result<(Vec<u8>, String), HarnessRevisionError> {
    validate_complete_configuration(configuration)
        .map_err(|_| HarnessRevisionError::IncompleteConfiguration)?;
    let envelope = HarnessEffectiveConfigurationEnvelope {
        contract_version: HARNESS_EFFECTIVE_CONFIGURATION_V1.into(),
        configuration: configuration.clone(),
    };
    let bytes = serde_json::to_vec(&envelope).map_err(|_| HarnessRevisionError::Invalid)?;
    let digest = sha256(&bytes);
    Ok((bytes, digest))
}

pub(crate) fn decode_verified_configuration(
    bytes: &[u8],
    expected_digest: &str,
) -> Result<HarnessEffectiveConfiguration, HarnessRevisionError> {
    if sha256(bytes) != expected_digest {
        return Err(HarnessRevisionError::InvalidLocalCommitEvidence);
    }
    let envelope: HarnessEffectiveConfigurationEnvelope = serde_json::from_slice(bytes)
        .map_err(|_| HarnessRevisionError::InvalidLocalCommitEvidence)?;
    if envelope.contract_version != HARNESS_EFFECTIVE_CONFIGURATION_V1 {
        return Err(HarnessRevisionError::InvalidLocalCommitEvidence);
    }
    validate_complete_configuration(&envelope.configuration)
        .map_err(|_| HarnessRevisionError::InvalidLocalCommitEvidence)?;
    Ok(envelope.configuration)
}

pub(crate) fn revision_id() -> String {
    format!("harness-revision-{}", Uuid::new_v4())
}

fn validate_revision_id(value: &str) -> Result<(), HarnessRevisionError> {
    let Some(suffix) = value.strip_prefix("harness-revision-") else {
        return Err(HarnessRevisionError::InvalidStoredState);
    };
    Uuid::parse_str(suffix)
        .map(|_| ())
        .map_err(|_| HarnessRevisionError::InvalidStoredState)
}

fn validate_digest(value: &str) -> Result<(), HarnessRevisionError> {
    if value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
    {
        Ok(())
    } else {
        Err(HarnessRevisionError::InvalidStoredState)
    }
}

fn parse_commit_reference(value: &str) -> Result<&str, LocalHarnessRevisionRepositoryError> {
    let prefix = format!("{HARNESS_REVISION_COMMIT_CONTRACT_V1}/");
    let revision_id = value
        .strip_prefix(&prefix)
        .ok_or(LocalHarnessRevisionRepositoryError::InvalidEvidence)?;
    validate_revision_id(revision_id)
        .map_err(|_| LocalHarnessRevisionRepositoryError::InvalidEvidence)?;
    Ok(revision_id)
}

fn bounded_text(value: &str, maximum: usize) -> bool {
    !value.trim().is_empty()
        && value.len() <= maximum
        && value
            .chars()
            .all(|character| !character.is_control() || matches!(character, '\n' | '\r' | '\t'))
}

fn bounded_token(value: &str, maximum: usize) -> bool {
    bounded_text(value, maximum) && value.trim() == value
}

fn sha256(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn read_evidence(path: &Path) -> Result<Vec<u8>, LocalHarnessRevisionRepositoryError> {
    match fs::read(path) {
        Ok(bytes) => Ok(bytes),
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Err(LocalHarnessRevisionRepositoryError::InvalidEvidence)
        }
        Err(_) => Err(LocalHarnessRevisionRepositoryError::Unavailable),
    }
}

fn write_immutable(path: &Path, bytes: &[u8]) -> Result<(), LocalHarnessRevisionRepositoryError> {
    if path.exists() {
        return if read_evidence(path)? == bytes {
            Ok(())
        } else {
            Err(LocalHarnessRevisionRepositoryError::InvalidEvidence)
        };
    }
    let parent = path
        .parent()
        .ok_or(LocalHarnessRevisionRepositoryError::Unavailable)?;
    fs::create_dir_all(parent).map_err(|_| LocalHarnessRevisionRepositoryError::Unavailable)?;
    let temporary = parent.join(format!(".install-{}.tmp", Uuid::new_v4()));
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&temporary)
        .map_err(|_| LocalHarnessRevisionRepositoryError::Unavailable)?;
    if file.write_all(bytes).is_err() || file.sync_all().is_err() {
        let _ = fs::remove_file(&temporary);
        return Err(LocalHarnessRevisionRepositoryError::Unavailable);
    }
    drop(file);
    match fs::rename(&temporary, path) {
        Ok(()) => Ok(()),
        Err(_) if path.exists() => {
            let _ = fs::remove_file(&temporary);
            if read_evidence(path)? == bytes {
                Ok(())
            } else {
                Err(LocalHarnessRevisionRepositoryError::InvalidEvidence)
            }
        }
        Err(_) => {
            let _ = fs::remove_file(&temporary);
            Err(LocalHarnessRevisionRepositoryError::Unavailable)
        }
    }
}

impl From<HarnessWorkingCopyError> for HarnessRevisionError {
    fn from(value: HarnessWorkingCopyError) -> Self {
        match value {
            HarnessWorkingCopyError::Invalid => Self::Invalid,
            HarnessWorkingCopyError::Conflict => Self::Conflict,
            HarnessWorkingCopyError::InvalidStoredState => Self::InvalidStoredState,
            HarnessWorkingCopyError::Unavailable => Self::Unavailable,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::orchestration::{
        application::OrchestrationApplication,
        conversation_harness_working_copy::*,
        repository::{OrchestrationClock, SqliteOrchestrationRepository},
    };
    use chrono::TimeZone;
    use rusqlite::Connection;
    use std::{fs, path::Path, sync::Arc};

    struct FixedClock(DateTime<Utc>);
    impl OrchestrationClock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    fn fixed_time() -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 8, 2, 12, 0, 0).single().unwrap()
    }

    fn open_repository(
        database_path: &Path,
        repository_root: &Path,
    ) -> Arc<SqliteOrchestrationRepository> {
        let connection = Connection::open(database_path).unwrap();
        crate::storage::configure_sqlite_connection(&connection).unwrap();
        crate::storage::initialize_active_database(&connection).unwrap();
        Arc::new(
            SqliteOrchestrationRepository::new_with_clock_and_harness_revision_repository(
                connection,
                Arc::new(FixedClock(fixed_time())),
                repository_root.to_path_buf(),
            )
            .unwrap(),
        )
    }

    fn configuration(key: &str) -> HarnessEffectiveConfiguration {
        HarnessEffectiveConfiguration {
            identity: HarnessIdentityConfiguration {
                name: "Epic Plan Builder".into(),
                machine_key: key.into(),
                permitted_agent_names: Some(vec!["Avery".into()]),
                visual_identity: Some(HarnessVisualIdentity {
                    token: "sunflower".into(),
                    accent: "gold".into(),
                }),
            },
            prompt_prefix: HarnessPromptPrefixConfiguration {
                content: "Build the Epic plan from the accepted discussion.".into(),
                initial_delivery: HarnessInitialDelivery::Prepend,
                context_compression_delivery: HarnessContextCompressionDelivery::Deferred,
            },
            skills: HarnessSkillsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: vec![HarnessSkillConfiguration {
                    name: "epic-plan-builder".into(),
                    path: ".agents/skills/epic-plan-builder/SKILL.md".into(),
                    purpose: "Build one Epic proposal.".into(),
                    use_when: "The product owns an Epic planning Session.".into(),
                    policy: HarnessSkillPolicy::AlwaysApplicable,
                }],
            },
            tools: HarnessToolsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: vec![HarnessToolConfiguration {
                    name: "submit_epic_plan_proposal".into(),
                    policy: HarnessToolPolicy::Available,
                }],
                schema_boundary: "Application-owned proposal schema.".into(),
            },
            runtime: HarnessRuntimeConfiguration {
                model_policy_mode: HarnessModelPolicyMode::RevisionOwned,
                models: vec![HarnessModelConstraint {
                    model_id: "gpt-5.6-terra".into(),
                    allowed: true,
                    min_reasoning: HarnessReasoningLevel::Medium,
                    max_reasoning: HarnessReasoningLevel::High,
                }],
                default_model: Some("gpt-5.6-terra".into()),
                default_reasoning: Some(HarnessReasoningLevel::Medium),
                sandbox: HarnessSandbox::ReadOnly,
                sandbox_options: vec![HarnessSandbox::ReadOnly, HarnessSandbox::WorkspaceWrite],
                approval_policy: HarnessApprovalPolicy::Never,
                approval_policy_options: vec![HarnessApprovalPolicy::Never],
                authority_summary: "Plan discussion and proposal submission only.".into(),
            },
            hooks: vec![HarnessHookConfiguration {
                name: "completion".into(),
                status: HarnessHookStatus::NotConnected,
                detail: "No typed completion hook is connected.".into(),
            }],
            update_policy: HarnessUpdatePolicy::NotConfigured {
                reason: "Runtime updates are deferred.".into(),
            },
        }
    }

    fn save_command(
        key: &str,
        expected: u64,
        idempotency_key: &str,
        configuration: HarnessEffectiveConfiguration,
    ) -> SaveHarnessWorkingCopyCommand {
        SaveHarnessWorkingCopyCommand {
            harness_key: key.into(),
            configuration,
            expected_current_revision: expected,
            editor: HarnessWorkingCopyEditor {
                kind: HarnessEditorKind::ApplicationUser,
                reference: "local-user".into(),
            },
            idempotency_key: idempotency_key.into(),
        }
    }

    fn create_command(
        key: &str,
        draft_revision: u64,
        predecessor: Option<String>,
        idempotency_key: &str,
    ) -> CreateHarnessRevisionCommand {
        CreateHarnessRevisionCommand {
            harness_key: key.into(),
            expected_source_draft_revision: draft_revision,
            expected_predecessor_revision_id: predecessor,
            idempotency_key: idempotency_key.into(),
            creation_provenance: HarnessRevisionCreationProvenance {
                kind: HarnessRevisionProvenanceKind::ApplicationUser,
                reference: "local-user".into(),
            },
        }
    }

    fn published(result: CreateHarnessRevisionResult) -> HarnessRevision {
        let CreateHarnessRevisionResult::Published(revision) = result else {
            panic!("expected publication")
        };
        revision
    }

    fn row_count(database_path: &Path, table: &str) -> i64 {
        Connection::open(database_path)
            .unwrap()
            .query_row(&format!("SELECT count(*) FROM {table}"), [], |row| {
                row.get(0)
            })
            .unwrap()
    }

    #[test]
    fn publishes_root_and_successor_with_verified_exact_reads_and_linear_history() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        let repository_root = directory.path().join("harness-revisions");
        let repository = open_repository(&database_path, &repository_root);
        let application = OrchestrationApplication::new(repository);
        let first_configuration = configuration("epic_plan_builder");
        application
            .save_harness_working_copy(save_command(
                "epic_plan_builder",
                0,
                "save-1",
                first_configuration.clone(),
            ))
            .unwrap();

        let first = published(
            application
                .create_harness_revision(create_command("epic_plan_builder", 1, None, "publish-1"))
                .unwrap(),
        );
        assert!(first.revision_id.starts_with("harness-revision-"));
        assert_eq!(first.source_draft_revision, 1);
        assert_eq!(first.predecessor_revision_id, None);
        assert_eq!(first.configuration, first_configuration);
        assert_eq!(first.created_at, fixed_time());
        assert_eq!(first.configuration_digest.len(), 64);
        assert_eq!(
            first.repository_commit_ref,
            LocalHarnessRevisionRepository::commit_reference(&first.revision_id)
        );
        assert!(repository_root.starts_with(directory.path()));

        let mut second_configuration = configuration("epic_plan_builder");
        second_configuration.prompt_prefix.content = "Second complete prompt.".into();
        application
            .save_harness_working_copy(save_command(
                "epic_plan_builder",
                1,
                "save-2",
                second_configuration.clone(),
            ))
            .unwrap();
        let second = published(
            application
                .create_harness_revision(create_command(
                    "epic_plan_builder",
                    2,
                    Some(first.revision_id.clone()),
                    "publish-2",
                ))
                .unwrap(),
        );
        assert_eq!(
            second.predecessor_revision_id,
            Some(first.revision_id.clone())
        );
        assert_ne!(first.revision_id, second.revision_id);
        assert_ne!(first.configuration_digest, second.configuration_digest);
        assert_eq!(
            application.load_harness_revision(&first.revision_id),
            HarnessRevisionReadOutcome::AvailableAndVerified {
                revision: first.clone()
            }
        );
        assert_eq!(
            application.load_harness_revision_history("epic_plan_builder"),
            HarnessRevisionHistoryOutcome::AvailableAndVerified {
                revisions: vec![first, second]
            }
        );
    }

    #[test]
    fn replay_is_stable_after_later_draft_save_and_conflicts_fail_closed() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        let repository_root = directory.path().join("harness-revisions");
        let repository = open_repository(&database_path, &repository_root);
        let application = OrchestrationApplication::new(repository);
        application
            .save_harness_working_copy(save_command(
                "epic_plan_builder",
                0,
                "save-1",
                configuration("epic_plan_builder"),
            ))
            .unwrap();
        let original_command = create_command("epic_plan_builder", 1, None, "publish-1");
        let original = published(
            application
                .create_harness_revision(original_command.clone())
                .unwrap(),
        );
        assert_eq!(
            application.create_harness_revision(create_command(
                "epic_plan_builder",
                1,
                Some(original.revision_id.clone()),
                "duplicate-source",
            )),
            Err(HarnessRevisionError::Conflict)
        );
        let mut later = configuration("epic_plan_builder");
        later.prompt_prefix.content = "A later mutable draft.".into();
        application
            .save_harness_working_copy(save_command("epic_plan_builder", 1, "save-2", later))
            .unwrap();

        assert_eq!(
            application
                .create_harness_revision(original_command.clone())
                .unwrap(),
            CreateHarnessRevisionResult::IdempotentReplay(original.clone())
        );
        let mut mismatched_replay = original_command;
        mismatched_replay.creation_provenance.reference = "another-user".into();
        assert_eq!(
            application.create_harness_revision(mismatched_replay),
            Err(HarnessRevisionError::Conflict)
        );
        assert_eq!(
            application.create_harness_revision(create_command(
                "epic_plan_builder",
                1,
                Some(original.revision_id.clone()),
                "stale-draft",
            )),
            Err(HarnessRevisionError::Conflict)
        );
        assert_eq!(
            application.create_harness_revision(create_command(
                "epic_plan_builder",
                2,
                None,
                "stale-predecessor",
            )),
            Err(HarnessRevisionError::Conflict)
        );
        assert_eq!(row_count(&database_path, "harness_revisions"), 1);
        assert_eq!(
            application.load_harness_revision(&original.revision_id),
            HarnessRevisionReadOutcome::AvailableAndVerified { revision: original }
        );
    }

    #[test]
    fn incomplete_draft_cannot_publish_but_remains_losslessly_mutable() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        let repository_root = directory.path().join("harness-revisions");
        let repository = open_repository(&database_path, &repository_root);
        let application = OrchestrationApplication::new(repository);
        let mut incomplete = configuration("epic_plan_builder");
        incomplete.prompt_prefix.content.clear();
        application
            .save_harness_working_copy(save_command(
                "epic_plan_builder",
                0,
                "partial-save",
                incomplete.clone(),
            ))
            .unwrap();

        assert_eq!(
            application.create_harness_revision(create_command(
                "epic_plan_builder",
                1,
                None,
                "publish-partial",
            )),
            Err(HarnessRevisionError::IncompleteConfiguration)
        );
        assert_eq!(row_count(&database_path, "harness_revisions"), 0);
        assert_eq!(
            application
                .load_harness_working_copy("epic_plan_builder")
                .unwrap()
                .unwrap()
                .configuration,
            incomplete
        );
    }

    #[test]
    fn repository_failure_leaves_sqlite_unpublished() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        let repository_root = directory.path().join("not-a-directory");
        fs::write(&repository_root, b"blocks repository creation").unwrap();
        let repository = open_repository(&database_path, &repository_root);
        let application = OrchestrationApplication::new(repository);
        application
            .save_harness_working_copy(save_command(
                "epic_plan_builder",
                0,
                "save-1",
                configuration("epic_plan_builder"),
            ))
            .unwrap();

        assert_eq!(
            application.create_harness_revision(create_command(
                "epic_plan_builder",
                1,
                None,
                "publish-1",
            )),
            Err(HarnessRevisionError::Unavailable)
        );
        for table in [
            "harness_revisions",
            "harness_revision_publications",
            "harness_revision_commands",
        ] {
            assert_eq!(
                row_count(&database_path, table),
                0,
                "unexpected {table} row"
            );
        }
    }

    #[test]
    fn sqlite_failure_leaves_only_unpublished_local_orphans() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        let repository_root = directory.path().join("harness-revisions");
        let repository = open_repository(&database_path, &repository_root);
        let application = OrchestrationApplication::new(repository);
        application
            .save_harness_working_copy(save_command(
                "epic_plan_builder",
                0,
                "save-1",
                configuration("epic_plan_builder"),
            ))
            .unwrap();
        Connection::open(&database_path)
            .unwrap()
            .execute_batch("CREATE TRIGGER reject_harness_revision_command BEFORE INSERT ON harness_revision_commands BEGIN SELECT RAISE(ABORT, 'test rollback'); END;")
            .unwrap();

        assert_eq!(
            application.create_harness_revision(create_command(
                "epic_plan_builder",
                1,
                None,
                "publish-1",
            )),
            Err(HarnessRevisionError::Unavailable)
        );
        for table in [
            "harness_revisions",
            "harness_revision_publications",
            "harness_revision_commands",
        ] {
            assert_eq!(
                row_count(&database_path, table),
                0,
                "unexpected {table} row"
            );
        }
        assert_eq!(
            fs::read_dir(repository_root.join("commits"))
                .unwrap()
                .count(),
            1
        );
        assert_eq!(
            application.load_harness_revision_history("epic_plan_builder"),
            HarnessRevisionHistoryOutcome::Missing
        );
    }

    #[test]
    fn missing_or_tampered_local_evidence_is_never_reported_available() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        let repository_root = directory.path().join("harness-revisions");
        let repository = open_repository(&database_path, &repository_root);
        let local_repository = LocalHarnessRevisionRepository::new(repository_root.clone());
        let application = OrchestrationApplication::new(repository);
        application
            .save_harness_working_copy(save_command(
                "epic_plan_builder",
                0,
                "save-1",
                configuration("epic_plan_builder"),
            ))
            .unwrap();
        let revision = published(
            application
                .create_harness_revision(create_command("epic_plan_builder", 1, None, "publish-1"))
                .unwrap(),
        );
        fs::write(
            local_repository.commit_path_for_test(&revision.revision_id),
            b"{}",
        )
        .unwrap();

        assert_eq!(
            application.load_harness_revision(&revision.revision_id),
            HarnessRevisionReadOutcome::InvalidLocalCommitEvidence
        );
        assert_eq!(
            application.load_harness_revision_history("epic_plan_builder"),
            HarnessRevisionHistoryOutcome::InvalidLocalCommitEvidence
        );
        let mut next = configuration("epic_plan_builder");
        next.prompt_prefix.content = "Next complete prompt.".into();
        application
            .save_harness_working_copy(save_command("epic_plan_builder", 1, "save-2", next))
            .unwrap();
        assert_eq!(
            application.create_harness_revision(create_command(
                "epic_plan_builder",
                2,
                Some(revision.revision_id),
                "publish-2",
            )),
            Err(HarnessRevisionError::InvalidLocalCommitEvidence)
        );
        assert_eq!(row_count(&database_path, "harness_revisions"), 1);
    }

    #[test]
    fn missing_content_object_and_invalid_publication_ledger_fail_strict_reads() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        let repository_root = directory.path().join("harness-revisions");
        let local_repository = LocalHarnessRevisionRepository::new(repository_root.clone());
        let repository = open_repository(&database_path, &repository_root);
        let application = OrchestrationApplication::new(repository);
        application
            .save_harness_working_copy(save_command(
                "epic_plan_builder",
                0,
                "save-1",
                configuration("epic_plan_builder"),
            ))
            .unwrap();
        let revision = published(
            application
                .create_harness_revision(create_command("epic_plan_builder", 1, None, "publish-1"))
                .unwrap(),
        );
        fs::remove_file(local_repository.object_path_for_test(&revision.configuration_digest))
            .unwrap();
        assert_eq!(
            application.load_harness_revision(&revision.revision_id),
            HarnessRevisionReadOutcome::InvalidLocalCommitEvidence
        );
        drop(application);

        let repository = open_repository(&database_path, &repository_root);
        drop(repository);
        Connection::open(&database_path)
            .unwrap()
            .execute(
                "DELETE FROM harness_revision_publications WHERE revision_id=?1",
                [&revision.revision_id],
            )
            .unwrap();
        let repository = open_repository(&database_path, &repository_root);
        let application = OrchestrationApplication::new(repository);
        assert_eq!(
            application.load_harness_revision(&revision.revision_id),
            HarnessRevisionReadOutcome::InvalidLocalCommitEvidence
        );
    }

    #[test]
    fn sqlite_storage_class_tamper_is_invalid_evidence_and_blocks_successor_publication() {
        for tamper in [
            "UPDATE harness_revisions SET configuration_digest=zeroblob(64)",
            "UPDATE harness_revisions SET source_draft_revision='abc'",
        ] {
            let directory = tempfile::tempdir().unwrap();
            let database_path = directory.path().join("active.sqlite");
            let repository_root = directory.path().join("harness-revisions");
            let repository = open_repository(&database_path, &repository_root);
            let application = OrchestrationApplication::new(repository);
            application
                .save_harness_working_copy(save_command(
                    "epic_plan_builder",
                    0,
                    "save-1",
                    configuration("epic_plan_builder"),
                ))
                .unwrap();
            let revision = published(
                application
                    .create_harness_revision(create_command(
                        "epic_plan_builder",
                        1,
                        None,
                        "publish-1",
                    ))
                    .unwrap(),
            );
            let mut successor_configuration = configuration("epic_plan_builder");
            successor_configuration.prompt_prefix.content = "Complete successor prompt.".into();
            application
                .save_harness_working_copy(save_command(
                    "epic_plan_builder",
                    1,
                    "save-2",
                    successor_configuration,
                ))
                .unwrap();
            Connection::open(&database_path)
                .unwrap()
                .execute_batch(tamper)
                .unwrap();

            assert_eq!(
                application.load_harness_revision(&revision.revision_id),
                HarnessRevisionReadOutcome::InvalidLocalCommitEvidence,
                "exact read accepted {tamper}"
            );
            assert_eq!(
                application.load_harness_revision_history("epic_plan_builder"),
                HarnessRevisionHistoryOutcome::InvalidLocalCommitEvidence,
                "history accepted {tamper}"
            );
            assert_eq!(
                application.create_harness_revision(create_command(
                    "epic_plan_builder",
                    2,
                    Some(revision.revision_id),
                    "publish-2",
                )),
                Err(HarnessRevisionError::InvalidStoredState),
                "successor did not fail on {tamper}"
            );
            assert_eq!(row_count(&database_path, "harness_revisions"), 1);
            assert_eq!(
                row_count(&database_path, "harness_revision_publications"),
                1
            );
            assert_eq!(row_count(&database_path, "harness_revision_commands"), 1);
        }
    }

    #[test]
    fn unavailable_revision_tables_remain_operationally_unavailable() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        let repository_root = directory.path().join("harness-revisions");
        let repository = open_repository(&database_path, &repository_root);
        let application = OrchestrationApplication::new(repository);
        Connection::open(&database_path)
            .unwrap()
            .execute_batch("DROP TABLE harness_revisions")
            .unwrap();

        assert_eq!(
            application
                .load_harness_revision("harness-revision-00000000-0000-0000-0000-000000000000"),
            HarnessRevisionReadOutcome::Unavailable
        );
        assert_eq!(
            application.load_harness_revision_history("epic_plan_builder"),
            HarnessRevisionHistoryOutcome::Unavailable
        );
    }

    #[test]
    fn malformed_history_revision_identity_is_invalid_local_evidence() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        let repository_root = directory.path().join("harness-revisions");
        let repository = open_repository(&database_path, &repository_root);
        let application = OrchestrationApplication::new(repository);
        application
            .save_harness_working_copy(save_command(
                "epic_plan_builder",
                0,
                "save-1",
                configuration("epic_plan_builder"),
            ))
            .unwrap();
        application
            .create_harness_revision(create_command("epic_plan_builder", 1, None, "publish-1"))
            .unwrap();
        let tamper = Connection::open(&database_path).unwrap();
        tamper.execute_batch("PRAGMA foreign_keys=OFF").unwrap();
        tamper
            .execute_batch("UPDATE harness_revisions SET revision_id=zeroblob(64)")
            .unwrap();

        assert_eq!(
            application.load_harness_revision_history("epic_plan_builder"),
            HarnessRevisionHistoryOutcome::InvalidLocalCommitEvidence
        );
    }

    #[test]
    fn history_row_step_failure_is_operationally_unavailable() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        let repository_root = directory.path().join("harness-revisions");
        let repository = open_repository(&database_path, &repository_root);
        let application = OrchestrationApplication::new(repository);
        application
            .save_harness_working_copy(save_command(
                "epic_plan_builder",
                0,
                "save-1",
                configuration("epic_plan_builder"),
            ))
            .unwrap();
        application
            .create_harness_revision(create_command("epic_plan_builder", 1, None, "publish-1"))
            .unwrap();

        let sabotage = Connection::open(&database_path).unwrap();
        sabotage
            .execute_batch(
                "ALTER TABLE harness_revisions RENAME TO harness_revisions_backing;
                 CREATE VIEW harness_revisions AS
                 SELECT abs(-9223372036854775808) AS revision_id,
                        harness_key,
                        source_draft_revision
                 FROM harness_revisions_backing;",
            )
            .unwrap();
        {
            let mut statement = sabotage
                .prepare(
                    "SELECT revision_id FROM harness_revisions WHERE harness_key=?1 ORDER BY source_draft_revision,revision_id",
                )
                .unwrap();
            let mut rows = statement.query(["epic_plan_builder"]).unwrap();
            assert!(rows.next().is_err(), "expected SQLite row-step failure");
        }

        assert_eq!(
            application.load_harness_revision_history("epic_plan_builder"),
            HarnessRevisionHistoryOutcome::Unavailable
        );
    }

    #[test]
    fn reopen_retains_verified_revision_and_repository_path() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("active.sqlite");
        let repository_root = directory.path().join("stable-harness-revisions");
        let repository = open_repository(&database_path, &repository_root);
        assert_eq!(
            repository.harness_revision_repository_root(),
            repository_root
        );
        let application = OrchestrationApplication::new(repository);
        application
            .save_harness_working_copy(save_command(
                "epic_plan_builder",
                0,
                "save-1",
                configuration("epic_plan_builder"),
            ))
            .unwrap();
        let revision = published(
            application
                .create_harness_revision(create_command("epic_plan_builder", 1, None, "publish-1"))
                .unwrap(),
        );
        drop(application);

        let reopened = open_repository(&database_path, &repository_root);
        assert_eq!(reopened.harness_revision_repository_root(), repository_root);
        let reopened = OrchestrationApplication::new(reopened);
        assert_eq!(
            reopened.load_harness_revision(&revision.revision_id),
            HarnessRevisionReadOutcome::AvailableAndVerified { revision }
        );
    }

    #[test]
    fn application_contract_exposes_no_repository_or_content_authority() {
        let command_json =
            serde_json::to_value(create_command("epic_plan_builder", 1, None, "publish-1"))
                .unwrap();
        for forbidden in [
            "path",
            "cwd",
            "repository",
            "worktree",
            "branch",
            "ref",
            "commit",
            "configuration",
            "envelope",
        ] {
            assert!(command_json.get(forbidden).is_none());
        }
        let mut unknown = command_json;
        unknown
            .as_object_mut()
            .unwrap()
            .insert("repositoryPath".into(), serde_json::json!("C:/project"));
        assert!(serde_json::from_value::<CreateHarnessRevisionCommand>(unknown).is_err());
        assert_eq!(
            serde_json::to_value(HarnessRevisionReadOutcome::Missing).unwrap()["status"],
            "missing"
        );
        assert_eq!(
            serde_json::to_value(HarnessRevisionReadOutcome::InvalidLocalCommitEvidence).unwrap()
                ["status"],
            "invalid_local_commit_evidence"
        );
        assert_eq!(
            serde_json::to_value(HarnessRevisionHistoryOutcome::Unavailable).unwrap()["status"],
            "unavailable"
        );
    }
}
