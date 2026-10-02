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
#[tauri::command]
pub(crate) async fn preview_session_git_action(
    state: State<'_, SessionNavigationTauriState>,
    input: super::git_actions::GitActionInput,
) -> Result<super::git_actions::GitActionPreview, String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || service.preview_git_action(&input))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub(crate) async fn execute_session_git_action(
    app: tauri::AppHandle,
    state: State<'_, SessionNavigationTauriState>,
    input: super::git_actions::GitActionInput,
    expected: super::git_actions::GitActionPreview,
) -> Result<String, String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = service.execute_git_action(&input, &expected)?;
        let refresh = service.clone();
        let refreshed_app = app.clone();
        tauri::async_runtime::spawn_blocking(move || {
            refresh
                .branch_inventory
                .refresh(&refresh.execution_targets, &refresh.catalog);
            let _ = refreshed_app.emit("agent-session-navigation-changed", ());
        });
        let _ = app.emit("agent-session-navigation-changed", ());
        Ok(result)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub(crate) async fn inspect_session_worktree(
    state: State<'_, SessionNavigationTauriState>,
    repository_id: String,
    branch_ref: String,
    device_id: String,
    path: String,
) -> Result<orchid_engine::protocol::WorktreeInspection, String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.inspect_navigation_worktree(&repository_id, &branch_ref, &device_id, &path)
    })
    .await
    .map_err(|e| e.to_string())?
}
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
