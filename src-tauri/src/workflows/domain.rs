use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowTypeSummary {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) active_recipe_id: Option<String>,
    pub(crate) edited_element_count: u32,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowNodeConfig {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) harness_name: String,
    pub(crate) role_name: Option<String>,
    pub(crate) position_x: f64,
    pub(crate) position_y: f64,
    pub(crate) is_starting_point: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowConnectionConfig {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) sender_node_id: String,
    pub(crate) receiver_node_id: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowNodeElement {
    pub(crate) id: String,
    pub(crate) draft: Option<WorkflowNodeConfig>,
    pub(crate) live: Option<WorkflowNodeConfig>,
    pub(crate) has_unpublished_changes: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowConnectionElement {
    pub(crate) id: String,
    pub(crate) draft: Option<WorkflowConnectionConfig>,
    pub(crate) live: Option<WorkflowConnectionConfig>,
    pub(crate) has_unpublished_changes: bool,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EffectiveRecipe {
    pub(crate) id: String,
    pub(crate) workflow_type_id: String,
    pub(crate) ordinal: u32,
    pub(crate) created_at: String,
    pub(crate) nodes: Vec<WorkflowNodeConfig>,
    pub(crate) connections: Vec<WorkflowConnectionConfig>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowDefinition {
    pub(crate) workflow_type: WorkflowTypeSummary,
    pub(crate) nodes: Vec<WorkflowNodeElement>,
    pub(crate) connections: Vec<WorkflowConnectionElement>,
    pub(crate) active_recipe: Option<EffectiveRecipe>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowElementKind {
    Node,
    Connection,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowElementRef {
    pub(crate) kind: WorkflowElementKind,
    pub(crate) id: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowNativeQuery {
    pub(crate) schema_version: &'static str,
    pub(crate) workflow_types: Vec<WorkflowDefinition>,
}
