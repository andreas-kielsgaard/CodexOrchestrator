use super::AgentSessionTauriState;
use crate::agent_sessions::{
    application::target_transition::RequestTargetTransitionInput,
    domain::AgentSessionId,
    target_transition::SessionTargetTransition,
};
use crate::execution_targets::domain::{SessionExecutionSelection, SessionExecutionTarget};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RequestTargetTransitionDto {
    session_id: AgentSessionId,
    source_target: SessionExecutionTarget,
    destination_selection: SessionExecutionSelection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct TargetTransitionQuery {
    session_id: AgentSessionId,
}

#[tauri::command]
pub(crate) fn request_agent_session_target_transition(
    state: State<'_, AgentSessionTauriState>,
    input: RequestTargetTransitionDto,
) -> Result<SessionTargetTransition, String> {
    state
        .application()
        .request_target_transition(RequestTargetTransitionInput {
            session_id: input.session_id,
            source_target: input.source_target,
            destination: input.destination_selection,
        })
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn load_agent_session_target_transition(
    state: State<'_, AgentSessionTauriState>,
    input: TargetTransitionQuery,
) -> Result<Option<SessionTargetTransition>, String> {
    state
        .application()
        .load_target_transition(&input.session_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn start_agent_session_target_transition(
    state: State<'_, AgentSessionTauriState>,
    input: TargetTransitionQuery,
) -> Result<SessionTargetTransition, String> {
    state
        .application()
        .start_target_transition(&input.session_id)
        .map_err(|error| error.to_string())
}
