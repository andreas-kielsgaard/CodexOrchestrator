use crate::agent_sessions::{application::import::AgentSessionImportService, imports::*};
use std::sync::Arc;
use tauri::State;
#[tauri::command]
pub(crate) async fn preview_codex_import(
    state: State<'_, Arc<AgentSessionImportService>>,
    link: String,
) -> Result<ImportPreview, String> {
    let service = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || service.preview(&link))
        .await
        .map_err(|e| e.to_string())?
}
#[tauri::command]
pub(crate) async fn import_codex_conversation(
    state: State<'_, Arc<AgentSessionImportService>>,
    input: ImportCommand,
) -> Result<String, String> {
    let service = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.import(input).map(|id| id.as_str().to_owned())
    })
    .await
    .map_err(|e| e.to_string())?
}
