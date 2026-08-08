use super::repository::{
    InitiatedSprintGitAuthority, InitiatedSprintGitAuthorityError,
    InitiatedSprintGitAuthorityWrite, SqliteOrchestrationRepository,
    StoreInitiatedSprintGitAuthorityResult,
};
use std::{error::Error, fmt, sync::Arc};

/// The application request carries only durable opaque identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BindInitiatedSprintGitAuthorityRequest {
    pub(crate) sprint_id: String,
    pub(crate) runtime_instance_ref: String,
    pub(crate) idempotency_key: String,
}

/// Private verified evidence returned by the Worktree Runtime boundary.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct VerifiedRuntimeGitComparison {
    pub(crate) repository_id: String,
    pub(crate) repository_root: String,
    pub(crate) repository_common_dir: String,
    pub(crate) worktree_id: String,
    pub(crate) worktree_root: String,
    pub(crate) baseline_object_id: String,
    pub(crate) current_object_id: String,
    pub(crate) runtime_instance_ref: String,
    pub(crate) runtime_source_ref: String,
    pub(crate) root_branch: String,
    pub(crate) source_fingerprint: String,
}

pub(crate) trait WorktreeRuntimeGitComparison: Send + Sync {
    fn resolve_verified_comparison(
        &self,
        runtime_instance_ref: &str,
    ) -> Result<VerifiedRuntimeGitComparison, BindInitiatedSprintGitAuthorityError>;
}

pub(crate) struct InitiatedSprintGitAuthorityService {
    repository: Arc<SqliteOrchestrationRepository>,
    runtime: Arc<dyn WorktreeRuntimeGitComparison>,
}

impl InitiatedSprintGitAuthorityService {
    pub(crate) fn new(
        repository: Arc<SqliteOrchestrationRepository>,
        runtime: Arc<dyn WorktreeRuntimeGitComparison>,
    ) -> Self {
        Self {
            repository,
            runtime,
        }
    }

    pub(crate) fn bind(
        &self,
        request: BindInitiatedSprintGitAuthorityRequest,
    ) -> Result<BindInitiatedSprintGitAuthorityResult, BindInitiatedSprintGitAuthorityError> {
        if !bounded_id(&request.sprint_id)
            || !bounded_id(&request.runtime_instance_ref)
            || !bounded_id(&request.idempotency_key)
        {
            return Err(BindInitiatedSprintGitAuthorityError::InvalidRequest);
        }
        let comparison = self
            .runtime
            .resolve_verified_comparison(&request.runtime_instance_ref)?;
        let root_branch = self
            .repository
            .load_epic_root_branch_for_sprint(&request.sprint_id)
            .map_err(|_| BindInitiatedSprintGitAuthorityError::SprintUnauthorized)?;
        if comparison.root_branch != root_branch {
            return Err(BindInitiatedSprintGitAuthorityError::RuntimeSourceIncompatible);
        }
        if comparison.runtime_instance_ref != request.runtime_instance_ref {
            return Err(BindInitiatedSprintGitAuthorityError::RuntimeEvidenceMismatch);
        }
        self.repository
            .store_initiated_sprint_git_authority(InitiatedSprintGitAuthorityWrite {
                sprint_id: request.sprint_id,
                idempotency_key: request.idempotency_key,
                repository_id: comparison.repository_id,
                repository_root: comparison.repository_root,
                repository_common_dir: comparison.repository_common_dir,
                worktree_id: comparison.worktree_id,
                worktree_root: comparison.worktree_root,
                baseline_object_id: comparison.baseline_object_id,
                current_object_id: comparison.current_object_id,
                runtime_instance_ref: comparison.runtime_instance_ref,
                runtime_source_ref: comparison.runtime_source_ref,
                root_branch,
                source_fingerprint: comparison.source_fingerprint,
            })
            .map(|result| match result {
                StoreInitiatedSprintGitAuthorityResult::Stored { authority_id } => {
                    BindInitiatedSprintGitAuthorityResult {
                        authority_ref: authority_id,
                        idempotent_replay: false,
                    }
                }
                StoreInitiatedSprintGitAuthorityResult::IdempotentReplay { authority_id } => {
                    BindInitiatedSprintGitAuthorityResult {
                        authority_ref: authority_id,
                        idempotent_replay: true,
                    }
                }
            })
            .map_err(Into::into)
    }

