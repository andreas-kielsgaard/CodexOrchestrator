use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowHarnessConfig {
    pub(crate) harness_name: String,
    pub(crate) role_identity: String,
    pub(crate) instructions: String,
    #[serde(default)]
    pub(crate) skills: Vec<String>,
    #[serde(default)]
    pub(crate) mcp_servers: Vec<WorkflowMcpServerExposure>,
    #[serde(default)]
    pub(crate) hooks: Vec<String>,
    #[serde(default)]
    pub(crate) runtime: WorkflowHarnessRuntimeSettings,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowMcpServerExposure {
    pub(crate) server_name: String,
    pub(crate) access: WorkflowMcpServerAccess,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum WorkflowMcpServerAccess {
    EntireServer,
    SelectedTools {
        #[serde(default)]
        tool_names: Vec<String>,
    },
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowHarnessRuntimeSettings {
    pub(crate) provider: String,
    pub(crate) model: String,
    pub(crate) reasoning_effort: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(default, rename_all = "camelCase")]
pub(crate) struct WorkflowHarnessOverrides {
    pub(crate) harness_name: Option<String>,
    pub(crate) role_identity: Option<String>,
    pub(crate) instructions: Option<String>,
    pub(crate) skills: Option<Vec<String>>,
    pub(crate) mcp_servers: Option<Vec<WorkflowMcpServerExposure>>,
    pub(crate) hooks: Option<Vec<String>>,
    pub(crate) runtime: Option<WorkflowHarnessRuntimeSettings>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum WorkflowNodeHarness {
    Role {
        role_id: String,
        #[serde(default)]
        overrides: WorkflowHarnessOverrides,
    },
    Standalone {
        config: WorkflowHarnessConfig,
    },
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowRole {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) harness: WorkflowHarnessConfig,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

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
    #[serde(default)]
    pub(crate) harness: Option<WorkflowNodeHarness>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowConnectionConfig {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) sender_node_id: String,
    pub(crate) receiver_node_id: Option<String>,
    #[serde(default)]
    pub(crate) mechanism: Option<WorkflowConnectionMechanism>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum WorkflowConnectionMechanism {
    TurnFinishedExpectedFile {
        file_selector: WorkflowExpectedFileSelector,
        description_text: String,
        prompt_text: String,
        match_selection: WorkflowMatchSelection,
        initial_check: WorkflowInitialCheck,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum WorkflowExpectedFileSelector {
    FolderFilenamePattern {
        folder: String,
        filename_pattern: String,
    },
    FolderOutputRegex {
        folder: String,
        output_regex: String,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowMatchSelection {
    Newest,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowInitialCheck {
    OnceImmediately,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowNodeElement {
    pub(crate) id: String,
    pub(crate) draft: Option<WorkflowNodeConfig>,
    pub(crate) live: Option<WorkflowNodeConfig>,
    pub(crate) has_unpublished_changes: bool,
    pub(crate) draft_effective_harness: Option<WorkflowHarnessConfig>,
    pub(crate) live_effective_harness: Option<WorkflowHarnessConfig>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct EffectiveWorkflowNodeConfig {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) position_x: f64,
    pub(crate) position_y: f64,
    pub(crate) is_starting_point: bool,
    pub(crate) harness: WorkflowHarnessConfig,
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
    pub(crate) nodes: Vec<EffectiveWorkflowNodeConfig>,
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
    pub(crate) roles: Vec<WorkflowRole>,
}
