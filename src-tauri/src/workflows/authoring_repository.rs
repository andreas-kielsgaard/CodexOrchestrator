use super::authoring::{WorkflowRecipeDraft, WorkflowRecipeState, WorkflowRecipeSummary};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::{path::Path, sync::Mutex};

const WORKFLOW_AUTHORING_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_recipe_authoring (
    recipe_id TEXT PRIMARY KEY CHECK (length(trim(recipe_id)) > 0),
    draft_json TEXT NOT NULL,
    active_json TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE INDEX IF NOT EXISTS workflow_recipe_authoring_updated
ON workflow_recipe_authoring(updated_at DESC, recipe_id);
"#;

pub(crate) trait WorkflowAuthoringRepository: Send + Sync {
    fn list(&self) -> Result<Vec<WorkflowRecipeSummary>, String>;
    fn load(&self, recipe_id: &str) -> Result<Option<WorkflowRecipeState>, String>;
    fn create(&self, draft: &WorkflowRecipeDraft) -> Result<WorkflowRecipeState, String>;
    fn save_draft(&self, draft: &WorkflowRecipeDraft) -> Result<WorkflowRecipeState, String>;
    fn activate(&self, recipe_id: &str) -> Result<WorkflowRecipeState, String>;
    fn activate_revision(
        &self,
        recipe_id: &str,
        expected_revision: u64,
    ) -> Result<WorkflowRecipeState, String>;
}

pub(crate) struct SqliteWorkflowAuthoringRepository {
    connection: Mutex<Connection>,
}

impl SqliteWorkflowAuthoringRepository {
    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path)
            .map_err(|error| format!("Unable to open Workflow authoring storage: {error}"))?;
        connection
            .execute_batch("PRAGMA foreign_keys = ON;")
            .map_err(storage_error("enable Workflow authoring foreign keys"))?;
        Self::from_connection(connection)
    }

    #[cfg(test)]
    pub(crate) fn in_memory() -> Self {
        Self::from_connection(Connection::open_in_memory().expect("Workflow authoring memory DB"))
            .expect("Workflow authoring schema")
    }

    fn from_connection(connection: Connection) -> Result<Self, String> {
        connection
            .execute_batch(WORKFLOW_AUTHORING_SCHEMA)
            .map_err(storage_error("initialize Workflow authoring storage"))?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.connection
            .lock()
            .map_err(|_| "Workflow authoring storage is unavailable".to_string())
    }
}

impl WorkflowAuthoringRepository for SqliteWorkflowAuthoringRepository {
    fn list(&self) -> Result<Vec<WorkflowRecipeSummary>, String> {
        let connection = self.lock()?;
        let mut statement = connection
            .prepare(
                "SELECT draft_json,active_json,updated_at \
                 FROM workflow_recipe_authoring \
                 ORDER BY json_extract(draft_json,'$.name') COLLATE NOCASE,recipe_id",
            )
            .map_err(storage_error("prepare Workflow recipe list"))?;
        let summaries = statement
            .query_map([], |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                ))
            })
            .map_err(storage_error("query Workflow recipe list"))?
            .map(|row| {
                let (draft_json, active_json, updated_at) =
                    row.map_err(storage_error("read Workflow recipe list"))?;
                let draft = decode_draft(&draft_json)?;
                let active = active_json.as_deref().map(decode_draft).transpose()?;
                Ok(WorkflowRecipeSummary {
                    recipe_id: draft.recipe_id,
                    name: draft.name,
                    draft_revision: draft.revision,
                    active_revision: active.map(|value| value.revision),
                    updated_at,
                })
            })
            .collect();
        summaries
    }

    fn load(&self, recipe_id: &str) -> Result<Option<WorkflowRecipeState>, String> {
        let connection = self.lock()?;
        load_state(&connection, recipe_id)
    }

    fn create(&self, draft: &WorkflowRecipeDraft) -> Result<WorkflowRecipeState, String> {
        draft.validate_storable()?;
        let connection = self.lock()?;
        let now = Utc::now().to_rfc3339();
        connection
            .execute(
                "INSERT INTO workflow_recipe_authoring(\
                    recipe_id,draft_json,active_json,created_at,updated_at\
                 ) VALUES(?1,?2,NULL,?3,?3)",
                params![draft.recipe_id, encode_draft(draft)?, now],
            )
            .map_err(storage_error("create Workflow recipe"))?;
        load_state(&connection, &draft.recipe_id)?.ok_or_else(|| {
            "Workflow recipe disappeared immediately after it was created".to_string()
        })
    }

    fn save_draft(&self, draft: &WorkflowRecipeDraft) -> Result<WorkflowRecipeState, String> {
        draft.validate_storable()?;
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "UPDATE workflow_recipe_authoring \
                 SET draft_json=?2,updated_at=?3 WHERE recipe_id=?1 AND json_extract(draft_json,'$.revision')=?4",
                params![
                    draft.recipe_id,
                    encode_draft(draft)?,
                    Utc::now().to_rfc3339(), draft.revision - 1
                ],
            )
            .map_err(storage_error("save Workflow recipe draft"))?;
        expect_one(
            changed,
            "Workflow recipe changed or was removed. Reload before saving again.",
        )?;
        load_state(&connection, &draft.recipe_id)?.ok_or_else(|| {
            "Workflow recipe disappeared immediately after its draft was saved".to_string()
        })
    }

    fn activate(&self, recipe_id: &str) -> Result<WorkflowRecipeState, String> {
        let revision = self
            .load(recipe_id)?
            .ok_or("Workflow recipe is missing")?
            .draft
            .revision;
        self.activate_revision(recipe_id, revision)
    }

    fn activate_revision(
        &self,
        recipe_id: &str,
        expected_revision: u64,
    ) -> Result<WorkflowRecipeState, String> {
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "UPDATE workflow_recipe_authoring \
                 SET active_json=draft_json,updated_at=?2 WHERE recipe_id=?1 AND json_extract(draft_json,'$.revision')=?3",
                params![recipe_id, Utc::now().to_rfc3339(), expected_revision],
            )
            .map_err(storage_error("activate Workflow recipe"))?;
        expect_one(
            changed,
            "Saved Workflow draft changed or was removed. Reload before activating.",
        )?;
        load_state(&connection, recipe_id)?
            .ok_or_else(|| "Workflow recipe disappeared immediately after activation".to_string())
    }
}

