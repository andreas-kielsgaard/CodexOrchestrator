use super::application::{NavigationRepository, SessionNavigationService, StartSessionRequest};
use super::order::{NavigationOrder, NavigationOrderScope};
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
use tauri::Emitter;
use tauri::State;
pub(crate) struct SessionNavigationTauriState(pub(crate) Arc<SessionNavigationService>);
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionNavigationDto {
    branches: Vec<super::branch_inventory::NavigationBranch>,
    session_branches: Vec<super::branch_inventory::SessionBranch>,
    summaries: Vec<AgentSessionSummaryDto>,
    repositories: Vec<NavigationRepository>,
    instances: Vec<NavigationInstance>,
    owners: Vec<WorkflowSessionOwner>,
    organization: Vec<SessionOrganization>,
    orders: Vec<NavigationOrder>,
}
#[tauri::command]
pub(crate) async fn load_agent_session_navigation(
    app: tauri::AppHandle,
    state: State<'_, SessionNavigationTauriState>,
) -> Result<SessionNavigationDto, String> {
    let application = state.0.clone();
    let discovery = application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if discovery
            .branch_inventory
            .refresh(&discovery.execution_targets, &discovery.catalog)
        {
            let _ = app.emit("agent-session-navigation-changed", ());
        }
    });
    tauri::async_runtime::spawn_blocking(move || {
        let data = application.load()?;
        Ok(SessionNavigationDto {
            branches: data.branches,
            session_branches: data.session_branches,
            summaries: data.summaries.into_iter().map(Into::into).collect(),
            repositories: data.repositories,
            instances: data.instances,
            owners: data.owners,
            organization: data.organization,
            orders: data.orders,
        })
    })
    .await
    .map_err(|error| error.to_string())?
}
#[tauri::command]
pub(crate) async fn move_agent_session(
    state: State<'_, SessionNavigationTauriState>,
    session_id: AgentSessionId,
    placement: SessionPlacement,
    ordered_ids: Option<Vec<String>>,
) -> Result<(), String> {
    let application = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application.move_session(session_id, placement, ordered_ids)
    })
    .await
    .map_err(|error| error.to_string())?
}
#[tauri::command]
pub(crate) async fn pin_agent_session(
    state: State<'_, SessionNavigationTauriState>,
    session_id: AgentSessionId,
    pinned: bool,
) -> Result<(), String> {
    let application = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || application.pin_session(session_id, pinned))
        .await
        .map_err(|error| error.to_string())?
}
#[tauri::command]
pub(crate) async fn reorder_session_navigation(
    state: State<'_, SessionNavigationTauriState>,
    scope: NavigationOrderScope,
    ordered_ids: Vec<String>,
) -> Result<(), String> {
    let application = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || application.reorder(scope, ordered_ids))
        .await
        .map_err(|error| error.to_string())?
}
#[tauri::command]
pub(crate) async fn start_direct_user_agent_session(
    state: State<'_, SessionNavigationTauriState>,
    input: StartSessionRequest,
) -> Result<SendDirectUserAgentSessionMessageResultDto, String> {
    let application = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || application.start_session(input).map(Into::into))
        .await
        .map_err(|error| error.to_string())?
}
