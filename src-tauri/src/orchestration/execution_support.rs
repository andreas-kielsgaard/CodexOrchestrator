//! Narrow, attempt-scoped execution support for a later authorized Harness action.
//!
//! The consumer receives an opaque capability reference and evidence references only. Repository,
//! workspace, Git, path, and role-routing identities stay inside application/native authority.

use rusqlite::{params, Connection, OptionalExtension};
use sha2::{Digest, Sha256};
use std::{
    error::Error,
    fmt,
    path::Path,
    sync::{Arc, Mutex},
};

pub(crate) const EXECUTION_SUPPORT_SCHEMA: &str = r#"
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
CREATE TABLE IF NOT EXISTS execution_support_evidence (
  capability_ref TEXT NOT NULL,
  evidence_ref TEXT NOT NULL,
  display_name TEXT NOT NULL,
  change_kind TEXT NOT NULL CHECK(change_kind IN ('added','modified','deleted','renamed')),
  content BLOB NOT NULL,
  PRIMARY KEY(capability_ref,evidence_ref),
  FOREIGN KEY(capability_ref) REFERENCES execution_support_grants(capability_ref) ON DELETE RESTRICT
);
"#;

/// This context is assembled by application-owned lifecycle code, never by a Harness consumer.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ApplicationOwnedExecutionAttempt {
    pub(crate) epic_id: String,
    pub(crate) sprint_id: String,
    pub(crate) work_unit_id: String,
    pub(crate) attempt_id: String,
    pub(crate) repository_id: String,
    pub(crate) role_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExecutionWorkspaceBinding {
    repository_id: String,
    workspace_id: String,
    workspace_fingerprint: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedExecutionEvidence {
    pub(crate) evidence_ref: String,
    pub(crate) display_name: String,
    pub(crate) change_kind: String,
    pub(crate) content: Vec<u8>,
}

/// Private native result; workspace routing data is deliberately absent from the consumer API.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedExecutionWorkspace {
    pub(crate) binding: ExecutionWorkspaceBinding,
    pub(crate) evidence: Vec<VerifiedExecutionEvidence>,
}

/// The productive adapter may create/recover an isolated workspace, but receives only app context.
pub(crate) trait ExecutionWorkspaceResolver: Send + Sync {
    fn resolve_or_create(
        &self,
        attempt: &ApplicationOwnedExecutionAttempt,
        existing: Option<&ExecutionWorkspaceBinding>,
    ) -> Result<VerifiedExecutionWorkspace, ExecutionSupportError>;
}

/// Honest production fallback until a live application-owned workspace authority is composed.
pub(crate) struct UnavailableExecutionWorkspaceResolver;

impl ExecutionWorkspaceResolver for UnavailableExecutionWorkspaceResolver {
    fn resolve_or_create(
        &self,
        _attempt: &ApplicationOwnedExecutionAttempt,
        _existing: Option<&ExecutionWorkspaceBinding>,
    ) -> Result<VerifiedExecutionWorkspace, ExecutionSupportError> {
        Err(ExecutionSupportError::Unavailable)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ExecutionSupportReference {
    pub(crate) capability_ref: String,
    pub(crate) available: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct ChangedFileManifestEntry {
    pub(crate) evidence_ref: String,
    pub(crate) display_name: String,
    pub(crate) change_kind: String,
}

/// A future Harness can issue only these semantic intents; there is no path, ref, or shell intent.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ExecutionSupportIntent {
    ChangedFileManifest,
    EvidenceContent { evidence_ref: String },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ExecutionSupportResponse {
    ChangedFileManifest(Vec<ChangedFileManifestEntry>),
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
            Self::Unavailable => "execution support is unavailable because its workspace authority cannot be verified",
            Self::CorrelationMismatch => "the durable execution-support correlation no longer matches the live workspace",
            Self::Conflict => "the execution-support grant conflicts with durable state",
        })
    }
}
impl Error for ExecutionSupportError {}

pub(crate) struct SqliteExecutionSupportRepository {
    connection: Mutex<Connection>,
}

