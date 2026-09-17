use super::{authoring::WorkflowRecipeDraft, instance_domain::ResolvedRepoBranchWorktreeTarget};
use crate::session_events::{ReferenceIdentity, SessionEventResult};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Arc};
use uuid::Uuid;

use crate::otp_api::{InvocationContext, OutputRef, SessionRequest};
use crate::persistence::{ActiveDatabase, ManagedOperationError};

pub(crate) const WORKFLOW_INSTANCE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_recipe_instances (
    id TEXT PRIMARY KEY, record_json TEXT NOT NULL CHECK(json_valid(record_json))
);
CREATE TABLE IF NOT EXISTS workflow_recipe_attempts (
    id TEXT PRIMARY KEY, instance_id TEXT NOT NULL, record_json TEXT NOT NULL CHECK(json_valid(record_json)),
    FOREIGN KEY(instance_id) REFERENCES workflow_recipe_instances(id)
);
"#;

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecipeInstance {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) recipe: WorkflowRecipeDraft,
    pub(crate) target: ResolvedRepoBranchWorktreeTarget,
    pub(crate) created_at: String,
}

/// A handoff can fail before a generic delivery exists. Keep that small preparation record
/// here; Session Events still own delivery outcomes.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowEventAttempt {
    pub(crate) id: String,
    pub(crate) instance_id: String,
    pub(crate) definition_ref: ReferenceIdentity,
    #[serde(default = "legacy_invocation_context")]
    pub(crate) context: InvocationContext,
    #[serde(default)]
    pub(crate) output: Option<OutputRef>,
    #[serde(default)]
    pub(crate) payload: serde_json::Value,
    pub(crate) created_at: String,
    #[serde(default)]
    pub(crate) session_requests: Vec<SessionRequest>,
    #[serde(default)]
    pub(crate) stop_outcomes: Vec<SessionStopOutcome>,
    #[serde(default)]
    pub(crate) message: String,
    #[serde(default)]
    pub(crate) event_groups: Vec<SessionEventResult>,
    #[serde(default)]
    pub(crate) error: Option<String>,
}

