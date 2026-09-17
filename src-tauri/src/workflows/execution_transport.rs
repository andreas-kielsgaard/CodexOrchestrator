use super::execution::WorkflowExecutionService;
use super::instances::WorkflowActionResult;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

pub(crate) struct WorkflowExecutionTauriState {
    service: Arc<WorkflowExecutionService>,
}

impl WorkflowExecutionTauriState {
    pub(crate) fn new(service: Arc<WorkflowExecutionService>) -> Self {
        Self { service }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DispatchWorkflowUserRequestInput {
    recipe_id: String,
    instance_id: String,
    text: String,
    node_id: Option<String>,
    #[serde(default = "empty_object")]
    data: serde_json::Value,
}

fn empty_object() -> serde_json::Value {
    serde_json::json!({})
}

#[tauri::command]
pub(crate) fn dispatch_workflow_user_request(
    state: State<'_, WorkflowExecutionTauriState>,
    input: DispatchWorkflowUserRequestInput,
) -> Result<WorkflowActionResult, String> {
    state.service.dispatch_node_user_request(
        &input.recipe_id,
        &input.instance_id,
        input.node_id.as_deref(),
        input.text,
        input.data,
    )
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateRecipeInstanceInput {
    recipe_id: String,
    expected_revision: u64,
    name: String,
    target: super::instance_domain::ResolvedRepoBranchWorktreeTarget,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LoadRecipeInstanceInput {
    instance_id: String,
}

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RecipeInstanceDetails {
    instance: super::instances::RecipeInstance,
    sessions: Vec<crate::session_events::SessionDirectoryEntry>,
    attempts: Vec<super::instances::WorkflowEventAttempt>,
}

#[tauri::command]
pub(crate) fn create_workflow_recipe_instance(
    state: State<'_, WorkflowExecutionTauriState>,
    input: CreateRecipeInstanceInput,
) -> Result<super::instances::RecipeInstance, String> {
    state.service.create_instance(
        &input.recipe_id,
        input.expected_revision,
        input.name,
        input.target,
    )
}

#[tauri::command]
pub(crate) fn list_workflow_recipe_instances(
    state: State<'_, WorkflowExecutionTauriState>,
) -> Result<Vec<super::instances::RecipeInstance>, String> {
    state.service.instances.list()
}

#[tauri::command]
pub(crate) fn load_workflow_recipe_instance(
    state: State<'_, WorkflowExecutionTauriState>,
    input: LoadRecipeInstanceInput,
) -> Result<RecipeInstanceDetails, String> {
    let instance = state.service.instances.load(&input.instance_id)?;
    Ok(RecipeInstanceDetails {
        sessions: state.service.instance_sessions(&instance)?,
        attempts: state.service.instances.attempts(&instance.id)?,
        instance,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn user_request_transport_does_not_accept_session_configuration() {
        assert!(
            serde_json::from_value::<DispatchWorkflowUserRequestInput>(serde_json::json!({
                "recipeId": "recipe-review",
                "instanceId": "instance-1",
                "text": "Review this",
                "model": "codex-a"
            }))
            .is_err()
        );
    }

    #[test]
    fn user_request_transport_accepts_structured_data() {
        let input = serde_json::from_value::<DispatchWorkflowUserRequestInput>(serde_json::json!({
            "recipeId": "recipe-review",
            "instanceId": "instance-1",
            "text": "Review this",
            "data": {"sourceUrl": "https://example.test"}
        }))
        .unwrap();
        assert_eq!(input.data, serde_json::json!({"sourceUrl": "https://example.test"}));
    }
}
