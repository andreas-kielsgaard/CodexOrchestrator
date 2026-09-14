mod associations;
mod attempts;
mod attentions;
mod builds;
mod cleanup;
mod repositories;
mod schema;
mod selection;
mod settings;

pub(crate) use associations::WorktreeAssociationRepository;
pub(crate) use attempts::OperationAttemptRepository;
pub(crate) use attentions::BuildAttentionRepository;
pub(crate) use builds::{BuildOutputRepository, ReviewBuildRepository, WorkspaceRepository};
pub(crate) use cleanup::CleanupRepository;
pub(crate) use repositories::ReviewRepositoryRepository;
pub(crate) use selection::{PersistedRepositorySelection, RepositorySelectionRepository};
pub(crate) use settings::ReviewSettingsRepository;

use associations::SqliteWorktreeAssociationRepository;
use attempts::SqliteOperationAttemptRepository;
use attentions::SqliteBuildAttentionRepository;
use builds::{SqliteBuildOutputRepository, SqliteReviewBuildRepository, SqliteWorkspaceRepository};
use cleanup::SqliteCleanupRepository;
use repositories::SqliteReviewRepositoryRepository;
use selection::SqliteRepositorySelectionRepository;
use settings::SqliteReviewSettingsRepository;

use rusqlite::{Connection, TransactionBehavior};
use std::{
    error::Error,
    fmt,
    path::Path,
    sync::{Mutex, MutexGuard},
    time::{Duration, Instant},
};

const CONNECTION_BUSY_TIMEOUT: Duration = Duration::from_secs(5);
const JOURNAL_MODE_RETRY_INTERVAL: Duration = Duration::from_millis(10);

pub(crate) type StorageResult<T> = Result<T, StorageError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum StorageErrorKind {
    NotFound,
    Conflict,
    CorruptData,
    Unavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct StorageError {
    pub(crate) kind: StorageErrorKind,
    pub(crate) message: String,
}

impl StorageError {
    pub(super) fn new(kind: StorageErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub(super) fn corrupt(message: impl Into<String>) -> Self {
        Self::new(StorageErrorKind::CorruptData, message)
    }
}

impl fmt::Display for StorageError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for StorageError {}

pub(crate) struct WorktreeReviewDatabase {
    connection: Mutex<Connection>,
}

impl WorktreeReviewDatabase {
    pub(crate) fn open(path: impl AsRef<Path>) -> StorageResult<Self> {
        let mut connection =
            Connection::open(path).map_err(sql_error("open worktree review state"))?;
        configure_shared_connection(&connection, true)?;
        schema::initialize(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    #[cfg(test)]
    pub(crate) fn open_in_memory() -> StorageResult<Self> {
        let mut connection =
            Connection::open_in_memory().map_err(sql_error("open memory database"))?;
        configure_shared_connection(&connection, false)?;
        schema::initialize(&mut connection)?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    pub(crate) fn repositories(&self) -> impl ReviewRepositoryRepository + '_ {
        SqliteReviewRepositoryRepository::new(self)
    }

    pub(crate) fn selection(&self) -> impl RepositorySelectionRepository + '_ {
        SqliteRepositorySelectionRepository::new(self)
    }

    pub(crate) fn associations(&self) -> impl WorktreeAssociationRepository + '_ {
        SqliteWorktreeAssociationRepository::new(self)
    }

    pub(crate) fn workspaces(&self) -> impl WorkspaceRepository + '_ {
        SqliteWorkspaceRepository::new(self)
    }

    pub(crate) fn builds(&self) -> impl ReviewBuildRepository + '_ {
        SqliteReviewBuildRepository::new(self)
    }

    pub(crate) fn attempts(&self) -> impl OperationAttemptRepository + '_ {
        SqliteOperationAttemptRepository::new(self)
    }

    pub(crate) fn attentions(&self) -> impl BuildAttentionRepository + '_ {
        SqliteBuildAttentionRepository::new(self)
    }

    pub(crate) fn outputs(&self) -> impl BuildOutputRepository + '_ {
        SqliteBuildOutputRepository::new(self)
    }

    pub(crate) fn cleanup(&self) -> impl CleanupRepository + '_ {
        SqliteCleanupRepository::new(self)
    }

    pub(crate) fn settings(&self) -> impl ReviewSettingsRepository + '_ {
        SqliteReviewSettingsRepository::new(self)
    }

    pub(crate) fn transaction<T>(
        &self,
        operation: impl FnOnce(&WorktreeReviewTransaction<'_>) -> StorageResult<T>,
    ) -> StorageResult<T> {
        let mut connection = self.lock()?;
        let transaction = connection
            .transaction_with_behavior(TransactionBehavior::Immediate)
            .map_err(sql_error("begin worktree review transaction"))?;
        let owner = WorktreeReviewTransaction { transaction };
        let result = operation(&owner);
        match result {
            Ok(value) => {
                owner
                    .transaction
                    .commit()
                    .map_err(sql_error("commit worktree review transaction"))?;
                Ok(value)
            }
            Err(error) => Err(error),
        }
    }

    fn lock(&self) -> StorageResult<MutexGuard<'_, Connection>> {
        self.connection.lock().map_err(|_| {
            StorageError::new(
                StorageErrorKind::Unavailable,
                "worktree review state lock is unavailable",
            )
        })
    }
}

