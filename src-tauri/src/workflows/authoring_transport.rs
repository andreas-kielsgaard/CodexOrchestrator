use super::{
    authoring::{WorkflowRecipeDraft, WorkflowRecipeState, WorkflowRecipeSummary},
    authoring_service::WorkflowAuthoringService,
};
use crate::session_events::SessionEventDefinition;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

pub(crate) struct WorkflowAuthoringTauriState {
    service: Arc<WorkflowAuthoringService>,
}

impl WorkflowAuthoringTauriState {
    pub(crate) fn new(service: Arc<WorkflowAuthoringService>) -> Self {
        Self { service }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateWorkflowRecipeInput {
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LoadWorkflowRecipeInput {
    recipe_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveWorkflowRecipeDraftInput {
    draft: WorkflowRecipeDraft,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CopyWorkflowNodeConfigurationInput {
    recipe_id: String,
    expected_revision: u64,
    source_node_id: String,
    destination_node_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CompileWorkflowRecipeInstanceInput {
    recipe_id: String,
    instance_id: String,
}

#[tauri::command]
pub(crate) fn list_workflow_recipes(
    state: State<'_, WorkflowAuthoringTauriState>,
) -> Result<Vec<WorkflowRecipeSummary>, String> {
    state.service.list()
}

#[tauri::command]
pub(crate) fn load_workflow_recipe(
    state: State<'_, WorkflowAuthoringTauriState>,
    input: LoadWorkflowRecipeInput,
) -> Result<WorkflowRecipeState, String> {
    state.service.load(&input.recipe_id)
}

#[tauri::command]
pub(crate) fn create_workflow_recipe(
    state: State<'_, WorkflowAuthoringTauriState>,
    input: CreateWorkflowRecipeInput,
) -> Result<WorkflowRecipeState, String> {
    state.service.create(input.name)
}

#[tauri::command]
pub(crate) fn save_workflow_recipe_draft(
    state: State<'_, WorkflowAuthoringTauriState>,
    input: SaveWorkflowRecipeDraftInput,
) -> Result<WorkflowRecipeState, String> {
    state.service.save_draft(input.draft)
}

#[tauri::command]
pub(crate) fn copy_workflow_node_configuration(
    state: State<'_, WorkflowAuthoringTauriState>,
    input: CopyWorkflowNodeConfigurationInput,
) -> Result<WorkflowRecipeState, String> {
    state.service.copy_node_configuration(
        &input.recipe_id,
        input.expected_revision,
        &input.source_node_id,
        &input.destination_node_id,
    )
}

#[tauri::command]
pub(crate) fn activate_workflow_recipe(
    state: State<'_, WorkflowAuthoringTauriState>,
    input: LoadWorkflowRecipeInput,
) -> Result<WorkflowRecipeState, String> {
    state.service.activate(&input.recipe_id)
}

#[tauri::command]
pub(crate) fn compile_workflow_recipe_instance(
    state: State<'_, WorkflowAuthoringTauriState>,
    input: CompileWorkflowRecipeInstanceInput,
) -> Result<Vec<SessionEventDefinition>, String> {
    state
        .service
        .compile_active_for_instance(&input.recipe_id, &input.instance_id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn transport_inputs_reject_unknown_fields() {
        assert!(
            serde_json::from_value::<CreateWorkflowRecipeInput>(serde_json::json!({
                "name": "Review",
                "legacyRoleId": "role-reviewer"
            }))
            .is_err()
        );
    }

    #[test]
    fn copy_input_names_the_independent_source_and_destination_nodes() {
        let input: CopyWorkflowNodeConfigurationInput = serde_json::from_value(serde_json::json!({
            "recipeId": "recipe-review",
            "expectedRevision": 4,
            "sourceNodeId": "planner",
            "destinationNodeId": "reviewer"
        }))
        .unwrap();

        assert_eq!(input.source_node_id, "planner");
        assert_eq!(input.destination_node_id, "reviewer");
    }
}
