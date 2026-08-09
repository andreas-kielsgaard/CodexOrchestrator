use super::{
    application::WorkflowRepository,
    domain::{
        EffectiveRecipe, EffectiveWorkflowNodeConfig, WorkflowConnectionConfig,
        WorkflowConnectionElement, WorkflowConnectionMechanism, WorkflowDefinition,
        WorkflowElementKind, WorkflowElementRef, WorkflowExpectedFileSelector,
        WorkflowHarnessConfig, WorkflowHarnessOverrides, WorkflowNativeQuery, WorkflowNodeConfig,
        WorkflowNodeElement, WorkflowNodeHarness, WorkflowRole, WorkflowTypeSummary,
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
    live_effective_json TEXT,
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
        initialize_workflow_role_schema(&connection)?;
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

    fn list_roles(&self) -> Result<Vec<WorkflowRole>, String> {
        let connection = self.lock()?;
        list_roles(&connection)
    }

    fn create_role(
        &self,
        name: &str,
        harness: WorkflowHarnessConfig,
    ) -> Result<WorkflowRole, String> {
        let name = required(name, "Role name")?;
        validate_harness(&harness)?;
        let connection = self.lock()?;
        let id = format!("workflow-role-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();
        connection
            .execute(
                "INSERT INTO workflow_roles(id,name,harness_json,created_at,updated_at) VALUES(?1,?2,?3,?4,?4)",
                params![id, name, json(&harness, "serialize Workflow Role")?, now],
            )
            .map_err(storage_error("create Workflow Role"))?;
        load_role(&connection, &id)
    }

    fn update_role(
        &self,
        role_id: &str,
        name: &str,
        harness: WorkflowHarnessConfig,
    ) -> Result<WorkflowRole, String> {
        let name = required(name, "Role name")?;
        validate_harness(&harness)?;
        let connection = self.lock()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(storage_error("begin Workflow Role update"))?;
        let changed = transaction
            .execute(
                "UPDATE workflow_roles SET name=?2,harness_json=?3,updated_at=?4 WHERE id=?1",
                params![
                    role_id,
                    name,
                    json(&harness, "serialize Workflow Role")?,
                    Utc::now().to_rfc3339()
                ],
            )
            .map_err(storage_error("update Workflow Role"))?;
        if changed == 0 {
            return Err(format!("Workflow Role {role_id} does not exist."));
        }
        refresh_role_dependents(&transaction, role_id)?;
        transaction
            .commit()
            .map_err(storage_error("commit Workflow Role update"))?;
        load_role(&connection, role_id)
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
        validate_node_harness(&transaction, &node)?;
        save_node_draft_in_transaction(&transaction, workflow_type_id, &node)?;
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

    fn detach_node_role(
        &self,
        workflow_type_id: &str,
        node_id: &str,
    ) -> Result<WorkflowDefinition, String> {
        let connection = self.lock()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(storage_error("begin Workflow Role detach"))?;
        ensure_workflow_type(&transaction, workflow_type_id)?;
        let mut node = load_draft_node(&transaction, workflow_type_id, node_id)?;
        if !matches!(node.harness, Some(WorkflowNodeHarness::Role { .. })) {
            return Err("Only a Role-backed Workflow node can be detached.".to_string());
        }
        let effective = resolve_node_harness(&transaction, &node)?;
        node.harness_name = effective.harness_name.clone();
        node.role_name = None;
        node.harness = Some(WorkflowNodeHarness::Standalone { config: effective });
        save_node_draft_in_transaction(&transaction, workflow_type_id, &node)?;
        touch(&transaction, workflow_type_id)?;
        transaction
            .commit()
            .map_err(storage_error("commit Workflow Role detach"))?;
        load_workflow_type(&connection, workflow_type_id)
    }

    fn save_node_as_role(
        &self,
        workflow_type_id: &str,
        node_id: &str,
        role_name: &str,
    ) -> Result<WorkflowDefinition, String> {
        let role_name = required(role_name, "Role name")?;
        let connection = self.lock()?;
        let transaction = connection
            .unchecked_transaction()
            .map_err(storage_error("begin save Workflow node as Role"))?;
        ensure_workflow_type(&transaction, workflow_type_id)?;
        let mut node = load_draft_node(&transaction, workflow_type_id, node_id)?;
        let effective = resolve_node_harness(&transaction, &node)?;
        validate_harness(&effective)?;
        let role_id = format!("workflow-role-{}", Uuid::new_v4());
        let now = Utc::now().to_rfc3339();
        transaction
            .execute(
                "INSERT INTO workflow_roles(id,name,harness_json,created_at,updated_at) VALUES(?1,?2,?3,?4,?4)",
                params![
                    role_id,
                    role_name,
                    json(&effective, "serialize Workflow Role")?,
                    now
                ],
            )
            .map_err(storage_error("create Workflow Role from node"))?;
        node.role_name = Some(role_name.to_string());
        node.harness = Some(WorkflowNodeHarness::Role {
            role_id,
            overrides: WorkflowHarnessOverrides::default(),
        });
        save_node_draft_in_transaction(&transaction, workflow_type_id, &node)?;
        touch(&transaction, workflow_type_id)?;
        transaction
            .commit()
            .map_err(storage_error("commit save Workflow node as Role"))?;
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
            .filter_map(|element| {
                element.live.as_ref().map(|node| {
                    let harness =
                        if selected.contains(&(WorkflowElementKind::Node, node.id.clone())) {
                            resolve_node_harness(&transaction, node)
                        } else {
                            element.live_effective_harness.clone().ok_or_else(|| {
                                format!(
                                    "Activated Workflow node {} has no materialized Harness.",
                                    node.id
                                )
                            })
                        }?;
                    Ok(materialize_node(node, harness))
                })
            })
            .collect::<Result<Vec<_>, String>>()?;
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
                WorkflowElementKind::Node => {
                    let effective_json = nodes
                        .get(&element.id)
                        .and_then(|candidate| candidate.draft.as_ref())
                        .map(|node| resolve_node_harness(&transaction, node))
                        .transpose()?
                        .as_ref()
                        .map(|effective| json(effective, "serialize activated Workflow Harness"))
                        .transpose()?;
                    transaction
                        .execute(
                            "UPDATE workflow_nodes SET live_json=draft_json,live_effective_json=?3,has_unpublished_changes=0 WHERE id=?1 AND workflow_type_id=?2",
                            params![element.id, workflow_type_id, effective_json],
                        )
                        .map_err(storage_error("publish Workflow node"))?;
                    "workflow_nodes"
                }
                WorkflowElementKind::Connection => {
                    transaction
                        .execute(
                            "UPDATE workflow_connections SET live_json=draft_json,has_unpublished_changes=0 WHERE id=?1 AND workflow_type_id=?2",
                            params![element.id, workflow_type_id],
                        )
                        .map_err(storage_error("publish Workflow connection"))?;
                    "workflow_connections"
                }
            };
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
            schema_version: "workflow-native-query/v3",
            workflow_types,
            roles: list_roles(&connection)?,
        })
    }
}

