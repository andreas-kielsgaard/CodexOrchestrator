use crate::{
    agent_sessions::domain::AgentSessionId, execution_configuration::RuntimeQuickFeatures,
};
use serde::Deserialize;
use tauri::State;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct LoadQuickFeaturesInput {
    #[serde(default)]
    execution_target: Option<crate::execution_targets::domain::SessionExecutionTarget>,
    session_id: Option<AgentSessionId>,
    working_directory: Option<String>,
    folder_target: Option<crate::agent_sessions::organization::SessionFolderTarget>,
    configuration: Option<orchid_engine::contracts::ProviderConfigurationRef>,
}

#[tauri::command]
pub(crate) async fn load_agent_session_quick_features(
    state: State<'_, crate::session_navigation::transport::SessionNavigationTauriState>,
    input: LoadQuickFeaturesInput,
) -> Result<RuntimeQuickFeatures, String> {
    if input
        .execution_target
        .as_ref()
        .is_some_and(|target| target.execution.is_remote())
    {
        return Err("Native quick-feature discovery is unavailable for remote sessions.".into());
    }
    let application = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application.load_quick_features_for_configuration(
            input.session_id.as_ref(),
            input.working_directory.as_deref(),
            input.folder_target.as_ref(),
            input.execution_target.as_ref(),
            input.configuration.as_ref(),
        )
    })
    .await
    .map_err(|e| e.to_string())?
}
