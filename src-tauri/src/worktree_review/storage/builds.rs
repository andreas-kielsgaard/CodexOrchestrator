use super::{
    decode_time, encode_time, sql_error, ConnectionProvider, StorageError, StorageErrorKind,
    StorageResult,
};
use crate::worktree_review::domain::{
    ArtifactRelativePath, ArtifactSetId, ArtifactStorageKey, BuildLifecycle, ContentHash,
    OperationAttemptId, RepositoryId, RetentionKey, ReviewBuild, ReviewBuildId, ReviewBuildName,
    ReviewWorkspace, SourceBinding, VerifiedArtifactFile, VerifiedArtifactSet, WorkspaceId,
    WorkspaceLifecycle, WorkspaceOwnership, WorktreeId, WorktreeLocation,
};
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};

pub(crate) trait WorkspaceRepository {
    fn save(&self, workspace: &ReviewWorkspace) -> StorageResult<()>;
    fn find(&self, id: &WorkspaceId) -> StorageResult<Option<ReviewWorkspace>>;
    fn list_for_worktree(&self, id: &WorktreeId) -> StorageResult<Vec<ReviewWorkspace>>;
}

pub(crate) trait ReviewBuildRepository {
    fn save(&self, build: &ReviewBuild) -> StorageResult<()>;
    fn find(&self, id: &ReviewBuildId) -> StorageResult<Option<ReviewBuild>>;
    fn list_for_branch(
        &self,
        repository_id: &RepositoryId,
        branch_ref: &crate::worktree_review::domain::BranchRef,
    ) -> StorageResult<Vec<ReviewBuild>>;
    fn set_current_artifact(
        &self,
        build_id: &ReviewBuildId,
        artifact_set_id: &ArtifactSetId,
        updated_at: DateTime<Utc>,
    ) -> StorageResult<()>;
}

pub(crate) trait ArtifactSetRepository {
    fn save_verified(&self, artifacts: &VerifiedArtifactSet) -> StorageResult<()>;
    fn find(&self, id: &ArtifactSetId) -> StorageResult<Option<VerifiedArtifactSet>>;
    fn list_for_build(&self, build_id: &ReviewBuildId) -> StorageResult<Vec<VerifiedArtifactSet>>;
}

pub(crate) struct SqliteWorkspaceRepository<'owner, Owner> {
    owner: &'owner Owner,
}

pub(crate) struct SqliteReviewBuildRepository<'owner, Owner> {
    owner: &'owner Owner,
}

pub(crate) struct SqliteArtifactSetRepository<'owner, Owner> {
    owner: &'owner Owner,
}

impl<'owner, Owner> SqliteWorkspaceRepository<'owner, Owner> {
    pub(super) fn new(owner: &'owner Owner) -> Self {
        Self { owner }
    }
}

impl<'owner, Owner> SqliteReviewBuildRepository<'owner, Owner> {
    pub(super) fn new(owner: &'owner Owner) -> Self {
        Self { owner }
    }
}

impl<'owner, Owner> SqliteArtifactSetRepository<'owner, Owner> {
    pub(super) fn new(owner: &'owner Owner) -> Self {
        Self { owner }
    }
}

impl<Owner: ConnectionProvider> WorkspaceRepository for SqliteWorkspaceRepository<'_, Owner> {
    fn save(&self, workspace: &ReviewWorkspace) -> StorageResult<()> {
        let ownership = serde_json::to_string(&workspace.ownership).map_err(|error| {
            StorageError::corrupt(format!("encode workspace ownership: {error}"))
        })?;
        self.owner.with_connection(|connection| {
            let changed = connection
                .execute(
                    "INSERT INTO review_workspaces(
                       workspace_id, repository_id, worktree_id, observed_location, ownership_json,
                       lifecycle, created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)
                     ON CONFLICT(workspace_id) DO UPDATE SET
                       observed_location = excluded.observed_location,
                       lifecycle = excluded.lifecycle,
                       updated_at = excluded.updated_at
                     WHERE repository_id = excluded.repository_id
                       AND worktree_id = excluded.worktree_id
                       AND ownership_json = excluded.ownership_json",
                    params![
                        workspace.id.as_str(),
                        workspace.repository_id.as_str(),
                        workspace.worktree_id.as_str(),
                        workspace.location.as_str(),
                        ownership,
                        workspace.lifecycle.as_str(),
                        encode_time(workspace.created_at),
                        encode_time(workspace.updated_at),
                    ],
                )
                .map_err(sql_error("save review workspace"))?;
            if changed == 0 {
                return Err(StorageError::new(
                    StorageErrorKind::Conflict,
                    "workspace identity or ownership conflicts with its durable record",
                ));
            }
            Ok(())
        })
    }

    fn find(&self, id: &WorkspaceId) -> StorageResult<Option<ReviewWorkspace>> {
        self.owner.with_connection(|connection| {
            let raw = connection
                .query_row(
                    "SELECT workspace_id, repository_id, worktree_id, observed_location,
                            ownership_json, lifecycle, created_at, updated_at
                     FROM review_workspaces WHERE workspace_id = ?1",
                    [id.as_str()],
                    decode_workspace_row,
                )
                .optional()
                .map_err(sql_error("load review workspace"))?;
            raw.map(decode_workspace).transpose()
        })
    }

    fn list_for_worktree(&self, id: &WorktreeId) -> StorageResult<Vec<ReviewWorkspace>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT workspace_id, repository_id, worktree_id, observed_location,
                            ownership_json, lifecycle, created_at, updated_at
                     FROM review_workspaces
                     WHERE worktree_id = ?1
                     ORDER BY created_at ASC, workspace_id ASC",
                )
                .map_err(sql_error("prepare review workspaces by worktree"))?;
            let rows = statement
                .query_map([id.as_str()], decode_workspace_row)
                .map_err(sql_error("query review workspaces by worktree"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error("load review workspaces by worktree"))?;
            rows.into_iter().map(decode_workspace).collect()
        })
    }
}

