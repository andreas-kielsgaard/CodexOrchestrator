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
    sessions: State<'_, super::AgentSessionTauriState>,
    input: LoadQuickFeaturesInput,
    skill_id: String,
) -> Result<SkillDetails, String> {
    let commands=sessions.commands()?;
    let skills=sessions.application.clone();
    let application = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let catalogue = if is_remote(&input,&commands)? {skills.synchronized_skill_choices()} else {application.load_quick_features_for_configuration(
            input.session_id.as_ref(),
            input.working_directory.as_deref(),
            input.folder_target.as_ref(),
            input.execution_target.as_ref(),
            input.configuration.as_ref(),
        )?};
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
    sessions: State<'_, super::AgentSessionTauriState>,
    input: LoadQuickFeaturesInput,
) -> Result<RuntimeQuickFeatures, String> {
    let commands=sessions.commands()?;
    let skills=sessions.application.clone();
    let application = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if is_remote(&input,&commands)? {return Ok(skills.synchronized_skill_choices());}
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

fn is_remote(input:&LoadQuickFeaturesInput,commands:&crate::agent_sessions::client::AgentSessionCommandClient)->Result<bool,String>{
    use crate::agent_sessions::ports::AgentSessionRepository;
    if input.execution_target.as_ref().is_some_and(|t|t.execution.is_remote()){return Ok(true);}
    if let Some(id)=&input.session_id{return Ok(commands.repository.get_session(id).map_err(|e|e.to_string())?.is_none()||!commands.repository.owns_execution(id).map_err(|e|e.to_string())?);}
    Ok(false)
}
