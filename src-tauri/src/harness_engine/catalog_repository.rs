use super::{
    catalog::{HarnessDraft, HarnessRecord, HarnessVersion},
    configuration::{HarnessConfiguration, HarnessMetadata, HARNESS_CONFIGURATION_VERSION},
    domain::{
        HarnessId, HarnessVersionNumber, HarnessVersionRef, HarnessVersionReplacement,
        HarnessVersionScope,
    },
};
use chrono::{DateTime, Utc};
use rusqlite::{params, Connection, OptionalExtension, Transaction};
use std::{path::Path, sync::Arc};

use crate::persistence::{ActiveDatabase, ManagedOperationError};

pub(crate) const HARNESS_CATALOG_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS harnesses (
    harness_id TEXT PRIMARY KEY,
    metadata_json TEXT NOT NULL CHECK (json_valid(metadata_json)),
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS harness_versions (
    harness_id TEXT NOT NULL,
    version_number INTEGER NOT NULL CHECK (version_number > 0),
    scope_json TEXT NOT NULL CHECK (json_valid(scope_json)),
    configuration_contract_version TEXT NOT NULL CHECK (configuration_contract_version = 'harness-configuration/v1'),
    configuration_json TEXT NOT NULL CHECK (json_valid(configuration_json)),
    configuration_digest TEXT NOT NULL CHECK (length(configuration_digest) = 64),
    created_at TEXT NOT NULL,
    PRIMARY KEY (harness_id, version_number),
    FOREIGN KEY (harness_id) REFERENCES harnesses(harness_id) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS harness_drafts (
    harness_id TEXT PRIMARY KEY,
    based_on_version INTEGER CHECK (based_on_version IS NULL OR based_on_version > 0),
    configuration_contract_version TEXT NOT NULL CHECK (configuration_contract_version = 'harness-configuration/v1'),
    configuration_json TEXT NOT NULL CHECK (json_valid(configuration_json)),
    draft_revision INTEGER NOT NULL CHECK (draft_revision > 0),
    saved_at TEXT NOT NULL,
    FOREIGN KEY (harness_id) REFERENCES harnesses(harness_id) ON DELETE RESTRICT,
    FOREIGN KEY (harness_id, based_on_version) REFERENCES harness_versions(harness_id, version_number) ON DELETE RESTRICT
);

CREATE TABLE IF NOT EXISTS harness_version_replacements (
    harness_id TEXT NOT NULL,
    source_version INTEGER NOT NULL CHECK (source_version > 0),
    target_version INTEGER NOT NULL CHECK (target_version > 0 AND target_version <> source_version),
    ordered_at TEXT NOT NULL,
    PRIMARY KEY (harness_id, source_version),
    FOREIGN KEY (harness_id, source_version) REFERENCES harness_versions(harness_id, version_number) ON DELETE RESTRICT,
    FOREIGN KEY (harness_id, target_version) REFERENCES harness_versions(harness_id, version_number) ON DELETE RESTRICT
);
"#;

pub(crate) trait HarnessCatalogRepository: Send + Sync {
    fn create_harness(&self, harness: &HarnessRecord) -> Result<(), String>;
    fn update_metadata(
        &self,
        harness_id: &HarnessId,
        metadata: &HarnessMetadata,
        updated_at: DateTime<Utc>,
    ) -> Result<(), String>;
    fn harness(&self, harness_id: &HarnessId) -> Result<Option<HarnessRecord>, String>;
    fn harnesses(&self) -> Result<Vec<HarnessRecord>, String>;
    fn draft(&self, harness_id: &HarnessId) -> Result<Option<HarnessDraft>, String>;
    fn save_draft(
        &self,
        draft: &HarnessDraft,
        expected_current_revision: u64,
    ) -> Result<(), String>;
    fn version(&self, reference: &HarnessVersionRef) -> Result<Option<HarnessVersion>, String>;
    fn versions(&self, harness_id: &HarnessId) -> Result<Vec<HarnessVersion>, String>;
    fn publish(&self, version: &HarnessVersion, draft_revision: Option<u64>) -> Result<(), String>;
    fn replacement(
        &self,
        source: &HarnessVersionRef,
    ) -> Result<Option<HarnessVersionReplacement>, String>;
    fn replacements(
        &self,
        harness_id: &HarnessId,
    ) -> Result<Vec<HarnessVersionReplacement>, String>;
    fn order_replacement(
        &self,
        replacement: &HarnessVersionReplacement,
        ordered_at: DateTime<Utc>,
    ) -> Result<(), String>;
}

pub(crate) struct SqliteHarnessCatalogRepository {
    database: Arc<ActiveDatabase>,
}

impl SqliteHarnessCatalogRepository {
    pub(crate) fn from_database(database: Arc<ActiveDatabase>) -> Self {
        Self { database }
    }

    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        ActiveDatabase::open(path, initialize_harness_catalog_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .map_err(|error| error.to_string())
    }

    #[cfg(test)]
    pub(super) fn in_memory() -> Self {
        let connection = Connection::open_in_memory().unwrap();
        ActiveDatabase::from_connection(connection, initialize_harness_catalog_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .unwrap()
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
        write: impl FnOnce(&Transaction<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        self.database.write(operation, write).map_err(managed_error)
    }
}

impl HarnessCatalogRepository for SqliteHarnessCatalogRepository {
    fn create_harness(&self, harness: &HarnessRecord) -> Result<(), String> {
        if harness.metadata.name.trim().is_empty() {
            return Err("Harness name must not be empty.".into());
        }
        self.write("create Harness", |transaction| {
            transaction.execute(
                "INSERT INTO harnesses(harness_id,metadata_json,created_at,updated_at) VALUES(?1,?2,?3,?4)",
                params![
                    harness.id.as_str(),
                    encode(&harness.metadata)?,
                    harness.created_at.to_rfc3339(),
                    harness.updated_at.to_rfc3339(),
                ],
            )
            .map_err(|error| format!("Unable to create Harness: {error}"))?;
            Ok(())
        })
    }

    fn update_metadata(
        &self,
        harness_id: &HarnessId,
        metadata: &HarnessMetadata,
        updated_at: DateTime<Utc>,
    ) -> Result<(), String> {
        if metadata.name.trim().is_empty() {
            return Err("Harness name must not be empty.".into());
        }
        self.write("update Harness metadata", |transaction| {
            let changed = transaction
                .execute(
                    "UPDATE harnesses SET metadata_json=?2,updated_at=?3 WHERE harness_id=?1",
                    params![
                        harness_id.as_str(),
                        encode(metadata)?,
                        updated_at.to_rfc3339()
                    ],
                )
                .map_err(|error| format!("Unable to update Harness metadata: {error}"))?;
            expect_one(changed, "Harness does not exist")
        })
    }

    fn harness(&self, harness_id: &HarnessId) -> Result<Option<HarnessRecord>, String> {
        self.read("load Harness", |connection| {
            connection.query_row(
                "SELECT harness_id,metadata_json,created_at,updated_at FROM harnesses WHERE harness_id=?1",
                [harness_id.as_str()],
                harness_row,
            )
            .optional()
            .map_err(|error| format!("Unable to load Harness: {error}"))?
            .map(decode_harness_row)
            .transpose()
        })
    }

    fn harnesses(&self) -> Result<Vec<HarnessRecord>, String> {
        self.read("list Harnesses", |connection| {
            let mut statement = connection
            .prepare("SELECT harness_id,metadata_json,created_at,updated_at FROM harnesses ORDER BY json_extract(metadata_json,'$.name'),harness_id")
            .map_err(|error| format!("Unable to prepare Harness list: {error}"))?;
        let harnesses = statement
            .query_map([], harness_row)
            .map_err(|error| format!("Unable to query Harnesses: {error}"))?
            .map(|row| {
                row.map_err(|error| format!("Unable to read Harness: {error}"))
                    .and_then(decode_harness_row)
            })
            .collect();
            harnesses
        })
    }

    fn draft(&self, harness_id: &HarnessId) -> Result<Option<HarnessDraft>, String> {
        self.read("load Harness draft", |connection| {
            connection.query_row(
                "SELECT harness_id,based_on_version,configuration_json,draft_revision,saved_at FROM harness_drafts WHERE harness_id=?1",
                [harness_id.as_str()],
                draft_row,
            )
            .optional()
            .map_err(|error| format!("Unable to load Harness draft: {error}"))?
            .map(decode_draft_row)
            .transpose()
        })
    }

    fn save_draft(
        &self,
        draft: &HarnessDraft,
        expected_current_revision: u64,
    ) -> Result<(), String> {
        draft.validate()?;
        if draft.draft_revision != expected_current_revision + 1 {
            return Err("Harness draft revision must advance exactly once.".into());
        }
        self.write("save Harness draft", |transaction| {
        let changed = if expected_current_revision == 0 {
            transaction.execute(
                "INSERT INTO harness_drafts(harness_id,based_on_version,configuration_contract_version,configuration_json,draft_revision,saved_at) VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    draft.harness_id.as_str(),
                    draft.based_on_version.map(|value| value.get()),
                    HARNESS_CONFIGURATION_VERSION,
                    encode(&draft.configuration)?,
                    draft.draft_revision,
                    draft.saved_at.to_rfc3339(),
                ],
            )
        } else {
            transaction.execute(
                "UPDATE harness_drafts SET based_on_version=?2,configuration_json=?3,draft_revision=?4,saved_at=?5 WHERE harness_id=?1 AND draft_revision=?6",
                params![
                    draft.harness_id.as_str(),
                    draft.based_on_version.map(|value| value.get()),
                    encode(&draft.configuration)?,
                    draft.draft_revision,
                    draft.saved_at.to_rfc3339(),
                    expected_current_revision,
                ],
            )
        }
        .map_err(|error| format!("Unable to save Harness draft: {error}"))?;
        expect_one(changed, "Harness draft changed before it could be saved")
        })
    }

    fn version(&self, reference: &HarnessVersionRef) -> Result<Option<HarnessVersion>, String> {
        self.read("load Harness version", |connection| {
            connection.query_row(
                "SELECT harness_id,version_number,scope_json,configuration_json,configuration_digest,created_at FROM harness_versions WHERE harness_id=?1 AND version_number=?2",
                params![reference.harness_id().as_str(), reference.version().get()],
                version_row,
            )
            .optional()
            .map_err(|error| format!("Unable to load Harness version: {error}"))?
            .map(decode_version_row)
            .transpose()
        })
    }

    fn versions(&self, harness_id: &HarnessId) -> Result<Vec<HarnessVersion>, String> {
        self.read("list Harness versions", |connection| {
            let mut statement = connection
            .prepare("SELECT harness_id,version_number,scope_json,configuration_json,configuration_digest,created_at FROM harness_versions WHERE harness_id=?1 ORDER BY version_number")
            .map_err(|error| format!("Unable to prepare Harness versions: {error}"))?;
        let versions = statement
            .query_map([harness_id.as_str()], version_row)
            .map_err(|error| format!("Unable to query Harness versions: {error}"))?
            .map(|row| {
                row.map_err(|error| format!("Unable to read Harness version: {error}"))
                    .and_then(decode_version_row)
            })
            .collect();
            versions
        })
    }

    fn publish(&self, version: &HarnessVersion, draft_revision: Option<u64>) -> Result<(), String> {
        version.verify()?;
        self.write("publish Harness version", |transaction| {
            insert_version(transaction, version)?;
            if let Some(draft_revision) = draft_revision {
                let changed = transaction
                    .execute(
                        "DELETE FROM harness_drafts WHERE harness_id=?1 AND draft_revision=?2",
                        params![version.reference.harness_id().as_str(), draft_revision],
                    )
                    .map_err(|error| format!("Unable to consume Harness draft: {error}"))?;
                expect_one(changed, "Harness draft changed before publication")?;
            }
            Ok(())
        })
    }

    fn replacement(
        &self,
        source: &HarnessVersionRef,
    ) -> Result<Option<HarnessVersionReplacement>, String> {
        self.read("load Harness replacement", |connection| {
            connection.query_row(
                "SELECT harness_id,source_version,target_version FROM harness_version_replacements WHERE harness_id=?1 AND source_version=?2",
                params![source.harness_id().as_str(), source.version().get()],
                replacement_row,
            )
            .optional()
            .map_err(|error| format!("Unable to load Harness replacement: {error}"))?
            .map(decode_replacement_row)
            .transpose()
        })
    }

    fn replacements(
        &self,
        harness_id: &HarnessId,
    ) -> Result<Vec<HarnessVersionReplacement>, String> {
        self.read("list Harness replacements", |connection| {
            let mut statement = connection
            .prepare("SELECT harness_id,source_version,target_version FROM harness_version_replacements WHERE harness_id=?1 ORDER BY source_version")
            .map_err(|error| format!("Unable to prepare Harness replacements: {error}"))?;
        let replacements = statement
            .query_map([harness_id.as_str()], replacement_row)
            .map_err(|error| format!("Unable to query Harness replacements: {error}"))?
            .map(|row| {
                row.map_err(|error| format!("Unable to read Harness replacement: {error}"))
                    .and_then(decode_replacement_row)
            })
            .collect();
            replacements
        })
    }

    fn order_replacement(
        &self,
        replacement: &HarnessVersionReplacement,
        ordered_at: DateTime<Utc>,
    ) -> Result<(), String> {
        replacement.validate().map_err(|error| error.to_string())?;
        self.write("order Harness replacement", |transaction| {
            transaction.execute(
                "INSERT INTO harness_version_replacements(harness_id,source_version,target_version,ordered_at) VALUES(?1,?2,?3,?4) ON CONFLICT(harness_id,source_version) DO UPDATE SET target_version=excluded.target_version,ordered_at=excluded.ordered_at",
                params![
                    replacement.source().harness_id().as_str(),
                    replacement.source().version().get(),
                    replacement.target().version().get(),
                    ordered_at.to_rfc3339(),
                ],
            )
            .map_err(|error| format!("Unable to order Harness replacement: {error}"))?;
            Ok(())
        })
    }
}

pub(crate) fn initialize_harness_catalog_storage(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(HARNESS_CATALOG_SCHEMA)
        .map_err(|error| format!("Unable to initialize Harness catalog storage: {error}"))
}

fn managed_error(error: ManagedOperationError<String>) -> String {
    match error {
        ManagedOperationError::Infrastructure(error) => error.to_string(),
        ManagedOperationError::Domain(error) => error,
    }
}

type HarnessRow = (String, String, String, String);
type DraftRow = (String, Option<u64>, String, u64, String);
type VersionRow = (String, u64, String, String, String, String);
type ReplacementRow = (String, u64, u64);

fn harness_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<HarnessRow> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
}

fn draft_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<DraftRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
    ))
}

