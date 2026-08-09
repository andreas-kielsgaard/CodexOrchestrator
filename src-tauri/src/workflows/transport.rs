use super::{
    application::WorkflowApplication,
    domain::{
        WorkflowConnectionConfig, WorkflowDefinition, WorkflowElementRef, WorkflowHarnessConfig,
        WorkflowInstance, WorkflowInstanceSummary, WorkflowMcpComponent, WorkflowNativeQuery,
        WorkflowNodeConfig, WorkflowRole, WorkflowTypeSummary,
    },
};
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

pub(crate) struct WorkflowTauriState {
    application: Arc<WorkflowApplication>,
}

impl WorkflowTauriState {
    pub(crate) fn new(application: Arc<WorkflowApplication>) -> Self {
        Self { application }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateWorkflowTypeInput {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateWorkflowRoleInput {
    name: String,
    harness: WorkflowHarnessConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateWorkflowRoleInput {
    role_id: String,
    name: String,
    harness: WorkflowHarnessConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadWorkflowTypeQuery {
    workflow_type_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateWorkflowTypeInput {
    workflow_type_id: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveWorkflowNodeDraftInput {
    workflow_type_id: String,
    node: WorkflowNodeConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteWorkflowNodeDraftInput {
    workflow_type_id: String,
    node_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowNodeRoleInput {
    workflow_type_id: String,
    node_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveWorkflowNodeAsRoleInput {
    workflow_type_id: String,
    node_id: String,
    role_name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SaveWorkflowConnectionDraftInput {
    workflow_type_id: String,
    connection: WorkflowConnectionConfig,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DeleteWorkflowConnectionDraftInput {
    workflow_type_id: String,
    connection_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActivateWorkflowChangesInput {
    workflow_type_id: String,
    elements: Vec<WorkflowElementRef>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LaunchWorkflowInstanceInput {
    workflow_type_id: String,
    name: Option<String>,
    starting_prompt: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LoadWorkflowInstanceQuery {
    workflow_instance_id: String,
}

#[tauri::command]
pub(crate) fn list_workflow_types(
    state: State<'_, WorkflowTauriState>,
) -> Result<Vec<WorkflowTypeSummary>, String> {
    state.application.list_workflow_types()
}

#[tauri::command]
pub(crate) fn list_workflow_roles(
    state: State<'_, WorkflowTauriState>,
) -> Result<Vec<WorkflowRole>, String> {
    state.application.list_roles()
}

#[tauri::command]
pub(crate) fn list_workflow_mcp_components(
    state: State<'_, WorkflowTauriState>,
) -> Result<Vec<WorkflowMcpComponent>, String> {
    Ok(state.application.list_mcp_components())
}

#[tauri::command]
pub(crate) fn create_workflow_role(
    state: State<'_, WorkflowTauriState>,
    input: CreateWorkflowRoleInput,
) -> Result<WorkflowRole, String> {
    state.application.create_role(&input.name, input.harness)
}

#[tauri::command]
pub(crate) fn update_workflow_role(
    state: State<'_, WorkflowTauriState>,
    input: UpdateWorkflowRoleInput,
) -> Result<WorkflowRole, String> {
    state
        .application
        .update_role(&input.role_id, &input.name, input.harness)
}

#[tauri::command]
pub(crate) fn create_workflow_type(
    state: State<'_, WorkflowTauriState>,
    input: CreateWorkflowTypeInput,
) -> Result<WorkflowDefinition, String> {
    state.application.create_workflow_type(&input.name)
}

#[tauri::command]
pub(crate) fn load_workflow_type(
    state: State<'_, WorkflowTauriState>,
    query: LoadWorkflowTypeQuery,
) -> Result<WorkflowDefinition, String> {
    state
        .application
        .load_workflow_type(&query.workflow_type_id)
}

#[tauri::command]
pub(crate) fn update_workflow_type(
    state: State<'_, WorkflowTauriState>,
    input: UpdateWorkflowTypeInput,
) -> Result<WorkflowDefinition, String> {
    state
        .application
        .update_workflow_type(&input.workflow_type_id, &input.name)
}

#[tauri::command]
pub(crate) fn save_workflow_node_draft(
    state: State<'_, WorkflowTauriState>,
    input: SaveWorkflowNodeDraftInput,
) -> Result<WorkflowDefinition, String> {
    state
        .application
        .save_node_draft(&input.workflow_type_id, input.node)
}

#[tauri::command]
pub(crate) fn delete_workflow_node_draft(
    state: State<'_, WorkflowTauriState>,
    input: DeleteWorkflowNodeDraftInput,
) -> Result<WorkflowDefinition, String> {
    state
        .application
        .delete_node_draft(&input.workflow_type_id, &input.node_id)
}

#[tauri::command]
pub(crate) fn detach_workflow_node_role(
    state: State<'_, WorkflowTauriState>,
    input: WorkflowNodeRoleInput,
) -> Result<WorkflowDefinition, String> {
    state
        .application
        .detach_node_role(&input.workflow_type_id, &input.node_id)
}

#[tauri::command]
pub(crate) fn save_workflow_node_as_role(
    state: State<'_, WorkflowTauriState>,
    input: SaveWorkflowNodeAsRoleInput,
) -> Result<WorkflowDefinition, String> {
    state
        .application
        .save_node_as_role(&input.workflow_type_id, &input.node_id, &input.role_name)
}

#[tauri::command]
pub(crate) fn save_workflow_connection_draft(
    state: State<'_, WorkflowTauriState>,
    input: SaveWorkflowConnectionDraftInput,
) -> Result<WorkflowDefinition, String> {
    state
        .application
        .save_connection_draft(&input.workflow_type_id, input.connection)
}

#[tauri::command]
pub(crate) fn delete_workflow_connection_draft(
    state: State<'_, WorkflowTauriState>,
    input: DeleteWorkflowConnectionDraftInput,
) -> Result<WorkflowDefinition, String> {
    state
        .application
        .delete_connection_draft(&input.workflow_type_id, &input.connection_id)
}

#[tauri::command]
pub(crate) fn activate_workflow_changes(
    state: State<'_, WorkflowTauriState>,
    input: ActivateWorkflowChangesInput,
) -> Result<WorkflowDefinition, String> {
    state
        .application
        .activate_changes(&input.workflow_type_id, &input.elements)
}

#[tauri::command]
pub(crate) fn load_workflow_native_query(
    state: State<'_, WorkflowTauriState>,
) -> Result<WorkflowNativeQuery, String> {
    state.application.native_query()
}

#[tauri::command]
pub(crate) fn launch_workflow_instance(
    state: State<'_, WorkflowTauriState>,
    input: LaunchWorkflowInstanceInput,
) -> Result<WorkflowInstance, String> {
    state.application.launch_workflow_instance(
        &input.workflow_type_id,
        input.name.as_deref(),
        &input.starting_prompt,
    )
}

#[tauri::command]
pub(crate) fn list_workflow_instances(
    state: State<'_, WorkflowTauriState>,
) -> Result<Vec<WorkflowInstanceSummary>, String> {
    state.application.list_workflow_instances()
}

#[tauri::command]
pub(crate) fn load_workflow_instance(
    state: State<'_, WorkflowTauriState>,
    query: LoadWorkflowInstanceQuery,
) -> Result<WorkflowInstance, String> {
    state
        .application
        .load_workflow_instance(&query.workflow_instance_id)
}