fn legacy_invocation_context() -> InvocationContext {
    InvocationContext {
        instance_id: "legacy".into(),
        occurrence_id: "legacy".into(),
        capability: crate::otp_api::CapabilityRef {
            package: "workflow".into(),
            tool: "on_invocation_completed".into(),
        },
        source: None,
        connection_id: None,
        output_node_id: None,
    }
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionStopOutcome {
    pub(crate) node_id: String,
    pub(crate) session_id: String,
    pub(crate) invocation_id: Option<String>,
    pub(crate) status: String,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowActionResult {
    pub(crate) attempt_id: String,
    pub(crate) event_groups: Vec<SessionEventResult>,
    pub(crate) stop_outcomes: Vec<SessionStopOutcome>,
    pub(crate) message: String,
}

pub(crate) struct WorkflowInstanceStore {
    database: Arc<ActiveDatabase>,
}

impl WorkflowInstanceStore {
    pub(crate) fn from_database(database: Arc<ActiveDatabase>) -> Self {
        Self { database }
    }

    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        ActiveDatabase::open(path, initialize_workflow_instance_storage)
            .map(Arc::new)
            .map(Self::from_database)
            .map_err(|error| error.to_string())
    }

    #[cfg(test)]
    pub(crate) fn in_memory() -> Self {
        ActiveDatabase::from_connection(
            Connection::open_in_memory().unwrap(),
            initialize_workflow_instance_storage,
        )
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
        write: impl FnOnce(&rusqlite::Transaction<'_>) -> Result<T, String>,
    ) -> Result<T, String> {
        self.database.write(operation, write).map_err(managed_error)
    }

    pub(crate) fn create(
        &self,
        name: String,
        recipe: WorkflowRecipeDraft,
        target: ResolvedRepoBranchWorktreeTarget,
    ) -> Result<RecipeInstance, String> {
        recipe.validate_activatable()?;
        if name.trim().is_empty() {
            return Err("An instance name is required".into());
        }
        target.validate()?;
        if !Path::new(&target.worktree.path).is_absolute()
            || !Path::new(&target.worktree.path).is_dir()
        {
            return Err("Choose an existing absolute worktree folder".into());
        }
        let record = RecipeInstance {
            id: format!("workflow-instance-{}", Uuid::new_v4()),
            name: name.trim().into(),
            recipe,
            target,
            created_at: Utc::now().to_rfc3339(),
        };
        self.write("create Workflow instance", |transaction| {
            transaction
                .execute(
                    "INSERT INTO workflow_recipe_instances(id,record_json) VALUES(?1,?2)",
                    params![
                        record.id,
                        serde_json::to_string(&record).map_err(|error| error.to_string())?
                    ],
                )
                .map_err(|error| error.to_string())?;
            Ok(record)
        })
    }

    pub(crate) fn list_navigation(
        &self,
    ) -> Result<Vec<super::session_navigation::NavigationInstance>, String> {
        self.read("list Workflow navigation", |connection| {
            let mut query = connection.prepare("SELECT id,json_extract(record_json,'$.name'),json_extract(record_json,'$.target.repository.id'),(SELECT json_group_array(json_object('id',json_extract(value,'$.nodeId'),'name',json_extract(value,'$.name'))) FROM json_each(record_json,'$.recipe.nodes')) FROM workflow_recipe_instances ORDER BY json_extract(record_json,'$.createdAt') DESC,id").map_err(|e| e.to_string())?;
            let rows = query.query_map([], |row| Ok((row.get::<_,String>(0)?,row.get::<_,String>(1)?,row.get::<_,String>(2)?,row.get::<_,String>(3)?))).map_err(|e| e.to_string())?;
            rows.map(|row| {
                let (id,name,repository_id,nodes) = row.map_err(|e| e.to_string())?;
                Ok(super::session_navigation::NavigationInstance { id,name,repository_id,nodes: serde_json::from_str(&nodes).map_err(|e| e.to_string())? })
            }).collect()
        })
    }

    pub(crate) fn list(&self) -> Result<Vec<RecipeInstance>, String> {
        self.read("list Workflow instances", |connection| {
        let mut query = connection.prepare("SELECT record_json FROM workflow_recipe_instances ORDER BY json_extract(record_json,'$.createdAt') DESC,id")
            .map_err(|error| error.to_string())?;
        let result = query
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .map(|row| decode_instance(&row.map_err(|error| error.to_string())?))
            .collect();
        result
        })
    }

    pub(crate) fn load(&self, id: &str) -> Result<RecipeInstance, String> {
        self.read("load Workflow instance", |connection| {
            let value: String = connection
                .query_row(
                    "SELECT record_json FROM workflow_recipe_instances WHERE id=?1",
                    [id],
                    |row| row.get(0),
                )
                .optional()
                .map_err(|error| error.to_string())?
                .ok_or_else(|| format!("Workflow instance `{id}` does not exist"))?;
            decode_instance(&value)
        })
    }

    pub(crate) fn begin_attempt(&self, attempt: &WorkflowEventAttempt) -> Result<bool, String> {
        self.write("begin Workflow event attempt", |transaction| Ok(transaction.execute("INSERT OR IGNORE INTO workflow_recipe_attempts(id,instance_id,record_json) VALUES(?1,?2,?3)",
            params![attempt.id, attempt.instance_id, serde_json::to_string(attempt).map_err(|error| error.to_string())?])
            .map_err(|error| error.to_string())? == 1))
    }

    pub(crate) fn finish_attempt(
        &self,
        mut attempt: WorkflowEventAttempt,
        result: &Result<SessionEventResult, String>,
    ) -> Result<(), String> {
        match result {
            Ok(result) => attempt.event_groups.push(result.clone()),
            Err(error) => attempt.error = Some(error.clone()),
        }
        self.write("finish Workflow event attempt", |transaction| {
            transaction
                .execute(
                    "UPDATE workflow_recipe_attempts SET record_json=?2 WHERE id=?1",
                    params![
                        attempt.id,
                        serde_json::to_string(&attempt).map_err(|error| error.to_string())?
                    ],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        })
    }

    pub(crate) fn update_attempt(&self, attempt: &WorkflowEventAttempt) -> Result<(), String> {
        self.write("update Workflow event attempt", |transaction| {
            transaction
                .execute(
                    "UPDATE workflow_recipe_attempts SET record_json=?2 WHERE id=?1",
                    params![
                        attempt.id,
                        serde_json::to_string(attempt).map_err(|error| error.to_string())?
                    ],
                )
                .map_err(|error| error.to_string())?;
            Ok(())
        })
    }

    pub(crate) fn attempts(&self, instance_id: &str) -> Result<Vec<WorkflowEventAttempt>, String> {
        self.read("list Workflow event attempts", |connection| {
        let mut query = connection.prepare("SELECT record_json FROM workflow_recipe_attempts WHERE instance_id=?1 ORDER BY rowid DESC")
            .map_err(|error| error.to_string())?;
        let result = query
            .query_map([instance_id], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .map(|row| {
                serde_json::from_str(&row.map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())
            })
            .collect();
        result
        })
    }
}

fn decode_instance(value: &str) -> Result<RecipeInstance, String> {
    let mut value: serde_json::Value =
        serde_json::from_str(value).map_err(|error| error.to_string())?;
    let recipe = value
        .get_mut("recipe")
        .ok_or("Workflow instance has no recipe")?
        .take();
    *value
        .get_mut("recipe")
        .ok_or("Workflow instance has no recipe")? =
        serde_json::to_value(super::authoring::decode_recipe_value(recipe)?)
            .map_err(|error| error.to_string())?;
    serde_json::from_value(value).map_err(|error| error.to_string())
}

pub(crate) fn initialize_workflow_instance_storage(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(WORKFLOW_INSTANCE_SCHEMA)
        .map_err(|error| format!("Unable to initialize Workflow instance storage: {error}"))
}

fn managed_error(error: ManagedOperationError<String>) -> String {
    match error {
        ManagedOperationError::Infrastructure(error) => error.to_string(),
        ManagedOperationError::Domain(error) => error,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn older_attempt_records_load_with_a_safe_legacy_context() {
        let attempt: WorkflowEventAttempt = serde_json::from_value(serde_json::json!({
            "id": "attempt-1",
            "instanceId": "instance-1",
            "definitionRef": {"namespace": "workflow", "kind": "event_definition", "id": "definition-1"},
            "createdAt": "2026-09-08T00:00:00Z",
            "error": null
        }))
        .unwrap();

        assert_eq!(attempt.context.occurrence_id, "legacy");
    }
}
