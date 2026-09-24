use super::{
    capability_profile::CapabilityProfile,
    ports::{CapabilityProfileRepository, CapabilityProfileRepositoryError},
};
use rusqlite::{params, Connection, ErrorCode, OptionalExtension};
use serde_json::Value;
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Arc, Mutex, MutexGuard},
};

use crate::persistence::{ActiveDatabase, ManagedOperationError};

pub(crate) const CAPABILITY_PROFILE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS execution_capability_profiles (
    capability_profile_id TEXT PRIMARY KEY CHECK (length(trim(capability_profile_id)) > 0),
    revision INTEGER NOT NULL CHECK (revision > 0),
    profile_json TEXT NOT NULL CHECK (json_valid(profile_json))
);

CREATE INDEX IF NOT EXISTS execution_capability_profiles_by_id
ON execution_capability_profiles(capability_profile_id COLLATE NOCASE);

CREATE TABLE IF NOT EXISTS execution_default_capability_profile (
    singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
    capability_profile_id TEXT NOT NULL REFERENCES execution_capability_profiles(capability_profile_id) ON DELETE RESTRICT
);
CREATE TABLE IF NOT EXISTS execution_model_catalogues (
    configuration_ref TEXT PRIMARY KEY,
    catalogue_json TEXT NOT NULL CHECK (json_valid(catalogue_json))
);
"#;

#[derive(Default)]
pub(crate) struct InMemoryCapabilityProfileRepository {
    profiles: Mutex<BTreeMap<String, CapabilityProfile>>,
    default_profile: Mutex<Option<String>>,
    model_catalogues: Mutex<BTreeMap<String, super::StoredModelCatalogue>>,
}

impl CapabilityProfileRepository for InMemoryCapabilityProfileRepository {
    fn model_catalogue(
        &self,
        configuration_ref: &str,
    ) -> Result<Option<super::StoredModelCatalogue>, CapabilityProfileRepositoryError> {
        Ok(self
            .model_catalogues
            .lock()
            .map_err(|_| {
                CapabilityProfileRepositoryError::Storage("Model catalogue lock unavailable".into())
            })?
            .get(configuration_ref)
            .cloned())
    }
    fn save_model_catalogue(
        &self,
        configuration_ref: &str,
        catalogue: &super::StoredModelCatalogue,
    ) -> Result<(), CapabilityProfileRepositoryError> {
        self.model_catalogues
            .lock()
            .map_err(|_| {
                CapabilityProfileRepositoryError::Storage("Model catalogue lock unavailable".into())
            })?
            .insert(configuration_ref.into(), catalogue.clone());
        Ok(())
    }
    fn default_profile(
        &self,
    ) -> Result<Option<CapabilityProfile>, CapabilityProfileRepositoryError> {
        let profiles = self.lock()?;
        Ok(self
            .default_profile_id()?
            .and_then(|id| profiles.get(&id).cloned()))
    }
    fn default_profile_id(&self) -> Result<Option<String>, CapabilityProfileRepositoryError> {
        Ok(self
            .default_profile
            .lock()
            .map_err(|_| {
                CapabilityProfileRepositoryError::Storage("Default profile lock unavailable".into())
            })?
            .clone())
    }
    fn set_default_profile(&self, id: &str) -> Result<(), CapabilityProfileRepositoryError> {
        let profiles = self.lock()?;
        if !profiles.contains_key(id) {
            return Err(CapabilityProfileRepositoryError::NotFound(id.into()));
        }
        *self.default_profile.lock().map_err(|_| {
            CapabilityProfileRepositoryError::Storage("Default profile lock unavailable".into())
        })? = Some(id.into());
        Ok(())
    }
    fn list(&self) -> Result<Vec<CapabilityProfile>, CapabilityProfileRepositoryError> {
        Ok(self.lock()?.values().cloned().collect())
    }

    fn find(
        &self,
        capability_profile_id: &str,
    ) -> Result<Option<CapabilityProfile>, CapabilityProfileRepositoryError> {
        Ok(self.lock()?.get(capability_profile_id).cloned())
    }