impl<Owner: ConnectionProvider> ReviewBuildRepository for SqliteReviewBuildRepository<'_, Owner> {
    fn save(&self, build: &ReviewBuild) -> StorageResult<()> {
        build
            .validate()
            .map_err(|error| StorageError::corrupt(error.to_string()))?;
        let source = serde_json::to_string(&build.source)
            .map_err(|error| StorageError::corrupt(format!("encode build source: {error}")))?;
        self.owner.with_connection(|connection| {
            let changed = connection
                .execute(
                    "INSERT INTO review_builds(
                       build_id, name, repository_id, full_branch_ref, workspace_id,
                       source_binding_json, retention_key, current_artifact_set_id, lifecycle,
                       created_at, updated_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)
                     ON CONFLICT(build_id) DO UPDATE SET
                       current_artifact_set_id = excluded.current_artifact_set_id,
                       lifecycle = excluded.lifecycle,
                       updated_at = excluded.updated_at
                     WHERE name = excluded.name
                       AND repository_id = excluded.repository_id
                       AND full_branch_ref = excluded.full_branch_ref
                       AND workspace_id = excluded.workspace_id
                       AND source_binding_json = excluded.source_binding_json
                       AND retention_key = excluded.retention_key",
                    params![
                        build.id.as_str(),
                        build.name.as_str(),
                        build.source.repository_id.as_str(),
                        build.source.branch_ref.as_str(),
                        build.workspace_id.as_str(),
                        source,
                        build.retention_key.as_str(),
                        build
                            .current_artifact_set_id
                            .as_ref()
                            .map(ArtifactSetId::as_str),
                        build.lifecycle.as_str(),
                        encode_time(build.created_at),
                        encode_time(build.updated_at),
                    ],
                )
                .map_err(sql_error("save review build"))?;
            if changed == 0 {
                return Err(StorageError::new(
                    StorageErrorKind::Conflict,
                    "build identity conflicts with its immutable source binding",
                ));
            }
            Ok(())
        })
    }

    fn find(&self, id: &ReviewBuildId) -> StorageResult<Option<ReviewBuild>> {
        self.owner.with_connection(|connection| {
            let raw = connection
                .query_row(
                    "SELECT build_id, workspace_id, source_binding_json, retention_key,
                            name, current_artifact_set_id, lifecycle, created_at, updated_at
                     FROM review_builds WHERE build_id = ?1",
                    [id.as_str()],
                    decode_build_row,
                )
                .optional()
                .map_err(sql_error("load review build"))?;
            raw.map(decode_build).transpose()
        })
    }

    fn list_for_branch(
        &self,
        repository_id: &RepositoryId,
        branch_ref: &crate::worktree_review::domain::BranchRef,
    ) -> StorageResult<Vec<ReviewBuild>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT build_id, workspace_id, source_binding_json, retention_key,
                            name, current_artifact_set_id, lifecycle, created_at, updated_at
                     FROM review_builds
                     WHERE repository_id = ?1 AND full_branch_ref = ?2
                     ORDER BY created_at DESC, build_id",
                )
                .map_err(sql_error("prepare review build query"))?;
            let rows = statement
                .query_map(
                    params![repository_id.as_str(), branch_ref.as_str()],
                    decode_build_row,
                )
                .map_err(sql_error("query review builds"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error("read review builds"))?;
            rows.into_iter().map(decode_build).collect()
        })
    }

    fn set_current_artifact(
        &self,
        build_id: &ReviewBuildId,
        artifact_set_id: &ArtifactSetId,
        updated_at: DateTime<Utc>,
    ) -> StorageResult<()> {
        self.owner.with_connection(|connection| {
            let changed = connection
                .execute(
                    "UPDATE review_builds
                     SET current_artifact_set_id = ?2, updated_at = ?3
                     WHERE build_id = ?1
                       AND EXISTS (
                         SELECT 1 FROM verified_artifact_sets
                         WHERE artifact_set_id = ?2 AND build_id = ?1
                       )",
                    params![
                        build_id.as_str(),
                        artifact_set_id.as_str(),
                        encode_time(updated_at)
                    ],
                )
                .map_err(sql_error("promote current artifact set"))?;
            if changed != 1 {
                return Err(StorageError::new(
                    StorageErrorKind::Conflict,
                    "artifact set is not verified for the selected build",
                ));
            }
            Ok(())
        })
    }
}