fn validate_candidate(
    nodes: &[EffectiveWorkflowNodeConfig],
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
        validate_connection_mechanism(connection)?;
    }
    Ok(())
}

fn validate_connection_mechanism(connection: &WorkflowConnectionConfig) -> Result<(), String> {
    let Some(mechanism) = &connection.mechanism else {
        return Err(format!(
            "Workflow connection {} needs a connecting mechanism before activation.",
            connection.id
        ));
    };
    match mechanism {
        WorkflowConnectionMechanism::TurnFinishedExpectedFile {
            file_selector,
            description_text,
            prompt_text,
            ..
        } => {
            required(description_text, "Connection file description")?;
            required(prompt_text, "Connection prompt")?;
            match file_selector {
                WorkflowExpectedFileSelector::FolderFilenamePattern {
                    folder,
                    filename_pattern,
                } => {
                    required(folder, "Expected file folder")?;
                    required(filename_pattern, "Expected filename pattern")?;
                }
                WorkflowExpectedFileSelector::FolderOutputRegex {
                    folder,
                    output_regex,
                } => {
                    required(folder, "Expected file folder")?;
                    required(output_regex, "Agent output regex")?;
                }
            }
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
                    nodes: read_effective_nodes(&nodes_json)?,
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
        .prepare("SELECT id,draft_json,live_json,has_unpublished_changes,live_effective_json FROM workflow_nodes WHERE workflow_type_id=?1 ORDER BY id")
        .map_err(storage_error("prepare Workflow node query"))?;
    let rows = statement
        .query_map([workflow_type_id], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, Option<String>>(1)?,
                row.get::<_, Option<String>>(2)?,
                row.get::<_, bool>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(storage_error("query Workflow nodes"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error("read Workflow nodes"))?;
    rows.into_iter()
        .map(
            |(id, draft_json, live_json, has_unpublished_changes, live_effective_json)| {
                let draft: Option<WorkflowNodeConfig> = draft_json
                    .as_deref()
                    .map(|value| from_json(value, "read Workflow node draft"))
                    .transpose()?;
                let live: Option<WorkflowNodeConfig> = live_json
                    .as_deref()
                    .map(|value| from_json(value, "read live Workflow node"))
                    .transpose()?;
                let draft_effective_harness = draft
                    .as_ref()
                    .map(|node| resolve_node_harness(connection, node))
                    .transpose()?;
                let live_effective_harness = live_effective_json
                    .as_deref()
                    .map(|value| from_json(value, "read activated Workflow Harness"))
                    .transpose()?
                    .or_else(|| live.as_ref().map(legacy_harness));
                Ok(WorkflowNodeElement {
                    id,
                    draft,
                    live,
                    has_unpublished_changes,
                    draft_effective_harness,
                    live_effective_harness,
                })
            },
        )
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

fn list_roles(connection: &Connection) -> Result<Vec<WorkflowRole>, String> {
    let mut statement = connection
        .prepare(
            "SELECT id,name,harness_json,created_at,updated_at FROM workflow_roles ORDER BY name COLLATE NOCASE,id",
        )
        .map_err(storage_error("prepare Workflow Role query"))?;
    let roles = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
            ))
        })
        .map_err(storage_error("query Workflow Roles"))?
        .map(|row| {
            let (id, name, harness_json, created_at, updated_at) =
                row.map_err(storage_error("read Workflow Role"))?;
            Ok(WorkflowRole {
                id,
                name,
                harness: from_json(&harness_json, "read Workflow Role Harness")?,
                created_at,
                updated_at,
            })
        })
        .collect();
    roles
}

fn load_role(connection: &Connection, role_id: &str) -> Result<WorkflowRole, String> {
    connection
        .query_row(
            "SELECT id,name,harness_json,created_at,updated_at FROM workflow_roles WHERE id=?1",
            [role_id],
            |row| {
                Ok((
                    row.get::<_, String>(0)?,
                    row.get::<_, String>(1)?,
                    row.get::<_, String>(2)?,
                    row.get::<_, String>(3)?,
                    row.get::<_, String>(4)?,
                ))
            },
        )
        .optional()
        .map_err(storage_error("load Workflow Role"))?
        .ok_or_else(|| format!("Workflow Role {role_id} does not exist."))
        .and_then(|(id, name, harness_json, created_at, updated_at)| {
            Ok(WorkflowRole {
                id,
                name,
                harness: from_json(&harness_json, "read Workflow Role Harness")?,
                created_at,
                updated_at,
            })
        })
}

fn validate_node_harness(connection: &Connection, node: &WorkflowNodeConfig) -> Result<(), String> {
    if let Some(WorkflowNodeHarness::Role { role_id, .. }) = &node.harness {
        load_role(connection, role_id)?;
    }
    validate_harness(&resolve_node_harness(connection, node)?)
}

fn validate_harness(harness: &WorkflowHarnessConfig) -> Result<(), String> {
    required(&harness.harness_name, "Harness name")?;
    for server in &harness.mcp_servers {
        required(&server.server_name, "MCP server name")?;
        if let super::domain::WorkflowMcpServerAccess::SelectedTools { tool_names } = &server.access
        {
            for tool_name in tool_names {
                required(tool_name, "MCP tool name")?;
            }
        }
    }
    Ok(())
}

fn resolve_node_harness(
    connection: &Connection,
    node: &WorkflowNodeConfig,
) -> Result<WorkflowHarnessConfig, String> {
    match &node.harness {
        Some(WorkflowNodeHarness::Standalone { config }) => Ok(config.clone()),
        Some(WorkflowNodeHarness::Role { role_id, overrides }) => {
            let role = load_role(connection, role_id)?;
            Ok(apply_overrides(role.harness, overrides))
        }
        None => Ok(legacy_harness(node)),
    }
}

fn apply_overrides(
    mut harness: WorkflowHarnessConfig,
    overrides: &WorkflowHarnessOverrides,
) -> WorkflowHarnessConfig {
    if let Some(value) = &overrides.harness_name {
        harness.harness_name = value.clone();
    }
    if let Some(value) = &overrides.role_identity {
        harness.role_identity = value.clone();
    }
    if let Some(value) = &overrides.instructions {
        harness.instructions = value.clone();
    }
    if let Some(value) = &overrides.skills {
        harness.skills = value.clone();
    }
    if let Some(value) = &overrides.mcp_servers {
        harness.mcp_servers = value.clone();
    }
    if let Some(value) = &overrides.hooks {
        harness.hooks = value.clone();
    }
    if let Some(value) = &overrides.runtime {
        harness.runtime = value.clone();
    }
    harness
}

fn legacy_harness(node: &WorkflowNodeConfig) -> WorkflowHarnessConfig {
    WorkflowHarnessConfig {
        harness_name: node.harness_name.clone(),
        role_identity: node.role_name.clone().unwrap_or_default(),
        ..WorkflowHarnessConfig::default()
    }
}

fn materialize_node(
    node: &WorkflowNodeConfig,
    harness: WorkflowHarnessConfig,
) -> EffectiveWorkflowNodeConfig {
    EffectiveWorkflowNodeConfig {
        id: node.id.clone(),
        name: node.name.clone(),
        position_x: node.position_x,
        position_y: node.position_y,
        is_starting_point: node.is_starting_point,
        harness,
    }
}

fn read_effective_nodes(value: &str) -> Result<Vec<EffectiveWorkflowNodeConfig>, String> {
    if let Ok(nodes) = serde_json::from_str(value) {
        return Ok(nodes);
    }
    let legacy: Vec<WorkflowNodeConfig> = from_json(value, "read legacy effective Workflow nodes")?;
    Ok(legacy
        .iter()
        .map(|node| materialize_node(node, legacy_harness(node)))
        .collect())
}

fn save_node_draft_in_transaction(
    connection: &Connection,
    workflow_type_id: &str,
    node: &WorkflowNodeConfig,
) -> Result<(), String> {
    let draft_json = json(node, "serialize Workflow node")?;
    let draft_effective_json = json(
        &resolve_node_harness(connection, node)?,
        "serialize effective Workflow Harness draft",
    )?;
    let existing: Option<(Option<String>, Option<String>)> = connection
        .query_row(
            "SELECT live_json,live_effective_json FROM workflow_nodes WHERE id=?1",
            [&node.id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        )
        .optional()
        .map_err(storage_error("read Workflow node"))?;
    let (live_json, live_effective_json) = existing.unwrap_or((None, None));
    let changed = live_json.as_deref() != Some(draft_json.as_str())
        || live_effective_json.as_deref() != Some(draft_effective_json.as_str());
    connection
        .execute(
            "INSERT INTO workflow_nodes(id,workflow_type_id,draft_json,live_json,live_effective_json,has_unpublished_changes) VALUES(?1,?2,?3,?4,?5,?6) ON CONFLICT(id) DO UPDATE SET draft_json=excluded.draft_json,has_unpublished_changes=excluded.has_unpublished_changes",
            params![
                node.id,
                workflow_type_id,
                draft_json,
                live_json,
                live_effective_json,
                changed
            ],
        )
        .map_err(storage_error("save Workflow node draft"))?;
    Ok(())
}

fn load_draft_node(
    connection: &Connection,
    workflow_type_id: &str,
    node_id: &str,
) -> Result<WorkflowNodeConfig, String> {
    connection
        .query_row(
            "SELECT draft_json FROM workflow_nodes WHERE id=?1 AND workflow_type_id=?2",
            params![node_id, workflow_type_id],
            |row| row.get::<_, Option<String>>(0),
        )
        .optional()
        .map_err(storage_error("load Workflow node draft"))?
        .flatten()
        .ok_or_else(|| format!("Workflow node {node_id} has no draft."))
        .and_then(|value| from_json(&value, "read Workflow node draft"))
}

fn refresh_role_dependents(connection: &Connection, role_id: &str) -> Result<(), String> {
    let mut statement = connection
        .prepare(
            "SELECT id,workflow_type_id,draft_json,live_json,live_effective_json FROM workflow_nodes WHERE draft_json IS NOT NULL",
        )
        .map_err(storage_error("prepare Role dependent query"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, Option<String>>(3)?,
                row.get::<_, Option<String>>(4)?,
            ))
        })
        .map_err(storage_error("query Role dependents"))?
        .collect::<Result<Vec<_>, _>>()
        .map_err(storage_error("read Role dependents"))?;
    drop(statement);
    let mut touched = HashSet::new();
    for (node_id, workflow_type_id, draft_json, live_json, live_effective_json) in rows {
        let node: WorkflowNodeConfig = from_json(&draft_json, "read Role-dependent node")?;
        let bound = matches!(
            &node.harness,
            Some(WorkflowNodeHarness::Role { role_id: bound, .. }) if bound == role_id
        );
        if !bound {
            continue;
        }
        let effective_json = json(
            &resolve_node_harness(connection, &node)?,
            "serialize Role-dependent Harness",
        )?;
        let changed = live_json.as_deref() != Some(draft_json.as_str())
            || live_effective_json.as_deref() != Some(effective_json.as_str());
        connection
            .execute(
                "UPDATE workflow_nodes SET has_unpublished_changes=?2 WHERE id=?1",
                params![node_id, changed],
            )
            .map_err(storage_error("mark Role-dependent node draft"))?;
        touched.insert(workflow_type_id);
    }
    for workflow_type_id in touched {
        touch(connection, &workflow_type_id)?;
    }
    Ok(())
}

