use super::{
    application::WorkflowApplication,
    domain::{
        WorkflowConnectionConfig, WorkflowDefinition, WorkflowElementRef, WorkflowNativeQuery,
        WorkflowNodeConfig, WorkflowTypeSummary,
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

#[tauri::command]
pub(crate) fn list_workflow_types(
    state: State<'_, WorkflowTauriState>,
) -> Result<Vec<WorkflowTypeSummary>, String> {
    state.application.list_workflow_types()
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