impl SqliteExecutionSupportRepository {
    pub(crate) fn open(path: &Path) -> Result<Self, ExecutionSupportError> {
        let connection = Connection::open(path).map_err(|_| ExecutionSupportError::Unavailable)?;
        crate::storage::configure_sqlite_connection(&connection)
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        connection
            .execute_batch(EXECUTION_SUPPORT_SCHEMA)
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    #[cfg(test)]
    fn memory() -> Self {
        let connection = Connection::open_in_memory().unwrap();
        crate::storage::configure_sqlite_connection(&connection).unwrap();
        connection.execute_batch(EXECUTION_SUPPORT_SCHEMA).unwrap();
        Self {
            connection: Mutex::new(connection),
        }
    }
}

pub(crate) struct ExecutionSupportService {
    repository: Arc<SqliteExecutionSupportRepository>,
    resolver: Arc<dyn ExecutionWorkspaceResolver>,
}

impl ExecutionSupportService {
    pub(crate) fn new(
        repository: Arc<SqliteExecutionSupportRepository>,
        resolver: Arc<dyn ExecutionWorkspaceResolver>,
    ) -> Self {
        Self {
            repository,
            resolver,
        }
    }

    /// Grants/replays one attempt-scoped reference. The lock covers first resolution and write,
    /// making duplicate callers observe one durable workspace binding.
    pub(crate) fn grant(
        &self,
        attempt: ApplicationOwnedExecutionAttempt,
    ) -> Result<ExecutionSupportReference, ExecutionSupportError> {
        validate_attempt(&attempt)?;
        let mut connection = self
            .repository
            .connection
            .lock()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        let existing = load_grant(&connection, &attempt.attempt_id)?;
        let workspace = self
            .resolver
            .resolve_or_create(&attempt, existing.as_ref().map(|grant| &grant.binding))?;
        validate_workspace(&attempt, &workspace)?;
        if let Some(existing) = existing {
            if existing.context != attempt || existing.binding != workspace.binding {
                return Err(ExecutionSupportError::CorrelationMismatch);
            }
            return Ok(ExecutionSupportReference {
                capability_ref: existing.capability_ref,
                available: true,
            });
        }
        let capability_ref = stable_id("execution-support", &attempt.attempt_id);
        let correlation = correlation_fingerprint(&attempt, &workspace.binding);
        let transaction = connection
            .unchecked_transaction()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        transaction.execute(
            "INSERT INTO execution_support_grants (attempt_id,capability_ref,epic_id,sprint_id,work_unit_id,repository_id,role_id,workspace_id,workspace_fingerprint,correlation_fingerprint,recorded_at) VALUES (?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,datetime('now'))",
            params![attempt.attempt_id, capability_ref, attempt.epic_id, attempt.sprint_id, attempt.work_unit_id, attempt.repository_id, attempt.role_id, workspace.binding.workspace_id, workspace.binding.workspace_fingerprint, correlation],
        ).map_err(|_| ExecutionSupportError::Conflict)?;
        for evidence in &workspace.evidence {
            transaction.execute(
                "INSERT INTO execution_support_evidence (capability_ref,evidence_ref,display_name,change_kind,content) VALUES (?1,?2,?3,?4,?5)",
                params![capability_ref, evidence.evidence_ref, evidence.display_name, evidence.change_kind, evidence.content],
            ).map_err(|_| ExecutionSupportError::Conflict)?;
        }
        transaction
            .commit()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        Ok(ExecutionSupportReference {
            capability_ref,
            available: true,
        })
    }

    /// Rechecks durable correlation against the live resolver; it never turns stale state into availability.
    pub(crate) fn recover(
        &self,
        attempt: ApplicationOwnedExecutionAttempt,
    ) -> Result<ExecutionSupportReference, ExecutionSupportError> {
        self.grant(attempt)
    }