fn load_state(
    connection: &Connection,
    recipe_id: &str,
) -> Result<Option<WorkflowRecipeState>, String> {
    connection
        .query_row(
            "SELECT draft_json,active_json,created_at,updated_at \
             FROM workflow_recipe_authoring WHERE recipe_id=?1",
            [recipe_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, Option<String>>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error("load Workflow recipe"))?
        .map(|(draft_json, active_json, created_at, updated_at)| {
            Ok(WorkflowRecipeState {
                draft: decode_draft(&draft_json)?,
                active: active_json.as_deref().map(decode_draft).transpose()?,
                created_at,
                updated_at,
            })
        })
        .transpose()
}

fn encode_draft(draft: &WorkflowRecipeDraft) -> Result<String, String> {
    serde_json::to_string(draft)
        .map_err(|error| format!("Unable to encode Workflow recipe: {error}"))
}

fn decode_draft(value: &str) -> Result<WorkflowRecipeDraft, String> {
    let draft: WorkflowRecipeDraft = serde_json::from_str(value)
        .map_err(|error| format!("Unable to decode Workflow recipe: {error}"))?;
    draft.validate_storable()?;
    Ok(draft)
}

fn expect_one(changed: usize, missing: &str) -> Result<(), String> {
    match changed {
        1 => Ok(()),
        0 => Err(missing.into()),
        _ => Err("Workflow authoring mutation affected more than one recipe".into()),
    }
}

fn storage_error(operation: &'static str) -> impl FnOnce(rusqlite::Error) -> String {
    move |error| format!("Unable to {operation}: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        execution_configuration::{CapabilitySet, NodeProfile, RuntimeSelections},
        workflows::authoring::WORKFLOW_RECIPE_CONTRACT_VERSION,
    };

    fn draft(id: &str, revision: u64) -> WorkflowRecipeDraft {
        WorkflowRecipeDraft {
            contract_version: WORKFLOW_RECIPE_CONTRACT_VERSION,
            recipe_id: id.into(),
            name: "Review".into(),
            revision,
            starting_node_id: None,
            nodes: vec![super::super::authoring::WorkflowAuthoringNode {
                node_id: "reviewer".into(),
                name: "Reviewer".into(),
                position_x: 0.0,
                position_y: 0.0,
                capability_profile_id: "capability-default".into(),
                node_profile: NodeProfile {
                    contract_version: 1,
                    allowed_capabilities: CapabilitySet::default(),
                    pinned_defaults: RuntimeSelections::default(),
                },
                initial_prompt: None,
                agent_identity_id: None,
            }],
            connections: Vec::new(),
        }
    }

    #[test]
    fn sqlite_repository_keeps_independent_draft_and_active_recipe() {
        let repository = SqliteWorkflowAuthoringRepository::in_memory();
        repository.create(&draft("recipe-1", 1)).unwrap();
        let activated = repository.activate("recipe-1").unwrap();
        assert_eq!(activated.active.as_ref().unwrap().revision, 1);

        let mut changed = draft("recipe-1", 2);
        changed.name = "Changed review".into();
        let saved = repository.save_draft(&changed).unwrap();

        assert_eq!(saved.draft.revision, 2);
        assert_eq!(saved.active.as_ref().unwrap().revision, 1);
        assert_eq!(repository.list().unwrap()[0].active_revision, Some(1));
    }

    #[test]
    fn repository_rejects_duplicate_recipe_identity() {
        let repository = SqliteWorkflowAuthoringRepository::in_memory();
        repository.create(&draft("recipe-1", 1)).unwrap();

        assert!(repository.create(&draft("recipe-1", 1)).is_err());
    }
}
