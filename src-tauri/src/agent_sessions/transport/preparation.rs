use super::*;
use crate::agent_sessions::{
    application::preparation::PreparedMessageInput,
    domain::{AgentInvocationId, AgentSessionId},
    preparation::SessionPreparation,
};
use serde::{Deserialize, Serialize};
#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PreparationQuery {
    session_id: AgentSessionId,
}
#[derive(Deserialize, serde::Serialize)]
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
    input: PreparedMessageInput,
) -> Result<serde_json::Value, String> {
    let command=state.command("send",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub(crate) async fn load_agent_session_preparation(
    state: State<'_, AgentSessionTauriState>,
    input: PreparationQuery,
) -> Result<serde_json::Value, String> {
    let command=state.command("preparation",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub(crate) async fn cancel_agent_session_preparation(
    state: State<'_, AgentSessionTauriState>,
    input: PreparationAction,
) -> Result<serde_json::Value, String> {
    let command=state.command("cancelPreparation",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}
#[tauri::command]
pub(crate) async fn retry_agent_session_preparation(
    state: State<'_, AgentSessionTauriState>,
    input: PreparationAction,
) -> Result<serde_json::Value, String> {
    let command=state.command("retryPreparation",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}