    pub(crate) fn consume(
        &self,
        capability_ref: &str,
        intent: ExecutionSupportIntent,
    ) -> Result<ExecutionSupportResponse, ExecutionSupportError> {
        if !bounded(capability_ref) {
            return Err(ExecutionSupportError::Denied);
        }
        let connection = self
            .repository
            .connection
            .lock()
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        let exists: bool = connection
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM execution_support_grants WHERE capability_ref=?1)",
                [capability_ref],
                |row| row.get(0),
            )
            .map_err(|_| ExecutionSupportError::Unavailable)?;
        if !exists {
            return Err(ExecutionSupportError::Denied);
        }
        match intent {
            ExecutionSupportIntent::ChangedFileManifest => {
                let mut statement = connection.prepare("SELECT evidence_ref,display_name,change_kind FROM execution_support_evidence WHERE capability_ref=?1 ORDER BY evidence_ref").map_err(|_| ExecutionSupportError::Unavailable)?;
                let rows = statement
                    .query_map([capability_ref], |row| {
                        Ok(ChangedFileManifestEntry {
                            evidence_ref: row.get(0)?,
                            display_name: row.get(1)?,
                            change_kind: row.get(2)?,
                        })
                    })
                    .map_err(|_| ExecutionSupportError::Unavailable)?;
                Ok(ExecutionSupportResponse::ChangedFileManifest(
                    rows.collect::<Result<Vec<_>, _>>()
                        .map_err(|_| ExecutionSupportError::Unavailable)?,
                ))
            }
            ExecutionSupportIntent::EvidenceContent { evidence_ref } => {
                if !bounded(&evidence_ref) {
                    return Err(ExecutionSupportError::Denied);
                }
                let content = connection.query_row("SELECT content FROM execution_support_evidence WHERE capability_ref=?1 AND evidence_ref=?2", params![capability_ref, evidence_ref], |row| row.get(0)).optional().map_err(|_| ExecutionSupportError::Unavailable)?;
                content
                    .map(ExecutionSupportResponse::EvidenceContent)
                    .ok_or(ExecutionSupportError::Denied)
            }
        }
    }
}

