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
use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt,
    path::{Path, PathBuf},
    process::Command,
    sync::{Arc, Mutex},
};

pub(crate) const EXECUTION_SUPPORT_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS execution_support_attempt_authorizations (
  attempt_id TEXT PRIMARY KEY,
  work_unit_id TEXT NOT NULL,
  role_kind TEXT NOT NULL CHECK(role_kind IN ('work_unit_handler','work_unit_implementer')),
  sprint_git_authority_id TEXT NOT NULL UNIQUE,
  baseline_object_id TEXT NOT NULL,
  recorded_at TEXT NOT NULL,
  FOREIGN KEY(sprint_git_authority_id) REFERENCES initiated_sprint_git_authorities(authority_id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS execution_support_grants (
  attempt_id TEXT PRIMARY KEY,
  capability_ref TEXT NOT NULL UNIQUE,
  epic_id TEXT NOT NULL,
  sprint_id TEXT NOT NULL,
  work_unit_id TEXT NOT NULL,
  repository_id TEXT NOT NULL,
  role_id TEXT NOT NULL,
  workspace_id TEXT NOT NULL,
  workspace_fingerprint TEXT NOT NULL,
  correlation_fingerprint TEXT NOT NULL,
  recorded_at TEXT NOT NULL
);
"#;
pub(crate) const EXECUTION_SUPPORT_BASELINE_MIGRATION: &str =
    "ALTER TABLE execution_support_attempt_authorizations ADD COLUMN baseline_object_id TEXT NOT NULL DEFAULT '';";

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
}

pub(crate) struct ProductExecutionWorkspaceResolver {
    repository: Arc<SqliteOrchestrationRepository>,
}

impl ProductExecutionWorkspaceResolver {
    pub(crate) fn new(repository: Arc<SqliteOrchestrationRepository>) -> Self {
        Self { repository }
    }

    fn verified_workspace(
        &self,
        attempt: &AuthorizedExecutionAttempt,
        existing: Option<&ExecutionWorkspaceBinding>,
    ) -> Result<(PathBuf, String), ExecutionSupportError> {
        let authority = &attempt.authority;
        let root = canonical_authorized_root(&authority.worktree_root)?;
        let repository = canonical_authorized_root(&authority.repository_root)?;
        let common = git_path(&root, &["rev-parse", "--git-common-dir"])?;
        if common != canonical_authorized_root(&authority.repository_common_dir)?
            || git_path(&repository, &["rev-parse", "--git-common-dir"])? != common
            || git_text(
                &root,
                &[
                    "rev-parse",
                    "--verify",
                    &format!("{}^{{commit}}", attempt.baseline_object_id),
                ],
            )? != attempt.baseline_object_id
        {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        let head = git_text(&root, &["rev-parse", "--verify", "HEAD^{commit}"])?;
        if existing.is_none()
            && (head != attempt.baseline_object_id
                || !git_text(&root, &["status", "--porcelain"])?.is_empty())
        {
            return Err(ExecutionSupportError::Unavailable);
        }
        Ok((root, head))
    }
}

impl ExecutionWorkspaceResolver for ProductExecutionWorkspaceResolver {
    fn resolve(
        &self,
        attempt: &AuthorizedExecutionAttempt,
        existing: Option<&ExecutionWorkspaceBinding>,
    ) -> Result<ExecutionWorkspaceBinding, ExecutionSupportError> {
        let (root, _) = self.verified_workspace(attempt, existing)?;
        let binding = ExecutionWorkspaceBinding {
            workspace_id: attempt.authority.worktree_id.clone(),
            workspace_fingerprint: workspace_fingerprint(&attempt.authority, &root),
        };
        if existing.is_some_and(|existing| existing != &binding) {
            return Err(ExecutionSupportError::CorrelationMismatch);
        }
        Ok(binding)
    }

    fn inspect(
        &self,
        attempt: &AuthorizedExecutionAttempt,
        binding: &ExecutionWorkspaceBinding,
        capability_ref: &str,
    ) -> Result<CapturedInspection, ExecutionSupportError> {
        let (root, current_object_id) = self.verified_workspace(attempt, Some(binding))?;
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
                worktree_id: attempt.authority.worktree_id.clone(),
                worktree_root: attempt.authority.worktree_root.clone(),
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
        Ok(Self {
            connection: Mutex::new(connection),
            orchestration,
        })
    }
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

    /// This application-only operation accepts an opaque existing attempt reference. Its context
    /// is re-derived from durable authority; it does not create a Work Unit or execution attempt.
    pub(crate) fn grant(
        &self,
        attempt_id: &str,
    ) -> Result<ExecutionSupportReference, ExecutionSupportError> {
        let attempt = self.authorized_attempt(attempt_id)?;
        let connection = self
            .repository
            .connection
            .lock()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        let existing = load_grant(&connection, attempt_id)?;
        let binding = self
            .resolver
            .resolve(&attempt, existing.as_ref().map(|grant| &grant.binding))?;
        if let Some(existing) = existing {
            if existing.correlation != correlation_fingerprint(&attempt, &binding) {
                return Err(ExecutionSupportError::CorrelationMismatch);
            }
            return Ok(ExecutionSupportReference {
                capability_ref: existing.capability_ref,
            });
        }
        let capability_ref = stable_id("execution-support", attempt_id);
        let correlation = correlation_fingerprint(&attempt, &binding);
        connection.execute(
            "INSERT INTO execution_support_grants (attempt_id,capability_ref,epic_id,sprint_id,work_unit_id,repository_id,role_id,workspace_id,workspace_fingerprint,correlation_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,datetime('now'))",
            params![attempt.attempt_id, capability_ref, attempt.authority.epic_id, attempt.authority.sprint_id, attempt.work_unit_id, attempt.authority.repository_id, attempt.role_kind, binding.workspace_id, binding.workspace_fingerprint, correlation],
        ).map_err(|_| ExecutionSupportError::Conflict)?;
        Ok(ExecutionSupportReference { capability_ref })
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
        let attempt = self.authorized_attempt(&grant.attempt_id)?;
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

    fn authorized_attempt(
        &self,
        attempt_id: &str,
    ) -> Result<AuthorizedExecutionAttempt, ExecutionSupportError> {
        if !bounded_id(attempt_id) {
            return Err(ExecutionSupportError::Denied);
        }
        let (work_unit_id, role_kind, authority_id, baseline_object_id): (String, String, String, String) = self.repository.connection.lock().map_err(|_| ExecutionSupportError::Unavailable)?.query_row(
            "SELECT work_unit_id,role_kind,sprint_git_authority_id,baseline_object_id FROM execution_support_attempt_authorizations WHERE attempt_id=?1",
            [attempt_id], |row| Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?)),
        ).optional().map_err(|_| ExecutionSupportError::Unavailable)?.ok_or(ExecutionSupportError::Denied)?;
        if !bounded_id(&work_unit_id)
            || !matches!(
                role_kind.as_str(),
                "work_unit_handler" | "work_unit_implementer"
            )
            || !git_object_id(&baseline_object_id)
        {
            return Err(ExecutionSupportError::Denied);
        }
        let authority = self
            .repository
            .orchestration
            .load_initiated_sprint_git_authority(&authority_id)
            .map_err(|_| ExecutionSupportError::Unavailable)?
            .ok_or(ExecutionSupportError::Denied)?;
        Ok(AuthorizedExecutionAttempt {
            attempt_id: attempt_id.into(),
            work_unit_id,
            role_kind,
            baseline_object_id,
            authority,
        })
    }
}

/// Product boot composes the real narrow adapter. It can have no current attempt while still being
/// implemented; a missing durable authorization is reported only when an application later asks.
pub(crate) struct ProductExecutionSupportState {
    service: Arc<ExecutionSupportService>,
}
impl ProductExecutionSupportState {
    pub(crate) fn new(
        database_path: &Path,
        orchestration: Arc<SqliteOrchestrationRepository>,
    ) -> Result<Self, ExecutionSupportError> {
        let repository = Arc::new(SqliteExecutionSupportRepository::open(
            database_path,
            orchestration.clone(),
        )?);
        Ok(Self {
            service: Arc::new(ExecutionSupportService::new(
                repository,
                Arc::new(ProductExecutionWorkspaceResolver::new(orchestration)),
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
    capability_ref: String,
    binding: ExecutionWorkspaceBinding,
    correlation: String,
}

fn load_grant(
    connection: &Connection,
    attempt_id: &str,
) -> Result<Option<StoredGrant>, ExecutionSupportError> {
    load_grant_where(connection, "attempt_id", attempt_id)
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
    let query = format!("SELECT attempt_id,capability_ref,workspace_id,workspace_fingerprint,correlation_fingerprint FROM execution_support_grants WHERE {field}=?1");
    connection
        .query_row(&query, [value], |row| {
            Ok(StoredGrant {
                attempt_id: row.get(0)?,
                capability_ref: row.get(1)?,
                binding: ExecutionWorkspaceBinding {
                    workspace_id: row.get(2)?,
                    workspace_fingerprint: row.get(3)?,
                },
                correlation: row.get(4)?,
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
fn workspace_fingerprint(authority: &InitiatedSprintGitAuthority, root: &Path) -> String {
    fingerprint(&[
        &authority.authority_id,
        &authority.repository_id,
        &authority.worktree_id,
        &authority.source_fingerprint,
        root.to_string_lossy().as_ref(),
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
    use super::*;
    use std::{fs, process::Command};

    #[test]
    fn canonical_evidence_requires_manifest_membership() {
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
    fn productive_adapter_rejects_replaced_or_unclean_workspace() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo");
        fs::create_dir(&root).unwrap();
        let git = |args: &[&str]| {
            let output = Command::new("git")
                .args(args)
                .current_dir(&root)
                .output()
                .unwrap();
            assert!(output.status.success(), "{:?}", output);
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        };
        git(&["init"]);
        git(&["config", "user.email", "test@example.invalid"]);
        git(&["config", "user.name", "Test"]);
        fs::write(root.join("file.txt"), "base\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-m", "base"]);
        let baseline = git(&["rev-parse", "HEAD"]);
        fs::write(root.join("file.txt"), "next\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-m", "next"]);
        let head = git(&["rev-parse", "HEAD"]);
        let common = git_path(&root, &["rev-parse", "--git-common-dir"]).unwrap();
        let canonical_root = root.canonicalize().unwrap();
        let authority = InitiatedSprintGitAuthority {
            authority_id: "authority-1".into(),
            epic_id: "epic-1".into(),
            sprint_id: "sprint-1".into(),
            provenance_id: "provenance-1".into(),
            repository_id: "repository-1".into(),
            repository_root: canonical_root.to_string_lossy().into_owned(),
            repository_common_dir: common.to_string_lossy().into_owned(),
            worktree_id: "worktree-1".into(),
            worktree_root: canonical_root.to_string_lossy().into_owned(),
            baseline_object_id: baseline,
            current_object_id: head.clone(),
            runtime_instance_ref: "runtime-1".into(),
            runtime_source_ref: "source-1".into(),
            source_fingerprint: "a".repeat(64),
        };
        let attempt = AuthorizedExecutionAttempt {
            attempt_id: "attempt-1".into(),
            work_unit_id: "work-unit-1".into(),
            role_kind: "work_unit_implementer".into(),
            baseline_object_id: head.clone(),
            authority,
        };
        let database = temp.path().join("db.sqlite");
        let connection = Connection::open(&database).unwrap();
        crate::storage::configure_sqlite_connection(&connection).unwrap();
        crate::storage::initialize_active_database(&connection).unwrap();
        drop(connection);
        let orchestration = Arc::new(SqliteOrchestrationRepository::open(&database).unwrap());
        let resolver = ProductExecutionWorkspaceResolver::new(orchestration);
        let binding = resolver.resolve(&attempt, None).unwrap();
        fs::write(root.join("dirty.txt"), "dirty\n").unwrap();
        assert!(resolver.resolve(&attempt, None).is_err());
        assert_eq!(resolver.resolve(&attempt, Some(&binding)).unwrap(), binding);
        assert!(matches!(
            resolver.inspect(&attempt, &binding, "capability-1"),
            Err(ExecutionSupportError::Unavailable)
        ));
    }

    #[test]
    fn restart_reopens_durable_authority_and_refreshes_canonical_comparison() {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("repo");
        fs::create_dir(&root).unwrap();
        let git = |args: &[&str]| {
            let output = Command::new("git")
                .args(args)
                .current_dir(&root)
                .output()
                .unwrap();
            assert!(output.status.success(), "{:?}", output);
            String::from_utf8(output.stdout).unwrap().trim().to_owned()
        };
        git(&["init"]);
        git(&["config", "user.email", "test@example.invalid"]);
        git(&["config", "user.name", "Test"]);
        fs::write(root.join("file.txt"), "base\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-m", "base"]);
        let baseline = git(&["rev-parse", "HEAD"]);
        fs::write(root.join("file.txt"), "next\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-m", "next"]);
        let current = git(&["rev-parse", "HEAD"]);
        let canonical_root = root.canonicalize().unwrap();
        let common = git_path(&root, &["rev-parse", "--git-common-dir"]).unwrap();
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
        let authority_id = match orchestration.store_initiated_sprint_git_authority(super::super::repository::InitiatedSprintGitAuthorityWrite {
            sprint_id: "sprint-1".into(), idempotency_key: "execution-attempt-authority".into(), repository_id: "repository-1".into(), repository_root: canonical_root.to_string_lossy().into_owned(), repository_common_dir: common.to_string_lossy().into_owned(), worktree_id: "worktree-1".into(), worktree_root: canonical_root.to_string_lossy().into_owned(), baseline_object_id: baseline, current_object_id: current.clone(), runtime_instance_ref: "runtime-1".into(), runtime_source_ref: "source-1".into(), source_fingerprint: "a".repeat(64),
        }).unwrap() { super::super::repository::StoreInitiatedSprintGitAuthorityResult::Stored { authority_id } | super::super::repository::StoreInitiatedSprintGitAuthorityResult::IdempotentReplay { authority_id } => authority_id };
        let connection = Connection::open(&database).unwrap();
        connection.execute("INSERT INTO execution_support_attempt_authorizations (attempt_id,work_unit_id,role_kind,sprint_git_authority_id,baseline_object_id,recorded_at) VALUES ('attempt-1','work-unit-1','work_unit_implementer',?1,?2,'t')", params![authority_id, current]).unwrap();
        drop(connection);
        let service = ExecutionSupportService::new(
            Arc::new(
                SqliteExecutionSupportRepository::open(&database, orchestration.clone()).unwrap(),
            ),
            Arc::new(ProductExecutionWorkspaceResolver::new(orchestration)),
        );
        let reference = service.grant("attempt-1").unwrap();
        drop(service);
        let reopened_orchestration =
            Arc::new(SqliteOrchestrationRepository::open(&database).unwrap());
        let reopened = ExecutionSupportService::new(
            Arc::new(
                SqliteExecutionSupportRepository::open(&database, reopened_orchestration.clone())
                    .unwrap(),
            ),
            Arc::new(ProductExecutionWorkspaceResolver::new(
                reopened_orchestration,
            )),
        );
        assert!(
            matches!(reopened.consume(&reference.capability_ref, ExecutionSupportIntent::ChangedFileManifest), Ok(ExecutionSupportResponse::ChangedFileManifest(ref files)) if files.is_empty())
        );
        fs::write(root.join("file.txt"), "later\n").unwrap();
        git(&["add", "."]);
        git(&["commit", "-m", "later"]);
        let manifest = reopened
            .consume(
                &reference.capability_ref,
                ExecutionSupportIntent::ChangedFileManifest,
            )
            .unwrap();
        assert!(
            matches!(manifest, ExecutionSupportResponse::ChangedFileManifest(ref files) if files.len() == 1)
        );
        assert!(matches!(
            reopened.consume(
                &reference.capability_ref,
                ExecutionSupportIntent::Comparison
            ),
            Ok(ExecutionSupportResponse::Comparison(_))
        ));
    }
}
