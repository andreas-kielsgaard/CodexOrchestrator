use super::{sql_error, ConnectionProvider, StorageResult};
use rusqlite::{params, OptionalExtension};
use std::path::PathBuf;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PersistedRepositorySelection {
    pub(crate) repository_id: String,
    pub(crate) repository_root: PathBuf,
}

pub(crate) trait RepositorySelectionRepository {
    fn load(&self) -> StorageResult<Option<PersistedRepositorySelection>>;
    fn save(&self, selection: &PersistedRepositorySelection) -> StorageResult<()>;
}

pub(crate) struct SqliteRepositorySelectionRepository<'owner, Owner> {
    owner: &'owner Owner,
}

impl<'owner, Owner> SqliteRepositorySelectionRepository<'owner, Owner> {
    pub(super) fn new(owner: &'owner Owner) -> Self {
        Self { owner }
    }
}

impl<Owner: ConnectionProvider> RepositorySelectionRepository
    for SqliteRepositorySelectionRepository<'_, Owner>
{
    fn load(&self) -> StorageResult<Option<PersistedRepositorySelection>> {
        self.owner.with_connection(|connection| {
            connection
                .query_row(
                    "SELECT repository_id, repository_root
                     FROM worktree_review_selection WHERE singleton = 1",
                    [],
                    |row| {
                        Ok(PersistedRepositorySelection {
                            repository_id: row.get(0)?,
                            repository_root: PathBuf::from(row.get::<_, String>(1)?),
                        })
                    },
                )
                .optional()
                .map_err(sql_error("load selected Worktree Review repository"))
        })
    }

    fn save(&self, selection: &PersistedRepositorySelection) -> StorageResult<()> {
        self.owner.with_connection(|connection| {
            connection
                .execute(
                    "INSERT INTO worktree_review_selection (
                       singleton, repository_id, repository_root
                     ) VALUES (1, ?1, ?2)
                     ON CONFLICT(singleton) DO UPDATE SET
                       repository_id = excluded.repository_id,
                       repository_root = excluded.repository_root",
                    params![
                        selection.repository_id,
                        selection.repository_root.to_string_lossy().as_ref(),
                    ],
                )
                .map_err(sql_error("save selected Worktree Review repository"))?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_review::storage::WorktreeReviewDatabase;

    #[test]
    fn replaces_the_single_selection_without_deleting_repository_state() {
        let database = WorktreeReviewDatabase::open_in_memory().unwrap();
        let first = PersistedRepositorySelection {
            repository_id: "repository-first".into(),
            repository_root: "C:/repositories/first".into(),
        };
        let second = PersistedRepositorySelection {
            repository_id: "repository-second".into(),
            repository_root: "C:/repositories/second".into(),
        };
        database.selection().save(&first).unwrap();
        database.selection().save(&second).unwrap();
        assert_eq!(database.selection().load().unwrap(), Some(second));
    }
}
