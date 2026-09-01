use super::{
    capability_profile::CapabilityProfile,
    ports::{CapabilityProfileRepository, CapabilityProfileRepositoryError},
};
use rusqlite::{params, Connection, ErrorCode, OptionalExtension};
use std::{
    collections::BTreeMap,
    path::Path,
    sync::{Mutex, MutexGuard},
};

pub(crate) const CAPABILITY_PROFILE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS execution_capability_profiles (
    capability_profile_id TEXT PRIMARY KEY CHECK (length(trim(capability_profile_id)) > 0),
    revision INTEGER NOT NULL CHECK (revision > 0),
    profile_json TEXT NOT NULL CHECK (json_valid(profile_json))
);

CREATE INDEX IF NOT EXISTS execution_capability_profiles_by_id
ON execution_capability_profiles(capability_profile_id COLLATE NOCASE);
"#;

#[derive(Default)]
pub(crate) struct InMemoryCapabilityProfileRepository {
    profiles: Mutex<BTreeMap<String, CapabilityProfile>>,
}

impl CapabilityProfileRepository for InMemoryCapabilityProfileRepository {
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
        if self.lock()?.remove(capability_profile_id).is_some() {
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
    connection: Mutex<Connection>,
}

impl SqliteCapabilityProfileRepository {
    pub(crate) fn open(database_path: &Path) -> Result<Self, CapabilityProfileRepositoryError> {
        let connection = crate::storage::open_active_database(database_path)
            .map_err(CapabilityProfileRepositoryError::Storage)?;
        Self::new(connection)
    }

    pub(crate) fn new(connection: Connection) -> Result<Self, CapabilityProfileRepositoryError> {
        crate::storage::configure_sqlite_connection(&connection).map_err(|error| {
            CapabilityProfileRepositoryError::Storage(format!(
                "Unable to configure Capability Profile storage: {error}"
            ))
        })?;
        connection
            .execute_batch(CAPABILITY_PROFILE_SCHEMA)
            .map_err(|error| {
                CapabilityProfileRepositoryError::Storage(format!(
                    "Unable to initialize Capability Profile storage: {error}"
                ))
            })?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock(&self) -> Result<MutexGuard<'_, Connection>, CapabilityProfileRepositoryError> {
        self.connection.lock().map_err(|_| {
            CapabilityProfileRepositoryError::Storage(
                "Capability Profile storage lock was poisoned".into(),
            )
        })
    }
}

impl CapabilityProfileRepository for SqliteCapabilityProfileRepository {
    fn list(&self) -> Result<Vec<CapabilityProfile>, CapabilityProfileRepositoryError> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT capability_profile_id,revision,profile_json \
                 FROM execution_capability_profiles ORDER BY capability_profile_id COLLATE NOCASE",
            )
            .map_err(storage_error("prepare Capability Profile list"))?;
        let profiles = statement
            .query_map([], profile_row)
            .map_err(storage_error("query Capability Profile list"))?
            .map(|row| {
                row.map_err(storage_error("read Capability Profile row"))
                    .and_then(decode_profile_row)
            })
            .collect();
        profiles
    }

    fn find(
        &self,
        capability_profile_id: &str,
    ) -> Result<Option<CapabilityProfile>, CapabilityProfileRepositoryError> {
        self.lock()?
            .query_row(
                "SELECT capability_profile_id,revision,profile_json \
                 FROM execution_capability_profiles WHERE capability_profile_id=?1",
                [capability_profile_id],
                profile_row,
            )
            .optional()
            .map_err(storage_error("read Capability Profile"))?
            .map(decode_profile_row)
            .transpose()
    }

    fn insert(
        &self,
        capability_profile: &CapabilityProfile,
    ) -> Result<(), CapabilityProfileRepositoryError> {
        validate_profile(capability_profile)?;
        let json = encode_profile(capability_profile)?;
        match self.lock()?.execute(
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
    }

    fn replace(
        &self,
        capability_profile: &CapabilityProfile,
        expected_revision: u64,
    ) -> Result<(), CapabilityProfileRepositoryError> {
        validate_profile(capability_profile)?;
        let json = encode_profile(capability_profile)?;
        let connection = self.lock()?;
        let changed = connection
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
        let exists = connection
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
    }

    fn remove(&self, capability_profile_id: &str) -> Result<(), CapabilityProfileRepositoryError> {
        match self
            .lock()?
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
        }
    }
}

type CapabilityProfileRow = (String, i64, String);

fn profile_row(row: &rusqlite::Row<'_>) -> rusqlite::Result<CapabilityProfileRow> {
    Ok((row.get(0)?, row.get(1)?, row.get(2)?))
}

fn decode_profile_row(
    (stored_id, stored_revision, json): CapabilityProfileRow,
) -> Result<CapabilityProfile, CapabilityProfileRepositoryError> {
    let profile: CapabilityProfile = serde_json::from_str(&json).map_err(|error| {
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
    serde_json::to_string(profile).map_err(|error| {
        CapabilityProfileRepositoryError::Storage(format!(
            "Unable to encode Capability Profile: {error}"
        ))
    })
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

    fn profile(id: &str, revision: u64) -> CapabilityProfile {
        CapabilityProfile {
            contract_version: CAPABILITY_PROFILE_CONTRACT_VERSION,
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
}
