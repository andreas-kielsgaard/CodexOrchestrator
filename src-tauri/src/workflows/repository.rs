use super::{
    application::WorkflowRepository,
    domain::{
        EffectiveRecipe, WorkflowConnectionConfig, WorkflowConnectionElement, WorkflowDefinition,
        WorkflowElementKind, WorkflowElementRef, WorkflowNativeQuery, WorkflowNodeConfig,
        WorkflowNodeElement, WorkflowTypeSummary,
    },
};
use chrono::Utc;
use rusqlite::{params, Connection, OptionalExtension};
use std::{
    collections::{BTreeMap, HashSet},
    path::Path,
    sync::Mutex,
};
use uuid::Uuid;

pub(crate) const WORKFLOW_SCHEMA: &str = r#"
CREATE TABLE IF NOT EXISTS workflow_types (
    id TEXT PRIMARY KEY,
    name TEXT NOT NULL,
    active_recipe_id TEXT,
    created_at TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

CREATE TABLE IF NOT EXISTS workflow_nodes (
    id TEXT PRIMARY KEY,
    workflow_type_id TEXT NOT NULL,
    draft_json TEXT,
    live_json TEXT,
    has_unpublished_changes INTEGER NOT NULL CHECK (has_unpublished_changes IN (0, 1)),
    FOREIGN KEY (workflow_type_id) REFERENCES workflow_types(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS workflow_nodes_by_type
ON workflow_nodes(workflow_type_id, id);

CREATE TABLE IF NOT EXISTS workflow_connections (
    id TEXT PRIMARY KEY,
    workflow_type_id TEXT NOT NULL,
    draft_json TEXT,
    live_json TEXT,
    has_unpublished_changes INTEGER NOT NULL CHECK (has_unpublished_changes IN (0, 1)),
    FOREIGN KEY (workflow_type_id) REFERENCES workflow_types(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS workflow_connections_by_type
ON workflow_connections(workflow_type_id, id);

CREATE TABLE IF NOT EXISTS workflow_effective_recipes (
    id TEXT PRIMARY KEY,
    workflow_type_id TEXT NOT NULL,
    ordinal INTEGER NOT NULL CHECK (ordinal > 0),
    nodes_json TEXT NOT NULL,
    connections_json TEXT NOT NULL,
    created_at TEXT NOT NULL,
    UNIQUE (workflow_type_id, ordinal),
    FOREIGN KEY (workflow_type_id) REFERENCES workflow_types(id) ON DELETE CASCADE
);

CREATE INDEX IF NOT EXISTS workflow_effective_recipes_by_type
ON workflow_effective_recipes(workflow_type_id, ordinal);
"#;

pub(crate) struct SqliteWorkflowRepository {
    connection: Mutex<Connection>,
}

impl SqliteWorkflowRepository {
    pub(crate) fn open(path: &Path) -> Result<Self, String> {
        let connection = Connection::open(path)
            .map_err(|error| format!("Unable to open Workflow storage: {error}"))?;
        crate::storage::configure_sqlite_connection(&connection)
            .map_err(|error| format!("Unable to configure Workflow storage: {error}"))?;
        Self::new(connection)
    }

    pub(crate) fn new(connection: Connection) -> Result<Self, String> {
        connection
            .execute_batch(WORKFLOW_SCHEMA)
            .map_err(|error| format!("Unable to initialize Workflow storage: {error}"))?;
        Ok(Self {
            connection: Mutex::new(connection),
        })
    }

    fn lock(&self) -> Result<std::sync::MutexGuard<'_, Connection>, String> {
        self.connection
            .lock()
            .map_err(|_| "Workflow storage is unavailable.".to_string())
    }
}

impl WorkflowRepository for SqliteWorkflowRepository {
    fn list_workflow_types(&self) -> Result<Vec<WorkflowTypeSummary>, String> {
        let connection = self.lock()?;
        list_workflow_types(&connection)
    }

    fn create_workflow_type(&self, name: &str) -> Result<WorkflowDefinition, String> {
        let name = required(name, "Workflow name")?;
        let connection = self.lock()?;
        let id = format!("workflow-type-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();
        connection
            .execute(
                "INSERT INTO workflow_types(id,name,created_at,updated_at) VALUES(?1,?2,?3,?3)",
                params![id, name, now],
            )
            .map_err(storage_error("create Workflow type"))?;
        load_workflow_type(&connection, &id)
    }

    fn load_workflow_type(&self, workflow_type_id: &str) -> Result<WorkflowDefinition, String> {
        let connection = self.lock()?;
        load_workflow_type(&connection, workflow_type_id)
    }

    fn update_workflow_type(
        &self,
        workflow_type_id: &str,
        name: &str,
    ) -> Result<WorkflowDefinition, String> {
        let name = required(name, "Workflow name")?;
        let connection = self.lock()?;
        let changed = connection
            .execute(
                "UPDATE workflow_types SET name=?2,updated_at=?3 WHERE id=?1",
                params![workflow_type_id, name, Utc::now().to_rfc3339()],
            )
            .map_err(storage_error("update Workflow type"))?;
        if changed == 0 {
            return Err(not_found(workflow_type_id));
        }
        load_workflow_type(&connection, workflow_type_id)
    }

    fn save_node_draft(
        &self,
        workflow_type_id: &str,
        node: WorkflowNodeConfig,
    ) -> Result<WorkflowDefinition, String> {
        required(&node.id, "Node id")?;
        let connection = self.lock()?;
        ensure_workflow_type(&connection, workflow_type_id)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(storage_error("begin Workflow node draft save"))?;
        ensure_element_owner(&transaction, "workflow_nodes", &node.id, workflow_type_id)?;
        let draft_json = json(&node, "serialize Workflow node")?;
        let live_json: Option<String> = transaction
            .query_row(
                "SELECT live_json FROM workflow_nodes WHERE id=?1",
                [&node.id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error("read Workflow node"))?
            .flatten();
        let changed = live_json.as_deref() != Some(draft_json.as_str());
        transaction
            .execute(
                "INSERT INTO workflow_nodes(id,workflow_type_id,draft_json,live_json,has_unpublished_changes) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET draft_json=excluded.draft_json,has_unpublished_changes=excluded.has_unpublished_changes",
                params![node.id, workflow_type_id, draft_json, live_json, changed],
            )
            .map_err(storage_error("save Workflow node draft"))?;
        touch(&transaction, workflow_type_id)?;
        transaction
            .commit()
            .map_err(storage_error("commit Workflow node draft save"))?;
        load_workflow_type(&connection, workflow_type_id)
    }

    fn delete_node_draft(
        &self,
        workflow_type_id: &str,
        node_id: &str,
    ) -> Result<WorkflowDefinition, String> {
        let connection = self.lock()?;
        ensure_workflow_type(&connection, workflow_type_id)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(storage_error("begin Workflow node deletion"))?;
        let live_json: Option<Option<String>> = transaction
            .query_row(
                "SELECT live_json FROM workflow_nodes WHERE id=?1 AND workflow_type_id=?2",
                params![node_id, workflow_type_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error("read Workflow node"))?;
        let Some(live_json) = live_json else {
            return Err(format!("Workflow node {node_id} does not exist."));
        };
        transaction
            .execute(
                "UPDATE workflow_nodes SET draft_json=NULL,has_unpublished_changes=?2 WHERE id=?1",
                params![node_id, live_json.is_some()],
            )
            .map_err(storage_error("delete Workflow node draft"))?;

        let connections = read_connection_elements(&transaction, workflow_type_id)?;
        for element in connections {
            let Some(mut draft) = element.draft else {
                continue;
            };
            let next_draft = if draft.sender_node_id == node_id {
                None
            } else if draft.receiver_node_id.as_deref() == Some(node_id) {
                draft.receiver_node_id = None;
                Some(draft)
            } else {
                continue;
            };
            let next_json = next_draft
                .as_ref()
                .map(|value| json(value, "serialize Workflow connection"))
                .transpose()?;
            let live_json = element
                .live
                .as_ref()
                .map(|value| json(value, "serialize live Workflow connection"))
                .transpose()?;
            transaction
                .execute(
                    "UPDATE workflow_connections SET draft_json=?2,has_unpublished_changes=?3 WHERE id=?1",
                    params![element.id, next_json, next_json != live_json],
                )
                .map_err(storage_error("update connections for deleted Workflow node"))?;
        }
        transaction
            .execute(
                "DELETE FROM workflow_connections WHERE workflow_type_id=?1 AND draft_json IS NULL AND live_json IS NULL AND has_unpublished_changes=0",
                [workflow_type_id],
            )
            .map_err(storage_error("retire unused Workflow connections"))?;
        transaction
            .execute(
                "DELETE FROM workflow_nodes WHERE id=?1 AND workflow_type_id=?2 AND draft_json IS NULL AND live_json IS NULL AND has_unpublished_changes=0",
                params![node_id, workflow_type_id],
            )
            .map_err(storage_error("retire unused Workflow node"))?;
        touch(&transaction, workflow_type_id)?;
        transaction
            .commit()
            .map_err(storage_error("commit Workflow node deletion"))?;
        load_workflow_type(&connection, workflow_type_id)
    }

    fn save_connection_draft(
        &self,
        workflow_type_id: &str,
        connection_config: WorkflowConnectionConfig,
    ) -> Result<WorkflowDefinition, String> {
        required(&connection_config.id, "Connection id")?;
        let connection = self.lock()?;
        ensure_workflow_type(&connection, workflow_type_id)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(storage_error("begin Workflow connection draft save"))?;
        ensure_element_owner(
            &transaction,
            "workflow_connections",
            &connection_config.id,
            workflow_type_id,
        )?;
        let draft_json = json(&connection_config, "serialize Workflow connection")?;
        let live_json: Option<String> = transaction
            .query_row(
                "SELECT live_json FROM workflow_connections WHERE id=?1",
                [&connection_config.id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error("read Workflow connection"))?
            .flatten();
        let changed = live_json.as_deref() != Some(draft_json.as_str());
        transaction
            .execute(
                "INSERT INTO workflow_connections(id,workflow_type_id,draft_json,live_json,has_unpublished_changes) VALUES(?1,?2,?3,?4,?5) ON CONFLICT(id) DO UPDATE SET draft_json=excluded.draft_json,has_unpublished_changes=excluded.has_unpublished_changes",
                params![connection_config.id, workflow_type_id, draft_json, live_json, changed],
            )
            .map_err(storage_error("save Workflow connection draft"))?;
        touch(&transaction, workflow_type_id)?;
        transaction
            .commit()
            .map_err(storage_error("commit Workflow connection draft save"))?;
        load_workflow_type(&connection, workflow_type_id)
    }

    fn delete_connection_draft(
        &self,
        workflow_type_id: &str,
        connection_id: &str,
    ) -> Result<WorkflowDefinition, String> {
        let connection = self.lock()?;
        ensure_workflow_type(&connection, workflow_type_id)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(storage_error("begin Workflow connection draft deletion"))?;
        let live_json: Option<Option<String>> = transaction
            .query_row(
                "SELECT live_json FROM workflow_connections WHERE id=?1 AND workflow_type_id=?2",
                params![connection_id, workflow_type_id],
                |row| row.get(0),
            )
            .optional()
            .map_err(storage_error("read Workflow connection"))?;
        let Some(live_json) = live_json else {
            return Err(format!(
                "Workflow connection {connection_id} does not exist."
            ));
        };
        if live_json.is_some() {
            transaction
                .execute(
                    "UPDATE workflow_connections SET draft_json=NULL,has_unpublished_changes=1 WHERE id=?1",
                    [connection_id],
                )
                .map_err(storage_error("delete Workflow connection draft"))?;
        } else {
            transaction
                .execute(
                    "DELETE FROM workflow_connections WHERE id=?1",
                    [connection_id],
                )
                .map_err(storage_error("retire unused Workflow connection"))?;
        }
        touch(&transaction, workflow_type_id)?;
        transaction
            .commit()
            .map_err(storage_error("commit Workflow connection draft deletion"))?;
        load_workflow_type(&connection, workflow_type_id)
    }

    fn activate_changes(
        &self,
        workflow_type_id: &str,
        elements: &[WorkflowElementRef],
    ) -> Result<WorkflowDefinition, String> {
        if elements.is_empty() {
            return Err("Select at least one edited Workflow element to activate.".to_string());
        }
        let connection = self.lock()?;
        ensure_workflow_type(&connection, workflow_type_id)?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(storage_error("begin Workflow activation"))?;
        let mut nodes = read_node_elements(&transaction, workflow_type_id)?
            .into_iter()
            .map(|element| (element.id.clone(), element))
            .collect::<BTreeMap<_, _>>();
        let mut connections = read_connection_elements(&transaction, workflow_type_id)?
            .into_iter()
            .map(|element| (element.id.clone(), element))
            .collect::<BTreeMap<_, _>>();
        let mut selected = HashSet::new();
        for element in elements {
            if !selected.insert((element.kind, element.id.clone())) {
                return Err(format!(
                    "Workflow element {} was selected more than once.",
                    element.id
                ));
            }
            match element.kind {
                WorkflowElementKind::Node => {
                    let node = nodes
                        .get_mut(&element.id)
                        .ok_or_else(|| format!("Workflow node {} does not exist.", element.id))?;
                    if !node.has_unpublished_changes {
                        return Err(format!(
                            "Workflow node {} has no unpublished changes.",
                            element.id
                        ));
                    }
                    node.live = node.draft.clone();
                }
                WorkflowElementKind::Connection => {
                    let connection = connections.get_mut(&element.id).ok_or_else(|| {
                        format!("Workflow connection {} does not exist.", element.id)
                    })?;
                    if !connection.has_unpublished_changes {
                        return Err(format!(
                            "Workflow connection {} has no unpublished changes.",
                            element.id
                        ));
                    }
                    connection.live = connection.draft.clone();
                }
            }
        }

        let candidate_nodes = nodes
            .values()
            .filter_map(|element| element.live.clone())
            .collect::<Vec<_>>();
        let candidate_connections = connections
            .values()
            .filter_map(|element| element.live.clone())
            .collect::<Vec<_>>();
        validate_candidate(&candidate_nodes, &candidate_connections)?;

        let ordinal = transaction
            .query_row(
                "SELECT COALESCE(MAX(ordinal),0)+1 FROM workflow_effective_recipes WHERE workflow_type_id=?1",
                [workflow_type_id],
                |row| row.get::<_, u32>(0),
            )
            .map_err(storage_error("allocate Workflow recipe ordinal"))?;
        let recipe_id = format!("workflow-recipe-{}", Uuid::new_v4());
        let created_at = Utc::now().to_rfc3339();
        transaction
            .execute(
                "INSERT INTO workflow_effective_recipes(id,workflow_type_id,ordinal,nodes_json,connections_json,created_at) VALUES(?1,?2,?3,?4,?5,?6)",
                params![
                    recipe_id,
                    workflow_type_id,
                    ordinal,
                    json(&candidate_nodes, "serialize effective Workflow nodes")?,
                    json(
                        &candidate_connections,
                        "serialize effective Workflow connections"
                    )?,
                    created_at
                ],
            )
            .map_err(storage_error("store effective Workflow recipe"))?;

        for element in elements {
            let table = match element.kind {
                WorkflowElementKind::Node => "workflow_nodes",
                WorkflowElementKind::Connection => "workflow_connections",
            };
            transaction
                .execute(
                    &format!("UPDATE {table} SET live_json=draft_json,has_unpublished_changes=0 WHERE id=?1 AND workflow_type_id=?2"),
                    params![element.id, workflow_type_id],
                )
                .map_err(storage_error("publish Workflow element"))?;
            transaction
                .execute(
                    &format!("DELETE FROM {table} WHERE id=?1 AND workflow_type_id=?2 AND draft_json IS NULL AND live_json IS NULL"),
                    params![element.id, workflow_type_id],
                )
                .map_err(storage_error("retire deleted Workflow element"))?;
        }
        transaction
            .execute(
                "UPDATE workflow_types SET active_recipe_id=?2,updated_at=?3 WHERE id=?1",
                params![workflow_type_id, recipe_id, created_at],
            )
            .map_err(storage_error("publish effective Workflow recipe"))?;
        transaction
            .commit()
            .map_err(storage_error("commit Workflow activation"))?;
        load_workflow_type(&connection, workflow_type_id)
    }

    fn native_query(&self) -> Result<WorkflowNativeQuery, String> {
        let connection = self.lock()?;
        let workflow_types = list_workflow_types(&connection)?
            .into_iter()
            .map(|summary| load_workflow_type(&connection, &summary.id))
            .collect::<Result<Vec<_>, _>>()?;
        Ok(WorkflowNativeQuery {
            schema_version: "workflow-native-query/v1",
            workflow_types,
        })
    }
}

fn validate_candidate(
    nodes: &[WorkflowNodeConfig],
    connections: &[WorkflowConnectionConfig],
) -> Result<(), String> {
    let starting_points = nodes.iter().filter(|node| node.is_starting_point).count();
    if starting_points != 1 {
        return Err(format!(
            "Workflow activation requires exactly one starting node; found {starting_points}."
        ));
    }
    let node_ids = nodes
        .iter()
        .map(|node| node.id.as_str())
        .collect::<HashSet<_>>();
    for connection in connections {
        if !node_ids.contains(connection.sender_node_id.as_str()) {
            return Err(format!(
                "Workflow connection {} has no live sender node.",
                connection.id
            ));
        }
        let Some(receiver_node_id) = connection.receiver_node_id.as_deref() else {
            return Err(format!(
                "Workflow connection {} is dangling.",
                connection.id
            ));
        };
        if !node_ids.contains(receiver_node_id) {
            return Err(format!(
                "Workflow connection {} has no live receiver node.",
                connection.id
            ));
        }
    }
    Ok(())
}

fn list_workflow_types(connection: &Connection) -> Result<Vec<WorkflowTypeSummary>, String> {
    let mut statement = connection
        .prepare(
            "SELECT type.id,type.name,type.active_recipe_id,\
             (SELECT COUNT(*) FROM workflow_nodes node WHERE node.workflow_type_id=type.id AND node.has_unpublished_changes=1) + \
             (SELECT COUNT(*) FROM workflow_connections edge WHERE edge.workflow_type_id=type.id AND edge.has_unpublished_changes=1),\
             type.created_at,type.updated_at FROM workflow_types type ORDER BY type.name COLLATE NOCASE,type.id",
        )
        .map_err(storage_error("prepare Workflow type query"))?;
    let workflow_types = statement
        .query_map([], |row| {
            Ok(WorkflowTypeSummary {
                id: row.get(0)?,
                name: row.get(1)?,
                active_recipe_id: row.get(2)?,
                edited_element_count: row.get(3)?,
                created_at: row.get(4)?,
                updated_at: row.get(5)?,
            })
        })
        .map_err(storage_error("query Workflow types"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error("read Workflow types"))?;
    Ok(workflow_types)
}

fn load_workflow_type(
    connection: &Connection,
    workflow_type_id: &str,
) -> Result<WorkflowDefinition, String> {
    let workflow_type = connection
        .query_row(
            "SELECT type.id,type.name,type.active_recipe_id,\
             (SELECT COUNT(*) FROM workflow_nodes node WHERE node.workflow_type_id=type.id AND node.has_unpublished_changes=1) + \
             (SELECT COUNT(*) FROM workflow_connections edge WHERE edge.workflow_type_id=type.id AND edge.has_unpublished_changes=1),\
             type.created_at,type.updated_at FROM workflow_types type WHERE type.id=?1",
            [workflow_type_id],
            |row| {
                Ok(WorkflowTypeSummary {
                    id: row.get(0)?,
                    name: row.get(1)?,
                    active_recipe_id: row.get(2)?,
                    edited_element_count: row.get(3)?,
                    created_at: row.get(4)?,
                    updated_at: row.get(5)?,
                })
            },
        )
        .optional()
        .map_err(storage_error("load Workflow type"))?
        .ok_or_else(|| not_found(workflow_type_id))?;
    let active_recipe = workflow_type
        .active_recipe_id
        .as_deref()
        .map(|recipe_id| load_recipe(connection, recipe_id))
        .transpose()?;
    Ok(WorkflowDefinition {
        workflow_type,
        nodes: read_node_elements(connection, workflow_type_id)?,
        connections: read_connection_elements(connection, workflow_type_id)?,
        active_recipe,
    })
}

fn load_recipe(connection: &Connection, recipe_id: &str) -> Result<EffectiveRecipe, String> {
    connection
        .query_row(
            "SELECT id,workflow_type_id,ordinal,created_at,nodes_json,connections_json FROM workflow_effective_recipes WHERE id=?1",
            [recipe_id],
            |row| {
                let nodes_json: String = row.get(4)?;
                let connections_json: String = row.get(5)?;
                Ok((
                    row.get(0)?,
                    row.get(1)?,
                    row.get(2)?,
                    row.get(3)?,
                    nodes_json,
                    connections_json,
                ))
            },
        )
        .optional()
        .map_err(storage_error("load effective Workflow recipe"))?
        .ok_or_else(|| format!("Effective Workflow recipe {recipe_id} does not exist."))
        .and_then(
            |(id, workflow_type_id, ordinal, created_at, nodes_json, connections_json)| {
                Ok(EffectiveRecipe {
                    id,
                    workflow_type_id,
                    ordinal,
                    created_at,
                    nodes: from_json(&nodes_json, "read effective Workflow nodes")?,
                    connections: from_json(
                        &connections_json,
                        "read effective Workflow connections",
                    )?,
                })
            },
        )
}

fn read_node_elements(
    connection: &Connection,
    workflow_type_id: &str,
) -> Result<Vec<WorkflowNodeElement>, String> {
    let mut statement = connection
        .prepare("SELECT id,draft_json,live_json,has_unpublished_changes FROM workflow_nodes WHERE workflow_type_id=?1 ORDER BY id")
        .map_err(storage_error("prepare Workflow node query"))?;
    let rows = statement
        .query_map([workflow_type_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, bool>(3)?,
            ))
        })
        .map_err(storage_error("query Workflow nodes"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error("read Workflow nodes"))?;
    rows.into_iter()
        .map(|(id, draft_json, live_json, has_unpublished_changes)| {
            Ok(WorkflowNodeElement {
                id,
                draft: draft_json
                    .as_deref()
                    .map(|value| from_json(value, "read Workflow node draft"))
                    .transpose()?,
                live: live_json
                    .as_deref()
                    .map(|value| from_json(value, "read live Workflow node"))
                    .transpose()?,
                has_unpublished_changes,
            })
        })
        .collect()
}

fn read_connection_elements(
    connection: &Connection,
    workflow_type_id: &str,
) -> Result<Vec<WorkflowConnectionElement>, String> {
    let mut statement = connection
        .prepare("SELECT id,draft_json,live_json,has_unpublished_changes FROM workflow_connections WHERE workflow_type_id=?1 ORDER BY id")
        .map_err(storage_error("prepare Workflow connection query"))?;
    let rows = statement
        .query_map([workflow_type_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, bool>(3)?,
            ))
        })
        .map_err(storage_error("query Workflow connections"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error("read Workflow connections"))?;
    rows.into_iter()
        .map(|(id, draft_json, live_json, has_unpublished_changes)| {
            Ok(WorkflowConnectionElement {
                id,
                draft: draft_json
                    .as_deref()
                    .map(|value| from_json(value, "read Workflow connection draft"))
                    .transpose()?,
                live: live_json
                    .as_deref()
                    .map(|value| from_json(value, "read live Workflow connection"))
                    .transpose()?,
                has_unpublished_changes,
            })
        })
        .collect()
}

fn ensure_workflow_type(connection: &Connection, workflow_type_id: &str) -> Result<(), String> {
    let exists = connection
        .query_row(
            "SELECT EXISTS(SELECT 1 FROM workflow_types WHERE id=?1)",
            [workflow_type_id],
            |row| row.get::<_, bool>(0),
        )
        .map_err(storage_error("find Workflow type"))?;
    if exists {
        Ok(())
    } else {
        Err(not_found(workflow_type_id))
    }
}

fn ensure_element_owner(
    connection: &Connection,
    table: &str,
    element_id: &str,
    workflow_type_id: &str,
) -> Result<(), String> {
    let owner: Option<String> = connection
        .query_row(
            &format!("SELECT workflow_type_id FROM {table} WHERE id=?1"),
            [element_id],
            |row| row.get(0),
        )
        .optional()
        .map_err(storage_error("inspect Workflow element ownership"))?;
    if owner
        .as_deref()
        .is_some_and(|owner| owner != workflow_type_id)
    {
        Err(format!(
            "Workflow element {element_id} belongs to another Workflow type."
        ))
    } else {
        Ok(())
    }
}

fn touch(connection: &Connection, workflow_type_id: &str) -> Result<(), String> {
    connection
        .execute(
            "UPDATE workflow_types SET updated_at=?2 WHERE id=?1",
            params![workflow_type_id, Utc::now().to_rfc3339()],
        )
        .map_err(storage_error("update Workflow timestamp"))?;
    Ok(())
}

fn required<'a>(value: &'a str, label: &str) -> Result<&'a str, String> {
    let value = value.trim();
    if value.is_empty() {
        Err(format!("{label} is required."))
    } else {
        Ok(value)
    }
}

fn json<T: serde::Serialize>(value: &T, action: &str) -> Result<String, String> {
    serde_json::to_string(value).map_err(|error| format!("Unable to {action}: {error}"))
}

fn from_json<T: serde::de::DeserializeOwned>(value: &str, action: &str) -> Result<T, String> {
    serde_json::from_str(value).map_err(|error| format!("Unable to {action}: {error}"))
}

fn not_found(workflow_type_id: &str) -> String {
    format!("Workflow type {workflow_type_id} does not exist.")
}

fn storage_error(action: &'static str) -> impl FnOnce(rusqlite::Error) -> String {
    move |error| format!("Unable to {action}: {error}")
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::workflows::application::WorkflowRepository;

    fn node(id: &str, start: bool) -> WorkflowNodeConfig {
        WorkflowNodeConfig {
            id: id.to_string(),
            name: id.to_string(),
            harness_name: "Default Harness".to_string(),
            role_name: None,
            position_x: 10.0,
            position_y: 20.0,
            is_starting_point: start,
        }
    }

    fn connection(id: &str, sender: &str, receiver: Option<&str>) -> WorkflowConnectionConfig {
        WorkflowConnectionConfig {
            id: id.to_string(),
            name: id.to_string(),
            sender_node_id: sender.to_string(),
            receiver_node_id: receiver.map(str::to_string),
        }
    }

    fn all(definition: &WorkflowDefinition) -> Vec<WorkflowElementRef> {
        definition
            .nodes
            .iter()
            .filter(|element| element.has_unpublished_changes)
            .map(|element| WorkflowElementRef {
                kind: WorkflowElementKind::Node,
                id: element.id.clone(),
            })
            .chain(
                definition
                    .connections
                    .iter()
                    .filter(|element| element.has_unpublished_changes)
                    .map(|element| WorkflowElementRef {
                        kind: WorkflowElementKind::Connection,
                        id: element.id.clone(),
                    }),
            )
            .collect()
    }

    #[test]
    fn drafts_may_be_incomplete_but_activation_requires_one_start_and_no_dangling_edges() {
        let repository = SqliteWorkflowRepository::new(Connection::open_in_memory().unwrap())
            .expect("repository");
        let workflow = repository.create_workflow_type("Review").unwrap();
        let id = workflow.workflow_type.id;
        let definition = repository.save_node_draft(&id, node("a", false)).unwrap();
        let error = repository
            .activate_changes(&id, &all(&definition))
            .expect_err("missing start blocks activation");
        assert!(error.contains("exactly one starting node"));

        repository.save_node_draft(&id, node("a", true)).unwrap();
        repository.save_node_draft(&id, node("b", false)).unwrap();
        let definition = repository
            .save_connection_draft(&id, connection("edge", "a", None))
            .unwrap();
        let error = repository
            .activate_changes(&id, &all(&definition))
            .expect_err("dangling connection blocks activation");
        assert!(error.contains("dangling"));

        let definition = repository
            .save_connection_draft(&id, connection("edge", "a", Some("b")))
            .unwrap();
        let activated = repository.activate_changes(&id, &all(&definition)).unwrap();
        assert_eq!(activated.active_recipe.as_ref().unwrap().ordinal, 1);
        assert_eq!(activated.active_recipe.as_ref().unwrap().nodes.len(), 2);
        assert_eq!(
            activated.active_recipe.as_ref().unwrap().connections.len(),
            1
        );
        assert_eq!(activated.workflow_type.edited_element_count, 0);
    }

    #[test]
    fn deleting_a_node_deletes_outgoing_drafts_and_leaves_incoming_drafts_dangling() {
        let repository = SqliteWorkflowRepository::new(Connection::open_in_memory().unwrap())
            .expect("repository");
        let id = repository
            .create_workflow_type("Cycle")
            .unwrap()
            .workflow_type
            .id;
        repository.save_node_draft(&id, node("a", true)).unwrap();
        repository.save_node_draft(&id, node("b", false)).unwrap();
        repository
            .save_connection_draft(&id, connection("out", "b", Some("a")))
            .unwrap();
        let definition = repository
            .save_connection_draft(&id, connection("in", "a", Some("b")))
            .unwrap();
        repository.activate_changes(&id, &all(&definition)).unwrap();

        let deleted = repository.delete_node_draft(&id, "b").unwrap();
        assert!(deleted
            .nodes
            .iter()
            .find(|node| node.id == "b")
            .unwrap()
            .draft
            .is_none());
        assert!(deleted
            .connections
            .iter()
            .find(|edge| edge.id == "out")
            .unwrap()
            .draft
            .is_none());
        assert_eq!(
            deleted
                .connections
                .iter()
                .find(|edge| edge.id == "in")
                .unwrap()
                .draft
                .as_ref()
                .unwrap()
                .receiver_node_id,
            None
        );
        assert!(repository.activate_changes(&id, &all(&deleted)).is_err());
    }

    #[test]
    fn deleting_an_unpublished_node_retires_it_and_its_outgoing_connection() {
        let repository = SqliteWorkflowRepository::new(Connection::open_in_memory().unwrap())
            .expect("repository");
        let id = repository
            .create_workflow_type("Draft")
            .unwrap()
            .workflow_type
            .id;
        repository.save_node_draft(&id, node("a", true)).unwrap();
        repository.save_node_draft(&id, node("b", false)).unwrap();
        repository
            .save_connection_draft(&id, connection("out", "b", Some("a")))
            .unwrap();

        let deleted = repository.delete_node_draft(&id, "b").unwrap();

        assert!(deleted.nodes.iter().all(|node| node.id != "b"));
        assert!(deleted.connections.iter().all(|edge| edge.id != "out"));
        assert_eq!(deleted.workflow_type.edited_element_count, 1);

        repository
            .save_connection_draft(&id, connection("unused", "a", Some("a")))
            .unwrap();
        let deleted = repository.delete_connection_draft(&id, "unused").unwrap();
        assert!(deleted.connections.iter().all(|edge| edge.id != "unused"));
    }

    #[test]
    fn effective_recipes_are_immutable_and_reopen_with_deterministic_queries() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("workflow.sqlite");
        let workflow_type_id;
        let first_recipe_id;
        {
            let repository = SqliteWorkflowRepository::open(&path).unwrap();
            workflow_type_id = repository
                .create_workflow_type("Zeta")
                .unwrap()
                .workflow_type
                .id;
            let definition = repository
                .save_node_draft(&workflow_type_id, node("start", true))
                .unwrap();
            let activated = repository
                .activate_changes(&workflow_type_id, &all(&definition))
                .unwrap();
            first_recipe_id = activated.active_recipe.unwrap().id;
            let mut edited = node("start", true);
            edited.name = "Edited draft".to_string();
            repository
                .save_node_draft(&workflow_type_id, edited)
                .unwrap();
            repository.create_workflow_type("Alpha").unwrap();
        }

        let reopened = SqliteWorkflowRepository::open(&path).unwrap();
        let definition = reopened.load_workflow_type(&workflow_type_id).unwrap();
        assert_eq!(
            definition.active_recipe.as_ref().unwrap().id,
            first_recipe_id
        );
        assert_eq!(
            definition.active_recipe.as_ref().unwrap().nodes[0].name,
            "start"
        );
        assert_eq!(
            definition.nodes[0].draft.as_ref().unwrap().name,
            "Edited draft"
        );
        assert_eq!(definition.nodes[0].live.as_ref().unwrap().name, "start");
        let query = reopened.native_query().unwrap();
        assert_eq!(query.schema_version, "workflow-native-query/v1");
        assert_eq!(
            query
                .workflow_types
                .iter()
                .map(|definition| definition.workflow_type.name.as_str())
                .collect::<Vec<_>>(),
            vec!["Alpha", "Zeta"]
        );

        let second = reopened
            .activate_changes(&workflow_type_id, &all(&definition))
            .unwrap();
        assert_eq!(second.active_recipe.as_ref().unwrap().ordinal, 2);
        assert_eq!(
            second.active_recipe.as_ref().unwrap().nodes[0].name,
            "Edited draft"
        );
        let connection = reopened.lock().unwrap();
        let retained_first = load_recipe(&connection, &first_recipe_id).unwrap();
        assert_eq!(retained_first.ordinal, 1);
        assert_eq!(retained_first.nodes[0].name, "start");
    }
}
