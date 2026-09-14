use crate::{
    agent_sessions::domain::AgentSessionId, execution_configuration::RuntimeQuickFeatures,
};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LoadQuickFeaturesInput {
    session_id: Option<AgentSessionId>,
    working_directory: Option<String>,
}

#[tauri::command]
pub(crate) async fn load_agent_session_quick_features(
    state: State<'_, super::AgentSessionTauriState>,
    input: LoadQuickFeaturesInput,
) -> Result<RuntimeQuickFeatures, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application.load_quick_features(
            input.session_id.as_ref(),
            input.working_directory.as_deref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