fn version_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<VersionRow> {
    Ok((
        row.get(0)?,
        row.get(1)?,
        row.get(2)?,
        row.get(3)?,
        row.get(4)?,
        row.get(5)?,
    ))
}

fn replacement_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<ReplacementRow> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
}

fn decode_harness_row(row: HarnessRow) -> Result<HarnessRecord, String> {
    Ok(HarnessRecord {
        id: HarnessId::new(row.0).map_err(|error| error.to_string())?,
        metadata: decode(&row.1, "Harness metadata")?,
        created_at: parse_time(&row.2)?,
        updated_at: parse_time(&row.3)?,
    })
}

fn decode_draft_row(row: DraftRow) -> Result<HarnessDraft, String> {
    let draft = HarnessDraft {
        harness_id: HarnessId::new(row.0).map_err(|error| error.to_string())?,
        based_on_version: row
            .1
            .map(HarnessVersionNumber::new)
            .transpose()
            .map_err(|error| error.to_string())?,
        configuration: decode(&row.2, "Harness draft configuration")?,
        draft_revision: row.3,
        saved_at: parse_time(&row.4)?,
    };
    draft.validate()?;
    Ok(draft)
}

fn decode_version_row(row: VersionRow) -> Result<HarnessVersion, String> {
    let version = HarnessVersion {
        reference: HarnessVersionRef::new(
            HarnessId::new(row.0).map_err(|error| error.to_string())?,
            HarnessVersionNumber::new(row.1).map_err(|error| error.to_string())?,
        ),
        scope: decode::<HarnessVersionScope>(&row.2, "Harness version scope")?,
        configuration: decode::<HarnessConfiguration>(&row.3, "Harness configuration")?,
        configuration_digest: row.4,
        created_at: parse_time(&row.5)?,
    };
    version.verify()?;
    Ok(version)
}

