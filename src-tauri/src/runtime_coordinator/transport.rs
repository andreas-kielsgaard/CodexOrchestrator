use super::{domain::*, RuntimeCoordinator};
use orchid_coordinator::protocol::*;
use std::sync::Arc;
use tauri::State;
pub(crate) struct RuntimeCoordinatorState(pub Arc<RuntimeCoordinator>);
#[tauri::command]
pub(crate) async fn runtime_coordinator_overview(
    state: State<'_, RuntimeCoordinatorState>,
) -> Result<Overview, String> {
    state.0.overview().await
}
#[tauri::command]
pub(crate) async fn runtime_coordinator_settings(
    state: State<'_, RuntimeCoordinatorState>,
    input: SettingsInput,
) -> Result<Overview, String> {
    state.0.settings(input).await
}
#[tauri::command]
pub(crate) async fn runtime_coordinator_docker_settings(
    state: State<'_, RuntimeCoordinatorState>,
) -> Result<Overview, String> {
    state.0.docker_settings().await
}
#[tauri::command]
pub(crate) async fn runtime_coordinator_connect(
    state: State<'_, RuntimeCoordinatorState>,
    token: String,
) -> Result<Discovery, String> {
    state.0.connect(token).await
}
#[tauri::command]
pub(crate) async fn runtime_coordinator_register(
    state: State<'_, RuntimeCoordinatorState>,
    input: RegisterInput,
) -> Result<Device, String> {
    state.0.register(input).await
}
#[tauri::command]
pub(crate) async fn runtime_coordinator_refresh(
    state: State<'_, RuntimeCoordinatorState>,
    id: String,
) -> Result<Device, String> {
    state.0.refresh(id).await
}
#[tauri::command]
pub(crate) async fn runtime_coordinator_rename(
    state: State<'_, RuntimeCoordinatorState>,
    id: String,
    name: String,
) -> Result<Device, String> {
    state.0.rename(id, name).await
}
#[tauri::command]
pub(crate) async fn runtime_coordinator_power(
    state: State<'_, RuntimeCoordinatorState>,
    id: String,
    input: PowerRequest,
) -> Result<Operation, String> {
    state.0.power(id, input).await
}