impl<Owner: ConnectionProvider> ArtifactSetRepository for SqliteArtifactSetRepository<'_, Owner> {
    fn save_verified(&self, artifacts: &VerifiedArtifactSet) -> StorageResult<()> {
        artifacts
            .validate()
            .map_err(|error| StorageError::corrupt(error.to_string()))?;
        self.owner.with_connection(|connection| {
            connection
                .execute_batch("SAVEPOINT save_verified_artifact_set")
                .map_err(sql_error("begin artifact set savepoint"))?;
            let result = save_artifact_set(connection, artifacts);
            match result {
                Ok(()) => connection
                    .execute_batch("RELEASE save_verified_artifact_set")
                    .map_err(sql_error("commit artifact set savepoint")),
                Err(error) => {
                    let _ = connection.execute_batch(
                        "ROLLBACK TO save_verified_artifact_set; RELEASE save_verified_artifact_set",
                    );
                    Err(error)
                }
            }
        })
    }

    fn find(&self, id: &ArtifactSetId) -> StorageResult<Option<VerifiedArtifactSet>> {
        self.owner
            .with_connection(|connection| load_artifact_set(connection, id))
    }

    fn list_for_build(&self, build_id: &ReviewBuildId) -> StorageResult<Vec<VerifiedArtifactSet>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT artifact_set_id FROM verified_artifact_sets
                     WHERE build_id = ?1 ORDER BY verified_at DESC, artifact_set_id",
                )
                .map_err(sql_error("prepare artifact set query"))?;
            let ids = statement
                .query_map([build_id.as_str()], |row| row.get::<_, String>(0))
                .map_err(sql_error("query artifact sets"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error("read artifact set IDs"))?;
            ids.into_iter()
                .map(|id| {
                    let id = ArtifactSetId::new(id)
                        .map_err(|error| StorageError::corrupt(error.to_string()))?;
                    load_artifact_set(connection, &id)?.ok_or_else(|| {
                        StorageError::corrupt("artifact set disappeared during durable query")
                    })
                })
                .collect()
        })
    }
}

fn save_artifact_set(
    connection: &rusqlite::Connection,
    artifacts: &VerifiedArtifactSet,
) -> StorageResult<()> {
    let changed = connection
        .execute(
            "INSERT INTO verified_artifact_sets(
               artifact_set_id, build_id, attempt_id, storage_key, manifest_hash, verified_at
             )
             SELECT ?1, ?2, ?3, ?4, ?5, ?6
             WHERE EXISTS (
               SELECT 1 FROM review_operation_attempts
               WHERE attempt_id = ?3 AND build_id = ?2
                 AND operation_kind = 'build'
                 AND execution_state = 'completed' AND verdict = 'passed'
             )",
            params![
                artifacts.id.as_str(),
                artifacts.build_id.as_str(),
                artifacts.attempt_id.as_str(),
                artifacts.storage_key.as_str(),
                artifacts.manifest_hash.as_str(),
                encode_time(artifacts.verified_at),
            ],
        )
        .map_err(sql_error("save verified artifact set"))?;
    if changed != 1 {
        return Err(StorageError::new(
            StorageErrorKind::Conflict,
            "artifact set requires a passed build attempt for the same build",
        ));
    }
    for (ordinal, file) in artifacts.files.iter().enumerate() {
        let bytes = i64::try_from(file.bytes)
            .map_err(|_| StorageError::corrupt("artifact byte size exceeds SQLite range"))?;
        connection
            .execute(
                "INSERT INTO verified_artifact_files(
                   artifact_set_id, ordinal, relative_path, content_hash, bytes
                 ) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![
                    artifacts.id.as_str(),
                    ordinal as i64,
                    file.relative_path.as_str(),
                    file.content_hash.as_str(),
                    bytes,
                ],
            )
            .map_err(sql_error("save verified artifact file"))?;
    }
    Ok(())
}

type WorkspaceRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    String,
);
type BuildRow = (
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    String,
    String,
);

fn decode_workspace_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<WorkspaceRow> {
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
}

fn decode_workspace(raw: WorkspaceRow) -> StorageResult<ReviewWorkspace> {
    let ownership: WorkspaceOwnership = serde_json::from_str(&raw.4)
        .map_err(|error| StorageError::corrupt(format!("invalid workspace ownership: {error}")))?;
    Ok(ReviewWorkspace {
        id: WorkspaceId::new(raw.0).map_err(|error| StorageError::corrupt(error.to_string()))?,
        repository_id: RepositoryId::new(raw.1)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        worktree_id: WorktreeId::new(raw.2)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        location: WorktreeLocation::new(raw.3)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        ownership,
        lifecycle: WorkspaceLifecycle::parse(&raw.5)
            .ok_or_else(|| StorageError::corrupt("unknown workspace lifecycle"))?,
        created_at: decode_time(raw.6, "workspace creation")?,
        updated_at: decode_time(raw.7, "workspace update")?,
    })
}

fn decode_build_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BuildRow> {
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
    ))
}

fn decode_build(raw: BuildRow) -> StorageResult<ReviewBuild> {
    let source: SourceBinding = serde_json::from_str(&raw.2)
        .map_err(|error| StorageError::corrupt(format!("invalid build source binding: {error}")))?;
    let build = ReviewBuild {
        id: ReviewBuildId::new(raw.0).map_err(|error| StorageError::corrupt(error.to_string()))?,
        workspace_id: WorkspaceId::new(raw.1)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        source,
        retention_key: RetentionKey::new(raw.3)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        name: ReviewBuildName::new(raw.4)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        current_artifact_set_id: raw
            .5
            .map(ArtifactSetId::new)
            .transpose()
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        lifecycle: BuildLifecycle::parse(&raw.6)
            .ok_or_else(|| StorageError::corrupt("unknown build lifecycle"))?,
        created_at: decode_time(raw.7, "build creation")?,
        updated_at: decode_time(raw.8, "build update")?,
    };
    build
        .validate()
        .map_err(|error| StorageError::corrupt(error.to_string()))?;
    Ok(build)
}

fn load_artifact_set(
    connection: &rusqlite::Connection,
    id: &ArtifactSetId,
) -> StorageResult<Option<VerifiedArtifactSet>> {
    let raw = connection
        .query_row(
            "SELECT artifact_set_id, build_id, attempt_id, storage_key, manifest_hash, verified_at
             FROM verified_artifact_sets WHERE artifact_set_id = ?1",
            [id.as_str()],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                    row.get::<_, String>(5)?,
                ))
            },
        )
        .optional()
        .map_err(sql_error("load verified artifact set"))?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let mut statement = connection
        .prepare(
            "SELECT relative_path, content_hash, bytes FROM verified_artifact_files
             WHERE artifact_set_id = ?1 ORDER BY ordinal",
        )
        .map_err(sql_error("prepare artifact manifest query"))?;
    let file_rows = statement
        .query_map([id.as_str()], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(sql_error("query artifact manifest"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read artifact manifest"))?;
    let files = file_rows
        .into_iter()
        .map(|file| {
            Ok(VerifiedArtifactFile {
                relative_path: ArtifactRelativePath::new(file.0)
                    .map_err(|error| StorageError::corrupt(error.to_string()))?,
                content_hash: ContentHash::new(file.1)
                    .map_err(|error| StorageError::corrupt(error.to_string()))?,
                bytes: u64::try_from(file.2)
                    .map_err(|_| StorageError::corrupt("invalid artifact byte size"))?,
            })
        })
        .collect::<StorageResult<Vec<_>>>()?;
    let artifacts = VerifiedArtifactSet {
        id: ArtifactSetId::new(raw.0).map_err(|error| StorageError::corrupt(error.to_string()))?,
        build_id: ReviewBuildId::new(raw.1)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        attempt_id: OperationAttemptId::new(raw.2)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        storage_key: ArtifactStorageKey::new(raw.3)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        manifest_hash: ContentHash::new(raw.4)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        files,
        verified_at: decode_time(raw.5, "artifact verification")?,
    };
    artifacts
        .validate()
        .map_err(|error| StorageError::corrupt(error.to_string()))?;
    Ok(Some(artifacts))
}