fn configure_shared_connection(connection: &Connection, require_wal: bool) -> StorageResult<()> {
    connection
        .busy_timeout(CONNECTION_BUSY_TIMEOUT)
        .map_err(sql_error("configure worktree review busy timeout"))?;
    connection
        .pragma_update(None, "foreign_keys", true)
        .map_err(sql_error("enable worktree review foreign keys"))?;
    let journal_mode = enable_wal(connection)?;
    if require_wal && !journal_mode.eq_ignore_ascii_case("wal") {
        return Err(StorageError::new(
            StorageErrorKind::Unavailable,
            format!("worktree review database did not enter WAL mode: {journal_mode}"),
        ));
    }
    Ok(())
}

fn enable_wal(connection: &Connection) -> StorageResult<String> {
    let deadline = Instant::now() + CONNECTION_BUSY_TIMEOUT;
    loop {
        match connection.query_row("PRAGMA journal_mode = WAL", [], |row| {
            row.get::<_, String>(0)
        }) {
            Ok(mode) => return Ok(mode),
            Err(rusqlite::Error::SqliteFailure(failure, _))
                if matches!(
                    failure.code,
                    rusqlite::ErrorCode::DatabaseBusy | rusqlite::ErrorCode::DatabaseLocked
                ) && Instant::now() < deadline =>
            {
                std::thread::sleep(JOURNAL_MODE_RETRY_INTERVAL);
            }
            Err(error) => return Err(sql_error("enable worktree review WAL")(error)),
        }
    }
}

pub(crate) struct WorktreeReviewTransaction<'connection> {
    transaction: rusqlite::Transaction<'connection>,
}

impl WorktreeReviewTransaction<'_> {
    pub(crate) fn repositories(&self) -> impl ReviewRepositoryRepository + '_ {
        SqliteReviewRepositoryRepository::new(self)
    }

    pub(crate) fn selection(&self) -> impl RepositorySelectionRepository + '_ {
        SqliteRepositorySelectionRepository::new(self)
    }

    pub(crate) fn associations(&self) -> impl WorktreeAssociationRepository + '_ {
        SqliteWorktreeAssociationRepository::new(self)
    }

    pub(crate) fn workspaces(&self) -> impl WorkspaceRepository + '_ {
        SqliteWorkspaceRepository::new(self)
    }

    pub(crate) fn builds(&self) -> impl ReviewBuildRepository + '_ {
        SqliteReviewBuildRepository::new(self)
    }

    pub(crate) fn attempts(&self) -> impl OperationAttemptRepository + '_ {
        SqliteOperationAttemptRepository::new(self)
    }

    pub(crate) fn attentions(&self) -> impl BuildAttentionRepository + '_ {
        SqliteBuildAttentionRepository::new(self)
    }

    pub(crate) fn outputs(&self) -> impl BuildOutputRepository + '_ {
        SqliteBuildOutputRepository::new(self)
    }

    pub(crate) fn cleanup(&self) -> impl CleanupRepository + '_ {
        SqliteCleanupRepository::new(self)
    }

    pub(crate) fn settings(&self) -> impl ReviewSettingsRepository + '_ {
        SqliteReviewSettingsRepository::new(self)
    }
}