    /// Reauthorizes the durable relation against the live Worktree Runtime boundary.
    pub(crate) fn reauthorize(
        &self,
        authority_ref: &str,
    ) -> Result<InitiatedSprintGitAuthority, BindInitiatedSprintGitAuthorityError> {
        if !bounded_id(authority_ref) {
            return Err(BindInitiatedSprintGitAuthorityError::InvalidRequest);
        }
        let authority = self
            .repository
            .load_initiated_sprint_git_authority(authority_ref)?
            .ok_or(BindInitiatedSprintGitAuthorityError::SprintUnauthorized)?;
        let comparison = self
            .runtime
            .resolve_verified_comparison(&authority.runtime_instance_ref)?;
        if comparison.repository_id != authority.repository_id
            || comparison.repository_root != authority.repository_root
            || comparison.repository_common_dir != authority.repository_common_dir
            || comparison.worktree_id != authority.worktree_id
            || comparison.worktree_root != authority.worktree_root
            || !comparison
                .baseline_object_id
                .eq_ignore_ascii_case(&authority.baseline_object_id)
            || !comparison
                .current_object_id
                .eq_ignore_ascii_case(&authority.current_object_id)
            || comparison.runtime_instance_ref != authority.runtime_instance_ref
            || comparison.runtime_source_ref != authority.runtime_source_ref
            || comparison.root_branch != authority.root_branch
            || comparison.source_fingerprint != authority.source_fingerprint
        {
            return Err(BindInitiatedSprintGitAuthorityError::RuntimeEvidenceMismatch);
        }
        Ok(authority)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BindInitiatedSprintGitAuthorityResult {
    pub(crate) authority_ref: String,
    pub(crate) idempotent_replay: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum BindInitiatedSprintGitAuthorityError {
    InvalidRequest,
    SprintUnauthorized,
    RuntimeSourceUnavailable,
    RuntimeSourceStale,
    RuntimeSourceDirty,
    RuntimeSourceIncompatible,
    RuntimeEvidenceMismatch,
    ComparisonUnavailable,
    Conflict,
    Unavailable,
}

impl fmt::Display for BindInitiatedSprintGitAuthorityError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::InvalidRequest => "the initiated Sprint Git-authority request is invalid",
            Self::SprintUnauthorized => "the initiated Sprint ownership chain is unavailable",
            Self::RuntimeSourceUnavailable => "the prepared runtime source is unavailable",
            Self::RuntimeSourceStale => "the prepared runtime source is stale or superseded",
            Self::RuntimeSourceDirty => "the prepared runtime source is dirty",
            Self::RuntimeSourceIncompatible => "the prepared runtime source is incompatible",
            Self::RuntimeEvidenceMismatch => "the runtime comparison evidence does not match",
            Self::ComparisonUnavailable => {
                "the runtime cannot supply a stable immutable Git comparison"
            }
            Self::Conflict => "the initiated Sprint Git-authority request conflicts",
            Self::Unavailable => "the initiated Sprint Git-authority transition is unavailable",
        })
    }
}

impl Error for BindInitiatedSprintGitAuthorityError {}

impl From<InitiatedSprintGitAuthorityError> for BindInitiatedSprintGitAuthorityError {
    fn from(value: InitiatedSprintGitAuthorityError) -> Self {
        match value {
            InitiatedSprintGitAuthorityError::Invalid => Self::RuntimeEvidenceMismatch,
            InitiatedSprintGitAuthorityError::Forbidden => Self::SprintUnauthorized,
            InitiatedSprintGitAuthorityError::Conflict => Self::Conflict,
            InitiatedSprintGitAuthorityError::Unavailable => Self::Unavailable,
        }
    }
}