    fn insert(
        &self,
        capability_profile: &CapabilityProfile,
    ) -> Result<(), CapabilityProfileRepositoryError> {
        validate_profile(capability_profile)?;
        let mut profiles = self.lock()?;
        if profiles.contains_key(&capability_profile.capability_profile_id) {
            return Err(CapabilityProfileRepositoryError::AlreadyExists(
                capability_profile.capability_profile_id.clone(),
            ));
        }
        profiles.insert(
            capability_profile.capability_profile_id.clone(),
            capability_profile.clone(),
        );
        Ok(())
    }

    fn replace(
        &self,
        capability_profile: &CapabilityProfile,
        expected_revision: u64,
    ) -> Result<(), CapabilityProfileRepositoryError> {
        validate_profile(capability_profile)?;
        let mut profiles = self.lock()?;
        let Some(current) = profiles.get(&capability_profile.capability_profile_id) else {
            return Err(CapabilityProfileRepositoryError::NotFound(
                capability_profile.capability_profile_id.clone(),
            ));
        };
        if current.revision != expected_revision {
            return Err(CapabilityProfileRepositoryError::RevisionConflict {
                capability_profile_id: capability_profile.capability_profile_id.clone(),
                expected_revision,
            });
        }
        profiles.insert(
            capability_profile.capability_profile_id.clone(),
            capability_profile.clone(),
        );
        Ok(())
    }

    fn remove(&self, capability_profile_id: &str) -> Result<(), CapabilityProfileRepositoryError> {
        let mut profiles = self.lock()?;
        if self.default_profile_id()?.as_deref() == Some(capability_profile_id) {
            return Err(CapabilityProfileRepositoryError::Storage(
                "Choose another default Capability Profile before deleting this one".into(),
            ));
        }
        if profiles.remove(capability_profile_id).is_some() {
            Ok(())
        } else {
            Err(CapabilityProfileRepositoryError::NotFound(
                capability_profile_id.to_owned(),
            ))
        }
    }
}

impl InMemoryCapabilityProfileRepository {
    fn lock(
        &self,
    ) -> Result<MutexGuard<'_, BTreeMap<String, CapabilityProfile>>, CapabilityProfileRepositoryError>
    {
        self.profiles.lock().map_err(|_| {
            CapabilityProfileRepositoryError::Storage(
                "Capability Profile storage lock was poisoned".into(),
            )
        })
    }
}

pub(crate) struct SqliteCapabilityProfileRepository {
    database: Arc<ActiveDatabase>,
}

impl SqliteCapabilityProfileRepository {
    pub(crate) fn from_database(database: Arc<ActiveDatabase>) -> Self {
        Self { database }
    }