pub(super) trait ConnectionProvider {
    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> StorageResult<T>,
    ) -> StorageResult<T>;
}

impl ConnectionProvider for WorktreeReviewDatabase {
    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> StorageResult<T>,
    ) -> StorageResult<T> {
        let connection = self.lock()?;
        operation(&connection)
    }
}

impl ConnectionProvider for WorktreeReviewTransaction<'_> {
    fn with_connection<T>(
        &self,
        operation: impl FnOnce(&Connection) -> StorageResult<T>,
    ) -> StorageResult<T> {
        operation(&self.transaction)
    }
}

pub(super) fn sql_error(context: &'static str) -> impl FnOnce(rusqlite::Error) -> StorageError {
    move |error| {
        let kind = match error {
            rusqlite::Error::QueryReturnedNoRows => StorageErrorKind::NotFound,
            rusqlite::Error::SqliteFailure(ref failure, _)
                if failure.code == rusqlite::ErrorCode::ConstraintViolation =>
            {
                StorageErrorKind::Conflict
            }
            _ => StorageErrorKind::Unavailable,
        };
        StorageError::new(kind, format!("{context}: {error}"))
    }
}

pub(super) fn encode_time(value: chrono::DateTime<chrono::Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}

pub(super) fn decode_time(
    value: String,
    field: &str,
) -> StorageResult<chrono::DateTime<chrono::Utc>> {
    chrono::DateTime::parse_from_rfc3339(&value)
        .map(|value| value.with_timezone(&chrono::Utc))
        .map_err(|error| StorageError::corrupt(format!("invalid {field} timestamp: {error}")))
}

