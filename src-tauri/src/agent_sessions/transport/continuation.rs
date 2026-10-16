use super::*;
use crate::agent_sessions::{client::HostCommand, domain::AgentSessionId};
#[tauri::command]
pub(crate) async fn agent_session_continuation(
    state: State<'_, AgentSessionTauriState>,
    session_id: AgentSessionId,
) -> Result<serde_json::Value, String> {
    let client = state.commands()?;
    tauri::async_runtime::spawn_blocking(move || client.continuation(&session_id))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub(crate) async fn fork_agent_session(
    state: State<'_, AgentSessionTauriState>,
    source_session_id: AgentSessionId,
    fork_id: AgentSessionId,
    target: crate::execution_targets::domain::SessionExecutionTarget,
) -> Result<serde_json::Value, String> {
    let client = state.commands()?;
    let queries = state.queries.clone();
    tauri::async_runtime::spawn_blocking(move||{
        let source=queries.load_session(&source_session_id).map_err(|e|e.to_string())?;
        use crate::agent_sessions::ports::AgentSessionRepository;
        let prefix=match client.replicas.get(source_session_id.as_str())?{Some(replica)=>replica.session.initial_prompt_prefix,None=>client.repository.initial_prompt_prefix(&source_session_id).map_err(|e|e.to_string())?};
        let device=target.execution.device_id.clone();
        client.execute(&device,HostCommand{method:"fork".into(),input:serde_json::json!({"history":source,"forkId":fork_id,"target":target,"initialPromptPrefix":prefix})})
    }).await.map_err(|e|e.to_string())?
}