    pub(crate) fn open(database_path: &Path) -> Result<Self, CapabilityProfileRepositoryError> {
        ActiveDatabase::open(database_path, initialize_capability_profile_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .map_err(|error| CapabilityProfileRepositoryError::Storage(error.to_string()))
    }

    pub(crate) fn new(connection: Connection) -> Result<Self, CapabilityProfileRepositoryError> {
        ActiveDatabase::from_connection(connection, initialize_capability_profile_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .map_err(|error| CapabilityProfileRepositoryError::Storage(error.to_string()))
    }

    fn read<T>(
        &self,
        operation: &'static str,
        read: impl FnOnce(&Connection) -> Result<T, CapabilityProfileRepositoryError>,
    ) -> Result<T, CapabilityProfileRepositoryError> {
        self.database.read(operation, read).map_err(managed_error)
    }

    fn write<T>(
        &self,
        operation: &'static str,
        write: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, CapabilityProfileRepositoryError>,
    ) -> Result<T, CapabilityProfileRepositoryError> {
        self.database.write(operation, write).map_err(managed_error)
    }
}

impl CapabilityProfileRepository for SqliteCapabilityProfileRepository {
    fn model_catalogue(
        &self,
        configuration_ref: &str,
    ) -> Result<Option<super::StoredModelCatalogue>, CapabilityProfileRepositoryError> {
        self.read("read model catalogue", |connection| {
            let json: Option<String> = connection.query_row("SELECT catalogue_json FROM execution_model_catalogues WHERE configuration_ref=?1", [configuration_ref], |row| row.get(0)).optional().map_err(storage_error("read model catalogue"))?;
            json.map(|json| serde_json::from_str(&json).map_err(|error| CapabilityProfileRepositoryError::Storage(format!("Invalid cached model catalogue: {error}")))).transpose()
        })
    }
    fn save_model_catalogue(
        &self,
        configuration_ref: &str,
        catalogue: &super::StoredModelCatalogue,
    ) -> Result<(), CapabilityProfileRepositoryError> {
        let json = serde_json::to_string(catalogue)
            .map_err(|error| CapabilityProfileRepositoryError::Storage(error.to_string()))?;
        self.write("save model catalogue", |transaction| {
            transaction.execute("INSERT INTO execution_model_catalogues(configuration_ref,catalogue_json) VALUES(?1,?2) ON CONFLICT(configuration_ref) DO UPDATE SET catalogue_json=excluded.catalogue_json", params![configuration_ref,json]).map_err(storage_error("save model catalogue"))?;
            Ok(())
        })
    }
    fn default_profile(
        &self,
    ) -> Result<Option<CapabilityProfile>, CapabilityProfileRepositoryError> {
        self.read("read default Capability Profile", |connection| connection.query_row("SELECT p.capability_profile_id,p.revision,p.profile_json FROM execution_capability_profiles p JOIN execution_default_capability_profile d ON d.capability_profile_id=p.capability_profile_id WHERE d.singleton=1", [], profile_row).optional().map_err(storage_error("read default Capability Profile"))?.map(|row| decode_profile_row(connection, row)).transpose())
    }
    fn default_profile_id(&self) -> Result<Option<String>, CapabilityProfileRepositoryError> {
        self.read("read default Capability Profile identity", |connection| connection.query_row("SELECT capability_profile_id FROM execution_default_capability_profile WHERE singleton=1", [], |row| row.get(0)).optional().map_err(storage_error("read default Capability Profile")))
    }
    fn set_default_profile(&self, id: &str) -> Result<(), CapabilityProfileRepositoryError> {
        self.write("set default Capability Profile", |connection| {
        let exists: bool = connection.query_row("SELECT EXISTS(SELECT 1 FROM execution_capability_profiles WHERE capability_profile_id=?1)", [id], |row| row.get(0)).map_err(storage_error("find default Capability Profile"))?;
        if !exists {
            return Err(CapabilityProfileRepositoryError::NotFound(id.into()));
        }
        connection.execute("INSERT INTO execution_default_capability_profile(singleton,capability_profile_id) VALUES(1,?1) ON CONFLICT(singleton) DO UPDATE SET capability_profile_id=excluded.capability_profile_id", [id]).map_err(storage_error("set default Capability Profile"))?;
        Ok(())
        })
    }
    fn list(&self) -> Result<Vec<CapabilityProfile>, CapabilityProfileRepositoryError> {
        self.read("list Capability Profiles", |connection| {
            let mut statement = connection
                .prepare(
                    "SELECT capability_profile_id,revision,profile_json \
                 FROM execution_capability_profiles ORDER BY capability_profile_id COLLATE NOCASE",
                )
                .map_err(storage_error("prepare Capability Profile list"))?;
            let rows = statement
                .query_map([], profile_row)
                .map_err(storage_error("query Capability Profile list"))?
                .map(|row| row.map_err(storage_error("read Capability Profile row")))
                .collect::<Result<Vec<_>, _>>()?;
            drop(statement);
            rows.into_iter()
                .map(|row| decode_profile_row(connection, row))
                .collect()
        })
    }

    fn find(
        &self,
        capability_profile_id: &str,
    ) -> Result<Option<CapabilityProfile>, CapabilityProfileRepositoryError> {
        self.read("find Capability Profile", |connection| {
            connection
                .query_row(
                    "SELECT capability_profile_id,revision,profile_json \
                 FROM execution_capability_profiles WHERE capability_profile_id=?1",
                    [capability_profile_id],
                    profile_row,
                )
                .optional()
                .map_err(storage_error("read Capability Profile"))?
                .map(|row| decode_profile_row(connection, row))
                .transpose()
        })
    }

    fn insert(
        &self,
        capability_profile: &CapabilityProfile,
    ) -> Result<(), CapabilityProfileRepositoryError> {
        validate_profile(capability_profile)?;
        let json = encode_profile(capability_profile)?;
        self.write("insert Capability Profile", |transaction| {
            validate_registered_routes(transaction, capability_profile)?;
            match transaction.execute(
                "INSERT INTO execution_capability_profiles(capability_profile_id,revision,profile_json) \
                 VALUES(?1,?2,?3)",
                params![
                    capability_profile.capability_profile_id,
                    persisted_revision(capability_profile.revision)?,
                    json,
                ],
            ) {
                Ok(1) => Ok(()),
                Ok(changed) => Err(CapabilityProfileRepositoryError::Storage(format!(
                    "Capability Profile insert affected {changed} rows"
                ))),
                Err(error) if error.sqlite_error_code() == Some(ErrorCode::ConstraintViolation) => {
                    Err(CapabilityProfileRepositoryError::AlreadyExists(
                        capability_profile.capability_profile_id.clone(),
                    ))
                }
                Err(error) => Err(storage_error("insert Capability Profile")(error)),
            }
        })
    }

    fn replace(
        &self,
        capability_profile: &CapabilityProfile,
        expected_revision: u64,
    ) -> Result<(), CapabilityProfileRepositoryError> {
        validate_profile(capability_profile)?;
        let json = encode_profile(capability_profile)?;
        self.write("replace Capability Profile", |transaction| {
        validate_registered_routes(transaction, capability_profile)?;
        let changed = transaction
            .execute(
                "UPDATE execution_capability_profiles SET revision=?2,profile_json=?3 \
                 WHERE capability_profile_id=?1 AND revision=?4",
                params![
                    capability_profile.capability_profile_id,
                    persisted_revision(capability_profile.revision)?,
                    json,
                    persisted_revision(expected_revision)?,
                ],
            )
            .map_err(storage_error("replace Capability Profile"))?;
        if changed == 1 {
            return Ok(());
        }
        let exists = transaction
            .query_row(
                "SELECT EXISTS(SELECT 1 FROM execution_capability_profiles WHERE capability_profile_id=?1)",
                [&capability_profile.capability_profile_id],
                |row| row.get::<_, bool>(0),
            )
            .map_err(storage_error(
                "inspect Capability Profile revision conflict",
            ))?;
        if exists {
            Err(CapabilityProfileRepositoryError::RevisionConflict {
                capability_profile_id: capability_profile.capability_profile_id.clone(),
                expected_revision,
            })
        } else {
            Err(CapabilityProfileRepositoryError::NotFound(
                capability_profile.capability_profile_id.clone(),
            ))
        }
        })
    }

    fn remove(&self, capability_profile_id: &str) -> Result<(), CapabilityProfileRepositoryError> {
        self.write("delete Capability Profile", |transaction| match transaction
            .execute(
                "DELETE FROM execution_capability_profiles WHERE capability_profile_id=?1",
                [capability_profile_id],
            )
            .map_err(storage_error("delete Capability Profile"))?
        {
            1 => Ok(()),
            0 => Err(CapabilityProfileRepositoryError::NotFound(
                capability_profile_id.to_owned(),
            )),
            changed => Err(CapabilityProfileRepositoryError::Storage(format!(
                "Capability Profile delete affected {changed} rows"
            ))),
        })
    }
}

pub(crate) fn initialize_capability_profile_storage(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(CAPABILITY_PROFILE_SCHEMA)
        .map_err(|error| format!("Unable to initialize Capability Profile storage: {error}"))?;
    connection
        .execute_batch(crate::execution_devices::SCHEMA)
        .map_err(|error| format!("Unable to initialize execution device storage: {error}"))?;
    connection
        .execute(
            "INSERT OR IGNORE INTO execution_devices(device_id,display_name,connection_json,updated_at) VALUES('local','This device','{\"kind\":\"local\"}',?1)",
            [chrono::Utc::now().to_rfc3339()],
        )
        .map_err(|error| format!("Unable to initialize the local execution device: {error}"))?;
    migrate_profile_route_references(connection)
}

fn managed_error(
    error: ManagedOperationError<CapabilityProfileRepositoryError>,
) -> CapabilityProfileRepositoryError {
    match error {
        ManagedOperationError::Infrastructure(error) => {
            CapabilityProfileRepositoryError::Storage(error.to_string())
        }
        ManagedOperationError::Domain(error) => error,
    }
}

type CapabilityProfileRow = (String, i64, String);

fn profile_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CapabilityProfileRow> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
}

fn decode_profile_row(
    connection: &Connection,
    (stored_id, stored_revision, json): CapabilityProfileRow,
) -> Result<CapabilityProfile, CapabilityProfileRepositoryError> {
    let mut value: Value = serde_json::from_str(&json).map_err(|error| {
        CapabilityProfileRepositoryError::InvalidStoredProfile(error.to_string())
    })?;
    hydrate_profile_routes(connection, &mut value)?;
    let profile: CapabilityProfile = serde_json::from_value(value).map_err(|error| {
        CapabilityProfileRepositoryError::InvalidStoredProfile(error.to_string())
    })?;
    profile
        .validate()
        .map_err(CapabilityProfileRepositoryError::InvalidStoredProfile)?;
    if stored_revision <= 0
        || profile.capability_profile_id != stored_id
        || profile.revision != stored_revision as u64
    {
        return Err(CapabilityProfileRepositoryError::InvalidStoredProfile(
            "indexed identity or revision does not match the stored document".into(),
        ));
    }
    Ok(profile)
}

fn encode_profile(profile: &CapabilityProfile) -> Result<String, CapabilityProfileRepositoryError> {
    let mut value = serde_json::to_value(profile).map_err(|error| {
        CapabilityProfileRepositoryError::Storage(format!(
            "Unable to encode Capability Profile: {error}"
        ))
    })?;
    strip_resolved_profile_routes(&mut value)?;
    serde_json::to_string(&value).map_err(|error| {
        CapabilityProfileRepositoryError::Storage(format!(
            "Unable to encode Capability Profile: {error}"
        ))
    })
}

fn strip_resolved_profile_routes(
    value: &mut Value,
) -> Result<(), CapabilityProfileRepositoryError> {
    for_each_execution_mut(value, |execution| {
        execution.remove("deviceName");
        execution.remove("connection");
        Ok(())
    })
}

fn hydrate_profile_routes(
    connection: &Connection,
    value: &mut Value,
) -> Result<(), CapabilityProfileRepositoryError> {
    for_each_execution_mut(value, |execution| {
        if execution.contains_key("connection") && execution.contains_key("deviceName") {
            return Ok(());
        }
        let device_id = execution
            .get("deviceId")
            .and_then(Value::as_str)
            .ok_or_else(|| {
                CapabilityProfileRepositoryError::InvalidStoredProfile(
                    "Capability Profile route does not identify a device".into(),
                )
            })?;
        let device = connection
            .query_row(
                "SELECT display_name,connection_json FROM execution_devices WHERE device_id=?1",
                [device_id],
                |row| Ok((row.get::<_, String>(0)?, row.get::<_, Option<String>>(1)?)),
            )
            .optional()
            .map_err(storage_error("resolve Capability Profile device"))?
            .ok_or_else(|| {
                CapabilityProfileRepositoryError::InvalidStoredProfile(format!(
                    "Capability Profile references unregistered device `{device_id}`"
                ))
            })?;
        let connection_value = device
            .1
            .as_deref()
            .ok_or_else(|| {
                CapabilityProfileRepositoryError::InvalidStoredProfile(format!(
                    "Capability Profile device `{device_id}` has no developer connection"
                ))
            })
            .and_then(|json| {
                serde_json::from_str(json).map_err(|error| {
                    CapabilityProfileRepositoryError::InvalidStoredProfile(format!(
                        "Capability Profile device `{device_id}` has an invalid connection: {error}"
                    ))
                })
            })?;
        execution.insert("deviceName".into(), Value::String(device.0));
        execution.insert("connection".into(), connection_value);
        Ok(())
    })
}

fn for_each_execution_mut(
    value: &mut Value,
    mut visit: impl FnMut(
        &mut serde_json::Map<String, Value>,
    ) -> Result<(), CapabilityProfileRepositoryError>,
) -> Result<(), CapabilityProfileRepositoryError> {
    if let Some(execution) = value.get_mut("execution").and_then(Value::as_object_mut) {
        visit(execution)?;
    }
    if let Some(routes) = value.get_mut("routePolicies").and_then(Value::as_array_mut) {
        for route in routes {
            let execution = route
                .get_mut("execution")
                .and_then(Value::as_object_mut)
                .ok_or_else(|| {
                    CapabilityProfileRepositoryError::InvalidStoredProfile(
                        "Capability Profile route has no execution reference".into(),
                    )
                })?;
            visit(execution)?;
        }
    }
    Ok(())
}

fn validate_registered_routes(
    connection: &Connection,
    profile: &CapabilityProfile,
) -> Result<(), CapabilityProfileRepositoryError> {
    let bindings = std::iter::once(&profile.execution)
        .chain(profile.route_policies.iter().map(|route| &route.execution));
    for binding in bindings {
        let stored_connection = connection
            .query_row(
                "SELECT connection_json FROM execution_devices WHERE device_id=?1",
                [&binding.device_id],
                |row| row.get::<_, Option<String>>(0),
            )
            .optional()
            .map_err(storage_error("resolve Capability Profile device"))?
            .flatten()
            .ok_or_else(|| {
                CapabilityProfileRepositoryError::Storage(format!(
                    "Configure device `{}` in Technical Settings before using it in a Capability Profile",
                    binding.device_id
                ))
            })?;
        let stored_connection = serde_json::from_str::<
            crate::execution_targets::domain::ExecutionConnection,
        >(&stored_connection)
        .map_err(|error| {
            CapabilityProfileRepositoryError::Storage(format!(
                "Configured device `{}` has an invalid connection: {error}",
                binding.device_id
            ))
        })?;
        if stored_connection != binding.connection {
            return Err(CapabilityProfileRepositoryError::Storage(format!(
                "Capability Profile route for device `{}` does not match its developer connection",
                binding.device_id
            )));
        }
    }
    Ok(())
}

/// Moves legacy embedded connection definitions into device-owned rows and rewrites profiles to
/// stable route references. Existing Session snapshots remain untouched.
pub(crate) fn migrate_profile_route_references(connection: &Connection) -> Result<(), String> {
    let mut statement = connection
        .prepare("SELECT capability_profile_id,profile_json FROM execution_capability_profiles")
        .map_err(|error| error.to_string())?;
    let rows = statement
        .query_map([], |row| {
            Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?))
        })
        .map_err(|error| error.to_string())?
        .collect::<Result<Vec<_>, _>>()
        .map_err(|error| error.to_string())?;
    drop(statement);
    for (profile_id, json) in rows {
        let mut value: Value = serde_json::from_str(&json)
            .map_err(|error| format!("Capability Profile `{profile_id}` is invalid: {error}"))?;
        let mut bindings = Vec::new();
        for_each_execution_mut(&mut value, |execution| {
            let Some(connection_value) = execution.get("connection").cloned() else {
                return Ok(());
            };
            let device_id = execution
                .get("deviceId")
                .and_then(Value::as_str)
                .ok_or_else(|| {
                    CapabilityProfileRepositoryError::InvalidStoredProfile(
                        "Legacy route does not identify a device".into(),
                    )
                })?
                .to_string();
            let display_name = execution
                .get("deviceName")
                .and_then(Value::as_str)
                .unwrap_or(&device_id)
                .to_string();
            bindings.push((device_id, display_name, connection_value));
            Ok(())
        })
        .map_err(|error| error.to_string())?;
        for (device_id, display_name, connection_value) in bindings {
            let encoded = serde_json::to_string(&connection_value).map_err(|e| e.to_string())?;
            let existing = connection
                .query_row(
                    "SELECT connection_json FROM execution_devices WHERE device_id=?1",
                    [&device_id],
                    |row| row.get::<_, Option<String>>(0),
                )
                .optional()
                .map_err(|error| error.to_string())?
                .flatten();
            if let Some(existing) = existing {
                let existing: Value = serde_json::from_str(&existing).map_err(|e| e.to_string())?;
                if existing != connection_value {
                    return Err(format!(
                        "Device `{device_id}` has conflicting developer connection definitions"
                    ));
                }
            }
            connection
                .execute(
                    "INSERT INTO execution_devices(device_id,display_name,connection_json,updated_at) VALUES(?1,?2,?3,?4) ON CONFLICT(device_id) DO UPDATE SET display_name=excluded.display_name,connection_json=COALESCE(execution_devices.connection_json,excluded.connection_json),updated_at=excluded.updated_at",
                    params![device_id, display_name, encoded, chrono::Utc::now().to_rfc3339()],
                )
                .map_err(|error| error.to_string())?;
        }
        strip_resolved_profile_routes(&mut value).map_err(|error| error.to_string())?;
        connection
            .execute(
                "UPDATE execution_capability_profiles SET profile_json=?2 WHERE capability_profile_id=?1",
                params![profile_id, serde_json::to_string(&value).map_err(|e| e.to_string())?],
            )
            .map_err(|error| error.to_string())?;
    }
    Ok(())
}

