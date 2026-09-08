use super::{load_recipe, storage_error};
use crate::workflows::{
    domain::WorkflowCompletedTurnTrigger,
    instance_domain::{
        CreateWorkflowInstancePreparation, ResolvedRepoBranchWorktreeTarget, WorkflowBranchTarget,
        WorkflowInstanceRecord, WorkflowRepositoryTarget, WorkflowSessionAssociationRecord,
        WorkflowWorktreeTarget,
    },
};
use rusqlite::{params, Connection, OptionalExtension};

pub(crate) const WORKFLOW_INSTANCE_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_instances (
    id TEXT PRIMARY KEY,
    workflow_type_id TEXT NOT NULL,
    recipe_id TEXT NOT NULL,
    name TEXT NOT NULL,
    repository_id TEXT NOT NULL,
    repository_name TEXT NOT NULL,
    repository_git_common_directory TEXT NOT NULL,
    branch_id TEXT NOT NULL,
    branch_name TEXT NOT NULL,
    worktree_id TEXT NOT NULL,
    worktree_root TEXT NOT NULL,
    created_at TEXT NOT NULL,
    FOREIGN KEY (workflow_type_id) REFERENCES workflow_types(id),
    FOREIGN KEY (recipe_id) REFERENCES workflow_effective_recipes(id)
);

CREATE INDEX IF NOT EXISTS workflow_instances_newest
ON workflow_instances(created_at DESC, id DESC);

