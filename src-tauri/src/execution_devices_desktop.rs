use super::*;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;
pub(crate) struct ExecutionDeviceTauriState(pub(crate) Arc<ExecutionDeviceService>);

#[tauri::command]
pub(crate) fn list_execution_devices(
    state: State<'_, ExecutionDeviceTauriState>,
) -> Result<Vec<ExecutionDeviceDto>, String> {
    state.0.list()
}

#[tauri::command]
pub(crate) fn save_execution_device(
    state: State<'_, ExecutionDeviceTauriState>,
    input: SaveExecutionDeviceInput,
) -> Result<Vec<ExecutionDeviceDto>, String> {
    state.0.save(input)?;
    state.0.list()
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ExecutionDeviceIdInput {
    device_id: String,
}

#[tauri::command]
pub(crate) async fn start_execution_device(
    state: State<'_, ExecutionDeviceTauriState>,
    input: ExecutionDeviceIdInput,
) -> Result<(), String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || service.run_start(&input.device_id))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn stop_execution_device(
    state: State<'_, ExecutionDeviceTauriState>,
    input: ExecutionDeviceIdInput,
) -> Result<(), String> {
    let service = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || service.run_stop(&input.device_id))
        .await
        .map_err(|error| error.to_string())?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HoldExecutionDeviceInput {
    device_id: String,
    seconds: u64,
}

#[tauri::command]
pub(crate) fn hold_execution_device_awake(
    state: State<'_, ExecutionDeviceTauriState>,
    input: HoldExecutionDeviceInput,
) -> Result<Vec<ExecutionDeviceDto>, String> {
    state.0.hold_awake(&input.device_id, input.seconds)?;
    state.0.list()
}