fn decode_replacement_row(row: ReplacementRow) -> Result<HarnessVersionReplacement, String> {
    let harness_id = HarnessId::new(row.0).map_err(|error| error.to_string())?;
    HarnessVersionReplacement::new(
        HarnessVersionRef::new(
            harness_id.clone(),
            HarnessVersionNumber::new(row.1).map_err(|error| error.to_string())?,
        ),
        HarnessVersionRef::new(
            harness_id,
            HarnessVersionNumber::new(row.2).map_err(|error| error.to_string())?,
        ),
    )
    .map_err(|error| error.to_string())
}

fn insert_version(transaction: &Transaction<'_>, version: &HarnessVersion) -> Result<(), String> {
    transaction
        .execute(
            "INSERT INTO harness_versions(harness_id,version_number,scope_json,configuration_contract_version,configuration_json,configuration_digest,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7)",
            params![
                version.reference.harness_id().as_str(),
                version.reference.version().get(),
                encode(&version.scope)?,
                HARNESS_CONFIGURATION_VERSION,
                encode(&version.configuration)?,
                version.configuration_digest,
                version.created_at.to_rfc3339(),
            ],
        )
        .map_err(|error| format!("Unable to publish Harness version: {error}"))?;
    Ok(())
}

fn encode<T: serde::Serialize>(value: &T) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| format!("Unable to encode Harness value: {error}"))
}