/// Native product composition owns this state. No recorded/development resolver is substituted.
pub(crate) struct ProductExecutionSupportState {
    service: Arc<ExecutionSupportService>,
}
impl ProductExecutionSupportState {
    pub(crate) fn unavailable(database_path: &Path) -> Result<Self, ExecutionSupportError> {
        Ok(Self {
            service: Arc::new(ExecutionSupportService::new(
                Arc::new(SqliteExecutionSupportRepository::open(database_path)?),
                Arc::new(UnavailableExecutionWorkspaceResolver),
            )),
        })
    }
    pub(crate) fn service(&self) -> Arc<ExecutionSupportService> {
        self.service.clone()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct StoredGrant {
    capability_ref: String,
    context: ApplicationOwnedExecutionAttempt,
    binding: ExecutionWorkspaceBinding,
}

fn load_grant(
    connection: &Connection,
    attempt_id: &str,
) -> Result<Option<StoredGrant>, ExecutionSupportError> {
    connection.query_row(
        "SELECT capability_ref,epic_id,sprint_id,work_unit_id,repository_id,role_id,workspace_id,workspace_fingerprint,correlation_fingerprint FROM execution_support_grants WHERE attempt_id=?1",
        [attempt_id],
        |row| Ok((row.get::<_, String>(0)?, ApplicationOwnedExecutionAttempt { epic_id: row.get(1)?, sprint_id: row.get(2)?, work_unit_id: row.get(3)?, attempt_id: attempt_id.into(), repository_id: row.get(4)?, role_id: row.get(5)? }, ExecutionWorkspaceBinding { repository_id: row.get(4)?, workspace_id: row.get(6)?, workspace_fingerprint: row.get(7)? }, row.get::<_, String>(8)?)),
    ).optional().map_err(|_| ExecutionSupportError::Unavailable)?.map(|(capability_ref, context, binding, correlation)| {
        (correlation == correlation_fingerprint(&context, &binding)).then_some(StoredGrant { capability_ref, context, binding }).ok_or(ExecutionSupportError::CorrelationMismatch)
    }).transpose()
}

fn validate_attempt(value: &ApplicationOwnedExecutionAttempt) -> Result<(), ExecutionSupportError> {
    [
        &value.epic_id,
        &value.sprint_id,
        &value.work_unit_id,
        &value.attempt_id,
        &value.repository_id,
        &value.role_id,
    ]
    .iter()
    .all(|value| bounded(value))
    .then_some(())
    .ok_or(ExecutionSupportError::Invalid)
}
fn validate_workspace(
    attempt: &ApplicationOwnedExecutionAttempt,
    value: &VerifiedExecutionWorkspace,
) -> Result<(), ExecutionSupportError> {
    if value.binding.repository_id != attempt.repository_id
        || !bounded(&value.binding.workspace_id)
        || !fingerprint(&value.binding.workspace_fingerprint)
        || value.evidence.is_empty()
    {
        return Err(ExecutionSupportError::CorrelationMismatch);
    }
    let mut refs = std::collections::HashSet::new();
    for evidence in &value.evidence {
        if !bounded(&evidence.evidence_ref)
            || !safe_relative_path(&evidence.display_name)
            || !matches!(
                evidence.change_kind.as_str(),
                "added" | "modified" | "deleted" | "renamed"
            )
            || evidence.content.len() > 1_000_000
            || !refs.insert(&evidence.evidence_ref)
        {
            return Err(ExecutionSupportError::Invalid);
        }
    }
    Ok(())
}
fn bounded(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-'))
}
fn fingerprint(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit() && !byte.is_ascii_uppercase())
}
fn safe_relative_path(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 4096
        && !value.starts_with('/')
        && !value.starts_with('\\')
        && !value.contains('\\')
        && !value
            .split('/')
            .any(|part| part.is_empty() || part == "." || part == "..")
}
fn stable_id(prefix: &str, value: &str) -> String {
    format!(
        "{prefix}-{}",
        &format!("{:x}", Sha256::digest(value.as_bytes()))[..24]
    )
}
fn correlation_fingerprint(
    context: &ApplicationOwnedExecutionAttempt,
    binding: &ExecutionWorkspaceBinding,
) -> String {
    let mut hash = Sha256::new();
    for value in [
        &context.epic_id,
        &context.sprint_id,
        &context.work_unit_id,
        &context.attempt_id,
        &context.repository_id,
        &context.role_id,
        &binding.workspace_id,
        &binding.workspace_fingerprint,
    ] {
        hash.update((value.len() as u64).to_be_bytes());
        hash.update(value.as_bytes());
    }
    format!("{:x}", hash.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        sync::atomic::{AtomicUsize, Ordering},
        thread,
    };

    struct Resolver {
        workspace: Mutex<Result<VerifiedExecutionWorkspace, ExecutionSupportError>>,
        first_resolutions: AtomicUsize,
    }
    impl ExecutionWorkspaceResolver for Resolver {
        fn resolve_or_create(
            &self,
            _attempt: &ApplicationOwnedExecutionAttempt,
            existing: Option<&ExecutionWorkspaceBinding>,
        ) -> Result<VerifiedExecutionWorkspace, ExecutionSupportError> {
            if existing.is_none() {
                self.first_resolutions.fetch_add(1, Ordering::SeqCst);
            }
            self.workspace.lock().unwrap().clone()
        }
    }
    fn attempt(id: &str) -> ApplicationOwnedExecutionAttempt {
        ApplicationOwnedExecutionAttempt {
            epic_id: "epic-1".into(),
            sprint_id: "sprint-1".into(),
            work_unit_id: "work-unit-1".into(),
            attempt_id: id.into(),
            repository_id: "repository-1".into(),
            role_id: "implementer-1".into(),
        }
    }
    fn workspace() -> VerifiedExecutionWorkspace {
        VerifiedExecutionWorkspace {
            binding: ExecutionWorkspaceBinding {
                repository_id: "repository-1".into(),
                workspace_id: "workspace-1".into(),
                workspace_fingerprint: "a".repeat(64),
            },
            evidence: vec![VerifiedExecutionEvidence {
                evidence_ref: "evidence-1".into(),
                display_name: "src/lib.rs".into(),
                change_kind: "modified".into(),
                content: b"changed".to_vec(),
            }],
        }
    }
    fn service(resolver: Arc<Resolver>) -> Arc<ExecutionSupportService> {
        Arc::new(ExecutionSupportService::new(
            Arc::new(SqliteExecutionSupportRepository::memory()),
            resolver,
        ))
    }

    #[test]
    fn concurrent_grants_create_one_workspace_binding_and_scope_evidence() {
        let resolver = Arc::new(Resolver {
            workspace: Mutex::new(Ok(workspace())),
            first_resolutions: AtomicUsize::new(0),
        });
        let service = service(resolver.clone());
        let joins = (0..4)
            .map(|_| {
                let service = service.clone();
                thread::spawn(move || service.grant(attempt("attempt-1")).unwrap())
            })
            .collect::<Vec<_>>();
        let references = joins
            .into_iter()
            .map(|join| join.join().unwrap().capability_ref)
            .collect::<Vec<_>>();
        assert!(references
            .iter()
            .all(|reference| reference == &references[0]));
        assert_eq!(resolver.first_resolutions.load(Ordering::SeqCst), 1);
        assert_eq!(
            service
                .consume(&references[0], ExecutionSupportIntent::ChangedFileManifest)
                .unwrap(),
            ExecutionSupportResponse::ChangedFileManifest(vec![ChangedFileManifestEntry {
                evidence_ref: "evidence-1".into(),
                display_name: "src/lib.rs".into(),
                change_kind: "modified".into()
            }])
        );
        assert_eq!(
            service
                .consume(
                    &references[0],
                    ExecutionSupportIntent::EvidenceContent {
                        evidence_ref: "evidence-1".into()
                    }
                )
                .unwrap(),
            ExecutionSupportResponse::EvidenceContent(b"changed".to_vec())
        );
    }

    #[test]
    fn denies_cross_attempt_unknown_evidence_and_path_like_manifest_entries() {
        let resolver = Arc::new(Resolver {
            workspace: Mutex::new(Ok(workspace())),
            first_resolutions: AtomicUsize::new(0),
        });
        let service = service(resolver.clone());
        let reference = service.grant(attempt("attempt-1")).unwrap();
        assert_eq!(
            service.consume(
                "execution-support-other",
                ExecutionSupportIntent::ChangedFileManifest
            ),
            Err(ExecutionSupportError::Denied)
        );
        assert_eq!(
            service.consume(
                &reference.capability_ref,
                ExecutionSupportIntent::EvidenceContent {
                    evidence_ref: "other-file".into()
                }
            ),
            Err(ExecutionSupportError::Denied)
        );
        resolver
            .workspace
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .evidence[0]
            .display_name = "../escape.rs".into();
        assert_eq!(
            service.grant(attempt("attempt-2")),
            Err(ExecutionSupportError::Invalid)
        );
    }

    #[test]
    fn restart_revalidates_and_fails_closed_on_workspace_drift() {
        let resolver = Arc::new(Resolver {
            workspace: Mutex::new(Ok(workspace())),
            first_resolutions: AtomicUsize::new(0),
        });
        let service = service(resolver.clone());
        let reference = service.grant(attempt("attempt-1")).unwrap();
        resolver
            .workspace
            .lock()
            .unwrap()
            .as_mut()
            .unwrap()
            .binding
            .workspace_fingerprint = "b".repeat(64);
        assert_eq!(
            service.recover(attempt("attempt-1")),
            Err(ExecutionSupportError::CorrelationMismatch)
        );
        assert_eq!(
            service
                .consume(
                    &reference.capability_ref,
                    ExecutionSupportIntent::EvidenceContent {
                        evidence_ref: "evidence-1".into()
                    }
                )
                .unwrap(),
            ExecutionSupportResponse::EvidenceContent(b"changed".to_vec())
        );
    }

    #[test]
    fn production_adapter_is_truthfully_unavailable_without_live_workspace_authority() {
        let resolver = UnavailableExecutionWorkspaceResolver;
        assert_eq!(
            resolver.resolve_or_create(&attempt("attempt-1"), None),
            Err(ExecutionSupportError::Unavailable)
        );
    }
}
