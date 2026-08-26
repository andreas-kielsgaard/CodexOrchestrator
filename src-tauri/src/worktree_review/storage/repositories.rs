use super::{decode_time, encode_time, sql_error, ConnectionProvider, StorageError, StorageResult};
use crate::worktree_review::domain::{
    BranchRef, GitObjectId, RepositoryId, ReviewBranch, ReviewRepository,
};
use rusqlite::{params, OptionalExtension};

pub(crate) trait ReviewRepositoryRepository {
    fn save_repository(&self, repository: &ReviewRepository) -> StorageResult<()>;
    fn find_repository(&self, id: &RepositoryId) -> StorageResult<Option<ReviewRepository>>;
    fn save_branch(&self, branch: &ReviewBranch) -> StorageResult<()>;
    fn find_branch(
        &self,
        repository_id: &RepositoryId,
        branch_ref: &BranchRef,
    ) -> StorageResult<Option<ReviewBranch>>;
    fn list_branches(&self, repository_id: &RepositoryId) -> StorageResult<Vec<ReviewBranch>>;
}

pub(crate) struct SqliteReviewRepositoryRepository<'owner, Owner> {
    owner: &'owner Owner,
}

impl<'owner, Owner> SqliteReviewRepositoryRepository<'owner, Owner> {
    pub(super) fn new(owner: &'owner Owner) -> Self {
        Self { owner }
    }
}

impl<Owner: ConnectionProvider> ReviewRepositoryRepository
    for SqliteReviewRepositoryRepository<'_, Owner>
{
    fn save_repository(&self, repository: &ReviewRepository) -> StorageResult<()> {
        if repository.label.trim().is_empty() {
            return Err(StorageError::corrupt("repository label must not be empty"));
        }
        self.owner.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO review_repositories(
                       repository_id, label, first_seen_at, last_seen_at
                     ) VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(repository_id) DO UPDATE SET
                       label = excluded.label,
                       last_seen_at = excluded.last_seen_at",
                    params![
                        repository.id.as_str(),
                        repository.label,
                        encode_time(repository.first_seen_at),
                        encode_time(repository.last_seen_at),
                    ],
                )
                .map_err(sql_error("save review repository"))?;
            Ok(())
        })
    }

    fn find_repository(&self, id: &RepositoryId) -> StorageResult<Option<ReviewRepository>> {
        self.owner.with_connection(|connection| {
            let raw = connection
                .query_row(
                    "SELECT repository_id, label, first_seen_at, last_seen_at
                     FROM review_repositories WHERE repository_id = ?1",
                    [id.as_str()],
                    |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                        ))
                    },
                )
                .optional()
                .map_err(sql_error("load review repository"))?;
            raw.map(decode_repository).transpose()
        })
    }

    fn save_branch(&self, branch: &ReviewBranch) -> StorageResult<()> {
        self.owner.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO review_branches(
                       repository_id, full_ref, observed_tip, observed_at
                     ) VALUES (?1, ?2, ?3, ?4)
                     ON CONFLICT(repository_id, full_ref) DO UPDATE SET
                       observed_tip = excluded.observed_tip,
                       observed_at = excluded.observed_at",
                    params![
                        branch.repository_id.as_str(),
                        branch.branch_ref.as_str(),
                        branch.observed_tip.as_str(),
                        encode_time(branch.observed_at),
                    ],
                )
                .map_err(sql_error("save review branch"))?;
            Ok(())
        })
    }

    fn find_branch(
        &self,
        repository_id: &RepositoryId,
        branch_ref: &BranchRef,
    ) -> StorageResult<Option<ReviewBranch>> {
        self.owner.with_connection(|connection| {
            let raw = connection
                .query_row(
                    "SELECT repository_id, full_ref, observed_tip, observed_at
                     FROM review_branches WHERE repository_id = ?1 AND full_ref = ?2",
                    params![repository_id.as_str(), branch_ref.as_str()],
                    decode_branch_row,
                )
                .optional()
                .map_err(sql_error("load review branch"))?;
            raw.map(decode_branch).transpose()
        })
    }

    fn list_branches(&self, repository_id: &RepositoryId) -> StorageResult<Vec<ReviewBranch>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT repository_id, full_ref, observed_tip, observed_at
                     FROM review_branches WHERE repository_id = ?1 ORDER BY full_ref",
                )
                .map_err(sql_error("prepare review branch query"))?;
            let raw = statement
                .query_map([repository_id.as_str()], decode_branch_row)
                .map_err(sql_error("query review branches"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error("read review branches"))?;
            raw.into_iter().map(decode_branch).collect()
        })
    }
}

type RepositoryRow = (String, String, String, String);
type BranchRow = (String, String, String, String);

fn decode_branch_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BranchRow> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn decode_repository(raw: RepositoryRow) -> StorageResult<ReviewRepository> {
    Ok(ReviewRepository {
        id: RepositoryId::new(raw.0).map_err(|error| StorageError::corrupt(error.to_string()))?,
        label: raw.1,
        first_seen_at: decode_time(raw.2, "repository first seen")?,
        last_seen_at: decode_time(raw.3, "repository last seen")?,
    })
}

fn decode_branch(raw: BranchRow) -> StorageResult<ReviewBranch> {
    Ok(ReviewBranch {
        repository_id: RepositoryId::new(raw.0)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        branch_ref: BranchRef::new(raw.1)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        observed_tip: GitObjectId::new(raw.2)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        observed_at: decode_time(raw.3, "branch observation")?,
    })
}