fn decode<T: serde::de::DeserializeOwned>(value: &str, label: &str) -> Result<T, String> {
    serde_json::from_str(value).map_err(|error| format!("Unable to decode {label}: {error}"))
}

fn parse_time(value: &str) -> Result<DateTime<Utc>, String> {
    DateTime::parse_from_rfc3339(value)
        .map(|value| value.with_timezone(&Utc))
        .map_err(|error| format!("Unable to decode Harness timestamp: {error}"))
}

fn expect_one(changed: usize, message: &str) -> Result<(), String> {
    if changed == 1 {
        Ok(())
    } else {
        Err(message.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness_engine::configuration::*;

    fn configuration(prompt: &str) -> HarnessConfiguration {
        HarnessConfiguration {
            identity_assignment: HarnessIdentityAssignmentPolicy::Unrestricted,
            prompt_prefix: HarnessPromptPrefixConfiguration {
                content: prompt.into(),
                initial_delivery: HarnessInitialDelivery::Prepend,
                context_compression_delivery: HarnessContextCompressionDelivery::Deferred,
            },
            skills: HarnessSkillsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: Vec::new(),
            },
            tools: HarnessToolsConfiguration {
                available_discovery_policy: HarnessDiscoveryPolicy::Whitelist,
                items: Vec::new(),
                schema_boundary: "Application tools".into(),
                mcp_servers: Vec::new(),
            },
            runtime: HarnessRuntimeConfiguration {
                preferred_model: None,
                sandbox: HarnessSandbox::WorkspaceWrite,
                approval_policy: HarnessApprovalPolicy::Never,
                authority_summary: "User authority".into(),
            },
            hooks: Vec::new(),
            update_policy: HarnessUpdatePolicy::NotConfigured {
                reason: "Not connected".into(),
            },
        }
    }

    fn record(id: &HarnessId) -> HarnessRecord {
        HarnessRecord {
            id: id.clone(),
            metadata: HarnessMetadata {
                name: "Plan builder".into(),
            },
            created_at: Utc::now(),
            updated_at: Utc::now(),
        }
    }

    #[test]
    fn one_draft_is_optimistically_replaced_and_consumed_by_publication() {
        let repository = SqliteHarnessCatalogRepository::in_memory();
        let id = HarnessId::new("opaque-id").unwrap();
        repository.create_harness(&record(&id)).unwrap();
        let draft = HarnessDraft {
            harness_id: id.clone(),
            based_on_version: None,
            configuration: configuration("Draft"),
            draft_revision: 1,
            saved_at: Utc::now(),
        };
        repository.save_draft(&draft, 0).unwrap();
        assert!(repository.save_draft(&draft, 0).is_err());

        let version = HarnessVersion::build(
            HarnessVersionRef::new(id.clone(), HarnessVersionNumber::new(1).unwrap()),
            HarnessVersionScope::Reusable,
            draft.configuration.clone(),
            Utc::now(),
        )
        .unwrap();
        repository.publish(&version, Some(1)).unwrap();

        assert_eq!(
            repository.version(&version.reference).unwrap(),
            Some(version)
        );
        assert_eq!(repository.draft(&id).unwrap(), None);
    }

    #[test]
    fn replacement_is_a_property_of_the_published_source_version() {
        let repository = SqliteHarnessCatalogRepository::in_memory();
        let id = HarnessId::new("opaque-id").unwrap();
        repository.create_harness(&record(&id)).unwrap();
        let first = HarnessVersion::build(
            HarnessVersionRef::new(id.clone(), HarnessVersionNumber::new(1).unwrap()),
            HarnessVersionScope::Reusable,
            configuration("One"),
            Utc::now(),
        )
        .unwrap();
        let second = HarnessVersion::build(
            HarnessVersionRef::new(id.clone(), HarnessVersionNumber::new(2).unwrap()),
            HarnessVersionScope::Reusable,
            configuration("Two"),
            Utc::now(),
        )
        .unwrap();
        repository.publish(&first, None).unwrap();
        repository.publish(&second, None).unwrap();
        let replacement =
            HarnessVersionReplacement::new(first.reference.clone(), second.reference.clone())
                .unwrap();

        repository
            .order_replacement(&replacement, Utc::now())
            .unwrap();

        assert_eq!(
            repository.replacement(&first.reference).unwrap(),
            Some(replacement)
        );
    }
}
