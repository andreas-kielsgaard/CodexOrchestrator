use super::{
    decode_time, encode_time, sql_error, ConnectionProvider, StorageError, StorageErrorKind,
    StorageResult,
};
use crate::worktree_review::domain::{
    BuildLifecycle, BuildOutputId, BuildOutputStorageKey, ExecutableRelativePath,
    OperationAttemptId, RepositoryId, RetainedBuildOutput, RetentionKey, ReviewBuild,
    ReviewBuildId, ReviewBuildName, ReviewWorkspace, SourceBinding, WorkspaceId,
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
    fn list_for_repository(&self, id: &RepositoryId) -> StorageResult<Vec<ReviewBuild>>;
    fn save(&self, build: &ReviewBuild) -> StorageResult<()>;
    fn find(&self, id: &ReviewBuildId) -> StorageResult<Option<ReviewBuild>>;
    fn list_for_branch(
        &self,
        repository_id: &RepositoryId,
        branch_ref: &Option<crate::worktree_review::domain::BranchRef>,
    ) -> StorageResult<Vec<ReviewBuild>>;
    fn set_current_output(
        &self,
        build_id: &ReviewBuildId,
        output_id: &BuildOutputId,
        updated_at: DateTime<Utc>,
    ) -> StorageResult<()>;
}

pub(crate) trait BuildOutputRepository {
    fn save(&self, output: &RetainedBuildOutput) -> StorageResult<()>;
    fn find(&self, id: &BuildOutputId) -> StorageResult<Option<RetainedBuildOutput>>;
    fn list_for_build(&self, build_id: &ReviewBuildId) -> StorageResult<Vec<RetainedBuildOutput>>;
}

pub(crate) struct SqliteWorkspaceRepository<'owner, Owner> {
    owner: &'owner Owner,
}

pub(crate) struct SqliteReviewBuildRepository<'owner, Owner> {
    owner: &'owner Owner,
}

pub(crate) struct SqliteBuildOutputRepository<'owner, Owner> {
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

impl<'owner, Owner> SqliteBuildOutputRepository<'owner, Owner> {
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
    fn list_for_repository(&self, id: &RepositoryId) -> StorageResult<Vec<ReviewBuild>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection.prepare("SELECT build_id, workspace_id, source_binding_json, retention_key, name, current_output_id, lifecycle, created_at, updated_at FROM review_builds WHERE repository_id = ?1 AND data_contract_version = 2 ORDER BY created_at DESC, build_id")
                .map_err(sql_error("prepare repository build query"))?;
            let rows = statement.query_map([id.as_str()], decode_build_row).map_err(sql_error("query repository builds"))?
                .collect::<Result<Vec<_>, _>>().map_err(sql_error("read repository builds"))?;
            rows.into_iter().map(decode_build).collect()
        })
    }

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
                       source_binding_json, retention_key, current_output_id, lifecycle,
                       created_at, updated_at, data_contract_version
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, 2)
                     ON CONFLICT(build_id) DO UPDATE SET
                       current_output_id = excluded.current_output_id,
                       lifecycle = excluded.lifecycle,
                       updated_at = excluded.updated_at
                     WHERE name = excluded.name
                       AND data_contract_version = 2
                       AND repository_id = excluded.repository_id
                       AND full_branch_ref IS excluded.full_branch_ref
                       AND workspace_id = excluded.workspace_id
                       AND source_binding_json = excluded.source_binding_json
                       AND retention_key = excluded.retention_key",
                    params![
                        build.id.as_str(),
                        build.name.as_str(),
                        build.source.repository_id.as_str(),
                        build
                            .source
                            .branch_ref
                            .as_ref()
                            .map(|reference| reference.as_str()),
                        build.workspace_id.as_str(),
                        source,
                        build.retention_key.as_str(),
                        build.current_output_id.as_ref().map(BuildOutputId::as_str),
                        build.lifecycle.as_str(),
                        encode_time(build.created_at),
                        encode_time(build.updated_at),
                    ],
                )
                .map_err(sql_error("save review build"))?;
            if changed == 0 {
                return Err(StorageError::new(
                    StorageErrorKind::Conflict,
                    "build identity conflicts with its recorded source binding",
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
                            name, current_output_id, lifecycle, created_at, updated_at
                     FROM review_builds WHERE build_id = ?1 AND data_contract_version = 2",
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
        branch_ref: &Option<crate::worktree_review::domain::BranchRef>,
    ) -> StorageResult<Vec<ReviewBuild>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT build_id, workspace_id, source_binding_json, retention_key,
                            name, current_output_id, lifecycle, created_at, updated_at
                     FROM review_builds
                     WHERE repository_id = ?1 AND full_branch_ref IS ?2
                       AND data_contract_version = 2
                     ORDER BY created_at DESC, build_id",
                )
                .map_err(sql_error("prepare review build query"))?;
            let rows = statement
                .query_map(
                    params![
                        repository_id.as_str(),
                        branch_ref.as_ref().map(|reference| reference.as_str())
                    ],
                    decode_build_row,
                )
                .map_err(sql_error("query review builds"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error("read review builds"))?;
            rows.into_iter().map(decode_build).collect()
        })
    }

    fn set_current_output(
        &self,
        build_id: &ReviewBuildId,
        output_id: &BuildOutputId,
        updated_at: DateTime<Utc>,
    ) -> StorageResult<()> {
        self.owner.with_connection(|connection| {
            let changed = connection
                .execute(
                    "UPDATE review_builds
                     SET current_output_id = ?2, updated_at = ?3
                     WHERE build_id = ?1 AND data_contract_version = 2
                       AND EXISTS (
                         SELECT 1 FROM retained_build_outputs
                         WHERE output_id = ?2 AND build_id = ?1
                       )",
                    params![
                        build_id.as_str(),
                        output_id.as_str(),
                        encode_time(updated_at)
                    ],
                )
                .map_err(sql_error("select current retained build output"))?;
            if changed != 1 {
                return Err(StorageError::new(
                    StorageErrorKind::Conflict,
                    "build output is not retained for the selected build",
                ));
            }
            Ok(())
        })
    }
}

