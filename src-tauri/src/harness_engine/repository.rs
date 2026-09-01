use super::domain::{HarnessBindingRecord, HarnessBindingStage};
use rusqlite::{params, Connection, OptionalExtension};
#[cfg(test)]
use std::path::Path;
use std:: sync::Arc;

use crate::persistence::ActiveDatabase;

pub(crate) const HARNESS_BINDING_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS session_harness_bindings (
    id TEXT PRIMARY KEY,
    contract_version TEXT NOT NULL CHECK (contract_version='harness-binding/v1'),
    session_id TEXT NOT NULL,
    runtime_instance_id TEXT NOT NULL,
    session_instance_token TEXT NOT NULL UNIQUE,
    stage TEXT NOT NULL CHECK (stage IN ('prepared','bound','retired')),
    harness_snapshot TEXT NOT NULL CHECK (json_valid(harness_snapshot)),
    mediation_plan TEXT NOT NULL CHECK (json_valid(mediation_plan)),
    configuration_digest TEXT NOT NULL,
    harness_token TEXT UNIQUE,
    source_workflow_instance_id TEXT NOT NULL,
    source_recipe_id TEXT NOT NULL,
    source_node_id TEXT NOT NULL,
    prepared_at TEXT NOT NULL,
    bound_at TEXT,
    retired_at TEXT,
    CHECK ((stage='prepared' AND harness_token IS NULL AND bound_at IS NULL AND retired_at IS NULL)
        OR (stage='bound' AND harness_token IS NOT NULL AND bound_at IS NOT NULL AND retired_at IS NULL)
        OR (stage='retired' AND harness_token IS NOT NULL AND bound_at IS NOT NULL AND retired_at IS NOT NULL)),
    FOREIGN KEY (session_id) REFERENCES agent_sessions(id)
);

CREATE UNIQUE INDEX IF NOT EXISTS one_current_harness_binding_per_session
ON session_harness_bindings(session_id) WHERE stage!='retired';

CREATE INDEX IF NOT EXISTS session_harness_bindings_by_stage
ON session_harness_bindings(stage, prepared_at, id);
"#;

pub(crate) trait HarnessBindingRepository: Send + Sync {
    fn insert_prepared(&self, binding: &HarnessBindingRecord) -> Result<(), String>;
    fn mark_bound(
        &self,
        binding_id: &str,
        harness_token: &str,
        bound_at: &str,
    ) -> Result<HarnessBindingRecord, String>;
    fn current_for_session(&self, session_id: &str)
        -> Result<Option<HarnessBindingRecord>, String>;
    fn non_retired(&self) -> Result<Vec<HarnessBindingRecord>, String>;
    fn retire(&self, binding_id: &str, retired_at: &str) -> Result<(), String>;
}

pub(crate) struct SqliteHarnessBindingRepository {
    database: Arc<ActiveDatabase>,
}

impl SqliteHarnessBindingRepository {
    pub(crate) fn from_database(database: Arc<ActiveDatabase>) -> Self {
        Self { database }
    }

