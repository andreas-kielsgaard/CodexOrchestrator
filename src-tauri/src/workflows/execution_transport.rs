use super::execution::WorkflowExecutionService;
use crate::session_events::SessionEventResult;
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
}

#[tauri::command]
pub(crate) fn dispatch_workflow_user_request(
    state: State<'_, WorkflowExecutionTauriState>,
    input: DispatchWorkflowUserRequestInput,
) -> Result<SessionEventResult, String> {
    state
        .service
        .dispatch_user_request(&input.recipe_id, &input.instance_id, input.text)
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
}