fn validate_profile(profile: &CapabilityProfile) -> Result<(), CapabilityProfileRepositoryError> {
    profile
        .validate()
        .map_err(CapabilityProfileRepositoryError::InvalidStoredProfile)
}

fn persisted_revision(revision: u64) -> Result<i64, CapabilityProfileRepositoryError> {
    i64::try_from(revision).map_err(|_| {
        CapabilityProfileRepositoryError::Storage(
            "Capability Profile revision exceeds SQLite's supported range".into(),
        )
    })
}

fn storage_error(
    action: &'static str,
) -> impl FnOnce(rusqlite::Error) -> CapabilityProfileRepositoryError {
    move |error| CapabilityProfileRepositoryError::Storage(format!("Unable to {action}: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_configuration::{
        runtime_profile::CapabilitySet, CAPABILITY_PROFILE_CONTRACT_VERSION,
    };
    use crate::execution_targets::domain::{ExecutionBinding, ExecutionConnection};

    fn profile(id: &str, revision: u64) -> CapabilityProfile {
        CapabilityProfile {
            execution: Default::default(),
            contract_version: CAPABILITY_PROFILE_CONTRACT_VERSION,
            defaults: Default::default(),
            route_policies: Vec::new(),
            default_route_id: None,
            capability_profile_id: id.into(),
            name: format!("{id} profile"),
            revision,
            allowed_capabilities: CapabilitySet {
                models: ["codex-a".into()].into_iter().collect(),
                ..CapabilitySet::default()
            },
        }
    }

    #[test]
    fn sqlite_repository_persists_profiles_and_checks_revision() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("capability-profiles.sqlite");
        let repository = SqliteCapabilityProfileRepository::open(&path).unwrap();
        repository.insert(&profile("review", 1)).unwrap();
        repository.insert(&profile("build", 1)).unwrap();

        let mut replacement = profile("review", 2);
        repository.replace(&replacement, 1).unwrap();
        replacement.revision = 3;
        assert!(matches!(
            repository.replace(&replacement, 1),
            Err(CapabilityProfileRepositoryError::RevisionConflict { .. })
        ));
        drop(repository);

        let reopened = SqliteCapabilityProfileRepository::open(&path).unwrap();
        assert_eq!(
            reopened
                .list()
                .unwrap()
                .into_iter()
                .map(|profile| profile.capability_profile_id)
                .collect::<Vec<_>>(),
            vec!["build", "review"]
        );
        assert_eq!(reopened.find("review").unwrap().unwrap().revision, 2);
        reopened.remove("review").unwrap();
        assert!(reopened.find("review").unwrap().is_none());
    }

    #[test]
    fn sqlite_repository_rejects_duplicate_and_missing_mutations() {
        let repository = SqliteCapabilityProfileRepository::new(
            Connection::open_in_memory().expect("in-memory database"),
        )
        .unwrap();
        repository.insert(&profile("review", 1)).unwrap();
        assert!(matches!(
            repository.insert(&profile("review", 1)),
            Err(CapabilityProfileRepositoryError::AlreadyExists(id)) if id == "review"
        ));
        assert!(matches!(
            repository.replace(&profile("missing", 2), 1),
            Err(CapabilityProfileRepositoryError::NotFound(id)) if id == "missing"
        ));
        assert!(matches!(
            repository.remove("missing"),
            Err(CapabilityProfileRepositoryError::NotFound(id)) if id == "missing"
        ));
    }

    #[test]
    fn legacy_embedded_route_is_migrated_to_a_device_owned_connection() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(CAPABILITY_PROFILE_SCHEMA).unwrap();
        connection
            .execute_batch(crate::execution_devices::SCHEMA)
            .unwrap();
        let mut legacy = profile("remote", 1);
        legacy.execution = ExecutionBinding {
            device_id: "worker".into(),
            device_name: "Remote worker".into(),
            provider: "codex".into(),
            configuration_ref: "worker-codex".into(),
            connection: ExecutionConnection::Ssh {
                target: "orchid@worker".into(),
                host_executable: "C:\\Orchid\\host.exe".into(),
            },
        };
        connection
            .execute(
                "INSERT INTO execution_capability_profiles(capability_profile_id,revision,profile_json) VALUES(?1,?2,?3)",
                params![legacy.capability_profile_id, 1, serde_json::to_string(&legacy).unwrap()],
            )
            .unwrap();

        initialize_capability_profile_storage(&connection).unwrap();

        let stored_connection: String = connection
            .query_row(
                "SELECT connection_json FROM execution_devices WHERE device_id='worker'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        assert_eq!(
            serde_json::from_str::<ExecutionConnection>(&stored_connection).unwrap(),
            legacy.execution.connection
        );
        let stored_profile: String = connection
            .query_row(
                "SELECT profile_json FROM execution_capability_profiles WHERE capability_profile_id='remote'",
                [],
                |row| row.get(0),
            )
            .unwrap();
        let stored_profile: Value = serde_json::from_str(&stored_profile).unwrap();
        assert!(stored_profile["execution"].get("connection").is_none());
        assert!(stored_profile["execution"].get("deviceName").is_none());
    }

    #[test]
    fn legacy_route_migration_rejects_conflicting_device_connections() {
        let connection = Connection::open_in_memory().unwrap();
        connection.execute_batch(CAPABILITY_PROFILE_SCHEMA).unwrap();
        connection
            .execute_batch(crate::execution_devices::SCHEMA)
            .unwrap();
        connection
            .execute(
                "INSERT INTO execution_devices(device_id,display_name,connection_json,updated_at) VALUES('worker','Worker',?1,'now')",
                [serde_json::to_string(&ExecutionConnection::Ssh {
                    target: "orchid@registered".into(),
                    host_executable: "C:\\Orchid\\host.exe".into(),
                })
                .unwrap()],
            )
            .unwrap();
        let mut legacy = profile("remote", 1);
        legacy.execution = ExecutionBinding {
            device_id: "worker".into(),
            device_name: "Worker".into(),
            provider: "codex".into(),
            configuration_ref: "worker-codex".into(),
            connection: ExecutionConnection::Ssh {
                target: "orchid@different".into(),
                host_executable: "C:\\Orchid\\host.exe".into(),
            },
        };
        connection
            .execute(
                "INSERT INTO execution_capability_profiles(capability_profile_id,revision,profile_json) VALUES(?1,?2,?3)",
                params![legacy.capability_profile_id, 1, serde_json::to_string(&legacy).unwrap()],
            )
            .unwrap();

        let error = initialize_capability_profile_storage(&connection).unwrap_err();
        assert!(error.contains("conflicting developer connection definitions"));
    }
}
