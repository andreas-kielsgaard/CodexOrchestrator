use super::*;
use crate::{
    agent_sessions::{
        application::jev::{CreateJevApiSessionInput, JevApiInvocationView},
        domain::{AgentInvocationId, AgentSessionId},
    },
    runtime::providers::jev::contracts::JevEvaluation,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InvokeJevApiSessionInput {
    session_id: AgentSessionId,
    invocation_id: AgentInvocationId,
    evaluation: JevEvaluation,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct JevApiLaunchReceipt {
    session_id: AgentSessionId,
    invocation_id: AgentInvocationId,
    launch_accepted: bool,
}

#[tauri::command]
pub(crate) async fn create_jev_api_session(
    state: State<'_, AgentSessionTauriState>,
    input: CreateJevApiSessionInput,
) -> Result<AgentSessionDto, String> {
    let app = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.create_jev_api_session(input)
            .map(Into::into)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn invoke_jev_api_session(
    state: State<'_, AgentSessionTauriState>,
    input: InvokeJevApiSessionInput,
) -> Result<JevApiLaunchReceipt, String> {
    let app = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let receipt = app
            .invoke_jev_api_session(input.session_id, input.invocation_id, input.evaluation)
            .map_err(|e| e.to_string())?;
        Ok(JevApiLaunchReceipt {
            session_id: receipt.acknowledgement.session_id,
            invocation_id: receipt.acknowledgement.invocation_id,
            launch_accepted: receipt.launch_accepted,
        })
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn load_jev_api_invocation(
    state: State<'_, AgentSessionTauriState>,
    session_id: AgentSessionId,
    invocation_id: AgentInvocationId,
) -> Result<JevApiInvocationView, String> {
    let app = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.load_jev_api_invocation(&session_id, &invocation_id)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}
