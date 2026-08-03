//! Attempt-scoped execution support for a later authorized Harness action.
//!
//! Harness-facing calls carry an opaque capability and semantic intents only. Application-owned
//! durable authority retains all Epic, Sprint, Work Unit, role, repository, workspace, and Git
//! routing facts.

use super::{
    file_review_git_producer::{produce_file_review_from_git, ProduceFileReviewFromGit},
    repository::{
        FileReviewGitCaptureAuthorizationWrite, InitiatedSprintGitAuthority, ScopedFileReviewLoad,
        SqliteOrchestrationRepository,
    },
};
use rusqlite::{params, Connection, OptionalExtension, TransactionBehavior};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt, fs,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

pub(crate) const EXECUTION_SUPPORT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS execution_support_attempt_authorizations (
  attempt_id TEXT NOT NULL,
  work_unit_id TEXT NOT NULL,
  role_kind TEXT NOT NULL CHECK(role_kind IN ('work_unit_handler','work_unit_implementer')),
  sprint_git_authority_id TEXT NOT NULL,
  baseline_object_id TEXT NOT NULL,
  authorization_fingerprint TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  FOREIGN KEY(sprint_git_authority_id) REFERENCES initiated_sprint_git_authorities(authority_id) ON DELETE RESTRICT,
  PRIMARY KEY(attempt_id,role_kind)
);
CREATE INDEX IF NOT EXISTS execution_support_attempt_authorizations_sprint_authority
  ON execution_support_attempt_authorizations(sprint_git_authority_id);
CREATE TABLE IF NOT EXISTS execution_support_grants (
  attempt_id TEXT NOT NULL,
  capability_ref TEXT NOT NULL UNIQUE,
  epic_id TEXT NOT NULL,
  sprint_id TEXT NOT NULL,
  work_unit_id TEXT NOT NULL,
  repository_id TEXT NOT NULL,
  role_id TEXT NOT NULL,
  workspace_id TEXT NOT NULL,
  workspace_fingerprint TEXT NOT NULL,
  correlation_fingerprint TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  PRIMARY KEY(attempt_id,role_id)
);
"#;
const EXECUTION_SUPPORT_ROLE_KEY_MIGRATION: &str = r#"
CREATE TABLE execution_support_attempt_authorizations_v3 (
 attempt_id TEXT NOT NULL, work_unit_id TEXT NOT NULL, role_kind TEXT NOT NULL CHECK(role_kind IN ('work_unit_handler','work_unit_implementer')), sprint_git_authority_id TEXT NOT NULL, baseline_object_id TEXT NOT NULL, authorization_fingerprint TEXT NOT NULL, recorded_at TEXT NOT NULL, FOREIGN KEY(sprint_git_authority_id) REFERENCES initiated_sprint_git_authorities(authority_id) ON DELETE RESTRICT, PRIMARY KEY(attempt_id,role_kind));
INSERT INTO execution_support_attempt_authorizations_v3 SELECT attempt_id,work_unit_id,role_kind,sprint_git_authority_id,baseline_object_id,authorization_fingerprint,recorded_at FROM execution_support_attempt_authorizations;
DROP TABLE execution_support_attempt_authorizations;
ALTER TABLE execution_support_attempt_authorizations_v3 RENAME TO execution_support_attempt_authorizations;
CREATE INDEX execution_support_attempt_authorizations_sprint_authority ON execution_support_attempt_authorizations(sprint_git_authority_id);
CREATE TABLE execution_support_grants_v3 (
 attempt_id TEXT NOT NULL, capability_ref TEXT NOT NULL UNIQUE, epic_id TEXT NOT NULL, sprint_id TEXT NOT NULL, work_unit_id TEXT NOT NULL, repository_id TEXT NOT NULL, role_id TEXT NOT NULL, workspace_id TEXT NOT NULL, workspace_fingerprint TEXT NOT NULL, correlation_fingerprint TEXT NOT NULL, recorded_at TEXT NOT NULL, PRIMARY KEY(attempt_id,role_id));
INSERT INTO execution_support_grants_v3 SELECT attempt_id,capability_ref,epic_id,sprint_id,work_unit_id,repository_id,role_id,workspace_id,workspace_fingerprint,correlation_fingerprint,recorded_at FROM execution_support_grants;
DROP TABLE execution_support_grants;
ALTER TABLE execution_support_grants_v3 RENAME TO execution_support_grants;
"#;
pub(crate) const EXECUTION_SUPPORT_BASELINE_MIGRATION: &str =
    "ALTER TABLE execution_support_attempt_authorizations ADD COLUMN baseline_object_id TEXT NOT NULL DEFAULT '';";
pub(crate) const EXECUTION_SUPPORT_ATTEMPT_AUTHORIZATION_MIGRATION: &str = r#"
CREATE TABLE execution_support_attempt_authorizations_v2 (
  attempt_id TEXT PRIMARY KEY,
  work_unit_id TEXT NOT NULL,
  role_kind TEXT NOT NULL CHECK(role_kind IN ('work_unit_handler','work_unit_implementer')),
  sprint_git_authority_id TEXT NOT NULL,
  baseline_object_id TEXT NOT NULL,
  authorization_fingerprint TEXT NOT NULL DEFAULT '',
  recorded_at TEXT NOT NULL,
  FOREIGN KEY(sprint_git_authority_id) REFERENCES initiated_sprint_git_authorities(authority_id) ON DELETE RESTRICT
);
INSERT INTO execution_support_attempt_authorizations_v2
  (attempt_id,work_unit_id,role_kind,sprint_git_authority_id,baseline_object_id,authorization_fingerprint,recorded_at)
  SELECT attempt_id,work_unit_id,role_kind,sprint_git_authority_id,baseline_object_id,'',recorded_at
  FROM execution_support_attempt_authorizations;
DROP TABLE execution_support_attempt_authorizations;
ALTER TABLE execution_support_attempt_authorizations_v2 RENAME TO execution_support_attempt_authorizations;
CREATE INDEX execution_support_attempt_authorizations_sprint_authority
  ON execution_support_attempt_authorizations(sprint_git_authority_id);
"#;

#[derive(Clone, Debug, Eq, PartialEq)]
struct AuthorizedExecutionAttempt {
    attempt_id: String,
    work_unit_id: String,
    role_kind: String,
    baseline_object_id: String,
    authority: InitiatedSprintGitAuthority,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct ExecutionWorkspaceBinding {
    workspace_id: String,
    workspace_fingerprint: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct CapturedInspection {
    manifest: Vec<ChangedFileManifestEntry>,
    comparison: Option<Vec<u8>>,
}

/// The only reference a later Harness action may retain for this capability.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExecutionSupportReference {
    pub(crate) capability_ref: String,
    pub(crate) working_directory: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChangedFileManifestEntry {
    pub(crate) evidence_ref: String,
    pub(crate) display_name: String,
    pub(crate) change_kind: String,
}

/// No path, repository, Git ref/object, workspace, or role identity is accepted here.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ExecutionSupportIntent {
    ChangedFileManifest,
    Comparison,
    EvidenceContent { evidence_ref: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ExecutionSupportResponse {
    ChangedFileManifest(Vec<ChangedFileManifestEntry>),
    Comparison(Vec<u8>),
    EvidenceContent(Vec<u8>),
}

/// The only roles the application can authorize for a future execution attempt.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum WorkUnitExecutionRole {
    Handler,
    Implementer,
}
impl WorkUnitExecutionRole {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Handler => "work_unit_handler",
            Self::Implementer => "work_unit_implementer",
        }
    }
}

