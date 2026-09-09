use super::{authoring::WorkflowRecipeDraft, instance_domain::ResolvedRepoBranchWorktreeTarget};
use crate::otp_api::{InvocationContext, OutputRef, SessionRequest};
use crate::session_events::ReferenceIdentity;
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use serde::{Deserialize, Serialize};
use std::{path::Path, sync::Mutex};
use uuid::Uuid;

const SCHEMA: &str = r#"
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
    pub(crate) context: InvocationContext,
    pub(crate) output: Option<OutputRef>,
    pub(crate) payload: serde_json::Value,
    pub(crate) session_requests: Vec<SessionRequest>,
    #[serde(default)]
    pub(crate) stop_outcomes: Vec<SessionStopOutcome>,
    #[serde(default)]
    pub(crate) message: String,
    pub(crate) created_at: String,
    pub(crate) event_groups: Vec<ReferenceIdentity>,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionStopOutcome {
    pub node_id: String,
    pub session_id: String,
    pub invocation_id: Option<String>,
    pub status: String,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowActionResult {
    pub attempt_id: String,
    pub event_groups: Vec<crate::session_events::SessionEventResult>,
    pub stop_outcomes: Vec<SessionStopOutcome>,
    pub message: String,
}

pub(crate) struct WorkflowInstanceStore {
    connection: Mutex<Connection>,
}

impl WorkflowInstanceStore {
    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        Self::from_connection(Connection::open(path).map_err(|error| error.to_string())?)
    }

    #[cfg(test)]
    pub(crate) fn in_memory() -> Self {
        Self::from_connection(Connection::open_in_memory().unwrap()).unwrap()
    }

    fn from_connection(connection: Connection) -> Result<Self, String> {
        crate::storage::configure_sqlite_connection(&connection)
            .map_err(|error| error.to_string())?;
        connection
            .execute_batch(SCHEMA)
            .map_err(|error| error.to_string())?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.connection
            .lock()
            .map_err(|_| "Workflow instance storage is unavailable".into())
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
        self.lock()?
            .execute(
                "INSERT INTO workflow_recipe_instances(id,record_json) VALUES(?1,?2)",
                params![
                    record.id,
                    serde_json::to_string(&record).map_err(|error| error.to_string())?
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(record)
    }

    pub(crate) fn list(&self) -> Result<Vec<RecipeInstance>, String> {
        let connection = self.lock()?;
        let mut query = connection.prepare("SELECT record_json FROM workflow_recipe_instances WHERE json_extract(record_json,'$.recipe.contractVersion')=2 ORDER BY json_extract(record_json,'$.createdAt') DESC,id")
            .map_err(|error| error.to_string())?;
        let result = query
            .query_map([], |row| row.get::<_, String>(0))
            .map_err(|error| error.to_string())?
            .map(|row| {
                serde_json::from_str(&row.map_err(|error| error.to_string())?)
                    .map_err(|error| error.to_string())
            })
            .collect();
        result
    }

    pub(crate) fn load(&self, id: &str) -> Result<RecipeInstance, String> {
        let value: String = self
            .lock()?
            .query_row(
                "SELECT record_json FROM workflow_recipe_instances WHERE id=?1",
                [id],
                |row| row.get(0),
            )
            .optional()
            .map_err(|error| error.to_string())?
            .ok_or_else(|| format!("Workflow instance `{id}` does not exist"))?;
        let raw: serde_json::Value = serde_json::from_str(&value).map_err(|e| e.to_string())?;
        if raw
            .pointer("/recipe/contractVersion")
            .and_then(|v| v.as_u64())
            != Some(2)
        {
            return Err("Unsupported Workflow instance contract; create a new instance".into());
        }
        serde_json::from_value(raw).map_err(|error| error.to_string())
    }

    pub(crate) fn begin_attempt(&self, attempt: &WorkflowEventAttempt) -> Result<bool, String> {
        Ok(self.lock()?.execute("INSERT OR IGNORE INTO workflow_recipe_attempts(id,instance_id,record_json) VALUES(?1,?2,?3)",
            params![attempt.id, attempt.instance_id, serde_json::to_string(attempt).map_err(|error| error.to_string())?])
            .map_err(|error| error.to_string())? == 1)
    }

    pub(crate) fn update_attempt(&self, attempt: &WorkflowEventAttempt) -> Result<(), String> {
        self.lock()?
            .execute(
                "UPDATE workflow_recipe_attempts SET record_json=?2 WHERE id=?1",
                params![
                    attempt.id,
                    serde_json::to_string(&attempt).map_err(|error| error.to_string())?
                ],
            )
            .map_err(|error| error.to_string())?;
        Ok(())
    }

    pub(crate) fn attempts(&self, instance_id: &str) -> Result<Vec<WorkflowEventAttempt>, String> {
        let connection = self.lock()?;
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
    }
}
