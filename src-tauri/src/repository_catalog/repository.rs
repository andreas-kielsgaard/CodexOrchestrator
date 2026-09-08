use super::domain::{RegisteredRepository, RepositoryDisclosure, RepositoryDisclosureKind};
use crate::persistence::ActiveDatabase;
use chrono::{DateTime, Utc};
use rusqlite::{params, OptionalExtension};
use std::{path::PathBuf, sync::Arc};

pub(crate) const REPOSITORY_CATALOG_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS registered_repositories (
  repository_id TEXT PRIMARY KEY,
  label TEXT NOT NULL CHECK(length(trim(label)) > 0),
  anchor_root TEXT NOT NULL,
  git_common_directory TEXT NOT NULL,
  first_registered_at TEXT NOT NULL,
  last_verified_at TEXT NOT NULL
);
CREATE UNIQUE INDEX IF NOT EXISTS registered_repositories_by_common_directory
  ON registered_repositories(git_common_directory);

CREATE TABLE IF NOT EXISTS registered_repository_disclosures (
  repository_id TEXT NOT NULL REFERENCES registered_repositories(repository_id) ON DELETE CASCADE,
  kind TEXT NOT NULL CHECK(kind IN ('manual_directory', 'codex_task')),
  observed_path TEXT NOT NULL,
  first_seen_at TEXT NOT NULL,
  last_seen_at TEXT NOT NULL,
  PRIMARY KEY(repository_id, kind, observed_path)
);
"#;

#[derive(Clone)]
pub(crate) struct SqliteRepositoryCatalogRepository {
    database: Arc<ActiveDatabase>,
}

impl SqliteRepositoryCatalogRepository {
    pub(crate) fn new(database: Arc<ActiveDatabase>) -> Self {
        Self { database }
    }

    pub(crate) fn save_registration(
        &self,
        repository: &RegisteredRepository,
        disclosure: &RepositoryDisclosure,
    ) -> Result<(), String> {
        self.database
            .write("save repository registration", |transaction| {
                let changed = transaction
                    .execute(
                        "INSERT INTO registered_repositories(
                           repository_id, label, anchor_root, git_common_directory,
                           first_registered_at, last_verified_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5, ?6)
                         ON CONFLICT(repository_id) DO UPDATE SET
                           label = excluded.label,
                           anchor_root = excluded.anchor_root,
                           last_verified_at = excluded.last_verified_at
                         WHERE registered_repositories.git_common_directory =
                               excluded.git_common_directory",
                        params![
                            repository.repository_id,
                            repository.label,
                            repository.anchor_root.to_string_lossy().as_ref(),
                            repository.git_common_directory.to_string_lossy().as_ref(),
                            encode_time(repository.first_registered_at),
                            encode_time(repository.last_verified_at),
                        ],
                    )
                    .map_err(|error| format!("Unable to save registered repository: {error}"))?;
                if changed != 1 {
                    return Err(
                        "The repository identity is already bound to another Git common directory."
                            .to_string(),
                    );
                }
                transaction
                    .execute(
                        "INSERT INTO registered_repository_disclosures(
                           repository_id, kind, observed_path, first_seen_at, last_seen_at
                         ) VALUES (?1, ?2, ?3, ?4, ?5)
                         ON CONFLICT(repository_id, kind, observed_path) DO UPDATE SET
                           last_seen_at = excluded.last_seen_at",
                        params![
                            disclosure.repository_id,
                            disclosure.kind.as_str(),
                            disclosure.observed_path.to_string_lossy().as_ref(),
                            encode_time(disclosure.first_seen_at),
                            encode_time(disclosure.last_seen_at),
                        ],
                    )
                    .map_err(|error| format!("Unable to save repository disclosure: {error}"))?;
                Ok(())
            })
            .map_err(|error| error.into_string())
    }

    pub(crate) fn find(&self, repository_id: &str) -> Result<Option<RegisteredRepository>, String> {
        self.database
            .read("load registered repository", |connection| {
                connection
                    .query_row(
                        "SELECT repository_id, label, anchor_root, git_common_directory,
                                first_registered_at, last_verified_at
                         FROM registered_repositories WHERE repository_id = ?1",
                        [repository_id],
                        decode_repository_row,
                    )
                    .optional()
                    .map_err(|error| format!("Unable to load registered repository: {error}"))?
                    .map(decode_repository)
                    .transpose()
            })
            .map_err(|error| error.into_string())
    }

    pub(crate) fn list(&self) -> Result<Vec<RegisteredRepository>, String> {
        self.database
            .read("list registered repositories", |connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT repository_id, label, anchor_root, git_common_directory,
                                first_registered_at, last_verified_at
                         FROM registered_repositories
                         ORDER BY label COLLATE NOCASE, repository_id",
                    )
                    .map_err(|error| format!("Unable to prepare repository catalog: {error}"))?;
                let rows = statement
                    .query_map([], decode_repository_row)
                    .map_err(|error| format!("Unable to query repository catalog: {error}"))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("Unable to read repository catalog: {error}"))?;
                rows.into_iter().map(decode_repository).collect()
            })
            .map_err(|error| error.into_string())
    }

    pub(crate) fn list_disclosures(
        &self,
        repository_id: &str,
    ) -> Result<Vec<RepositoryDisclosure>, String> {
        self.database
            .read("list repository disclosures", |connection| {
                let mut statement = connection
                    .prepare(
                        "SELECT repository_id, kind, observed_path, first_seen_at, last_seen_at
                         FROM registered_repository_disclosures
                         WHERE repository_id = ?1
                         ORDER BY first_seen_at, kind, observed_path",
                    )
                    .map_err(|error| {
                        format!("Unable to prepare repository disclosures: {error}")
                    })?;
                let rows = statement
                    .query_map([repository_id], |row| {
                        Ok((
                            row.get::<_, String>(0)?,
                            row.get::<_, String>(1)?,
                            row.get::<_, String>(2)?,
                            row.get::<_, String>(3)?,
                            row.get::<_, String>(4)?,
                        ))
                    })
                    .map_err(|error| format!("Unable to query repository disclosures: {error}"))?
                    .collect::<Result<Vec<_>, _>>()
                    .map_err(|error| format!("Unable to read repository disclosures: {error}"))?;
                rows.into_iter().map(decode_disclosure).collect()
            })
            .map_err(|error| error.into_string())
    }
}