    #[cfg(test)]
    pub(crate) fn new(connection: Connection) -> Result<Self, String> {
        ActiveDatabase::from_connection( connection, initialize_harness_binding_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .map_err(|error| error.to_string())
    }

    #[cfg(test)]
    pub( crate) fn open(path: &Path) -> Result<Self, String> {
        ActiveDatabase::open(path, initialize_harness_binding_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .map_err(|error| error.to_string())
    }

    fn read<T>(
        &self,
        operation: &'static str,
        read: impl FnOnce(&Connection) -> Result<T, String>,
    ) -> Result<T, String> {
        self.database
            .read(operation, read)
            .map_err(|error| error.into_string())
    }

    fn write<T>(
        &self,
        operation: &'static str,
        write: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        self.database
            .write(operation, write)
            .map_err(|error| error.into_string())
    }
}

impl HarnessBindingRepository for SqliteHarnessBindingRepository {
    fn insert_prepared(&self, binding: &HarnessBindingRecord) -> Result<(), String> {
        if binding.stage != HarnessBindingStage::Prepared
            || binding.harness_token.is_some()
            || binding.bound_at.is_some()
        {
            return Err("Only a prepared immutable Harness binding can be inserted.".to_string());
        }
        binding.verify_digest()?;
        self.write("persist prepared Harness binding", |transaction| {
            transaction
            .execute(
                "INSERT INTO session_harness_bindings(id,contract_version,session_id,runtime_instance_id,session_instance_token,stage,harness_snapshot,mediation_plan,configuration_digest,harness_token,source_workflow_instance_id,source_recipe_id,source_node_id,prepared_at,bound_at,retired_at) VALUES(?1,'harness-binding/v1',?2,?3,?4,'prepared',?5,?6,?7,NULL,?8,?9,?10,?11,NULL,NULL)",
                params![
                    binding.id,
                    binding.session_id,
                    binding.runtime_instance_id,
                    binding.session_instance_token,
                    binding.harness_snapshot,
                    binding.mediation_plan,
                    binding.configuration_digest,
                    binding.source_workflow_instance_id,
                    binding.source_recipe_id,
                    binding.source_node_id,
                    binding.prepared_at,
                ],
            )
            .map_err(|error| {
                if error.to_string().contains("one_current_harness_binding_per_session")
                    || error
                        .to_string()
                        .contains("session_harness_bindings.session_id")
                {
                    "The Agent Session already has a non-retired Harness binding.".to_string()
                } else {
                    format!("Unable to persist prepared Harness binding: {error}")
                }
            })?;
        Ok(())
    })
    }

    fn mark_bound(
        &self,
        binding_id: &str,
        harness_token: &str,
        bound_at: &str,
    ) -> Result<HarnessBindingRecord, String> { self.write("persist bound Harness binding", |transaction| {
        let changed = transaction
            .execute(
                "UPDATE session_harness_bindings SET stage='bound',harness_token=?2,bound_at=?3 WHERE id=?1 AND stage='prepared'",
                params![binding_id, harness_token, bound_at],
            )
            .map_err(|error| format!("Unable to persist bound Harness binding: {error}"))?;
        if changed != 1 {
            return Err("Harness binding was not in the prepared stage.".to_string());
        }
        load_binding(transaction, binding_id)?.ok_or_else(|| "Harness binding disappeared.".into())
    })
    }

    fn current_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<HarnessBindingRecord>, String> {
        self.read("load Session Harness binding", | connection| {
        connection
            .query_row(
                "SELECT id FROM session_harness_bindings WHERE session_id=?1 AND stage!='retired'",
                [session_id],
                |row| row.get::<_, String>(0),
            )
            .optional()
            .map_err(|error| format!("Unable to load Session Harness binding: {error}"))?
            .map(|id| load_binding(&connection, &id))
            .transpose()
            .map(Option::flatten)
    })
    }

    fn non_retired(&self) -> Result<Vec<HarnessBindingRecord>, String> {
        self.read("load retained Harness bindings", | connection| {
        let mut statement = connection
            .prepare("SELECT id FROM session_harness_bindings WHERE stage!='retired' ORDER BY prepared_at,id")
            .map_err(|error| format!("Unable to prepare retained Harness bindings: {error}"))?;
        let ids = statement
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| format!("Unable to query retained Harness bindings: {error}"))?
            .collect::<Result<Vec<_>, _>>()
            .map_err(|error| format!("Unable to read retained Harness bindings: {error}"))?;
        drop(statement);
        ids.into_iter()
            .map(|id| {
                load_binding(connection, &id)?
                    .ok_or_else(|| "Retained Harness binding disappeared.".to_string())
            })
            .collect()
    })
    }

    fn retire(&self, binding_id: &str, retired_at: &str) -> Result<(), String> {
        self.write("retire Harness binding", |transaction| {
        let changed = transaction
            .execute(
                "UPDATE session_harness_bindings SET stage='retired',retired_at=?2 WHERE id=?1 AND stage='bound'",
                params![binding_id, retired_at],
            )
            .map_err(|error| format!("Unable to retire Harness binding: {error}"))?;
        if changed == 1 {
            Ok(())
        } else {
            Err("Only a bound Harness binding can be retired.".to_string())
        }
    })
    }
}

#[cfg(test)]
fn initialize_harness_binding_storage(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(HARNESS_BINDING_SCHEMA)
        .map_err(|error| format!("Unable to initialize Harness binding storage: {error}"))
}

fn load_binding(
    connection: &Connection,
    binding_id: &str,
) -> Result<Option<HarnessBindingRecord>, String> {
    connection
        .query_row(
            "SELECT id,session_id,runtime_instance_id,session_instance_token,stage,harness_snapshot,mediation_plan,configuration_digest,harness_token,source_workflow_instance_id,source_recipe_id,source_node_id,prepared_at,bound_at,retired_at FROM session_harness_bindings WHERE id=?1",
            [binding_id],
            |row| {
                let stage = row.get::<_, String>(4)?;
                Ok((
                    row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?, stage, row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?, row.get::<_, String>(7)?, row.get::<_, Option<String>>(8)?,
                    row.get::<_, String>(9)?, row.get::<_, String>(10)?, row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?, row.get::<_, Option<String>>(13)?, row.get::<_, Option<String>>(14)?,
                ))
            },
        )
        .optional()
        .map_err(|error| format!("Unable to load Harness binding: {error}"))?
        .map(|row| {
            Ok(HarnessBindingRecord {
                id: row.0,
                session_id: row.1,
                runtime_instance_id: row.2,
                session_instance_token: row.3,
                stage: HarnessBindingStage::parse(&row.4)?,
                harness_snapshot: row.5,
                mediation_plan: row.6,
                configuration_digest: row.7,
                harness_token: row.8,
                source_workflow_instance_id: row.9,
                source_recipe_id: row.10,
                source_node_id: row.11,
                prepared_at: row.12,
                bound_at: row.13,
                retired_at: row.14,
            })
        })
        .transpose()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::harness_engine::domain::{
        binding_digest, HarnessMediationPlan, MEDIATION_PLAN_VERSION,
    };

    fn prepared(id: &str, session_id: &str) -> HarnessBindingRecord {
        let harness_snapshot = "{\"harnessName\":\"Review\"}".to_string();
        let mediation_plan = serde_json::to_string(&HarnessMediationPlan {
            contract_version: MEDIATION_PLAN_VERSION.to_string(),
            exposures: Vec::new(),
        })
        .unwrap();
        HarnessBindingRecord {
            id: id.into(),
            session_id: session_id.into(),
            runtime_instance_id: format!("runtime-{id}"),
            session_instance_token: format!("session-token-{id}"),
            stage: HarnessBindingStage::Prepared,
            configuration_digest: binding_digest(&harness_snapshot, &mediation_plan),
            harness_snapshot,
            mediation_plan,
            harness_token: None,
            source_workflow_instance_id: "workflow-instance-1".into(),
            source_recipe_id: "recipe-1".into(),
            source_node_id: "node-1".into(),
            prepared_at: "2026-08-09T00:00:00Z".into(),
            bound_at: None,
            retired_at: None,
        }
    }

    fn insert_session(connection: &Connection, session_id: &str) {
        connection
            .execute(
                "INSERT INTO agent_sessions(id,title,availability,external_context_id,runtime_version,working_directory,requested_options_json,created_at,updated_at) VALUES(?1,'Session','available',NULL,NULL,NULL,'{}','t','t')",
                [session_id],
            )
            .unwrap();
    }

    #[test]
    fn binding_bytes_tokens_and_single_current_rule_survive_reopen() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("bindings.sqlite");
        let connection = crate::storage::open_active_database(&path).unwrap();
        insert_session(&connection, "session-1");
        drop(connection);
        let repository = SqliteHarnessBindingRepository::open(&path).unwrap();
        let binding = prepared("binding-1", "session-1");
        repository.insert_prepared(&binding).unwrap();
        let duplicate = repository
            .insert_prepared(&prepared("binding-2", "session-1"))
            .unwrap_err();
        assert!(duplicate.contains("non-retired Harness binding"));
        let bound = repository
            .mark_bound("binding-1", "stable-harness-token", "2026-08-09T00:00:01Z")
            .unwrap();
        assert_eq!(bound.harness_snapshot, binding.harness_snapshot);
        assert_eq!(bound.mediation_plan, binding.mediation_plan);
        assert_eq!(bound.configuration_digest, binding.configuration_digest);
        drop(repository);

        let reopened = SqliteHarnessBindingRepository::open(&path).unwrap();
        let restored = reopened.current_for_session("session-1").unwrap().unwrap();
        assert_eq!(restored, bound);
        restored.verify_digest().unwrap();
        assert_eq!(reopened.non_retired().unwrap(), vec![bound.clone()]);
        reopened
            .retire("binding-1", "2026-08-09T00:00:02Z")
            .unwrap();
        reopened
            .insert_prepared(&prepared("binding-2", "session-1"))
            .unwrap();
    }
}