/// Application-only lifecycle input. It contains no filesystem, worktree, ref, or object route.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AuthorizeExistingWorkUnitExecutionAttempt {
    pub(crate) attempt_id: String,
    pub(crate) work_unit_id: String,
    pub(crate) role: WorkUnitExecutionRole,
    pub(crate) sprint_git_authority_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum AuthorizeExistingWorkUnitExecutionAttemptResult {
    Authorized { baseline_object_id: String },
    IdempotentReplay { baseline_object_id: String },
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ExecutionSupportError {
    Invalid,
    Denied,
    Unavailable,
    CorrelationMismatch,
    Conflict,
}

impl fmt::Display for ExecutionSupportError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Invalid => "the execution-support request is invalid",
            Self::Denied => "the requested execution-support scope is not authorized",
            Self::Unavailable => {
                "execution support is unavailable because its live workspace cannot be verified"
            }
            Self::CorrelationMismatch => {
                "the durable execution-support correlation no longer matches the live workspace"
            }
            Self::Conflict => "the execution-support grant conflicts with durable state",
        })
    }
}
impl Error for ExecutionSupportError {}

/// Narrow native adapter: it uses the durable initiated-Sprint Git authority and canonical File
/// Review producer, never a human launcher, debug controller, or caller-provided filesystem/Git route.
trait ExecutionWorkspaceResolver: Send + Sync {
    fn resolve(
        &self,
        attempt: &AuthorizedExecutionAttempt,
        existing: Option<&ExecutionWorkspaceBinding>,
    ) -> Result<ExecutionWorkspaceBinding, ExecutionSupportError>;
    fn inspect(
        &self,
        attempt: &AuthorizedExecutionAttempt,
        binding: &ExecutionWorkspaceBinding,
        capability_ref: &str,
    ) -> Result<CapturedInspection, ExecutionSupportError>;
    fn working_directory(
        &self,
        attempt: &AuthorizedExecutionAttempt,
        binding: &ExecutionWorkspaceBinding,
    ) -> Result<String, ExecutionSupportError>;
}

pub(crate) struct ProductExecutionWorkspaceResolver {
    repository: Arc<SqliteOrchestrationRepository>,
    workspace_parent: PathBuf,
}

impl ProductExecutionWorkspaceResolver {
    pub(crate) fn new(
        repository: Arc<SqliteOrchestrationRepository>,
        workspace_parent: PathBuf,
    ) -> Self {
        Self {
            repository,
            workspace_parent,
        }
    }

    fn workspace_id(&self, attempt: &AuthorizedExecutionAttempt) -> String {
        stable_id(
            "execution-workspace",
            &format!("{}:{}", attempt.authority.authority_id, attempt.attempt_id),
        )
    }

