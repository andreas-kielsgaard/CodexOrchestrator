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

#[derive(serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SkillDetails {
    name: String,
    description: String,
    content: String,
    source: String,
    path: String,
    created_at: Option<String>,
}

#[tauri::command]
pub(crate) async fn read_agent_session_skill(
    state: State<'_, crate::session_navigation::transport::SessionNavigationTauriState>,
    input: LoadQuickFeaturesInput,
    skill_id: String,
) -> Result<SkillDetails, String> {
    if input
        .execution_target
        .as_ref()
        .is_some_and(|target| target.execution.is_remote())
    {
        return Err("Skill content is unavailable for remote sessions".into());
    }
    let application = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let catalogue = application.load_quick_features_for_configuration(
            input.session_id.as_ref(),
            input.working_directory.as_deref(),
            input.folder_target.as_ref(),
            input.execution_target.as_ref(),
            input.configuration.as_ref(),
        )?;
        let skill = catalogue
            .skills
            .into_iter()
            .find(|skill| skill.id == skill_id)
            .ok_or("Skill is not in this session catalogue")?;
        let path = std::path::Path::new(&skill.id);
        let metadata = std::fs::metadata(path).map_err(|e| e.to_string())?;
        if metadata.len() > 1024 * 1024 {
            return Err("Skill content exceeds 1 MB".into());
        }
        let content = std::fs::read_to_string(path).map_err(|e| e.to_string())?;
        let created_at = metadata
            .created()
            .ok()
            .map(|date| chrono::DateTime::<chrono::Utc>::from(date).to_rfc3339());
        Ok(SkillDetails {
            name: skill.name,
            description: skill.description,
            source: skill.group_id,
            path: skill.id,
            content,
            created_at,
        })
    })
    .await
    .map_err(|e| e.to_string())?
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
