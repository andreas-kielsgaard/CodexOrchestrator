use super::application::{NavigationRepository, SessionNavigationService, StartSessionRequest};
use crate::{
    agent_sessions::{
        domain::AgentSessionId,
        organization::{SessionOrganization, SessionPlacement},
        transport::{
            selections::SendDirectUserAgentSessionMessageResultDto, AgentSessionSummaryDto,
        },
    },
    workflows::session_navigation::{NavigationInstance, WorkflowSessionOwner},
};
use serde::Serialize;
use std::sync::Arc;
use tauri::State;
pub(crate) struct SessionNavigationTauriState(pub(crate) Arc<SessionNavigationService>);
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionNavigationDto {
    summaries: Vec<AgentSessionSummaryDto>,
    repositories: Vec<NavigationRepository>,
    instances: Vec<NavigationInstance>,
    owners: Vec<WorkflowSessionOwner>,
    organization: Vec<SessionOrganization>,
}
#[tauri::command]
pub(crate) fn load_agent_session_navigation(
    state: State<'_, SessionNavigationTauriState>,
) -> Result<SessionNavigationDto, String> {
    let data = state.0.load()?;
    Ok(SessionNavigationDto {
        summaries: data.summaries.into_iter().map(Into::into).collect(),
        repositories: data.repositories,
        instances: data.instances,
        owners: data.owners,
        organization: data.organization,
    })
}
#[tauri::command]
pub(crate) fn move_agent_session(
    state: State<'_, SessionNavigationTauriState>,
    session_id: AgentSessionId,
    placement: SessionPlacement,
) -> Result<(), String> {
    state.0.move_session(session_id, placement)
}
#[tauri::command]
pub(crate) fn pin_agent_session(
    state: State<'_, SessionNavigationTauriState>,
    session_id: AgentSessionId,
    pinned: bool,
) -> Result<(), String> {
    state.0.pin_session(session_id, pinned)
}
#[tauri::command]
pub(crate) fn start_direct_user_agent_session(
    state: State<'_, SessionNavigationTauriState>,
    input: StartSessionRequest,
) -> Result<SendDirectUserAgentSessionMessageResultDto, String> {
    state.0.start_session(input).map(Into::into)
}
