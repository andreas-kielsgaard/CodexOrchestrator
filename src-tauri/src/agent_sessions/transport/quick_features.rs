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
    folder_target: Option<crate::agent_sessions::organization::SessionFolderTarget>,
}

#[tauri::command]
pub(crate) async fn load_agent_session_quick_features(
    state: State<'_, crate::session_navigation::transport::SessionNavigationTauriState>,
    input: LoadQuickFeaturesInput,
) -> Result<RuntimeQuickFeatures, String> {
    let application = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application.load_quick_features(
            input.session_id.as_ref(),
            input.working_directory.as_deref(),
            input.folder_target.as_ref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
