use super::{decode_time, encode_time, sql_error, ConnectionProvider, StorageError, StorageResult};
use crate::worktree_review::domain::{
    AssociationBaseline, AssociationBaselineKind, BranchRef, GitObjectId, RepositoryId,
    WorktreeAssociation, WorktreeAssociationId, WorktreeAssociationLifecycle,
    WorktreeAssociationProvenance, WorktreeChangeSummary, WorktreeId, WorktreeLocation,
    WorktreeObservedState,
};
use rusqlite::{params, OptionalExtension};

pub(crate) trait WorktreeAssociationRepository {
    fn save(&self, association: &WorktreeAssociation) -> StorageResult<()>;
    fn find(&self, id: &WorktreeAssociationId) -> StorageResult<Option<WorktreeAssociation>>;
    fn list_for_branch(
        &self,
        repository_id: &RepositoryId,
        branch_ref: &BranchRef,
    ) -> StorageResult<Vec<WorktreeAssociation>>;
    fn list_for_worktree(
        &self,
        repository_id: &RepositoryId,
        worktree_id: &WorktreeId,
    ) -> StorageResult<Vec<WorktreeAssociation>>;
}

pub(crate) struct SqliteWorktreeAssociationRepository<'owner, Owner> {
    owner: &'owner Owner,
}

impl<'owner, Owner> SqliteWorktreeAssociationRepository<'owner, Owner> {
    pub(super) fn new(owner: &'owner Owner) -> Self {
        Self { owner }
    }
}

impl<Owner: ConnectionProvider> WorktreeAssociationRepository
    for SqliteWorktreeAssociationRepository<'_, Owner>
{
    fn save(&self, association: &WorktreeAssociation) -> StorageResult<()> {
        self.owner.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO review_worktree_associations(
                       association_id, repository_id, full_branch_ref, worktree_id,
                       observed_location, provenance, baseline_kind, baseline_object_id,
                       observed_head, commits_ahead, commits_behind, staged_paths, unstaged_paths,
                       untracked_paths, detached_head, branch_reachable, observed_at, lifecycle,
                       associated_at, updated_at
                     ) VALUES (
                       ?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11, ?12, ?13, ?14, ?15,
                       ?16, ?17, ?18, ?19, ?20
                     )
                     ON CONFLICT(association_id) DO UPDATE SET
                       observed_location = excluded.observed_location,
                       observed_head = excluded.observed_head,
                       commits_ahead = excluded.commits_ahead,
                       commits_behind = excluded.commits_behind,
                       staged_paths = excluded.staged_paths,
                       unstaged_paths = excluded.unstaged_paths,
                       untracked_paths = excluded.untracked_paths,
                       detached_head = excluded.detached_head,
                       branch_reachable = excluded.branch_reachable,
                       observed_at = excluded.observed_at,
                       lifecycle = excluded.lifecycle,
                       updated_at = excluded.updated_at",
                    params![
                        association.id.as_str(),
                        association.repository_id.as_str(),
                        association.branch_ref.as_str(),
                        association.worktree_id.as_str(),
                        association.location.as_str(),
                        association.provenance.as_str(),
                        association.baseline.kind.as_str(),
                        association
                            .baseline
                            .object_id
                            .as_ref()
                            .map(GitObjectId::as_str),
                        association.observed_state.head.as_str(),
                        association.observed_state.commits_ahead_of_baseline,
                        association.observed_state.commits_behind_baseline,
                        association.observed_state.changes.staged_paths,
                        association.observed_state.changes.unstaged_paths,
                        association.observed_state.changes.untracked_paths,
                        association.observed_state.detached_head,
                        association.observed_state.reachable_from_associated_branch,
                        encode_time(association.observed_state.observed_at),
                        association.lifecycle.as_str(),
                        encode_time(association.associated_at),
                        encode_time(association.updated_at),
                    ],
                )
                .map_err(sql_error("save worktree association"))?;
            Ok(())
        })
    }

    fn find(&self, id: &WorktreeAssociationId) -> StorageResult<Option<WorktreeAssociation>> {
        self.owner.with_connection(|connection| {
            let raw = connection
                .query_row(
                    &format!("{SELECT_ASSOCIATION} WHERE association_id = ?1"),
                    [id.as_str()],
                    decode_row,
                )
                .optional()
                .map_err(sql_error("load worktree association"))?;
            raw.map(decode_association).transpose()
        })
    }

    fn list_for_branch(
        &self,
        repository_id: &RepositoryId,
        branch_ref: &BranchRef,
    ) -> StorageResult<Vec<WorktreeAssociation>> {
        self.owner.with_connection(|connection| {
            query_many(
                connection,
                &format!(
                    "{SELECT_ASSOCIATION} WHERE repository_id = ?1 AND full_branch_ref = ?2
                     ORDER BY observed_location, association_id"
                ),
                params![repository_id.as_str(), branch_ref.as_str()],
            )
        })
    }

    fn list_for_worktree(
        &self,
        repository_id: &RepositoryId,
        worktree_id: &WorktreeId,
    ) -> StorageResult<Vec<WorktreeAssociation>> {
        self.owner.with_connection(|connection| {
            query_many(
                connection,
                &format!(
                    "{SELECT_ASSOCIATION} WHERE repository_id = ?1 AND worktree_id = ?2
                     ORDER BY full_branch_ref, association_id"
                ),
                params![repository_id.as_str(), worktree_id.as_str()],
            )
        })
    }
}