CREATE TABLE IF NOT EXISTS workflow_instance_sessions (
    workflow_instance_id TEXT NOT NULL,
    node_id TEXT NOT NULL,
    session_id TEXT NOT NULL UNIQUE,
    associated_at TEXT NOT NULL,
    PRIMARY KEY (workflow_instance_id, session_id),
    FOREIGN KEY (workflow_instance_id) REFERENCES workflow_instances(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS workflow_instance_sessions_by_node
ON workflow_instance_sessions(workflow_instance_id, node_id, associated_at DESC);

CREATE TABLE IF NOT EXISTS workflow_connection_activations (
    id TEXT PRIMARY KEY,
    workflow_instance_id TEXT NOT NULL,
    recipe_id TEXT NOT NULL,
    connection_id TEXT NOT NULL,
    sender_node_id TEXT NOT NULL,
    receiver_node_id TEXT NOT NULL,
    source_session_id TEXT NOT NULL,
    source_invocation_id TEXT NOT NULL,
    target_session_id TEXT,
    target_invocation_id TEXT,
    delivery_kind TEXT NOT NULL CHECK (delivery_kind='direct_prompt_runtime_v1'),
    session_mode TEXT CHECK (session_mode IN ('fresh','continued')),
    context_inheritance TEXT NOT NULL CHECK (context_inheritance='none'),
    compression TEXT NOT NULL CHECK (compression='none'),
    resolved_file_path TEXT,
    resolved_output_json TEXT,
    requested_at TEXT NOT NULL,
    resolved_at TEXT,
    associated_at TEXT,
    launch_requested_at TEXT,
    launch_accepted_at TEXT,
    failed_at TEXT,
    failure_stage TEXT,
    failure_reason TEXT,
    FOREIGN KEY (workflow_instance_id) REFERENCES workflow_instances(id) ON DELETE CASCADE,
    FOREIGN KEY (recipe_id) REFERENCES workflow_effective_recipes(id),
    CHECK ((failed_at IS NULL AND failure_stage IS NULL AND failure_reason IS NULL)
        OR (failed_at IS NOT NULL AND failure_stage IS NOT NULL AND failure_reason IS NOT NULL))
);

CREATE INDEX IF NOT EXISTS workflow_connection_activations_by_instance
ON workflow_connection_activations(workflow_instance_id, requested_at, id);
"#;

pub(super) fn create_instance(
    connection: &Connection,
    preparation: CreateWorkflowInstancePreparation,
) -> Result<WorkflowInstanceRecord, String> {
    let active_recipe_id = connection
        .query_row(
            "SELECT active_recipe_id FROM workflow_types WHERE id=?1",
            [&preparation.workflow_type_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(storage_error("read active Workflow recipe"))?
        .flatten()
        .ok_or_else(|| "Activate the Workflow type before creating an instance.".to_string())?;
    if active_recipe_id != preparation.recipe_id {
        return Err(
            "The active Workflow recipe changed before instance creation. Try again.".into(),
        );
    }
    connection.execute(
            "INSERT INTO workflow_instances(id,workflow_type_id,recipe_id,name,repository_id,repository_name,repository_git_common_directory,branch_id,branch_name,worktree_id,worktree_root,created_at) VALUES(?1,?2,?3,?4,?5,?6,?7,?8,?9,?10,?11,?12)",
            params![
                preparation.instance_id,
                preparation.workflow_type_id,
                preparation.recipe_id,
                preparation.name,
                preparation.target.repository.id,
                preparation.target.repository.name,
                preparation.target.repository.git_common_directory,
                preparation.target.branch.id,
                preparation.target.branch.name,
                preparation.target.worktree.id,
                preparation.target.worktree.path,
                preparation.created_at,
            ],
        )
        .map_err(storage_error("create Workflow instance"))?;
    load_instance(connection, &preparation.instance_id)
}

pub(super) fn associate_session(
    connection: &Connection,
    workflow_instance_id: &str,
    node_id: &str,
    session_id: &str,
    associated_at: &str,
) -> Result<WorkflowInstanceRecord, String> {
    connection
        .execute(
            "INSERT INTO workflow_instance_sessions(workflow_instance_id,node_id,session_id,associated_at) VALUES(?1,?2,?3,?4)",
            params![workflow_instance_id, node_id, session_id, associated_at],
        )
        .map_err(|error| {
            if error
                .to_string()
                .contains("workflow_instance_sessions.session_id")
            {
                "An Agent Session can belong to at most one Workflow instance.".to_string()
            } else {
                format!("Unable to associate Workflow Session: {error}")
            }
        })?;
    load_instance(connection, workflow_instance_id)
}

pub(super) fn list_instances(
    connection: &Connection,
) -> Result<Vec<WorkflowInstanceRecord>, String> {
    let mut statement = connection
        .prepare("SELECT id FROM workflow_instances ORDER BY created_at DESC,id DESC")
        .map_err(storage_error("prepare Workflow instance list"))?;
    let ids = statement
        .query_map([], |row| row.get::<_, String>(0))
        .map_err(storage_error("query Workflow instances"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error("read Workflow instances"))?;
    drop(statement);
    ids.into_iter()
        .map(|id| load_instance(connection, &id))
        .collect()
}

pub(super) fn load_instance(
    connection: &Connection,
    workflow_instance_id: &str,
) -> Result<WorkflowInstanceRecord, String> {
    let row = connection
        .query_row(
            "SELECT instance.id,instance.workflow_type_id,type.name,instance.recipe_id,instance.name,instance.repository_id,instance.repository_name,instance.repository_git_common_directory,instance.branch_id,instance.branch_name,instance.worktree_id,instance.worktree_root,instance.created_at FROM workflow_instances instance JOIN workflow_types type ON type.id=instance.workflow_type_id WHERE instance.id=?1",
            [workflow_instance_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?, row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?, row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?, row.get::<_, String>(5)?,
                    row.get::<_, String>(6)?, row.get::<_, String>(7)?,
                    row.get::<_, String>(8)?, row.get::<_, String>(9)?,
                    row.get::<_, String>(10)?, row.get::<_, String>(11)?,
                    row.get::<_, String>(12)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error("load Workflow instance"))?
        .ok_or_else(|| format!("Workflow instance {workflow_instance_id} does not exist."))?;
    let mut statement = connection
        .prepare("SELECT node_id,session_id,associated_at FROM workflow_instance_sessions WHERE workflow_instance_id=?1 ORDER BY associated_at,session_id")
        .map_err(storage_error("prepare Workflow Session associations"))?;
    let session_associations = statement
        .query_map([workflow_instance_id], |row| {
            Ok(WorkflowSessionAssociationRecord {
                node_id: row.get(0)?,
                session_id: row.get(1)?,
                associated_at: row.get(2)?,
            })
        })
        .map_err(storage_error("query Workflow Session associations"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error("read Workflow Session associations"))?;
    Ok(WorkflowInstanceRecord {
        id: row.0,
        workflow_type_id: row.1,
        workflow_type_name: row.2,
        recipe: load_recipe(connection, &row.3)?,
        name: row.4,
        target: ResolvedRepoBranchWorktreeTarget {
            repository: WorkflowRepositoryTarget {
                id: row.5,
                name: row.6,
                git_common_directory: row.7,
            },
            branch: WorkflowBranchTarget {
                id: row.8,
                name: row.9,
            },
            worktree: WorkflowWorktreeTarget {
                id: row.10,
                path: row.11,
            },
        },
        created_at: row.12,
        session_associations,
    })
}

pub(super) fn load_completed_turn_trigger(
    connection: &Connection,
    source_session_id: &str,
) -> Result<Option<WorkflowCompletedTurnTrigger>, String> {
    let trigger = connection
        .query_row(
            "SELECT association.workflow_instance_id,instance.worktree_root,association.node_id,type.active_recipe_id
             FROM workflow_instance_sessions association
             JOIN workflow_instances instance ON instance.id=association.workflow_instance_id
             JOIN workflow_types type ON type.id=instance.workflow_type_id
             WHERE association.session_id=?1",
            [source_session_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?, row.get::<_, String>(2)?, row.get::<_, Option<String>>(3)?)),
        )
        .optional()
        .map_err(storage_error("load completed-turn Workflow association"))?;
    let Some((workflow_instance_id, worktree_root, sender_node_id, recipe_id)) = trigger else {
        return Ok(None);
    };
    let recipe_id = recipe_id.ok_or_else(|| {
        "The Workflow type has no current activated recipe for this trigger.".to_string()
    })?;
    Ok(Some(WorkflowCompletedTurnTrigger {
        workflow_instance_id,
        worktree_root,
        sender_node_id,
        recipe: load_recipe(connection, &recipe_id)?,
    }))
}

pub(super) fn load_mcp_prepared_trigger(
    connection: &Connection,
    workflow_instance_id: &str,
    recipe_id: &str,
    sender_node_id: &str,
    source_session_id: &str,
) -> Result<WorkflowCompletedTurnTrigger, String> {
    let (worktree_root, workflow_type_id) = connection
        .query_row(
            "SELECT instance.worktree_root,instance.workflow_type_id
             FROM workflow_instances instance
             JOIN workflow_instance_sessions association ON association.workflow_instance_id=instance.id
             WHERE instance.id=?1 AND association.session_id=?2 AND association.node_id=?3",
            params![workflow_instance_id, source_session_id, sender_node_id],
            |row| Ok((row.get::<_, String>(0)?, row.get::<_, String>(1)?)),
        )
        .optional()
        .map_err(storage_error("load prepared Workflow MCP association"))?
        .ok_or_else(|| "The prepared Workflow MCP Session association is unavailable.".to_string())?;
    let recipe = load_recipe(connection, recipe_id)?;
    if recipe.workflow_type_id != workflow_type_id {
        return Err("The prepared Workflow MCP recipe belongs to another type.".to_string());
    }
    Ok(WorkflowCompletedTurnTrigger {
        workflow_instance_id: workflow_instance_id.to_string(),
        worktree_root,
        sender_node_id: sender_node_id.to_string(),
        recipe,
    })
}