fn bounded_id(value: &str) -> bool {
    !value.is_empty()
        && value.len() <= 128
        && value
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'.' | b'_' | b'-' | b'/'))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::storage::{configure_sqlite_connection, initialize_active_database};
    use rusqlite::Connection;
    use std::sync::Mutex;

    struct RuntimeFixture(
        Mutex<Result<VerifiedRuntimeGitComparison, BindInitiatedSprintGitAuthorityError>>,
    );

    impl WorktreeRuntimeGitComparison for RuntimeFixture {
        fn resolve_verified_comparison(
            &self,
            _runtime_instance_ref: &str,
        ) -> Result<VerifiedRuntimeGitComparison, BindInitiatedSprintGitAuthorityError> {
            self.0.lock().unwrap().clone()
        }
    }

    fn comparison() -> VerifiedRuntimeGitComparison {
        let repository_root = std::env::temp_dir().join("authority-repository");
        VerifiedRuntimeGitComparison {
            repository_id: "repository-1".into(),
            repository_root: repository_root.to_string_lossy().into_owned(),
            repository_common_dir: repository_root.join(".git").to_string_lossy().into_owned(),
            worktree_id: "worktree-1".into(),
            worktree_root: std::env::temp_dir()
                .join("authority-worktree")
                .to_string_lossy()
                .into_owned(),
            baseline_object_id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            current_object_id: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            runtime_instance_ref: "runtime-instance-1".into(),
            runtime_source_ref: "runtime-source-1".into(),
            source_fingerprint: "0123456789abcdef0123456789abcdef0123456789abcdef0123456789abcdef"
                .into(),
        }
    }

    fn request(sprint_id: &str, idempotency_key: &str) -> BindInitiatedSprintGitAuthorityRequest {
        BindInitiatedSprintGitAuthorityRequest {
            sprint_id: sprint_id.into(),
            runtime_instance_ref: "runtime-instance-1".into(),
            idempotency_key: idempotency_key.into(),
        }
    }

    fn seeded_connection(path: &std::path::Path) -> Connection {
        let connection = Connection::open(path).unwrap();
        configure_sqlite_connection(&connection).unwrap();
        initialize_active_database(&connection).unwrap();
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection.execute_batch("INSERT INTO epic_initiation_provenance (id,command_id,result_id,event_id,recorded_at) VALUES ('provenance-1','command-1','result-1','event-1','t'),('provenance-2','command-2','result-2','event-2','t'); INSERT INTO epic_initiations (id,command_id,result_id,event_id,provenance_id,draft_id,proposal_revision_id,material_snapshot_id,epic_id,recorded_at) VALUES ('initiation-1','command-1','result-1','event-1','provenance-1','draft-1','revision-1','snapshot-1','epic-1','t'),('initiation-2','command-2','result-2','event-2','provenance-2','draft-2','revision-2','snapshot-2','epic-2','t'); INSERT INTO initiated_sprints (id,epic_id,ordinal,title,intended_movement,concern_summaries_json,sprint_plan_id,sprint_plan_revision_id) VALUES ('sprint-1','epic-1',0,'One','Move','[]','plan-1','plan-revision-1'),('sprint-2','epic-2',0,'Two','Move','[]','plan-2','plan-revision-2');").unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        connection
    }

    #[test]
    fn binds_only_opaque_request_to_same_epic_facts_and_replays_exactly() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("active.sqlite");
        let repository =
            Arc::new(SqliteOrchestrationRepository::new(seeded_connection(&path)).unwrap());
        let runtime = Arc::new(RuntimeFixture(Mutex::new(Ok(comparison()))));
        let service = InitiatedSprintGitAuthorityService::new(repository.clone(), runtime.clone());

        let stored = service.bind(request("sprint-1", "request-1")).unwrap();
        assert!(!stored.idempotent_replay);
        let replay = service.bind(request("sprint-1", "request-1")).unwrap();
        assert_eq!(replay.authority_ref, stored.authority_ref);
        assert!(replay.idempotent_replay);

        let loaded = repository
            .load_initiated_sprint_git_authority(&stored.authority_ref)
            .unwrap()
            .unwrap();
        assert_eq!(loaded.epic_id, "epic-1");
        assert_eq!(loaded.sprint_id, "sprint-1");
        assert_eq!(loaded.provenance_id, "provenance-1");
        assert_eq!(loaded.runtime_instance_ref, "runtime-instance-1");
        assert_eq!(loaded.runtime_source_ref, "runtime-source-1");
        assert_eq!(
            loaded.repository_common_dir,
            comparison().repository_common_dir
        );
        assert_eq!(loaded.baseline_object_id, comparison().baseline_object_id);
        assert_eq!(loaded.current_object_id, comparison().current_object_id);
        assert_eq!(loaded.source_fingerprint, comparison().source_fingerprint);

        *runtime.0.lock().unwrap() = Ok(VerifiedRuntimeGitComparison {
            current_object_id: "cccccccccccccccccccccccccccccccccccccccc".into(),
            ..comparison()
        });
        assert_eq!(
            service.bind(request("sprint-1", "request-1")),
            Err(BindInitiatedSprintGitAuthorityError::Conflict)
        );
        assert_eq!(
            service.bind(request("sprint-2", "request-2")),
            Err(BindInitiatedSprintGitAuthorityError::Conflict)
        );
    }

    #[test]
    fn denies_missing_ownership_or_runtime_evidence_without_partial_write() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("active.sqlite");
        let repository =
            Arc::new(SqliteOrchestrationRepository::new(seeded_connection(&path)).unwrap());
        let runtime = Arc::new(RuntimeFixture(Mutex::new(Ok(comparison()))));
        let service = InitiatedSprintGitAuthorityService::new(repository.clone(), runtime.clone());

        assert_eq!(
            service.bind(request("missing-sprint", "request-missing")),
            Err(BindInitiatedSprintGitAuthorityError::SprintUnauthorized)
        );
        *runtime.0.lock().unwrap() = Err(BindInitiatedSprintGitAuthorityError::RuntimeSourceDirty);
        assert_eq!(
            service.bind(request("sprint-1", "request-dirty")),
            Err(BindInitiatedSprintGitAuthorityError::RuntimeSourceDirty)
        );
        for (idempotency_key, invalid) in [
            (
                "request-object-tamper",
                VerifiedRuntimeGitComparison {
                    current_object_id: "not-an-object".into(),
                    ..comparison()
                },
            ),
            (
                "request-root-tamper",
                VerifiedRuntimeGitComparison {
                    worktree_root: "relative/worktree".into(),
                    ..comparison()
                },
            ),
            (
                "request-fingerprint-tamper",
                VerifiedRuntimeGitComparison {
                    source_fingerprint: "A".repeat(64),
                    ..comparison()
                },
            ),
        ] {
            *runtime.0.lock().unwrap() = Ok(invalid);
            assert_eq!(
                service.bind(request("sprint-1", idempotency_key)),
                Err(BindInitiatedSprintGitAuthorityError::RuntimeEvidenceMismatch)
            );
        }
        let connection = Connection::open(path).unwrap();
        assert_eq!(
            connection
                .query_row(
                    "SELECT COUNT(*) FROM initiated_sprint_git_authorities",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );
    }

    #[test]
    fn restart_retains_relation_and_read_reauthorizes_live_chain() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("active.sqlite");
        let repository =
            Arc::new(SqliteOrchestrationRepository::new(seeded_connection(&path)).unwrap());
        let service = InitiatedSprintGitAuthorityService::new(
            repository,
            Arc::new(RuntimeFixture(Mutex::new(Ok(comparison())))),
        );
        let stored = service
            .bind(request("sprint-1", "request-restart"))
            .unwrap();
        drop(service);

        let reopened = SqliteOrchestrationRepository::open(&path).unwrap();
        let loaded = reopened
            .load_initiated_sprint_git_authority(&stored.authority_ref)
            .unwrap();
        assert!(loaded.is_some());
        drop(reopened);

        let connection = Connection::open(&path).unwrap();
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection
            .execute(
                "UPDATE initiated_sprint_git_authorities SET source_fingerprint='tampered' WHERE authority_id=?1",
                [&stored.authority_ref],
            )
            .unwrap();
        drop(connection);
        let reopened = SqliteOrchestrationRepository::open(&path).unwrap();
        assert_eq!(
            reopened.load_initiated_sprint_git_authority(&stored.authority_ref),
            Err(InitiatedSprintGitAuthorityError::Invalid)
        );
        drop(reopened);
        let connection = Connection::open(&path).unwrap();
        connection
            .execute(
                "UPDATE initiated_sprint_git_authorities SET source_fingerprint=?2,epic_id='epic-2' WHERE authority_id=?1",
                [&stored.authority_ref, &comparison().source_fingerprint],
            )
            .unwrap();
        drop(connection);
        let reopened = SqliteOrchestrationRepository::open(&path).unwrap();
        assert!(reopened
            .load_initiated_sprint_git_authority(&stored.authority_ref)
            .unwrap()
            .is_none());
    }
}