const SELECT_ASSOCIATION: &str =
    "SELECT association_id, repository_id, full_branch_ref, worktree_id, observed_location,
            provenance, baseline_kind, baseline_object_id, observed_head, commits_ahead,
            commits_behind, staged_paths, unstaged_paths, untracked_paths, detached_head,
            branch_reachable, observed_at, lifecycle, associated_at, updated_at
     FROM review_worktree_associations";

type AssociationRow = (
    String,
    String,
    String,
    String,
    String,
    String,
    String,
    Option<String>,
    String,
    i64,
    i64,
    i64,
    i64,
    i64,
    bool,
    bool,
    String,
    String,
    String,
    String,
);

fn decode_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<AssociationRow> {
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
        row.get(13)?,
        row.get(14)?,
        row.get(15)?,
        row.get(16)?,
        row.get(17)?,
        row.get(18)?,
        row.get(19)?,
    ))
}

fn query_many<P: rusqlite::Params>(
    connection: &rusqlite::Connection,
    sql: &str,
    parameters: P,
) -> StorageResult<Vec<WorktreeAssociation>> {
    let mut statement = connection
        .prepare(sql)
        .map_err(sql_error("prepare worktree association query"))?;
    let rows = statement
        .query_map(parameters, decode_row)
        .map_err(sql_error("query worktree associations"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(sql_error("read worktree associations"))?;
    rows.into_iter().map(decode_association).collect()
}

fn decode_association(raw: AssociationRow) -> StorageResult<WorktreeAssociation> {
    fn nonnegative(value: i64, field: &str) -> StorageResult<u32> {
        u32::try_from(value).map_err(|_| StorageError::corrupt(format!("invalid {field} count")))
    }
    Ok(WorktreeAssociation {
        id: WorktreeAssociationId::new(raw.0)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        repository_id: RepositoryId::new(raw.1)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        branch_ref: BranchRef::new(raw.2)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        worktree_id: WorktreeId::new(raw.3)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        location: WorktreeLocation::new(raw.4)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        provenance: WorktreeAssociationProvenance::parse(&raw.5)
            .ok_or_else(|| StorageError::corrupt("unknown worktree association provenance"))?,
        baseline: AssociationBaseline {
            kind: AssociationBaselineKind::parse(&raw.6)
                .ok_or_else(|| StorageError::corrupt("unknown association baseline kind"))?,
            object_id: raw
                .7
                .map(GitObjectId::new)
                .transpose()
                .map_err(|error| StorageError::corrupt(error.to_string()))?,
        },
        observed_state: WorktreeObservedState {
            head: GitObjectId::new(raw.8)
                .map_err(|error| StorageError::corrupt(error.to_string()))?,
            commits_ahead_of_baseline: nonnegative(raw.9, "commits ahead")?,
            commits_behind_baseline: nonnegative(raw.10, "commits behind")?,
            changes: WorktreeChangeSummary {
                staged_paths: nonnegative(raw.11, "staged paths")?,
                unstaged_paths: nonnegative(raw.12, "unstaged paths")?,
                untracked_paths: nonnegative(raw.13, "untracked paths")?,
            },
            detached_head: raw.14,
            reachable_from_associated_branch: raw.15,
            observed_at: decode_time(raw.16, "worktree observation")?,
        },
        lifecycle: WorktreeAssociationLifecycle::parse(&raw.17)
            .ok_or_else(|| StorageError::corrupt("unknown worktree association lifecycle"))?,
        associated_at: decode_time(raw.18, "worktree association")?,
        updated_at: decode_time(raw.19, "worktree association update")?,
    })
}