pub(crate) fn initialize_workflow_role_schema(connection: &Connection) -> Result<(), String> {
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS workflow_roles (
                id TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                harness_json TEXT NOT NULL,
                created_at TEXT NOT NULL,
                updated_at TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS workflow_roles_by_name
            ON workflow_roles(name, id);",
        )
        .map_err(storage_error("initialize Workflow Role schema"))?;
    let mut statement = connection
        .prepare("PRAGMA table_info(workflow_nodes)")
        .map_err(storage_error("inspect Workflow node schema"))?;
    let columns = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(storage_error("query Workflow node schema"))?
        .collect::<Result<HashSet<_>, _>>()
        .map_err(storage_error("read Workflow node schema"))?;
    drop(statement);
    if !columns.contains("live_effective_json") {
        connection
            .execute(
                "ALTER TABLE workflow_nodes ADD COLUMN live_effective_json TEXT",
                [],
            )
            .map_err(storage_error("add materialized Workflow Harness storage"))?;
    }
    Ok(())
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
            harness: None,
        }
    }

    fn harness(name: &str, identity: &str, instructions: &str) -> WorkflowHarnessConfig {
        WorkflowHarnessConfig {
            harness_name: name.to_string(),
            role_identity: identity.to_string(),
            instructions: instructions.to_string(),
            skills: vec!["review".to_string()],
            mcp_servers: Vec::new(),
            hooks: vec!["turn_finished".to_string()],
            runtime: super::super::domain::WorkflowHarnessRuntimeSettings {
                provider: "codex".to_string(),
                model: "gpt-5".to_string(),
                reasoning_effort: "medium".to_string(),
            },
        }
    }

    fn connection(id: &str, sender: &str, receiver: Option<&str>) -> WorkflowConnectionConfig {
        WorkflowConnectionConfig {
            id: id.to_string(),
            name: id.to_string(),
            sender_node_id: sender.to_string(),
            receiver_node_id: receiver.map(str::to_string),
            mechanism: Some(WorkflowConnectionMechanism::TurnFinishedExpectedFile {
                file_selector: WorkflowExpectedFileSelector::FolderFilenamePattern {
                    folder: "handoffs".to_string(),
                    filename_pattern: "*.md".to_string(),
                },
                description_text: "The sender handoff".to_string(),
                prompt_text: "Continue from this handoff.".to_string(),
                match_selection: super::super::domain::WorkflowMatchSelection::Newest,
                initial_check: super::super::domain::WorkflowInitialCheck::OnceImmediately,
            }),
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
    fn connection_mechanisms_remain_drafts_until_their_declared_inputs_are_complete() {
        let repository = SqliteWorkflowRepository::new(Connection::open_in_memory().unwrap())
            .expect("repository");
        let id = repository
            .create_workflow_type("Handoff")
            .unwrap()
            .workflow_type
            .id;
        repository
            .save_node_draft(&id, node("sender", true))
            .unwrap();
        repository
            .save_node_draft(&id, node("receiver", false))
            .unwrap();
        let mut edge = connection("handoff", "sender", Some("receiver"));
        edge.mechanism = None;
        let definition = repository.save_connection_draft(&id, edge.clone()).unwrap();
        assert!(definition.connections[0].has_unpublished_changes);
        let error = repository
            .activate_changes(&id, &all(&definition))
            .expect_err("missing mechanism blocks activation");
        assert!(error.contains("needs a connecting mechanism"));

        edge.mechanism = Some(WorkflowConnectionMechanism::TurnFinishedExpectedFile {
            file_selector: WorkflowExpectedFileSelector::FolderOutputRegex {
                folder: "handoffs".to_string(),
                output_regex: String::new(),
            },
            description_text: "Handoff file".to_string(),
            prompt_text: "Continue.".to_string(),
            match_selection: super::super::domain::WorkflowMatchSelection::Newest,
            initial_check: super::super::domain::WorkflowInitialCheck::OnceImmediately,
        });
        let definition = repository.save_connection_draft(&id, edge.clone()).unwrap();
        let error = repository
            .activate_changes(&id, &all(&definition))
            .expect_err("empty regex blocks activation");
        assert!(error.contains("Agent output regex is required"));

        if let Some(WorkflowConnectionMechanism::TurnFinishedExpectedFile {
            file_selector: WorkflowExpectedFileSelector::FolderOutputRegex { output_regex, .. },
            ..
        }) = &mut edge.mechanism
        {
            *output_regex = r"handoff: (.+\.md)".to_string();
        }
        let definition = repository.save_connection_draft(&id, edge).unwrap();
        let activated = repository.activate_changes(&id, &all(&definition)).unwrap();
        assert!(matches!(
            activated.active_recipe.unwrap().connections[0].mechanism,
            Some(WorkflowConnectionMechanism::TurnFinishedExpectedFile {
                file_selector: WorkflowExpectedFileSelector::FolderOutputRegex { .. },
                ..
            })
        ));
    }

    #[test]
    fn legacy_connection_json_without_a_mechanism_reopens_as_an_incomplete_draft_shape() {
        let parsed: WorkflowConnectionConfig = serde_json::from_str(
            r#"{"id":"edge","name":"Old edge","senderNodeId":"a","receiverNodeId":"b"}"#,
        )
        .unwrap();
        assert_eq!(parsed.mechanism, None);
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

        let mut reconnected = deleted
            .connections
            .iter()
            .find(|edge| edge.id == "in")
            .unwrap()
            .draft
            .clone()
            .unwrap();
        reconnected.receiver_node_id = Some("a".to_string());
        let repaired = repository.save_connection_draft(&id, reconnected).unwrap();
        let activated = repository.activate_changes(&id, &all(&repaired)).unwrap();
        assert!(activated
            .active_recipe
            .unwrap()
            .nodes
            .iter()
            .all(|node| node.id != "b"));
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
        assert_eq!(query.schema_version, "workflow-native-query/v3");
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

    #[test]
    fn role_edits_flow_into_unoverridden_drafts_but_not_activated_recipes() {
        let repository =
            SqliteWorkflowRepository::new(Connection::open_in_memory().unwrap()).unwrap();
        let role = repository
            .create_role(
                "Reviewer",
                harness("Review Harness", "Review", "Base instructions"),
            )
            .unwrap();
        let workflow_type_id = repository
            .create_workflow_type("Review")
            .unwrap()
            .workflow_type
            .id;
        let mut start = node("start", true);
        start.harness = Some(WorkflowNodeHarness::Role {
            role_id: role.id.clone(),
            overrides: WorkflowHarnessOverrides {
                instructions: Some("Instance instructions".to_string()),
                ..WorkflowHarnessOverrides::default()
            },
        });
        let draft = repository
            .save_node_draft(&workflow_type_id, start)
            .unwrap();
        let first = repository
            .activate_changes(&workflow_type_id, &all(&draft))
            .unwrap();
        let first_recipe_id = first.active_recipe.as_ref().unwrap().id.clone();

        repository
            .update_role(
                &role.id,
                "Reviewer",
                harness("Review Harness", "Security review", "Changed base"),
            )
            .unwrap();
        let changed = repository.load_workflow_type(&workflow_type_id).unwrap();
        assert!(changed.nodes[0].has_unpublished_changes);
        let effective = changed.nodes[0].draft_effective_harness.as_ref().unwrap();
        assert_eq!(effective.role_identity, "Security review");
        assert_eq!(effective.instructions, "Instance instructions");
        assert_eq!(
            changed.active_recipe.as_ref().unwrap().nodes[0]
                .harness
                .role_identity,
            "Review"
        );

        let second = repository
            .activate_changes(
                &workflow_type_id,
                &[WorkflowElementRef {
                    kind: WorkflowElementKind::Node,
                    id: "start".to_string(),
                }],
            )
            .unwrap();
        assert_eq!(
            second.active_recipe.as_ref().unwrap().nodes[0]
                .harness
                .role_identity,
            "Security review"
        );
        let connection = repository.lock().unwrap();
        assert_eq!(
            load_recipe(&connection, &first_recipe_id).unwrap().nodes[0]
                .harness
                .role_identity,
            "Review"
        );
    }

    #[test]
    fn detach_and_save_as_role_are_atomic_one_level_operations() {
        let repository =
            SqliteWorkflowRepository::new(Connection::open_in_memory().unwrap()).unwrap();
        let role = repository
            .create_role(
                "Architecture",
                harness("Architecture Harness", "Architect", "Review"),
            )
            .unwrap();
        let workflow_type_id = repository
            .create_workflow_type("Review")
            .unwrap()
            .workflow_type
            .id;
        let mut start = node("start", true);
        start.harness = Some(WorkflowNodeHarness::Role {
            role_id: role.id,
            overrides: WorkflowHarnessOverrides {
                skills: Some(vec!["security".to_string()]),
                ..WorkflowHarnessOverrides::default()
            },
        });
        repository
            .save_node_draft(&workflow_type_id, start)
            .unwrap();

        let detached = repository
            .detach_node_role(&workflow_type_id, "start")
            .unwrap();
        let detached_node = detached.nodes[0].draft.as_ref().unwrap();
        let Some(WorkflowNodeHarness::Standalone { config }) = &detached_node.harness else {
            panic!("detached node should be standalone");
        };
        assert_eq!(config.role_identity, "Architect");
        assert_eq!(config.skills, vec!["security"]);

        let rebound = repository
            .save_node_as_role(&workflow_type_id, "start", "Security architect")
            .unwrap();
        let rebound_node = rebound.nodes[0].draft.as_ref().unwrap();
        let Some(WorkflowNodeHarness::Role { overrides, .. }) = &rebound_node.harness else {
            panic!("saved node should bind to the new Role");
        };
        assert_eq!(overrides, &WorkflowHarnessOverrides::default());
        assert_eq!(repository.list_roles().unwrap().len(), 2);
    }

    #[test]
    fn existing_workflow_node_tables_gain_materialized_harness_storage() {
        let connection = Connection::open_in_memory().unwrap();
        connection
            .execute_batch(
                "CREATE TABLE workflow_nodes(\
                 id TEXT PRIMARY KEY,workflow_type_id TEXT NOT NULL,draft_json TEXT,live_json TEXT,\
                 has_unpublished_changes INTEGER NOT NULL CHECK(has_unpublished_changes IN (0,1)));",
            )
            .unwrap();
        let repository = SqliteWorkflowRepository::new(connection).unwrap();
        let connection = repository.lock().unwrap();
        let has_column = connection
            .prepare("PRAGMA table_info(workflow_nodes)")
            .unwrap()
            .query_map([], |row| row.get::<_, String>(1))
            .unwrap()
            .any(|column| column.unwrap() == "live_effective_json");
        assert!(has_column);
    }

    #[test]
    fn role_catalog_and_one_level_binding_reopen_without_fabricating_legacy_bindings() {
        let directory = tempfile::tempdir().unwrap();
        let path = directory.path().join("roles.sqlite");
        let workflow_type_id;
        let role_id;
        {
            let repository = SqliteWorkflowRepository::open(&path).unwrap();
            let role = repository
                .create_role("Reviewer", harness("Review Harness", "Reviewer", "Review"))
                .unwrap();
            role_id = role.id.clone();
            workflow_type_id = repository
                .create_workflow_type("Review")
                .unwrap()
                .workflow_type
                .id;
            let mut bound = node("bound", true);
            bound.harness = Some(WorkflowNodeHarness::Role {
                role_id: role.id,
                overrides: WorkflowHarnessOverrides::default(),
            });
            repository
                .save_node_draft(&workflow_type_id, bound)
                .unwrap();
            repository
                .save_node_draft(&workflow_type_id, node("legacy", false))
                .unwrap();
        }

        let reopened = SqliteWorkflowRepository::open(&path).unwrap();
        assert_eq!(reopened.list_roles().unwrap()[0].id, role_id);
        let definition = reopened.load_workflow_type(&workflow_type_id).unwrap();
        let bound = definition
            .nodes
            .iter()
            .find(|node| node.id == "bound")
            .unwrap();
        assert!(matches!(
            bound.draft.as_ref().unwrap().harness,
            Some(WorkflowNodeHarness::Role { .. })
        ));
        assert_eq!(
            bound
                .draft_effective_harness
                .as_ref()
                .unwrap()
                .role_identity,
            "Reviewer"
        );
        let legacy = definition
            .nodes
            .iter()
            .find(|node| node.id == "legacy")
            .unwrap();
        assert_eq!(legacy.draft.as_ref().unwrap().harness, None);
        assert_eq!(
            legacy
                .draft_effective_harness
                .as_ref()
                .unwrap()
                .harness_name,
            "Default Harness"
        );
    }

    #[test]
    fn role_and_mcp_tagged_contracts_use_camel_case_transport_fields() {
        let binding: WorkflowNodeHarness = serde_json::from_value(serde_json::json!({
            "kind": "role",
            "roleId": "role-1",
            "overrides": {}
        }))
        .unwrap();
        assert!(matches!(
            binding,
            WorkflowNodeHarness::Role { role_id, overrides }
                if role_id == "role-1" && overrides == WorkflowHarnessOverrides::default()
        ));
        let access: super::super::domain::WorkflowMcpServerAccess =
            serde_json::from_value(serde_json::json!({
                "kind": "selected_tools",
                "toolNames": ["handoff"]
            }))
            .unwrap();
        assert!(matches!(
            access,
            super::super::domain::WorkflowMcpServerAccess::SelectedTools { tool_names }
                if tool_names == ["handoff"]
        ));
    }
}
