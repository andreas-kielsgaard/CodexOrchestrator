use super::domain::{IdentityDefinition, IdentityId, IdentityShape};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{path::Path, sync::Arc};

use crate::persistence::{ActiveDatabase, ManagedOperationError};

pub(crate) const IDENTITY_CATALOG_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS identity_definitions (
    identity_id TEXT PRIMARY KEY CHECK (length(trim(identity_id)) > 0),
    display_name TEXT NOT NULL CHECK (length(trim(display_name)) > 0),
    color TEXT NOT NULL CHECK (
        length(color) = 7 AND
        color GLOB '#[0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f][0-9A-Fa-f]'
    ),
    shape TEXT NOT NULL CHECK (shape IN ('circle', 'square', 'hexagon')),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS ix_identity_definitions_display_name
ON identity_definitions(display_name COLLATE NOCASE, identity_id);
"#;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct IdentityCatalogEntry {
    #[serde(flatten)]
    pub(crate) definition: IdentityDefinition,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

pub(crate) trait IdentityRepository: Send + Sync {
    fn list(&self) -> Result<Vec<IdentityCatalogEntry>, String>;
    fn find(&self, identity_id: &IdentityId) -> Result<Option<IdentityCatalogEntry>, String>;
    fn create(&self, entry: &IdentityCatalogEntry) -> Result<(), String>;
    fn update(&self, entry: &IdentityCatalogEntry) -> Result<(), String>;
    fn delete(&self, identity_id: &IdentityId) -> Result<(), String>;
}

pub(crate) struct SqliteIdentityRepository {
    database: Arc<ActiveDatabase>,
}

impl SqliteIdentityRepository {
    pub(crate) fn from_database(database: Arc<ActiveDatabase>) -> Self {
        Self { database }
    }

    pub(crate) fn open(database_path: &Path) -> Result<Self, String> {
        ActiveDatabase::open(database_path, initialize_identity_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .map_err(|error| error.to_string())
    }

    #[cfg(test)]
    pub(super) fn in_memory() -> Self {
        let connection = Connection::open_in_memory().expect("in-memory Identity storage");
        ActiveDatabase::from_connection(connection, initialize_identity_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .expect("Identity schema")
    }

    fn read<T>(
        &self,
        operation: &'static str,
        read: impl FnOnce(&Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        self.database.read(operation, read).map_err(managed_error)
    }

    fn write<T>(
        &self,
        operation: &'static str,
        write: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        self.database.write(operation, write).map_err(managed_error)
    }
}

impl IdentityRepository for SqliteIdentityRepository {
    fn list(&self) -> Result<Vec<IdentityCatalogEntry>, String> {
        self.read("list Identity definitions", |connection| {
            let mut statement = connection
                .prepare(
                    "SELECT identity_id,display_name,color,shape,created_at,updated_at \
                 FROM identity_definitions \
                 ORDER BY display_name COLLATE NOCASE,identity_id",
                )
                .map_err(|error| format!("Unable to prepare Identity catalog list: {error}"))?;
            let entries = statement
                .query_map([], identity_row)
                .map_err(|error| format!("Unable to query Identity catalog: {error}"))?
                .map(|row| {
                    row.map_err(|error| format!("Unable to read Identity catalog entry: {error}"))
                        .and_then(decode_identity_row)
                })
                .collect();
            entries
        })
    }

    fn find(&self, identity_id: &IdentityId) -> Result<Option<IdentityCatalogEntry>, String> {
        self.read("find Identity definition", |connection| {
            connection
                .query_row(
                    "SELECT identity_id,display_name,color,shape,created_at,updated_at \
                 FROM identity_definitions WHERE identity_id=?1",
                    [identity_id.as_str()],
                    identity_row,
                )
                .optional()
                .map_err(|error| format!("Unable to load Identity definition: {error}"))?
                .map(decode_identity_row)
                .transpose()
        })
    }

    fn create(&self, entry: &IdentityCatalogEntry) -> Result<(), String> {
        entry
            .definition
            .validate()
            .map_err(|error| error.to_string())?;
        self.write("create Identity definition", |transaction| {
            transaction
                .execute(
                    "INSERT INTO identity_definitions(\
                    identity_id,display_name,color,shape,created_at,updated_at\
                 ) VALUES(?1,?2,?3,?4,?5,?6)",
                    params![
                        entry.definition.id.as_str(),
                        entry.definition.display_name,
                        entry.definition.color,
                        shape_value(entry.definition.shape),
                        entry.created_at.to_rfc3339(),
                        entry.updated_at.to_rfc3339(),
                    ],
                )
                .map_err(|error| format!("Unable to create Identity definition: {error}"))?;
            Ok(())
        })
    }

    fn update(&self, entry: &IdentityCatalogEntry) -> Result<(), String> {
        entry
            .definition
            .validate()
            .map_err(|error| error.to_string())?;
        self.write("update Identity definition", |transaction| {
            let changed = transaction
                .execute(
                    "UPDATE identity_definitions \
                 SET display_name=?2,color=?3,shape=?4,updated_at=?5 \
                 WHERE identity_id=?1",
                    params![
                        entry.definition.id.as_str(),
                        entry.definition.display_name,
                        entry.definition.color,
                        shape_value(entry.definition.shape),
                        entry.updated_at.to_rfc3339(),
                    ],
                )
                .map_err(|error| format!("Unable to update Identity definition: {error}"))?;
            expect_one(changed, "Identity definition does not exist")
        })
    }

    fn delete(&self, identity_id: &IdentityId) -> Result<(), String> {
        self.write("delete Identity definition", |transaction| {
            let changed = transaction
                .execute(
                    "DELETE FROM identity_definitions WHERE identity_id=?1",
                    [identity_id.as_str()],
                )
                .map_err(|error| format!("Unable to delete Identity definition: {error}"))?;
            expect_one(changed, "Identity definition does not exist")
        })
    }
}

pub(crate) fn initialize_identity_storage(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(IDENTITY_CATALOG_SCHEMA)
        .map_err(|error| format!("Unable to initialize Identity storage: {error}"))
}

fn managed_error(error: ManagedOperationError<String>) -> String {
    match error {
        ManagedOperationError::Infrastructure(error) => error.to_string(),
        ManagedOperationError::Domain(error) => error,
    }
}

type IdentityRow = (String, String, String, String, String, String);

fn identity_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<IdentityRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn decode_identity_row(row: IdentityRow) -> Result<IdentityCatalogEntry, String> {
    let (identity_id, display_name, color, shape, created_at, updated_at) = row;
    Ok(IdentityCatalogEntry {
        definition: IdentityDefinition::new(
            IdentityId::new(identity_id).map_err(|error| error.to_string())?,
            display_name,
            color,
            decode_shape(&shape)?,
        )
        .map_err(|error| error.to_string())?,
        created_at: parse_timestamp(&created_at, "created")?,
        updated_at: parse_timestamp(&updated_at, "updated")?,
    })
}

fn shape_value(shape: IdentityShape) -> &'static str {
    match shape {
        IdentityShape::Circle => "circle",
        IdentityShape::Square => "square",
        IdentityShape::Hexagon => "hexagon",
    }
}

fn decode_shape(value: &str) -> Result<IdentityShape, String> {
    match value {
        "circle" => Ok(IdentityShape::Circle),
        "square" => Ok(IdentityShape::Square),
        "hexagon" => Ok(IdentityShape::Hexagon),
        _ => Err("Identity definition contains an unsupported shape.".into()),
    }
}

fn parse_timestamp(value: &str, kind: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|timestamp| timestamp.with_timezone(&Utc))
        .map_err(|error| format!("Identity definition has an invalid {kind} timestamp: {error}"))
}

fn expect_one(changed: usize, message: &str) -> Result<(), String> {
    match changed {
        1 => Ok(()),
        0 => Err(format!("{message}.")),
        _ => Err("Identity catalog mutation affected more than one definition.".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use chrono::TimeZone;

    fn entry(id: &str, name: &str, shape: IdentityShape) -> IdentityCatalogEntry {
        let created_at = Utc.with_ymd_and_hms(2026, 8, 27, 9, 0, 0).unwrap();
        IdentityCatalogEntry {
            definition: IdentityDefinition::new(
                IdentityId::new(id).unwrap(),
                name,
                "#39745a",
                shape,
            )
            .unwrap(),
            created_at,
            updated_at: created_at,
        }
    }

    #[test]
    fn persists_lists_updates_and_deletes_identity_definitions() {
        let repository = SqliteIdentityRepository::in_memory();
        let mut avery = entry("identity-avery", "Avery", IdentityShape::Circle);
        let grace = entry("identity-grace", "Grace", IdentityShape::Square);
        repository.create(&grace).unwrap();
        repository.create(&avery).unwrap();

        assert_eq!(
            repository
                .list()
                .unwrap()
                .iter()
                .map(|entry| entry.definition.id.as_str())
                .collect::<Vec<_>>(),
            vec!["identity-avery", "identity-grace"]
        );

        avery.definition.display_name = "Avery Stone".into();
        avery.definition.shape = IdentityShape::Hexagon;
        avery.updated_at = Utc.with_ymd_and_hms(2026, 8, 27, 10, 0, 0).unwrap();
        repository.update(&avery).unwrap();
        assert_eq!(
            repository.find(&avery.definition.id).unwrap().unwrap(),
            avery
        );

        repository.delete(&grace.definition.id).unwrap();
        assert!(repository.find(&grace.definition.id).unwrap().is_none());
        assert!(repository.delete(&grace.definition.id).is_err());
    }
}
