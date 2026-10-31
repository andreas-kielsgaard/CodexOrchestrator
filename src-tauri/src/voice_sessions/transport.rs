use super::{domain::VoiceSettings, VoiceSessionApplication};
use serde_json::Value;
use std::sync::Arc;
pub(crate) struct VoiceSessionTauriState(pub(crate) Arc<VoiceSessionApplication>);
#[tauri::command]
pub(crate) fn voice_session_view(
    state: tauri::State<'_, VoiceSessionTauriState>,
) -> Result<Value, String> {
    state.0.view()
}
#[tauri::command]
pub(crate) fn configure_voice_session(
    state: tauri::State<'_, VoiceSessionTauriState>,
    settings: VoiceSettings,
) -> Result<Value, String> {
    state.0.configure(settings)?;
    state.0.view()
}
#[tauri::command]
pub(crate) async fn start_voice_session(
    state: tauri::State<'_, VoiceSessionTauriState>,
    fresh: bool,
    instance_id: Option<String>,
) -> Result<Value, String> {
    let app = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        if let Some(id) = instance_id {
            app.select_instance(&id)?;
        }
        app.start(fresh)
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub(crate) async fn stop_voice_session(
    state: tauri::State<'_, VoiceSessionTauriState>,
) -> Result<Value, String> {
    let app = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.stop()?;
        app.view()
    })
    .await
    .map_err(|e| e.to_string())?
}
#[tauri::command]
pub(crate) async fn send_voice_session_text(
    state: tauri::State<'_, VoiceSessionTauriState>,
    text: String,
) -> Result<Value, String> {
    let app = state.0.clone();
    tauri::async_runtime::spawn_blocking(move || app.input(uuid::Uuid::new_v4().to_string(), text))
        .await
        .map_err(|e| e.to_string())?
}
