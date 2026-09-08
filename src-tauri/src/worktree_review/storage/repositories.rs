use super::{decode_time, encode_time, sql_error, ConnectionProvider, StorageError, StorageResult};
use crate::worktree_review::domain::{
    BranchRef, GitObjectId, RepositoryDisclosure, RepositoryDisclosureKind, RepositoryId,
    ReviewBranch, ReviewRepository,
};
use rusqlite::{params, OptionalExtension};
use std::path::PathBuf;

pub(crate) trait ReviewRepositoryRepository {
    fn save_repository(&self, repository: &ReviewRepository) -> StorageResult<()>;
    fn find_repository(&self, id: &RepositoryId) -> StorageResult<Option<ReviewRepository>>;
    fn list_repositories(&self) -> StorageResult<Vec<ReviewRepository>>;
    fn save_disclosure(&self, disclosure: &RepositoryDisclosure) -> StorageResult<()>;
    fn list_disclosures(
        &self,
        repository_id: &RepositoryId,
    ) -> StorageResult<Vec<RepositoryDisclosure>>;
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
                       repository_id, label, anchor_root, common_directory,
                       first_seen_at, last_seen_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                     ON CONFLICT(repository_id) DO UPDATE SET
                       label = excluded.label,
                       anchor_root = excluded.anchor_root,
                       common_directory = excluded.common_directory,
                       last_seen_at = excluded.last_seen_at",
                    params![
                        repository.id.as_str(),
                        repository.label,
                        repository.anchor_root.to_string_lossy().as_ref(),
                        repository.common_directory.to_string_lossy().as_ref(),
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
                    "SELECT repository_id, label, anchor_root, common_directory,
                            first_seen_at, last_seen_at
                     FROM review_repositories WHERE repository_id = ?1",
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
                .map_err(sql_error("load review repository"))?;
            raw.map(decode_repository).transpose()
        })
    }

    fn list_repositories(&self) -> StorageResult<Vec<ReviewRepository>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT repository_id, label, anchor_root, common_directory,
                            first_seen_at, last_seen_at
                     FROM review_repositories
                     WHERE anchor_root IS NOT NULL AND common_directory IS NOT NULL
                     ORDER BY label COLLATE NOCASE, repository_id",
                )
                .map_err(sql_error("prepare registered repository query"))?;
            let raw = statement
                .query_map([], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                        row.get::<_, String>(5)?,
                    ))
                })
                .map_err(sql_error("query registered repositories"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error("read registered repositories"))?;
            raw.into_iter().map(decode_repository).collect()
        })
    }

    fn save_disclosure(&self, disclosure: &RepositoryDisclosure) -> StorageResult<()> {
        self.owner.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO review_repository_disclosures(
                       repository_id, kind, observed_path, first_seen_at, last_seen_at
                     ) VALUES (?1, ?2, ?3, ?4, ?5)
                     ON CONFLICT(repository_id, kind, observed_path) DO UPDATE SET
                       last_seen_at = excluded.last_seen_at",
                    params![
                        disclosure.repository_id.as_str(),
                        disclosure.kind.as_str(),
                        disclosure.observed_path.to_string_lossy().as_ref(),
                        encode_time(disclosure.first_seen_at),
                        encode_time(disclosure.last_seen_at),
                    ],
                )
                .map_err(sql_error("save registered repository disclosure"))?;
            Ok(())
        })
    }

    fn list_disclosures(
        &self,
        repository_id: &RepositoryId,
    ) -> StorageResult<Vec<RepositoryDisclosure>> {
        self.owner.with_connection(|connection| {
            let mut statement = connection
                .prepare(
                    "SELECT repository_id, kind, observed_path, first_seen_at, last_seen_at
                     FROM review_repository_disclosures WHERE repository_id = ?1
                     ORDER BY first_seen_at, kind, observed_path",
                )
                .map_err(sql_error("prepare repository disclosure query"))?;
            let raw = statement
                .query_map([repository_id.as_str()], |row| {
                    Ok((
                        row.get::<_, String>(0)?,
                        row.get::<_, String>(1)?,
                        row.get::<_, String>(2)?,
                        row.get::<_, String>(3)?,
                        row.get::<_, String>(4)?,
                    ))
                })
                .map_err(sql_error("query repository disclosures"))?
                .collect::<Result<Vec<_>, _>>()
                .map_err(sql_error("read repository disclosures"))?;
            raw.into_iter().map(decode_disclosure).collect()
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

type RepositoryRow = (String, String, String, String, String, String);
type BranchRow = (String, String, String, String);
type DisclosureRow = (String, String, String, String, String);

fn decode_branch_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<BranchRow> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn decode_repository(raw: RepositoryRow) -> StorageResult<ReviewRepository> {
    Ok(ReviewRepository {
        id: RepositoryId::new(raw.0).map_err(|error| StorageError::corrupt(error.to_string()))?,
        label: raw.1,
        anchor_root: PathBuf::from(raw.2),
        common_directory: PathBuf::from(raw.3),
        first_seen_at: decode_time(raw.4, "repository first seen")?,
        last_seen_at: decode_time(raw.5, "repository last seen")?,
    })
}

fn decode_disclosure(raw: DisclosureRow) -> StorageResult<RepositoryDisclosure> {
    Ok(RepositoryDisclosure {
        repository_id: RepositoryId::new(raw.0)
            .map_err(|error| StorageError::corrupt(error.to_string()))?,
        kind: RepositoryDisclosureKind::parse(&raw.1)
            .ok_or_else(|| StorageError::corrupt("invalid repository disclosure kind"))?,
        observed_path: PathBuf::from(raw.2),
        first_seen_at: decode_time(raw.3, "repository disclosure first seen")?,
        last_seen_at: decode_time(raw.4, "repository disclosure last seen")?,
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