pub(super) fn decode_optional_time(
    value: Option<String>,
    field: &str,
) -> StorageResult<Option<chrono::DateTime<chrono::Utc>>> {
    value.map(|value| decode_time(value, field)).transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_review::domain::*;
    use chrono::Utc;

    fn object(value: char) -> GitObjectId {
        GitObjectId::new(value.to_string().repeat(40)).unwrap()
    }

    fn repository(now: chrono::DateTime<Utc>) -> ReviewRepository {
        ReviewRepository {
            id: RepositoryId::new("repository-one").unwrap(),
            label: "Codex Orchestrator".into(),
            anchor_root: "C:/repositories/codex-orchestrator".into(),
            common_directory: "C:/repositories/codex-orchestrator/.git".into(),
            first_seen_at: now,
            last_seen_at: now,
        }
    }

    fn branch(repository: &ReviewRepository, now: chrono::DateTime<Utc>) -> ReviewBranch {
        ReviewBranch {
            repository_id: repository.id.clone(),
            branch_ref: BranchRef::new("refs/heads/feature").unwrap(),
            observed_tip: object('a'),
            observed_at: now,
        }
    }

    fn association(
        repository: &ReviewRepository,
        branch: &ReviewBranch,
        suffix: &str,
        now: chrono::DateTime<Utc>,
    ) -> WorktreeAssociation {
        WorktreeAssociation {
            id: WorktreeAssociationId::new(format!("association-{suffix}")).unwrap(),
            repository_id: repository.id.clone(),
            branch_ref: branch.branch_ref.clone(),
            worktree_id: WorktreeId::new(format!("worktree-{suffix}")).unwrap(),
            location: WorktreeLocation::new(format!("C:/worktrees/{suffix}")).unwrap(),
            provenance: WorktreeAssociationProvenance::GitAttachedBranch,
            baseline: AssociationBaseline {
                object_id: Some(object('a')),
                kind: AssociationBaselineKind::ObservedAtAssociation,
            },
            observed_state: WorktreeObservedState {
                head: object('a'),
                commits_ahead_of_baseline: 0,
                commits_behind_baseline: 0,
                changes: WorktreeChangeSummary::default(),
                detached_head: false,
                reachable_from_associated_branch: true,
                observed_at: now,
            },
            lifecycle: WorktreeAssociationLifecycle::Active,
            associated_at: now,
            updated_at: now,
        }
    }

    fn build_graph(
        transaction: &WorktreeReviewTransaction<'_>,
        now: chrono::DateTime<Utc>,
    ) -> StorageResult<(ReviewBuild, ReviewOperationAttempt, RetainedBuildOutput)> {
        let repository = repository(now);
        let branch = branch(&repository, now);
        transaction.repositories().save_repository(&repository)?;
        transaction.repositories().save_branch(&branch)?;
        let association = association(&repository, &branch, "one", now);
        transaction.associations().save(&association)?;

        let build_id = ReviewBuildId::new("build-one").unwrap();
        let workspace = ReviewWorkspace {
            id: WorkspaceId::new("workspace-one").unwrap(),
            repository_id: repository.id.clone(),
            worktree_id: association.worktree_id.clone(),
            location: WorktreeLocation::new("C:/worktrees/one").unwrap(),
            ownership: WorkspaceOwnership::OwnedBuildWorktree {
                build_id: build_id.clone(),
            },
            lifecycle: WorkspaceLifecycle::Ready,
            created_at: now,
            updated_at: now,
        };
        transaction.workspaces().save(&workspace)?;
        let source = SourceBinding {
            repository_id: repository.id,
            branch_ref: Some(branch.branch_ref),
            selection: ReviewSourceSelection::WorktreeSnapshot {
                association_id: association.id,
                head_object_id: object('a'),
                captured_object_id: object('b'),
                virtual_commit_id: Some(object('b')),
            },
            workspace_id: workspace.id.clone(),
        };
        let build = ReviewBuild {
            id: build_id,
            name: ReviewBuildName::new("Snapshot review").unwrap(),
            source,
            workspace_id: workspace.id,
            retention_key: RetentionKey::new("logical-source-one").unwrap(),
            current_output_id: None,
            lifecycle: BuildLifecycle::Active,
            created_at: now,
            updated_at: now,
        };
        transaction.builds().save(&build)?;
        let attempt = ReviewOperationAttempt {
            id: OperationAttemptId::new("attempt-one").unwrap(),
            build_id: build.id.clone(),
            kind: ReviewOperationKind::Build,
            execution: OperationExecutionState::Completed,
            verdict: OperationVerdict::Passed,
            active_stage: None,
            failure: None,
            requested_at: now,
            started_at: Some(now),
            completed_at: Some(now),
        };
        transaction.attempts().save(&attempt)?;
        let output = RetainedBuildOutput {
            id: BuildOutputId::new("output-one").unwrap(),
            build_id: build.id.clone(),
            attempt_id: attempt.id.clone(),
            storage_key: BuildOutputStorageKey::new(
                "repositories/repository-one/build-output/build-one/attempt-one/output",
            )
            .unwrap(),
            executable_relative_path: ExecutableRelativePath::new("cargo-target/debug/app.exe")
                .unwrap(),
            published_at: now,
        };
        transaction.outputs().save(&output)?;
        transaction
            .builds()
            .set_current_output(&build.id, &output.id, now)?;
        Ok((build, attempt, output))
    }

    #[test]
    fn focused_repositories_share_one_atomic_transaction_owner() {
        let database = WorktreeReviewDatabase::open_in_memory().unwrap();
        let now = Utc::now();
        let (build, attempt, output) = database
            .transaction(|transaction| build_graph(transaction, now))
            .unwrap();

        let loaded_build = database.builds().find(&build.id).unwrap().unwrap();
        assert_eq!(loaded_build.name, build.name);
        assert_eq!(loaded_build.current_output_id, Some(output.id.clone()));
        assert_eq!(
            database.attempts().find(&attempt.id).unwrap(),
            Some(attempt)
        );
        assert_eq!(database.outputs().find(&output.id).unwrap(), Some(output));
    }

    #[test]
    fn build_name_is_immutable_request_data() {
        let database = WorktreeReviewDatabase::open_in_memory().unwrap();
        let now = Utc::now();
        let (mut build, _, _) = database
            .transaction(|transaction| build_graph(transaction, now))
            .unwrap();
        build.name = ReviewBuildName::new("Relabelled build").unwrap();

        let error = database.builds().save(&build).unwrap_err();

        assert_eq!(error.kind, StorageErrorKind::Conflict);
        assert_eq!(
            database.builds().find(&build.id).unwrap().unwrap().name,
            ReviewBuildName::new("Snapshot review").unwrap()
        );
    }

    #[test]
    fn typed_retention_settings_have_one_durable_owner() {
        let database = WorktreeReviewDatabase::open_in_memory().unwrap();
        let settings = ReviewSettings {
            retention_policy: RetentionPolicy::KeepNewestSuccessfulPerLogicalSource { count: 2 },
            updated_at: Utc::now(),
        };

        database.settings().save(&settings).unwrap();

        assert_eq!(database.settings().load().unwrap(), Some(settings));
    }

    #[test]
    fn independent_database_owners_share_committed_product_state() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("worktree-review.sqlite");
        let first = WorktreeReviewDatabase::open(&path).unwrap();
        let second = WorktreeReviewDatabase::open(&path).unwrap();
        let selection = PersistedRepositorySelection {
            repository_id: "repository-one".into(),
        };

        first
            .repositories()
            .save_repository(&ReviewRepository {
                id: RepositoryId::new("repository-one").unwrap(),
                label: "Repository".into(),
                anchor_root: directory.path().join("repository"),
                common_directory: directory.path().join("repository/.git"),
                first_seen_at: Utc::now(),
                last_seen_at: Utc::now(),
            })
            .unwrap();
        first.selection().save(&selection).unwrap();

        assert_eq!(second.selection().load().unwrap(), Some(selection));
        for database in [&first, &second] {
            database
                .with_connection(|connection| {
                    let mode: String = connection
                        .query_row("PRAGMA journal_mode", [], |row| row.get(0))
                        .map_err(sql_error("read worktree review journal mode"))?;
                    let timeout: u64 = connection
                        .query_row("PRAGMA busy_timeout", [], |row| row.get(0))
                        .map_err(sql_error("read worktree review busy timeout"))?;
                    assert_eq!(mode.to_ascii_lowercase(), "wal");
                    assert_eq!(timeout, 5_000);
                    Ok(())
                })
                .unwrap();
        }
    }

    #[test]
    fn concurrent_database_open_serializes_schema_migrations() {
        let directory = tempfile::tempdir().unwrap();
        let path = std::sync::Arc::new(directory.path().join("worktree-review.sqlite"));
        let barrier = std::sync::Arc::new(std::sync::Barrier::new(4));
        let handles = (0..4)
            .map(|_| {
                let path = path.clone();
                let barrier = barrier.clone();
                std::thread::spawn(move || {
                    barrier.wait();
                    WorktreeReviewDatabase::open(path.as_ref()).map(|_| ())
                })
            })
            .collect::<Vec<_>>();

        for handle in handles {
            handle.join().unwrap().unwrap();
        }
        let database = WorktreeReviewDatabase::open(path.as_ref()).unwrap();
        database
            .with_connection(|connection| {
                let count: u32 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM worktree_review_schema_migrations
                         WHERE version IN (1, 2, 3, 4, 5, 6)",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(sql_error("verify serialized worktree review migrations"))?;
                assert_eq!(count, 6);
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn legacy_source_and_artifact_rows_are_preserved_but_not_promoted_to_current_authority() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("worktree-review.sqlite");
        let legacy = Connection::open(&path).unwrap();
        legacy
            .execute_batch(
                "CREATE TABLE review_builds (
                   build_id TEXT PRIMARY KEY, name TEXT NOT NULL, repository_id TEXT NOT NULL,
                   full_branch_ref TEXT NOT NULL, workspace_id TEXT NOT NULL,
                   source_binding_json TEXT NOT NULL, retention_key TEXT NOT NULL,
                   current_artifact_set_id TEXT, lifecycle TEXT NOT NULL,
                   created_at TEXT NOT NULL, updated_at TEXT NOT NULL
                 );
                 CREATE TABLE review_cleanup_jobs (
                   cleanup_job_id TEXT PRIMARY KEY, build_id TEXT NOT NULL, trigger TEXT NOT NULL,
                   eligibility TEXT NOT NULL, state TEXT NOT NULL, created_at TEXT NOT NULL,
                   started_at TEXT, settled_at TEXT
                 );
                 CREATE TABLE verified_artifact_sets (
                   artifact_set_id TEXT PRIMARY KEY, build_id TEXT NOT NULL, attempt_id TEXT NOT NULL,
                   storage_key TEXT NOT NULL, manifest_hash TEXT NOT NULL, verified_at TEXT NOT NULL
                 );
                 INSERT INTO review_builds VALUES (
                   'legacy-build', 'Legacy', 'legacy-repository', 'refs/heads/legacy',
                   'legacy-workspace', '{\"opaqueLegacySource\":true}', 'legacy-source',
                   'legacy-artifacts', 'active', '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
                 );
                 INSERT INTO review_cleanup_jobs VALUES (
                   'legacy-cleanup', 'legacy-build', 'retention_policy', 'eligible', 'planned',
                   '2026-01-01T00:00:00Z', NULL, NULL
                 );
                 INSERT INTO verified_artifact_sets VALUES (
                   'legacy-artifacts', 'legacy-build', 'legacy-attempt', 'legacy/output',
                   'legacy-hash', '2026-01-01T00:00:00Z'
                 );",
            )
            .unwrap();
        drop(legacy);

        let database = WorktreeReviewDatabase::open(&path).unwrap();

        assert!(database
            .builds()
            .find(&ReviewBuildId::new("legacy-build").unwrap())
            .unwrap()
            .is_none());
        assert!(database
            .cleanup()
            .find_job(&CleanupJobId::new("legacy-cleanup").unwrap())
            .unwrap()
            .is_none());
        database
            .with_connection(|connection| {
                let legacy_builds: u32 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM review_builds
                         WHERE build_id = 'legacy-build' AND data_contract_version = 1
                           AND current_artifact_set_id = 'legacy-artifacts'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(sql_error("verify preserved legacy build"))?;
                let legacy_outputs: u32 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM verified_artifact_sets
                         WHERE artifact_set_id = 'legacy-artifacts'",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(sql_error("verify preserved legacy artifacts"))?;
                let legacy_cleanup: u32 = connection
                    .query_row(
                        "SELECT COUNT(*) FROM review_cleanup_jobs
                         WHERE cleanup_job_id = 'legacy-cleanup'
                           AND resource_contract_version = 1",
                        [],
                        |row| row.get(0),
                    )
                    .map_err(sql_error("verify preserved legacy cleanup"))?;
                assert_eq!((legacy_builds, legacy_outputs, legacy_cleanup), (1, 1, 1));
                Ok(())
            })
            .unwrap();
    }

    #[test]
    fn active_build_attentions_are_append_only_and_build_scoped() {
        let database = WorktreeReviewDatabase::open_in_memory().unwrap();
        let now = Utc::now();
        let (build, _, _) = database
            .transaction(|transaction| build_graph(transaction, now))
            .unwrap();
        let active = BuildAttention {
            id: BuildAttentionId::new("attention-active").unwrap(),
            build_id: build.id.clone(),
            category: BuildAttentionCategory::CleanupCoordination,
            summary: "Cleanup coordination could not be recorded.".into(),
            recorded_at: now,
            resolved_at: None,
        };
        let resolved = BuildAttention {
            id: BuildAttentionId::new("attention-resolved").unwrap(),
            build_id: build.id.clone(),
            category: BuildAttentionCategory::CleanupCoordination,
            summary: "Earlier cleanup coordination was resolved.".into(),
            recorded_at: now - chrono::Duration::seconds(1),
            resolved_at: Some(now),
        };

        database.attentions().append(&resolved).unwrap();
        database.attentions().append(&active).unwrap();

        assert_eq!(
            database
                .attentions()
                .find_active_for_build(&build.id)
                .unwrap(),
            vec![active.clone()]
        );
        assert_eq!(
            database.attentions().append(&active).unwrap_err().kind,
            StorageErrorKind::Conflict
        );
    }

    #[test]
    fn multiple_worktrees_for_the_same_branch_and_commit_remain_distinct() {
        let database = WorktreeReviewDatabase::open_in_memory().unwrap();
        let now = Utc::now();
        let repository = repository(now);
        let branch = branch(&repository, now);
        database
            .transaction(|transaction| {
                transaction.repositories().save_repository(&repository)?;
                transaction.repositories().save_branch(&branch)?;
                transaction
                    .associations()
                    .save(&association(&repository, &branch, "one", now))?;
                transaction
                    .associations()
                    .save(&association(&repository, &branch, "two", now))
            })
            .unwrap();

        let associations = database
            .associations()
            .list_for_branch(&repository.id, &branch.branch_ref)
            .unwrap();
        assert_eq!(associations.len(), 2);
        assert_ne!(associations[0].worktree_id, associations[1].worktree_id);
        assert_eq!(
            associations[0].observed_state.head,
            associations[1].observed_state.head
        );
    }

    #[test]
    fn transaction_failure_does_not_leave_partial_authority() {
        let database = WorktreeReviewDatabase::open_in_memory().unwrap();
        let now = Utc::now();
        let repository = repository(now);
        let result: StorageResult<()> = database.transaction(|transaction| {
            transaction.repositories().save_repository(&repository)?;
            Err(StorageError::new(StorageErrorKind::Conflict, "stop"))
        });
        assert!(result.is_err());
        assert!(database
            .repositories()
            .find_repository(&repository.id)
            .unwrap()
            .is_none());
    }

    #[test]
    fn cleanup_receipt_retains_exact_terminal_effects() {
        let database = WorktreeReviewDatabase::open_in_memory().unwrap();
        let now = Utc::now();
        let (build, _, output) = database
            .transaction(|transaction| build_graph(transaction, now))
            .unwrap();
        let resource = CleanupResource::BuildOutput {
            id: CleanupResourceId::new("artifact-resource").unwrap(),
            output_id: output.id,
            storage_key: output.storage_key,
            containment_root: ContainmentRoot::new("appdata-artifacts").unwrap(),
        };
        let job = CleanupJob {
            id: CleanupJobId::new("cleanup-one").unwrap(),
            build_id: build.id.clone(),
            trigger: CleanupTrigger::RetentionPolicy,
            eligibility: CleanupEligibility::Eligible,
            state: CleanupJobState::Planned,
            resources: vec![resource.clone()],
            created_at: now,
            started_at: None,
            settled_at: None,
        };
        database.cleanup().save_job(&job).unwrap();
        assert_eq!(
            database.cleanup().list_for_build(&build.id).unwrap()[0].id,
            job.id
        );
        let receipt = CleanupReceipt {
            job_id: job.id.clone(),
            build_id: build.id,
            effects: vec![CleanupEffect {
                resource_id: resource.id().clone(),
                disposition: CleanupDisposition::Removed,
                detail: None,
                recorded_at: now,
            }],
            completed_at: now,
        };
        database.cleanup().finish(&receipt).unwrap();
        assert_eq!(
            database.cleanup().find_receipt(&job.id).unwrap(),
            Some(receipt)
        );
    }
}
