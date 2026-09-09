//! Session collection and history queries.
use super::*;
#[tauri::command]
pub(crate) fn list_agent_sessions(
    state: State<'_, AgentSessionTauriState>,
    query: ListAgentSessionsQueryDto,
) -> Result<Vec<AgentSessionSummaryDto>, String> {
    state
        .application
        .list_sessions(query.into())
        .map(|summaries| summaries.into_iter().map(Into::into).collect())
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn load_agent_session(
    state: State<'_, AgentSessionTauriState>,
    query: LoadAgentSessionQueryDto,
) -> Result<AgentSessionDetailsDto, String> {
    state
        .application
        .load_session(&query.session_id)
        .map(AgentSessionDetailsDto::from_history)
        .map_err(|error| error.to_string())
}