type RepositoryRow = (String, String, String, String, String, String);

fn decode_repository_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<RepositoryRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn decode_repository(row: RepositoryRow) -> Result<RegisteredRepository, String> {
    Ok(RegisteredRepository {
        repository_id: row.0,
        label: row.1,
        anchor_root: PathBuf::from(row.2),
        git_common_directory: PathBuf::from(row.3),
        first_registered_at: decode_time(&row.4, "first registration")?,
        last_verified_at: decode_time(&row.5, "last verification")?,
    })
}

fn decode_disclosure(
    row: (String, String, String, String, String),
) -> Result<RepositoryDisclosure, String> {
    Ok(RepositoryDisclosure {
        repository_id: row.0,
        kind: RepositoryDisclosureKind::parse(&row.1)
            .ok_or_else(|| "Repository disclosure has an invalid kind.".to_string())?,
        observed_path: PathBuf::from(row.2),
        first_seen_at: decode_time(&row.3, "disclosure first seen")?,
        last_seen_at: decode_time(&row.4, "disclosure last seen")?,
    })
}

fn encode_time(value: DateTime<Utc>) -> String {
    value.to_rfc3339_opts(chrono::SecondsFormat::AutoSi, true)
}

fn decode_time(value: &str, label: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| format!("Repository catalog {label} is invalid: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use rusqlite::Connection;

    fn repository(database: Arc<ActiveDatabase>) -> SqliteRepositoryCatalogRepository {
        SqliteRepositoryCatalogRepository::new(database)
    }

    fn database() -> Arc<ActiveDatabase> {
        Arc::new(
            ActiveDatabase::from_connection(Connection::open_in_memory().unwrap(), |connection| {
                connection
                    .execute_batch(REPOSITORY_CATALOG_SCHEMA)
                    .map_err(|error| error.to_string())
            })
            .unwrap(),
        )
    }

    fn registration(
        repository_id: &str,
        common_directory: &str,
        observed_path: &str,
        at: DateTime<Utc>,
    ) -> (RegisteredRepository, RepositoryDisclosure) {
        (
            RegisteredRepository {
                repository_id: repository_id.into(),
                label: "Repository".into(),
                anchor_root: "C:/repositories/repository".into(),
                git_common_directory: common_directory.into(),
                first_registered_at: at,
                last_verified_at: at,
            },
            RepositoryDisclosure {
                repository_id: repository_id.into(),
                kind: RepositoryDisclosureKind::ManualDirectory,
                observed_path: observed_path.into(),
                first_seen_at: at,
                last_seen_at: at,
            },
        )
    }

    #[test]
    fn repeated_registration_preserves_first_seen_evidence() {
        let storage = repository(database());
        let first = Utc::now() - chrono::Duration::minutes(1);
        let second = Utc::now();
        let (record, disclosure) = registration(
            "repository-one",
            "C:/repositories/repository/.git",
            "C:/repositories/repository",
            first,
        );
        storage.save_registration(&record, &disclosure).unwrap();
        let (mut record, mut disclosure) = registration(
            "repository-one",
            "C:/repositories/repository/.git",
            "C:/repositories/repository",
            second,
        );
        record.first_registered_at = second;
        disclosure.first_seen_at = second;
        storage.save_registration(&record, &disclosure).unwrap();

        let stored = storage.find("repository-one").unwrap().unwrap();
        let stored_disclosure = storage
            .list_disclosures("repository-one")
            .unwrap()
            .remove(0);
        assert_eq!(stored.first_registered_at, first);
        assert_eq!(stored.last_verified_at, second);
        assert_eq!(stored_disclosure.first_seen_at, first);
        assert_eq!(stored_disclosure.last_seen_at, second);
    }

    #[test]
    fn identity_rebind_rolls_back_repository_and_disclosure_changes() {
        let storage = repository(database());
        let first = Utc::now() - chrono::Duration::minutes(1);
        let second = Utc::now();
        let (record, disclosure) = registration(
            "repository-one",
            "C:/repositories/repository/.git",
            "C:/repositories/repository",
            first,
        );
        storage.save_registration(&record, &disclosure).unwrap();
        let (replacement, replacement_disclosure) = registration(
            "repository-one",
            "D:/different/.git",
            "D:/different",
            second,
        );

        assert!(storage
            .save_registration(&replacement, &replacement_disclosure)
            .is_err());
        let stored = storage.find("repository-one").unwrap().unwrap();
        assert_eq!(
            stored.git_common_directory,
            PathBuf::from("C:/repositories/repository/.git")
        );
        assert_eq!(storage.list_disclosures("repository-one").unwrap().len(), 1);
    }
}
