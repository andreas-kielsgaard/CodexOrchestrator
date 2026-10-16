use super::*;
use serde::Deserialize;
use std::{path::Path, sync::Arc};
use tauri::State;
pub(crate) struct ClaudeSetupTauriState(pub(crate) Arc<ClaudeSetups>);

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct AddClaudeSetupInput {
    folder: String,
    #[serde(default)]
    executable: Option<String>,
}

#[tauri::command]
pub(crate) async fn add_claude_setup(
    state: State<'_, ClaudeSetupTauriState>,
    input: AddClaudeSetupInput,
) -> Result<ClaudeSetup, String> {
    let setups = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        setups.add(Path::new(&input.folder), input.executable)
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn remove_claude_setup(
    state: State<'_, ClaudeSetupTauriState>,
    setup_id: String,
) -> Result<(), String> {
    let setups = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || setups.remove(&setup_id))
        .await
        .map_err(|error| error.to_string())?
}
