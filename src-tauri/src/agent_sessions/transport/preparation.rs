use super::*;
use crate::agent_sessions::{
    application::preparation::PreparedMessageInput,
    domain::{AgentInvocationId, AgentSessionId},
    preparation::SessionPreparation,
};
use serde::{Deserialize, Serialize};
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PreparationQuery {
    session_id: AgentSessionId,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PreparationAction {
    invocation_id: AgentInvocationId,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PreparedMessageAcknowledgement {
    session_id: AgentSessionId,
    invocation_id: AgentInvocationId,
}
#[tauri::command]
pub(crate) async fn send_prepared_agent_session_message(
    state: State<'_, AgentSessionTauriState>,
    navigation: State<'_, crate::session_navigation::transport::SessionNavigationTauriState>,
    mut input: PreparedMessageInput,
) -> Result<PreparedMessageAcknowledgement, String> {
    let application = state.application.clone();
    let navigation = navigation.0.clone();
    let ack = tauri::async_runtime::spawn_blocking(move || {
        if input.session_id.is_none()
            && input.working_directory.is_none()
            && input.execution_selection.as_ref().is_none_or(|selection| {
                !selection.execution.is_remote()
                    && matches!(
                        selection.workspace,
                        crate::execution_targets::domain::SessionWorkspaceSelection::Auxiliary
                    )
            })
        {
            if let Some(folder) = &input.folder_target {
                input.working_directory = Some(navigation.working_directory_for_folder(folder)?);
            }
        }
        application
            .accept_prepared_message(input)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())??;
    Ok(PreparedMessageAcknowledgement {
        session_id: ack.session_id,
        invocation_id: ack.invocation_id,
    })
}
#[tauri::command]
pub(crate) fn load_agent_session_preparation(
    state: State<'_, AgentSessionTauriState>,
    input: PreparationQuery,
) -> Result<Option<SessionPreparation>, String> {
    state
        .application
        .load_preparation(&input.session_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn cancel_agent_session_preparation(
    state: State<'_, AgentSessionTauriState>,
    input: PreparationAction,
) -> Result<(), String> {
    state
        .application
        .cancel_preparation(&input.invocation_id)
        .map_err(|e| e.to_string())
}
#[tauri::command]
pub(crate) fn retry_agent_session_preparation(
    state: State<'_, AgentSessionTauriState>,
    input: PreparationAction,
) -> Result<(), String> {
    state
        .application
        .retry_preparation(&input.invocation_id)
        .map_err(|e| e.to_string())
}
