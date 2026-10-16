use super::{domain::*, ExecutionTargetService};
use crate::repository_catalog::device_locations::RepositoryDeviceLocation;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

pub(crate) struct ExecutionTargetTauriState(pub(crate) Arc<ExecutionTargetService>);
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchCommitQuery {
    repository_id: String,
    branch_ref: String,
    execution: ExecutionBinding,
}
#[tauri::command]
pub(crate) async fn resolve_worktree_branch_commit(
    state: State<'_, ExecutionTargetTauriState>,
    input: BranchCommitQuery,
) -> Result<serde_json::Value, String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let root = service.repository_root(&input.repository_id, &input.execution)?;
        let commit = service
            .endpoints
            .branch_commit(&input.execution, &root, &input.branch_ref)?;
        Ok(serde_json::json!({"commit":commit}))
    })
    .await
    .map_err(|e| e.to_string())?
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetQuery {
    repository_id: String,
    branch_ref: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RuntimeQuery {
    execution: ExecutionBinding,
    working_directory: Option<String>,
}

#[tauri::command]
pub(crate) async fn list_execution_target_devices(
    state: State<'_, ExecutionTargetTauriState>,
) -> Result<Vec<ConfiguredExecutionDevice>, String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || service.attached_devices())
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn list_execution_worktree_choices(
    state: State<'_, ExecutionTargetTauriState>,
    scope: WorktreeChoiceScope,
) -> Result<Vec<RepositoryWorktreeChoices>, String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || service.attached_worktree_choices(&scope))
        .await
        .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn list_session_execution_targets(
    state: State<'_, ExecutionTargetTauriState>,
    input: TargetQuery,
) -> Result<Vec<DeviceWorktreeTargets>, String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.attached_targets(&input.repository_id, &input.branch_ref)
    })
    .await
    .map_err(|e| e.to_string())?
}

#[tauri::command]
pub(crate) async fn load_execution_target_runtime(
    state: State<'_, ExecutionTargetTauriState>,
    input: RuntimeQuery,
) -> Result<ExecutionTargetRuntime, String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.describe_runtime(&input.execution, input.working_directory.as_deref())
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub(crate) fn list_repository_device_locations(
    state: State<'_, ExecutionTargetTauriState>,
) -> Result<Vec<RepositoryDeviceLocation>, String> {
    state.0.locations.list()
}
#[tauri::command]
pub(crate) async fn save_repository_device_location(
    state: State<'_, ExecutionTargetTauriState>,
    sessions: State<'_, crate::agent_sessions::transport::AgentSessionTauriState>,
    input: RepositoryDeviceLocation,
) -> Result<(), String> {
    let service = state.0.clone();
    let commands = sessions.commands()?;
    tauri::async_runtime::spawn_blocking(move||{
        if commands.remote.as_ref().and_then(|r|r.devices().ok()).is_some_and(|rows|rows.iter().any(|(d,_)|d.id==input.device_id)) {
            commands.execute(&input.device_id,crate::agent_sessions::client::HostCommand{method:"device.registerRepository".into(),input:serde_json::json!({"repositoryId":input.repository_id,"path":input.repository_root})})?;
        }
        service.locations.save(input)
    }).await.map_err(|e|e.to_string())?
}