    fn workspace_root(
        &self,
        attempt: &AuthorizedExecutionAttempt,
        create_parent: bool,
    ) -> Result<PathBuf, ExecutionSupportError> {
        if create_parent {
            fs::create_dir_all(&self.workspace_parent)
                .map_err(|_| ExecutionSupportError::Unavailable)?;
        }
        let metadata = fs::symlink_metadata(&self.workspace_parent)
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        let parent = self
            .workspace_parent
            .canonicalize()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        let root = parent.join(self.workspace_id(attempt));
        if root.parent() != Some(parent.as_path()) {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        Ok(root)
    }

    fn validate_attempt_workspace(
        &self,
        attempt: &AuthorizedExecutionAttempt,
        binding: &ExecutionWorkspaceBinding,
    ) -> Result<(PathBuf, String), ExecutionSupportError> {
        let expected_id = self.workspace_id(attempt);
        if binding.workspace_id != expected_id {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        let root = self.workspace_root(attempt, false)?;
        let metadata =
            fs::symlink_metadata(&root).map_err(|_| ExecutionSupportError::Unavailable)?;
        if !metadata.is_dir() || metadata.file_type().is_symlink() {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        let canonical_root = root
            .canonicalize()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        if canonical_root != root {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        let authority = &attempt.authority;
        let repository_root = canonical_authorized_root(&authority.repository_root)?;
        let authority_root = canonical_authorized_root(&authority.worktree_root)?;
        if canonical_root == repository_root || canonical_root == authority_root {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        let expected_common = canonical_authorized_root(&authority.repository_common_dir)?;
        if git_path(&repository_root, &["rev-parse", "--git-common-dir"])? != expected_common
            || git_path(&canonical_root, &["rev-parse", "--git-common-dir"])? != expected_common
            || git_path(&canonical_root, &["rev-parse", "--show-toplevel"])? != canonical_root
            || !registered_worktree(&repository_root, &canonical_root)?
            || git_text(
                &canonical_root,
                &[
                    "rev-parse",
                    "--verify",
                    &format!("{}^{{commit}}", attempt.baseline_object_id),
                ],
            )? != attempt.baseline_object_id
        {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        let fingerprint = workspace_fingerprint(attempt, &canonical_root);
        if binding.workspace_fingerprint != fingerprint {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        Ok((
            canonical_root.clone(),
            git_text(&canonical_root, &["rev-parse", "--verify", "HEAD^{commit}"])?,
        ))
    }

    fn create_attempt_workspace(
        &self,
        attempt: &AuthorizedExecutionAttempt,
    ) -> Result<ExecutionWorkspaceBinding, ExecutionSupportError> {
        let root = self.workspace_root(attempt, true)?;
        if root.exists() || fs::symlink_metadata(&root).is_ok() {
            // A stop after `git worktree add` but before the durable grant commits leaves this
            // exact deterministic side effect. Adopt only a complete, untouched first grant.
            let binding = ExecutionWorkspaceBinding {
                workspace_id: self.workspace_id(attempt),
                workspace_fingerprint: workspace_fingerprint(attempt, &root),
            };
            let (verified, head) = self.validate_attempt_workspace(attempt, &binding)?;
            if verified != root
                || head != attempt.baseline_object_id
                || !git_is_detached(&root)?
                || !git_text(&root, &["status", "--porcelain"])?.is_empty()
            {
                return Err(ExecutionSupportError::CorrelationMismatch);
            }
            return Ok(binding);
        }
        let authority = &attempt.authority;
        let repository_root = canonical_authorized_root(&authority.repository_root)?;
        let authority_root = canonical_authorized_root(&authority.worktree_root)?;
        if !git_text(&authority_root, &["status", "--porcelain"])?.is_empty()
            || git_text(&authority_root, &["rev-parse", "--verify", "HEAD^{commit}"])?
                != attempt.baseline_object_id
        {
            return Err(ExecutionSupportError::Unavailable);
        }
        git_success(
            &repository_root,
            &[
                "worktree",
                "add",
                "--detach",
                git_argument_path(&root).as_str(),
                &attempt.baseline_object_id,
            ],
        )?;
        let binding = ExecutionWorkspaceBinding {
            workspace_id: self.workspace_id(attempt),
            workspace_fingerprint: workspace_fingerprint(attempt, &root),
        };
        let (verified, head) = self.validate_attempt_workspace(attempt, &binding)?;
        if verified != root
            || head != attempt.baseline_object_id
            || !git_is_detached(&root)?
            || !git_text(&root, &["status", "--porcelain"])?.is_empty()
        {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        Ok(binding)
    }
}

impl ExecutionWorkspaceResolver for ProductExecutionWorkspaceResolver {
    fn resolve(
        &self,
        attempt: &AuthorizedExecutionAttempt,
        existing: Option<&ExecutionWorkspaceBinding>,
    ) -> Result<ExecutionWorkspaceBinding, ExecutionSupportError> {
        if let Some(existing) = existing {
            self.validate_attempt_workspace(attempt, existing)?;
            return Ok(existing.clone());
        }
        self.create_attempt_workspace(attempt)
    }

    fn inspect(
        &self,
        attempt: &AuthorizedExecutionAttempt,
        binding: &ExecutionWorkspaceBinding,
        capability_ref: &str,
    ) -> Result<CapturedInspection, ExecutionSupportError> {
        let (root, current_object_id) = self.validate_attempt_workspace(attempt, binding)?;
        if !git_text(&root, &["status", "--porcelain"])?.is_empty() {
            return Err(ExecutionSupportError::Unavailable);
        }
        if current_object_id == attempt.baseline_object_id {
            return Ok(CapturedInspection {
                manifest: vec![],
                comparison: None,
            });
        }
        let snapshot = stable_id(
            "execution-support-snapshot",
            &format!("{capability_ref}:{current_object_id}"),
        );
        self.repository
            .store_file_review_git_capture_authorization(FileReviewGitCaptureAuthorizationWrite {
                capture_authorization_id: snapshot.clone(),
                idempotency_key: stable_id("execution-support-capture", &snapshot),
                epic_id: attempt.authority.epic_id.clone(),
                sprint_id: attempt.authority.sprint_id.clone(),
                provenance_id: attempt.authority.provenance_id.clone(),
                repository_id: attempt.authority.repository_id.clone(),
                repository_root: attempt.authority.repository_root.clone(),
                worktree_id: binding.workspace_id.clone(),
                worktree_root: root.to_string_lossy().into_owned(),
                baseline_object_id: attempt.baseline_object_id.clone(),
                current_object_id,
            })
            .map_err(|_| ExecutionSupportError::CorrelationMismatch)?;
        let produced = produce_file_review_from_git(
            &self.repository,
            ProduceFileReviewFromGit {
                capture_authorization_id: snapshot,
            },
        )
        .map_err(|_| ExecutionSupportError::Unavailable)?;
        let document = match self
            .repository
            .load_scoped_file_review(&produced.opaque_reference)
        {
            Ok(ScopedFileReviewLoad::Available { document }) => document,
            _ => return Err(ExecutionSupportError::Unavailable),
        };
        let manifest = document
            .changed_files
            .into_iter()
            .map(|file| {
                if !safe_display_path(&file.display_name) {
                    return Err(ExecutionSupportError::CorrelationMismatch);
                }
                Ok(ChangedFileManifestEntry {
                    evidence_ref: file.changed_file_reference_id,
                    display_name: file.display_name,
                    change_kind: file.change_kind,
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        Ok(CapturedInspection {
            manifest,
            comparison: Some(document.payload),
        })
    }

    fn working_directory(
        &self,
        attempt: &AuthorizedExecutionAttempt,
        binding: &ExecutionWorkspaceBinding,
    ) -> Result<String, ExecutionSupportError> {
        let (root, _) = self.validate_attempt_workspace(attempt, binding)?;
        root.to_str()
            .map(str::to_owned)
            .ok_or(ExecutionSupportError::Unavailable)
    }
}

pub(crate) struct SqliteExecutionSupportRepository {
    connection: Mutex<Connection>,
    orchestration: Arc<SqliteOrchestrationRepository>,
}

impl SqliteExecutionSupportRepository {
    fn open(
        path: &Path,
        orchestration: Arc<SqliteOrchestrationRepository>,
    ) -> Result<Self, ExecutionSupportError> {
        let connection = Connection::open(path).map_err(|_| ExecutionSupportError::Unavailable)?;
        crate::storage::configure_sqlite_connection(&connection)
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        connection
            .execute_batch(EXECUTION_SUPPORT_SCHEMA)
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        let role_keyed = connection
            .query_row("SELECT sql FROM sqlite_master WHERE type='table' AND name='execution_support_attempt_authorizations'", [], |row| row.get::<_, String>(0))
            .map(|sql| sql.contains("PRIMARY KEY(attempt_id,role_kind)"))
            .unwrap_or(false);
        if !role_keyed { connection.execute_batch(EXECUTION_SUPPORT_ROLE_KEY_MIGRATION).map_err(|_| ExecutionSupportError::Unavailable)?; }
        Ok(Self {
            connection: Mutex::new(connection),
            orchestration,
        })
    }

    fn authorize_existing_attempt(
        &self,
        request: &AuthorizeExistingWorkUnitExecutionAttempt,
    ) -> Result<AuthorizeExistingWorkUnitExecutionAttemptResult, ExecutionSupportError> {
        if !bounded_id(&request.attempt_id)
            || !bounded_id(&request.work_unit_id)
            || !bounded_id(&request.sprint_git_authority_id)
        {
            return Err(ExecutionSupportError::Denied);
        }
        let authority = self
            .orchestration
            .load_initiated_sprint_git_authority(&request.sprint_git_authority_id)
            .map_err(|_| ExecutionSupportError::Unavailable)?
            .ok_or(ExecutionSupportError::Denied)?;
        if !git_object_id(&authority.current_object_id) {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        let baseline = authority.current_object_id;
        let role_kind = request.role.as_str();
        let fingerprint = authorization_fingerprint(
            &request.attempt_id,
            &request.work_unit_id,
            role_kind,
            &request.sprint_git_authority_id,
            &baseline,
        );
        let mut connection = self
            .connection
            .lock()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        let existing: Option<(String, String)> = transaction
            .query_row(
                "SELECT baseline_object_id,authorization_fingerprint FROM execution_support_attempt_authorizations WHERE attempt_id=?1 AND role_kind=?2",
                params![&request.attempt_id, role_kind],
                |row| Ok((row.get(0)?, row.get(1)?)),
            )
            .optional()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        if let Some((stored_baseline, stored_fingerprint)) = existing {
            if stored_baseline != baseline || stored_fingerprint != fingerprint {
                return Err(ExecutionSupportError::Conflict);
            }
            transaction
                .commit()
                .map_err(|_| ExecutionSupportError::Unavailable)?;
            return Ok(
                AuthorizeExistingWorkUnitExecutionAttemptResult::IdempotentReplay {
                    baseline_object_id: baseline,
                },
            );
        }
        transaction
            .execute(
                "INSERT INTO execution_support_attempt_authorizations (attempt_id,work_unit_id,role_kind,sprint_git_authority_id,baseline_object_id,authorization_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,datetime('now'))",
                params![request.attempt_id, request.work_unit_id, role_kind, request.sprint_git_authority_id, baseline, fingerprint],
            )
            .map_err(|_| ExecutionSupportError::Conflict)?;
        transaction
            .commit()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        Ok(
            AuthorizeExistingWorkUnitExecutionAttemptResult::Authorized {
                baseline_object_id: baseline,
            },
        )
    }

    fn load_authorized_attempt_for_role(&self, attempt_id: &str, role: WorkUnitExecutionRole,
    ) -> Result<AuthorizedExecutionAttempt, ExecutionSupportError> {
        let (work_unit_id, role_kind, authority_id, baseline_object_id, stored_fingerprint):
            (String, String, String, String, String) = self
            .connection
            .lock()
            .map_err(|_| ExecutionSupportError::Unavailable)?
            .query_row(
                "SELECT work_unit_id,role_kind,sprint_git_authority_id,baseline_object_id,authorization_fingerprint FROM execution_support_attempt_authorizations WHERE attempt_id=?1 AND role_kind=?2",
                params![attempt_id,role.as_str()],
                |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?, row.get(4)?)),
            )
            .optional()
            .map_err(|_| ExecutionSupportError::Unavailable)?
            .ok_or(ExecutionSupportError::Denied)?;
        if !bounded_id(&work_unit_id)
            || !matches!(
                role_kind.as_str(),
                "work_unit_handler" | "work_unit_implementer"
            )
            || !git_object_id(&baseline_object_id)
            || stored_fingerprint
                != authorization_fingerprint(
                    attempt_id,
                    &work_unit_id,
                    &role_kind,
                    &authority_id,
                    &baseline_object_id,
                )
        {
            return Err(ExecutionSupportError::Denied);
        }
        let authority = self
            .orchestration
            .load_initiated_sprint_git_authority(&authority_id)
            .map_err(|_| ExecutionSupportError::Unavailable)?
            .ok_or(ExecutionSupportError::Denied)?;
        if authority.current_object_id != baseline_object_id {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        Ok(AuthorizedExecutionAttempt {
            attempt_id: attempt_id.into(),
            work_unit_id,
            role_kind,
            baseline_object_id,
            authority,
        })
    }
    #[cfg(test)]
    fn load_authorized_attempt(&self, attempt_id: &str) -> Result<AuthorizedExecutionAttempt, ExecutionSupportError> { self.load_authorized_attempt_for_role(attempt_id, WorkUnitExecutionRole::Implementer) }
}

pub(crate) struct ExecutionSupportService {
    repository: Arc<SqliteExecutionSupportRepository>,
    resolver: Arc<dyn ExecutionWorkspaceResolver>,
}

impl ExecutionSupportService {
    fn new(
        repository: Arc<SqliteExecutionSupportRepository>,
        resolver: Arc<dyn ExecutionWorkspaceResolver>,
    ) -> Self {
        Self {
            repository,
            resolver,
        }
    }

    /// Control-side lifecycle seam. It records authority for an already-existing attempt only.
    /// There is intentionally no Tauri command or Harness input for this operation.
    pub(crate) fn authorize_existing_attempt(
        &self,
        request: AuthorizeExistingWorkUnitExecutionAttempt,
    ) -> Result<AuthorizeExistingWorkUnitExecutionAttemptResult, ExecutionSupportError> {
        self.repository.authorize_existing_attempt(&request)
    }

    /// This application-only operation accepts an opaque existing attempt reference. Its context
    /// is re-derived from durable authority; it does not create a Work Unit or execution attempt.
    pub(crate) fn grant_role(&self, attempt_id: &str, role: WorkUnitExecutionRole,
    ) -> Result<ExecutionSupportReference, ExecutionSupportError> {
        if !bounded_id(attempt_id) {
            return Err(ExecutionSupportError::Denied);
        }
        let attempt = self.repository.load_authorized_attempt_for_role(attempt_id, role)?;
        let mut connection = self
            .repository
            .connection
            .lock()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        // One ProductExecutionSupportState owns this mutex; the immediate writer transaction
        // serializes its native first-grant side effect. SQLite extends that serialization to a
        // second product process before either can create a duplicate deterministic worktree.
        let existing = load_grant(&transaction, attempt_id, role.as_str())?;
        let binding = self
            .resolver
            .resolve(&attempt, existing.as_ref().map(|grant| &grant.binding))?;
        if let Some(existing) = existing {
            if existing.correlation != correlation_fingerprint(&attempt, &binding) {
                return Err(ExecutionSupportError::CorrelationMismatch);
            }
            transaction
                .commit()
                .map_err(|_| ExecutionSupportError::Unavailable)?;
            return Ok(ExecutionSupportReference {
                capability_ref: existing.capability_ref,
                working_directory: self.resolver.working_directory(&attempt, &binding)?,
            });
        }
        let capability_ref = stable_id("execution-support", &format!("{attempt_id}:{}", role.as_str()));
        let correlation = correlation_fingerprint(&attempt, &binding);
        transaction.execute(
            "INSERT INTO execution_support_grants (attempt_id,capability_ref,epic_id,sprint_id,work_unit_id,repository_id,role_id,workspace_id,workspace_fingerprint,correlation_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,datetime('now'))",
            params![attempt.attempt_id, capability_ref, attempt.authority.epic_id, attempt.authority.sprint_id, attempt.work_unit_id, attempt.authority.repository_id, attempt.role_kind, binding.workspace_id, binding.workspace_fingerprint, correlation],
        ).map_err(|_| ExecutionSupportError::Conflict)?;
        transaction
            .commit()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        Ok(ExecutionSupportReference {
            capability_ref,
            working_directory: self.resolver.working_directory(&attempt, &binding)?,
        })
    }

    /// A role-specific package must match the durable authorization before it receives the
    /// opaque capability. Neither caller-provided routes nor an existing grant can change role.
    pub(crate) fn grant_for_role(
        &self,
        attempt_id: &str,
        role: WorkUnitExecutionRole,
    ) -> Result<ExecutionSupportReference, ExecutionSupportError> {
        if !bounded_id(attempt_id) {
            return Err(ExecutionSupportError::Denied);
        }
        self.grant_role(attempt_id, role)
    }

    #[cfg(test)]
    fn grant(&self, attempt_id: &str) -> Result<ExecutionSupportReference, ExecutionSupportError> {
        self.grant_role(attempt_id, WorkUnitExecutionRole::Implementer)
    }

    pub(crate) fn consume(
        &self,
        capability_ref: &str,
        intent: ExecutionSupportIntent,
    ) -> Result<ExecutionSupportResponse, ExecutionSupportError> {
        if !bounded_id(capability_ref) {
            return Err(ExecutionSupportError::Denied);
        }
        let grant = {
            let connection = self
                .repository
                .connection
                .lock()
                .map_err(|_| ExecutionSupportError::Unavailable)?;
            load_grant_for_capability(&connection, capability_ref)?
        }
        .ok_or(ExecutionSupportError::Denied)?;
        let role = match grant.role_id.as_str() { "work_unit_handler" => WorkUnitExecutionRole::Handler, "work_unit_implementer" => WorkUnitExecutionRole::Implementer, _ => return Err(ExecutionSupportError::Denied) };
        let attempt = self.repository.load_authorized_attempt_for_role(&grant.attempt_id, role)?;
        let binding = self.resolver.resolve(&attempt, Some(&grant.binding))?;
        if grant.correlation != correlation_fingerprint(&attempt, &binding) {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        let inspection = self.resolver.inspect(&attempt, &binding, capability_ref)?;
        match intent {
            ExecutionSupportIntent::ChangedFileManifest => Ok(
                ExecutionSupportResponse::ChangedFileManifest(inspection.manifest),
            ),
            ExecutionSupportIntent::Comparison => inspection
                .comparison
                .map(ExecutionSupportResponse::Comparison)
                .ok_or(ExecutionSupportError::Denied),
            ExecutionSupportIntent::EvidenceContent { evidence_ref } => {
                evidence_from_canonical_comparison(
                    inspection.comparison.as_deref(),
                    &inspection.manifest,
                    &evidence_ref,
                )
                .map(ExecutionSupportResponse::EvidenceContent)
            }
        }
    }
}

/// Product boot composes the real narrow adapter. It can have no current attempt while still being
/// implemented; startup itself only retains a private parent path and never creates a worktree.
pub(crate) struct ProductExecutionSupportState {
    service: Arc<ExecutionSupportService>,
}
impl ProductExecutionSupportState {
    pub(crate) fn new(
        database_path: &Path,
        workspace_parent: PathBuf,
        orchestration: Arc<SqliteOrchestrationRepository>,
    ) -> Result<Self, ExecutionSupportError> {
        let repository = Arc::new(SqliteExecutionSupportRepository::open(
            database_path,
            orchestration.clone(),
        )?);
        Ok(Self {
            service: Arc::new(ExecutionSupportService::new(
                repository,
                Arc::new(ProductExecutionWorkspaceResolver::new(
                    orchestration,
                    workspace_parent,
                )),
            )),
        })
    }
    pub(crate) fn service(&self) -> Arc<ExecutionSupportService> {
        self.service.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredGrant {
    attempt_id: String,
    role_id: String,
    capability_ref: String,
    binding: ExecutionWorkspaceBinding,
    correlation: String,
}

fn load_grant(connection: &Connection, attempt_id: &str, role_id: &str,
) -> Result<Option<StoredGrant>, ExecutionSupportError> {
    let query = "SELECT attempt_id,role_id,capability_ref,workspace_id,workspace_fingerprint,correlation_fingerprint FROM execution_support_grants WHERE attempt_id=?1 AND role_id=?2";
    connection.query_row(query, params![attempt_id,role_id], |row| Ok(StoredGrant { attempt_id:row.get(0)?, role_id:row.get(1)?, capability_ref:row.get(2)?, binding:ExecutionWorkspaceBinding{workspace_id:row.get(3)?,workspace_fingerprint:row.get(4)?}, correlation:row.get(5)? })).optional().map_err(|_|ExecutionSupportError::Unavailable)
}
fn load_grant_for_capability(
    connection: &Connection,
    capability_ref: &str,
) -> Result<Option<StoredGrant>, ExecutionSupportError> {
    load_grant_where(connection, "capability_ref", capability_ref)
}
fn load_grant_where(
    connection: &Connection,
    field: &str,
    value: &str,
) -> Result<Option<StoredGrant>, ExecutionSupportError> {
    let query = format!("SELECT attempt_id,role_id,capability_ref,workspace_id,workspace_fingerprint,correlation_fingerprint FROM execution_support_grants WHERE {field}=?1");
    connection
        .query_row(&query, [value], |row| {
            Ok(StoredGrant {
                attempt_id: row.get(0)?,
                role_id: row.get(1)?,
                capability_ref: row.get(2)?,
                binding: ExecutionWorkspaceBinding {
                    workspace_id: row.get(3)?,
                    workspace_fingerprint: row.get(4)?,
                },
                correlation: row.get(5)?,
            })
        })
        .optional()
        .map_err(|_| ExecutionSupportError::Unavailable)
}

fn evidence_from_canonical_comparison(
    comparison: Option<&[u8]>,
    manifest: &[ChangedFileManifestEntry],
    evidence_ref: &str,
) -> Result<Vec<u8>, ExecutionSupportError> {
    if !manifest
        .iter()
        .any(|entry| entry.evidence_ref == evidence_ref)
    {
        return Err(ExecutionSupportError::Denied);
    }
    let value: serde_json::Value =
        serde_json::from_slice(comparison.ok_or(ExecutionSupportError::Denied)?)
            .map_err(|_| ExecutionSupportError::Unavailable)?;
    let file = value
        .get("files")
        .and_then(serde_json::Value::as_array)
        .and_then(|files| {
            files.iter().find(|file| {
                file.get("changedFileReferenceId")
                    .and_then(serde_json::Value::as_str)
                    == Some(evidence_ref)
            })
        })
        .ok_or(ExecutionSupportError::Denied)?;
    serde_json::to_vec(file).map_err(|_| ExecutionSupportError::Unavailable)
}

fn canonical_authorized_root(value: &str) -> Result<PathBuf, ExecutionSupportError> {
    let root = PathBuf::from(value)
        .canonicalize()
        .map_err(|_| ExecutionSupportError::Unavailable)?;
    if root.to_string_lossy() != value {
        return Err(ExecutionSupportError::CorrelationMismatch);
    }
    Ok(root)
}
fn git_success(root: &Path, args: &[&str]) -> Result<(), ExecutionSupportError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|_| ExecutionSupportError::Unavailable)?;
    if output.status.success() {
        Ok(())
    } else {
        Err(ExecutionSupportError::Unavailable)
    }
}
fn git_text(root: &Path, args: &[&str]) -> Result<String, ExecutionSupportError> {
    let output = Command::new("git")
        .args(args)
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|_| ExecutionSupportError::Unavailable)?;
    if !output.status.success() || output.stdout.len() > 256_000 {
        return Err(ExecutionSupportError::Unavailable);
    }
    String::from_utf8(output.stdout)
        .map_err(|_| ExecutionSupportError::Unavailable)
        .map(|value| value.trim().to_owned())
}
fn git_path(root: &Path, args: &[&str]) -> Result<PathBuf, ExecutionSupportError> {
    let path = PathBuf::from(git_text(root, args)?);
    (if path.is_absolute() {
        path
    } else {
        root.join(path)
    })
    .canonicalize()
    .map_err(|_| ExecutionSupportError::Unavailable)
}
fn git_is_detached(root: &Path) -> Result<bool, ExecutionSupportError> {
    let output = Command::new("git")
        .args(["symbolic-ref", "-q", "HEAD"])
        .current_dir(root)
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|_| ExecutionSupportError::Unavailable)?;
    if output.stdout.len() > 256_000 {
        return Err(ExecutionSupportError::Unavailable);
    }
    match output.status.code() {
        Some(0) => Ok(false),
        Some(1) => Ok(true),
        _ => Err(ExecutionSupportError::Unavailable),
    }
}
fn git_argument_path(path: &Path) -> String {
    let value = path.to_string_lossy();
    value
        .strip_prefix(r"\\?\")
        .unwrap_or(value.as_ref())
        .to_owned()
}
fn registered_worktree(repository_root: &Path, root: &Path) -> Result<bool, ExecutionSupportError> {
    let listing = git_text(repository_root, &["worktree", "list", "--porcelain"])?;
    Ok(listing
        .lines()
        .filter_map(|line| line.strip_prefix("worktree "))
        .any(|value| {
            PathBuf::from(value)
                .canonicalize()
                .map(|candidate| candidate == root)
                .unwrap_or(false)
        }))
}
fn bounded_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
fn safe_display_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4_096
        && !value.chars().any(|character| character.is_control())
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.starts_with("\\\\")
        && !value.contains('\\')
        && !value.as_bytes().get(1).is_some_and(|byte| *byte == b':')
        && !value
            .split('/')
            .any(|part| part.is_empty() || matches!(part, "." | ".."))
}
fn stable_id(prefix: &str, value: &str) -> String {
    format!(
        "{prefix}-{}",
        &format!("{:x}", Sha256::digest(value.as_bytes()))[..24]
    )
}
fn workspace_fingerprint(attempt: &AuthorizedExecutionAttempt, root: &Path) -> String {
    fingerprint(&[
        &attempt.authority.authority_id,
        &attempt.attempt_id,
        &attempt.baseline_object_id,
        root.to_string_lossy().as_ref(),
    ])
}
fn authorization_fingerprint(
    attempt_id: &str,
    work_unit_id: &str,
    role_kind: &str,
    authority_id: &str,
    baseline_object_id: &str,
) -> String {
    fingerprint(&[
        attempt_id,
        work_unit_id,
        role_kind,
        authority_id,
        baseline_object_id,
    ])
}
fn correlation_fingerprint(
    attempt: &AuthorizedExecutionAttempt,
    binding: &ExecutionWorkspaceBinding,
) -> String {
    fingerprint(&[
        &attempt.attempt_id,
        &attempt.work_unit_id,
        &attempt.role_kind,
        &attempt.baseline_object_id,
        &attempt.authority.authority_id,
        &attempt.authority.epic_id,
        &attempt.authority.sprint_id,
        &attempt.authority.repository_id,
        &binding.workspace_id,
        &binding.workspace_fingerprint,
    ])
}
fn git_object_id(value: &str) -> bool {
    (value.len() == 40 || value.len() == 64) && value.bytes().all(|byte| byte.is_ascii_hexdigit())
}
fn fingerprint(values: &[&str]) -> String {
    let mut hash = Sha256::new();
    for value in values {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

#[cfg(test)]
mod tests {
    use super::super::repository::{
        InitiatedSprintGitAuthorityWrite, StoreInitiatedSprintGitAuthorityResult,
    };
    use super::*;

    struct Fixture {
        _temp: tempfile::TempDir,
        database: PathBuf,
        repository_root: PathBuf,
        sprint_root: PathBuf,
        workspace_parent: PathBuf,
        authority_id: String,
        baseline: String,
    }

    impl Fixture {
        fn git(&self, root: &Path, args: &[&str]) -> String {
            let output = Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .unwrap();
            assert!(output.status.success(), "{:?}", output);
            String::from_utf8(output.stdout).unwrap().trim().into()
        }

        fn service(&self) -> ExecutionSupportService {
            let orchestration =
                Arc::new(SqliteOrchestrationRepository::open(&self.database).unwrap());
            ExecutionSupportService::new(
                Arc::new(
                    SqliteExecutionSupportRepository::open(&self.database, orchestration.clone())
                        .unwrap(),
                ),
                Arc::new(ProductExecutionWorkspaceResolver::new(
                    orchestration,
                    self.workspace_parent.clone(),
                )),
            )
        }

        fn authorize(
            &self,
            service: &ExecutionSupportService,
            attempt_id: &str,
            work_unit_id: &str,
        ) {
            assert!(matches!(
                service.authorize_existing_attempt(AuthorizeExistingWorkUnitExecutionAttempt {
                    attempt_id: attempt_id.into(),
                    work_unit_id: work_unit_id.into(),
                    role: WorkUnitExecutionRole::Implementer,
                    sprint_git_authority_id: self.authority_id.clone(),
                }).unwrap(),
                AuthorizeExistingWorkUnitExecutionAttemptResult::Authorized { ref baseline_object_id } if baseline_object_id == &self.baseline
            ));
        }

        fn attempt_root(&self, attempt_id: &str) -> PathBuf {
            self.workspace_parent.join(stable_id(
                "execution-workspace",
                &format!("{}:{attempt_id}", self.authority_id),
            ))
        }
    }

    fn fixture() -> Fixture {
        let temp = tempfile::tempdir().unwrap();
        let repository_root = temp.path().join("repository");
        let sprint_root = temp.path().join("sprint-worktree");
        fs::create_dir(&repository_root).unwrap();
        let git = |root: &Path, args: &[&str]| {
            let output = Command::new("git")
                .args(args)
                .current_dir(root)
                .output()
                .unwrap();
            assert!(output.status.success(), "{:?}", output);
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        };
        git(&repository_root, &["init"]);
        git(
            &repository_root,
            &["config", "user.email", "test@example.invalid"],
        );
        git(&repository_root, &["config", "user.name", "Test"]);
        fs::write(repository_root.join("file.txt"), "base\n").unwrap();
        git(&repository_root, &["add", "."]);
        git(&repository_root, &["commit", "-m", "base"]);
        let initial = git(&repository_root, &["rev-parse", "HEAD"]);
        git(
            &repository_root,
            &[
                "worktree",
                "add",
                "-b",
                "sprint-source",
                sprint_root.to_string_lossy().as_ref(),
                &initial,
            ],
        );
        fs::write(sprint_root.join("file.txt"), "sprint\n").unwrap();
        git(&sprint_root, &["add", "."]);
        git(&sprint_root, &["commit", "-m", "sprint"]);
        let baseline = git(&sprint_root, &["rev-parse", "HEAD"]);
        let database = temp.path().join("db.sqlite");
        let connection = Connection::open(&database).unwrap();
        crate::storage::configure_sqlite_connection(&connection).unwrap();
        crate::storage::initialize_active_database(&connection).unwrap();
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection.execute_batch("INSERT INTO epic_initiation_provenance (id,command_id,result_id,event_id,recorded_at) VALUES ('provenance-1','command-1','result-1','event-1','t'); INSERT INTO epic_initiations (id,command_id,result_id,event_id,provenance_id,draft_id,proposal_revision_id,material_snapshot_id,epic_id,recorded_at) VALUES ('initiation-1','command-1','result-1','event-1','provenance-1','draft-1','revision-1','snapshot-1','epic-1','t'); INSERT INTO initiated_sprints (id,epic_id,ordinal,title,intended_movement,concern_summaries_json,sprint_plan_id,sprint_plan_revision_id) VALUES ('sprint-1','epic-1',0,'Sprint','Move','[]','plan-1','plan-revision-1');").unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        drop(connection);
        let orchestration = Arc::new(SqliteOrchestrationRepository::open(&database).unwrap());
        let repository_root = repository_root.canonicalize().unwrap();
        let sprint_root = sprint_root.canonicalize().unwrap();
        let authority_id = match orchestration
            .store_initiated_sprint_git_authority(InitiatedSprintGitAuthorityWrite {
                sprint_id: "sprint-1".into(),
                idempotency_key: "execution-attempt-authority".into(),
                repository_id: "repository-1".into(),
                repository_root: repository_root.to_string_lossy().into_owned(),
                repository_common_dir: git_path(
                    &repository_root,
                    &["rev-parse", "--git-common-dir"],
                )
                .unwrap()
                .to_string_lossy()
                .into_owned(),
                worktree_id: "sprint-worktree-1".into(),
                worktree_root: sprint_root.to_string_lossy().into_owned(),
                baseline_object_id: initial,
                current_object_id: baseline.clone(),
                runtime_instance_ref: "runtime-1".into(),
                runtime_source_ref: "source-1".into(),
                source_fingerprint: "a".repeat(64),
            })
            .unwrap()
        {
            StoreInitiatedSprintGitAuthorityResult::Stored { authority_id }
            | StoreInitiatedSprintGitAuthorityResult::IdempotentReplay { authority_id } => {
                authority_id
            }
        };
        let workspace_parent = temp
            .path()
            .join("product-data")
            .join("execution-workspaces");
        Fixture {
            _temp: temp,
            database,
            repository_root,
            sprint_root,
            workspace_parent,
            authority_id,
            baseline,
        }
    }

    #[test]
    fn canonical_evidence_requires_manifest_membership_and_path_denial() {
        let manifest = vec![ChangedFileManifestEntry {
            evidence_ref: "file-1".into(),
            display_name: "src/lib.rs".into(),
            change_kind: "modified".into(),
        }];
        let comparison =
            br#"{"files":[{"changedFileReferenceId":"file-1","content":{"encoding":"utf-8"}}]}"#;
        assert!(evidence_from_canonical_comparison(Some(comparison), &manifest, "file-1").is_ok());
        assert_eq!(
            evidence_from_canonical_comparison(Some(comparison), &manifest, "C:/secret"),
            Err(ExecutionSupportError::Denied)
        );
        assert!(!safe_display_path("C:/secret"));
        assert!(!safe_display_path("//server/share"));
        assert!(!safe_display_path("../escape"));
    }

    #[test]
    fn application_authorization_creates_one_distinct_attempt_workspace_and_reopens_it() {
        let fixture = fixture();
        let service = fixture.service();
        assert!(!fixture.workspace_parent.exists());
        fixture.authorize(&service, "attempt-1", "work-unit-1");
        let reference = service.grant("attempt-1").unwrap();
        let root = fixture.attempt_root("attempt-1");
        assert!(root.is_dir());
        assert_ne!(root, fixture.repository_root);
        assert_ne!(root, fixture.sprint_root);
        assert_eq!(fixture.git(&root, &["rev-parse", "HEAD"]), fixture.baseline);
        assert!(fixture
            .git(&fixture.repository_root, &["status", "--porcelain"])
            .is_empty());
        assert!(fixture
            .git(&fixture.sprint_root, &["status", "--porcelain"])
            .is_empty());
        assert_eq!(service.grant("attempt-1").unwrap(), reference);
        drop(service);
        let reopened = fixture.service();
        assert_eq!(reopened.grant("attempt-1").unwrap(), reference);
        assert!(
            matches!(reopened.consume(&reference.capability_ref, ExecutionSupportIntent::ChangedFileManifest), Ok(ExecutionSupportResponse::ChangedFileManifest(files)) if files.is_empty())
        );
    }

    #[test]
    fn role_bound_grant_denies_cross_role_and_preserves_existing_attempt_boundary() {
        let fixture = fixture();
        let service = fixture.service();
        fixture.authorize(&service, "attempt-1", "work-unit-1");
        assert_eq!(
            service.grant_for_role("attempt-1", WorkUnitExecutionRole::Handler),
            Err(ExecutionSupportError::Denied)
        );
        assert!(!fixture.workspace_parent.exists());
        let reference = service
            .grant_for_role("attempt-1", WorkUnitExecutionRole::Implementer)
            .unwrap();
        assert!(Path::new(&reference.working_directory).is_dir());
        assert!(reference.working_directory.ends_with(&stable_id(
            "execution-workspace",
            &format!("{}:attempt-1", fixture.authority_id),
        )));
        assert!(service
            .grant_for_role("C:/attempt", WorkUnitExecutionRole::Implementer)
            .is_err());
    }

    #[test]
    fn interrupted_first_grant_reopens_and_adopts_the_exact_detached_workspace() {
        let fixture = fixture();
        let service = fixture.service();
        fixture.authorize(&service, "attempt-1", "work-unit-1");
        let attempt = service
            .repository
            .load_authorized_attempt("attempt-1")
            .unwrap();
        let binding = service.resolver.resolve(&attempt, None).unwrap();
        let root = fixture.attempt_root("attempt-1");
        assert!(root.is_dir());
        assert_eq!(fixture.git(&root, &["rev-parse", "HEAD"]), fixture.baseline);
        assert!(git_is_detached(&root).unwrap());
        assert!(
            registered_worktree(&fixture.repository_root, &root.canonicalize().unwrap()).unwrap()
        );
        let grants: i64 = Connection::open(&fixture.database)
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM execution_support_grants WHERE attempt_id='attempt-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(grants, 0);
        drop(service);

        let reopened = fixture.service();
        let reference = reopened.grant("attempt-1").unwrap();
        let stored = load_grant_for_capability(
            &reopened.repository.connection.lock().unwrap(),
            &reference.capability_ref,
        )
        .unwrap()
        .unwrap();
        assert_eq!(stored.binding, binding);
        assert!(matches!(
            reopened.consume(
                &reference.capability_ref,
                ExecutionSupportIntent::ChangedFileManifest
            ),
            Ok(ExecutionSupportResponse::ChangedFileManifest(files)) if files.is_empty()
        ));
    }

    #[test]
    fn concurrent_first_grants_share_one_product_service_workspace_creation() {
        let fixture = fixture();
        let service = Arc::new(fixture.service());
        fixture.authorize(&service, "attempt-1", "work-unit-1");
        let barrier = Arc::new(std::sync::Barrier::new(2));
        let calls = (0..2)
            .map(|_| {
                let service = service.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    service.grant("attempt-1")
                })
            })
            .collect::<Vec<_>>();
        let results = calls
            .into_iter()
            .map(|call| call.join().unwrap().unwrap())
            .collect::<Vec<_>>();
        assert_eq!(results[0], results[1]);
        let root = fixture.attempt_root("attempt-1");
        let canonical_root = root.canonicalize().unwrap();
        let registrations = fixture
            .git(
                &fixture.repository_root,
                &["worktree", "list", "--porcelain"],
            )
            .lines()
            .filter(|line| {
                line.strip_prefix("worktree ")
                    .and_then(|value| PathBuf::from(value).canonicalize().ok())
                    .is_some_and(|candidate| candidate == canonical_root)
            })
            .count();
        assert_eq!(registrations, 1);
        let grants: i64 = Connection::open(&fixture.database)
            .unwrap()
            .query_row(
                "SELECT COUNT(*) FROM execution_support_grants WHERE attempt_id='attempt-1'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(grants, 1);
    }

    #[test]
    fn two_attempts_share_sprint_authority_but_not_workspaces_or_evidence() {
        let fixture = fixture();
        let service = fixture.service();
        fixture.authorize(&service, "attempt-1", "work-unit-1");
        fixture.authorize(&service, "attempt-2", "work-unit-2");
        let first = service.grant("attempt-1").unwrap();
        let second = service.grant("attempt-2").unwrap();
        let first_root = fixture.attempt_root("attempt-1");
        let second_root = fixture.attempt_root("attempt-2");
        assert_ne!(first, second);
        assert_ne!(first_root, second_root);
        fs::write(first_root.join("only-first.txt"), "first\n").unwrap();
        fixture.git(&first_root, &["add", "."]);
        fixture.git(&first_root, &["commit", "-m", "first attempt"]);
        assert!(
            matches!(service.consume(&first.capability_ref, ExecutionSupportIntent::ChangedFileManifest), Ok(ExecutionSupportResponse::ChangedFileManifest(files)) if files.len() == 1 && files[0].display_name == "only-first.txt")
        );
        assert!(
            matches!(service.consume(&second.capability_ref, ExecutionSupportIntent::ChangedFileManifest), Ok(ExecutionSupportResponse::ChangedFileManifest(files)) if files.is_empty())
        );
        assert_eq!(
            service.consume(
                &second.capability_ref,
                ExecutionSupportIntent::EvidenceContent {
                    evidence_ref: "unknown".into()
                }
            ),
            Err(ExecutionSupportError::Denied)
        );
    }

    #[test]
    fn occupied_and_drifted_attempt_routes_fail_closed() {
        let fixture = fixture();
        let service = fixture.service();
        fixture.authorize(&service, "attempt-occupied", "work-unit-1");
        fs::create_dir_all(fixture.attempt_root("attempt-occupied")).unwrap();
        fs::write(
            fixture
                .attempt_root("attempt-occupied")
                .join("not-a-worktree"),
            "x",
        )
        .unwrap();
        assert!(matches!(
            service.grant("attempt-occupied"),
            Err(ExecutionSupportError::Unavailable)
        ));
        fixture.authorize(&service, "attempt-drift", "work-unit-2");
        let reference = service.grant("attempt-drift").unwrap();
        let root = fixture.attempt_root("attempt-drift");
        fs::write(root.join("dirty.txt"), "dirty\n").unwrap();
        assert!(matches!(
            service.consume(
                &reference.capability_ref,
                ExecutionSupportIntent::ChangedFileManifest
            ),
            Err(ExecutionSupportError::Unavailable)
        ));
        fixture.authorize(&service, "attempt-replaced", "work-unit-3");
        let replaced = service.grant("attempt-replaced").unwrap();
        let root = fixture.attempt_root("attempt-replaced");
        fs::rename(&root, fixture.workspace_parent.join("replaced-worktree")).unwrap();
        fs::create_dir(&root).unwrap();
        assert!(matches!(
            service.consume(
                &replaced.capability_ref,
                ExecutionSupportIntent::ChangedFileManifest
            ),
            Err(ExecutionSupportError::Unavailable | ExecutionSupportError::CorrelationMismatch)
        ));
    }

    #[test]
    fn legacy_role_migration_retains_authority_foreign_key_and_reopens_idempotently() {
        let fixture = fixture();
        let connection = Connection::open(&fixture.database).unwrap();
        connection.execute_batch("DROP TABLE execution_support_attempt_authorizations; DROP TABLE execution_support_grants; CREATE TABLE execution_support_attempt_authorizations (attempt_id TEXT PRIMARY KEY,work_unit_id TEXT NOT NULL,role_kind TEXT NOT NULL CHECK(role_kind IN ('work_unit_handler','work_unit_implementer')),sprint_git_authority_id TEXT NOT NULL,baseline_object_id TEXT NOT NULL,authorization_fingerprint TEXT NOT NULL,recorded_at TEXT NOT NULL); CREATE TABLE execution_support_grants (attempt_id TEXT PRIMARY KEY,capability_ref TEXT NOT NULL UNIQUE,epic_id TEXT NOT NULL,sprint_id TEXT NOT NULL,work_unit_id TEXT NOT NULL,repository_id TEXT NOT NULL,role_id TEXT NOT NULL,workspace_id TEXT NOT NULL,workspace_fingerprint TEXT NOT NULL,correlation_fingerprint TEXT NOT NULL,recorded_at TEXT NOT NULL);").unwrap();
        let fingerprint = authorization_fingerprint("legacy-attempt", "legacy-unit", "work_unit_handler", &fixture.authority_id, &fixture.baseline);
        connection.execute("INSERT INTO execution_support_attempt_authorizations VALUES (?1,?2,?3,?4,?5,?6,datetime('now'))", params!["legacy-attempt","legacy-unit","work_unit_handler",fixture.authority_id,fixture.baseline,fingerprint]).unwrap();
        drop(connection);
        let service = fixture.service();
        let connection = Connection::open(&fixture.database).unwrap();
        assert_eq!(connection.query_row("SELECT COUNT(*) FROM pragma_foreign_key_list('execution_support_attempt_authorizations') WHERE \"table\"='initiated_sprint_git_authorities'", [], |row| row.get::<_,i64>(0)).unwrap(), 1);
        assert_eq!(connection.query_row("SELECT COUNT(*) FROM execution_support_attempt_authorizations WHERE attempt_id='legacy-attempt' AND role_kind='work_unit_handler'", [], |row| row.get::<_,i64>(0)).unwrap(), 1);
        drop(connection);
        assert!(matches!(service.grant_for_role("legacy-attempt", WorkUnitExecutionRole::Handler), Ok(_)));
        let reopened = fixture.service();
        assert!(matches!(reopened.grant_for_role("legacy-attempt", WorkUnitExecutionRole::Handler), Ok(_)));
    }

    #[test]
    fn file_review_compares_only_the_attempt_workspace() {
        let fixture = fixture();
        let service = fixture.service();
        fixture.authorize(&service, "attempt-1", "work-unit-1");
        let reference = service.grant("attempt-1").unwrap();
        let root = fixture.attempt_root("attempt-1");
        fs::write(root.join("attempt-only.txt"), "attempt\n").unwrap();
        fixture.git(&root, &["add", "."]);
        fixture.git(&root, &["commit", "-m", "attempt change"]);
        let manifest = service
            .consume(
                &reference.capability_ref,
                ExecutionSupportIntent::ChangedFileManifest,
            )
            .unwrap();
        let evidence = match manifest {
            ExecutionSupportResponse::ChangedFileManifest(files) => {
                assert_eq!(files.len(), 1);
                assert_eq!(files[0].display_name, "attempt-only.txt");
                files[0].evidence_ref.clone()
            }
            _ => panic!("expected manifest"),
        };
        assert!(matches!(
            service.consume(
                &reference.capability_ref,
                ExecutionSupportIntent::Comparison
            ),
            Ok(ExecutionSupportResponse::Comparison(_))
        ));
        assert!(matches!(
            service.consume(
                &reference.capability_ref,
                ExecutionSupportIntent::EvidenceContent {
                    evidence_ref: evidence
                }
            ),
            Ok(ExecutionSupportResponse::EvidenceContent(_))
        ));
        assert!(fixture
            .git(&fixture.sprint_root, &["status", "--porcelain"])
            .is_empty());
    }
}