impl<Owner: ConnectionProvider> BuildOutputRepository for SqliteBuildOutputRepository<'_, Owner> {
    fn save(&self, output: &RetainedBuildOutput) -> StorageResult<()> {
        output
            .validate()
            .map_err(|error| StorageError::corrupt(error.to_string()))?;
        self.owner.with_connection(|connection| {
            let changed = connection
                .execute(
                    "INSERT INTO retained_build_outputs(
                       output_id, build_id, attempt_id, storage_key,
                       executable_relative_path, published_at
                     )
                     SELECT ?1, ?2, ?3, ?4, ?5, ?6
                     WHERE EXISTS (
                       SELECT 1 FROM review_operation_attempts
                       WHERE attempt_id = ?3 AND build_id = ?2
                         AND operation_kind = 'build'
                         AND execution_state = 'completed' AND verdict = 'passed'
                     ) AND EXISTS (
                       SELECT 1 FROM review_builds
                       WHERE build_id = ?2 AND data_contract_version = 2
                     )",
                    params![
                        output.id.as_str(),
                        output.build_id.as_str(),
                        output.attempt_id.as_str(),
                        output.storage_key.as_str(),
                        output.executable_relative_path.as_str(),
                        encode_time(output.published_at),
                    ],
                )
                .map_err(sql_error("save retained build output"))?;
            if changed != 1 {
                return Err(StorageError::new(
                    StorageErrorKind::Conflict,
                    "retained build output requires a succeeded build attempt for the same build",
                ));
            }
            Ok(())
        })
    }

    fn find(&self, id: &BuildOutputId) -> StorageResult<Option<RetainedBuildOutput>> {
        self.owner
            .with_connection(|connection| load_build_output(connection, id))
    }

    fn list_for_build(&self, build_id: &ReviewBuildId) -> StorageResult<Vec<RetainedBuildOutput>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT output_id FROM retained_build_outputs
                     WHERE build_id = ?1 ORDER BY published_at DESC, output_id",
                )
                .map_err(sql_error("prepare retained build output query"))?;
            let ids = statement
                .query_map([build_id.as_str()], |row| row.get::<_, String>(0))
                .map_err(sql_error("query retained build outputs"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error("read retained build output IDs"))?;
            ids.into_iter()
                .map(|id| {
                    let id = BuildOutputId::new(id)
                        .map_err(|error| StorageError::corrupt(error.to_string()))?;
                    load_build_output(connection, &id)?.ok_or_else(|| {
                        StorageError::corrupt("build output disappeared during durable query")
                    })
                })
                .collect()
        })
    }
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
        current_output_id: raw
            .5
            .map(BuildOutputId::new)
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

fn load_build_output(
    connection: &rusqlite::Connection,
    id: &BuildOutputId,
) -> StorageResult<Option<RetainedBuildOutput>> {
    let raw = connection
        .query_row(
            "SELECT output_id, build_id, attempt_id, storage_key,
                    executable_relative_path, published_at
             FROM retained_build_outputs WHERE output_id = ?1",
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
        .map_err(sql_error("load retained build output"))?;
    let Some(raw) = raw else {
        return Ok(None);
    };
    let output = RetainedBuildOutput {
        id: BuildOutputId::new(raw.0).map_err(|error| StorageError::corrupt(error.to_string()))?,
        build_id: ReviewBuildId::new(raw.1)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        attempt_id: OperationAttemptId::new(raw.2)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        storage_key: BuildOutputStorageKey::new(raw.3)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        executable_relative_path: ExecutableRelativePath::new(raw.4)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        published_at: decode_time(raw.5, "build output publication")?,
    };
    output
        .validate()
        .map_err(|error| StorageError::corrupt(error.to_string()))?;
    Ok(Some(output))
}
