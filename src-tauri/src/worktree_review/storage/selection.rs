use super::{sql_error, ConnectionProvider, StorageResult};
use rusqlite::OptionalExtension;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PersistedRepositorySelection {
    pub(crate) repository_id: String,
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
                    "SELECT repository_id
                     FROM worktree_review_selection WHERE singleton = 1",
                    [],
                    |row| {
                        Ok(PersistedRepositorySelection {
                            repository_id: row.get(0)?,
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
                    "INSERT INTO worktree_review_selection (singleton, repository_id)
                     VALUES (1, ?1)
                     ON CONFLICT(singleton) DO UPDATE SET
                       repository_id = excluded.repository_id",
                    [selection.repository_id.as_str()],
                )
                .map_err(sql_error("save selected Worktree Review repository"))?;
            Ok(())
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_review::{
        domain::{RepositoryId, ReviewRepository},
        storage::{ReviewRepositoryRepository, WorktreeReviewDatabase},
    };
    use chrono::Utc;

    #[test]
    fn replaces_the_single_selection_without_deleting_repository_state() {
        let database = WorktreeReviewDatabase::open_in_memory().unwrap();
        let first = PersistedRepositorySelection {
            repository_id: "repository-first".into(),
        };
        let second = PersistedRepositorySelection {
            repository_id: "repository-second".into(),
        };
        for (id, root) in [
            ("repository-first", "C:/repositories/first"),
            ("repository-second", "C:/repositories/second"),
        ] {
            database
                .repositories()
                .save_repository(&ReviewRepository {
                    id: RepositoryId::new(id).unwrap(),
                    label: id.into(),
                    anchor_root: root.into(),
                    common_directory: format!("{root}/.git").into(),
                    first_seen_at: Utc::now(),
                    last_seen_at: Utc::now(),
                })
                .unwrap();
        }
        database.selection().save(&first).unwrap();
        database.selection().save(&second).unwrap();
        assert_eq!(database.selection().load().unwrap(), Some(second));
    }

    #[test]
    fn migrates_the_legacy_selected_path_into_registration_inventory() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("worktree-review.sqlite");
        let legacy = rusqlite::Connection::open(&path).unwrap();
        legacy
            .execute_batch(
                "CREATE TABLE review_repositories (
                   repository_id TEXT PRIMARY KEY, label TEXT NOT NULL,
                   first_seen_at TEXT NOT NULL, last_seen_at TEXT NOT NULL
                 );
                 CREATE TABLE worktree_review_selection (
                   singleton INTEGER PRIMARY KEY, repository_id TEXT NOT NULL,
                   repository_root TEXT NOT NULL
                 );
                 INSERT INTO review_repositories VALUES (
                   'repository-legacy', 'Legacy repository',
                   '2026-01-01T00:00:00Z', '2026-01-01T00:00:00Z'
                 );
                 INSERT INTO worktree_review_selection VALUES (
                   1, 'repository-legacy', 'C:/repositories/legacy'
                 );",
            )
            .unwrap();
        drop(legacy);

        let database = WorktreeReviewDatabase::open(&path).unwrap();
        let id = RepositoryId::new("repository-legacy").unwrap();
        let repository = database
            .repositories()
            .find_repository(&id)
            .unwrap()
            .unwrap();

        assert_eq!(
            repository.anchor_root,
            std::path::PathBuf::from("C:/repositories/legacy")
        );
        assert_eq!(
            database.selection().load().unwrap(),
            Some(PersistedRepositorySelection {
                repository_id: "repository-legacy".into()
            })
        );
        assert_eq!(
            database.repositories().list_disclosures(&id).unwrap().len(),
            1
        );
    }
}
