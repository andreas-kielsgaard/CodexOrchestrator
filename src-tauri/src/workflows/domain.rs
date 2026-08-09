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

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowSessionActivity {
    Active,
    Idle,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowInstanceSession {
    pub(crate) node_id: String,
    pub(crate) session_id: String,
    pub(crate) title: String,
    pub(crate) activity: WorkflowSessionActivity,
    pub(crate) latest_turn_summary: Option<String>,
    pub(crate) associated_at: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowLaunchStatus {
    Requested,
    Associated,
    LaunchRequested,
    LaunchAccepted,
    Failed,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowActivation {
    pub(crate) id: String,
    pub(crate) source_kind: String,
    pub(crate) target_node_id: String,
    pub(crate) target_session_id: String,
    pub(crate) target_invocation_id: String,
    pub(crate) delivery_kind: String,
    pub(crate) session_mode: String,
    pub(crate) context_inheritance: String,
    pub(crate) compression: String,
    pub(crate) status: WorkflowLaunchStatus,
    pub(crate) requested_at: String,
    pub(crate) associated_at: Option<String>,
    pub(crate) launch_requested_at: Option<String>,
    pub(crate) launch_accepted_at: Option<String>,
    pub(crate) failed_at: Option<String>,
    pub(crate) failure_stage: Option<String>,
    pub(crate) failure_reason: Option<String>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowInstanceSummary {
    pub(crate) id: String,
    pub(crate) workflow_type_id: String,
    pub(crate) workflow_type_name: String,
    pub(crate) recipe_id: String,
    pub(crate) name: String,
    pub(crate) session_count: u32,
    pub(crate) active_session_count: u32,
    pub(crate) idle_session_count: u32,
    pub(crate) launch_status: WorkflowLaunchStatus,
    pub(crate) created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowInstance {
    pub(crate) summary: WorkflowInstanceSummary,
    pub(crate) starting_prompt: String,
    pub(crate) working_directory: String,
    pub(crate) recipe: EffectiveRecipe,
    pub(crate) sessions: Vec<WorkflowInstanceSession>,
    pub(crate) launch_activation: WorkflowActivation,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkflowInstanceRecord {
    pub(crate) id: String,
    pub(crate) workflow_type_id: String,
    pub(crate) workflow_type_name: String,
    pub(crate) recipe: EffectiveRecipe,
    pub(crate) name: String,
    pub(crate) starting_prompt: String,
    pub(crate) working_directory: String,
    pub(crate) created_at: String,
    pub(crate) session_associations: Vec<WorkflowSessionAssociationRecord>,
    pub(crate) launch_activation: WorkflowActivation,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkflowSessionAssociationRecord {
    pub(crate) node_id: String,
    pub(crate) session_id: String,
    pub(crate) associated_at: String,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkflowCompletedTurnTrigger {
    pub(crate) workflow_instance_id: String,
    pub(crate) working_directory: String,
    pub(crate) sender_node_id: String,
    pub(crate) recipe: EffectiveRecipe,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkflowConnectionActivationPreparation {
    pub(crate) id: String,
    pub(crate) workflow_instance_id: String,
    pub(crate) recipe_id: String,
    pub(crate) connection_id: String,
    pub(crate) sender_node_id: String,
    pub(crate) receiver_node_id: String,
    pub(crate) source_session_id: String,
    pub(crate) source_invocation_id: String,
    pub(crate) requested_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorkflowConnectionActivationRecord {
    pub(crate) id: String,
    pub(crate) workflow_instance_id: String,
    pub(crate) recipe_id: String,
    pub(crate) connection_id: String,
    pub(crate) sender_node_id: String,
    pub(crate) receiver_node_id: String,
    pub(crate) source_session_id: String,
    pub(crate) source_invocation_id: String,
    pub(crate) target_session_id: Option<String>,
    pub(crate) target_invocation_id: Option<String>,
    pub(crate) delivery_kind: String,
    pub(crate) session_mode: Option<String>,
    pub(crate) context_inheritance: String,
    pub(crate) compression: String,
    pub(crate) resolved_file_path: Option<String>,
    pub(crate) requested_at: String,
    pub(crate) resolved_at: Option<String>,
    pub(crate) associated_at: Option<String>,
    pub(crate) launch_requested_at: Option<String>,
    pub(crate) launch_accepted_at: Option<String>,
    pub(crate) failed_at: Option<String>,
    pub(crate) failure_stage: Option<String>,
    pub(crate) failure_reason: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkflowLaunchPreparation {
    pub(crate) instance_id: String,
    pub(crate) workflow_type_id: String,
    pub(crate) recipe_id: String,
    pub(crate) name: String,
    pub(crate) starting_prompt: String,
    pub(crate) working_directory: String,
    pub(crate) activation_id: String,
    pub(crate) target_node_id: String,
    pub(crate) target_session_id: String,
    pub(crate) target_invocation_id: String,
    pub(crate) requested_at: String,
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
