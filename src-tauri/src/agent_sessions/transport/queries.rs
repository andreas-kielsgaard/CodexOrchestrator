//! Session collection and history queries.
use super::dto::AgentRuntimeEventDto;
use super::*;
#[tauri::command]
pub(crate) async fn load_agent_session_event_details(
    state: State<'_, crate::session_navigation::transport::SessionNavigationTauriState>,
    session_id: crate::agent_sessions::domain::AgentSessionId,
    event_id: String,
) -> Result<AgentRuntimeEventDto, String> {
    let application = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application.read_event_details(&session_id, &event_id)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub(crate) async fn load_agent_session_display(
    state: State<'_, AgentSessionTauriState>,
    query: LoadAgentSessionQueryDto,
) -> Result<AgentSessionDetailsDto, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .load_session_display(&query.session_id)
            .map(AgentSessionDetailsDto::from_history)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub(crate) async fn list_agent_sessions(
    state: State<'_, AgentSessionTauriState>,
    query: ListAgentSessionsQueryDto,
) -> Result<Vec<AgentSessionSummaryDto>, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .list_sessions(query.into())
            .map(|summaries| summaries.into_iter().map(Into::into).collect())
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn load_agent_session(
    state: State<'_, AgentSessionTauriState>,
    query: LoadAgentSessionQueryDto,
) -> Result<AgentSessionDetailsDto, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .load_session(&query.session_id)
            .map(AgentSessionDetailsDto::from_history)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
