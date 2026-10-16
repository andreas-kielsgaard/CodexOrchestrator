use super::AgentSessionTauriState;
use crate::agent_sessions::{
    application::{RespondToRuntimeRequestCommand, SessionInteraction, SteerAgentSessionCommand},
    domain::AgentSessionId,
};
use tauri::State;

#[tauri::command]
pub(crate) async fn edit_agent_session_steering(
    state: State<'_, AgentSessionTauriState>,
    input: crate::agent_sessions::application::steering_queue::EditSteeringCommand,
) -> Result<serde_json::Value, String> {
    let command=state.command("editSteering",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub(crate) async fn steer_agent_session(
    state: State<'_, AgentSessionTauriState>,
    input: SteerAgentSessionCommand,
) -> Result<serde_json::Value, String> {
    let command=state.command("steer",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub(crate) async fn respond_to_agent_runtime_request(
    state: State<'_, AgentSessionTauriState>,
    input: RespondToRuntimeRequestCommand,
) -> Result<serde_json::Value, String> {
    let command=state.command("respond",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub(crate) fn list_agent_session_interactions(
    state: State<'_, AgentSessionTauriState>,
    session_id: AgentSessionId,
) -> Result<Vec<SessionInteraction>, String> {
    state
        .application
        .session_interactions(&session_id)
        .map_err(|e| e.to_string())
}

#[tauri::command]
pub(crate) fn resolve_agent_session_working_directory(
    state: State<'_, AgentSessionTauriState>,
    session_id: AgentSessionId,
    directory: String,
) -> Result<(), String> {
    state
        .application
        .resolve_working_directory(&session_id, directory)
        .map_err(|e| e.to_string())
}
