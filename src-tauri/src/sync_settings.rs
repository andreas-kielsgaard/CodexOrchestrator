use crate::device_connection;
use orchid_core::synchronization::{repository::ReplicaStore, RetentionMode, ServerConnection};
use std::path::PathBuf;
use tauri::State;
pub(crate) struct SyncSettingsState {
    pub root: PathBuf,
    pub replicas: ReplicaStore,
}
#[tauri::command]
pub(crate) fn load_device_sync_settings(
    state: State<'_, SyncSettingsState>,
) -> Result<serde_json::Value, String> {
    let connection = device_connection::load(&state.root)?;
    let (known, complete) = state.replicas.completeness()?;
    Ok(
        serde_json::json!({"deviceId":state.replicas.device_id()?,"serverUrl":connection.as_ref().map(|c|c.url.clone()),"tokenConfigured":connection.is_some(),"sessions":state.replicas.retention("sessions")?,"knownSessions":known,"completeSessions":complete}),
    )
}
#[tauri::command]
pub(crate) fn save_device_sync_settings(
    state: State<'_, SyncSettingsState>,
    url: Option<String>,
    token: Option<String>,
    sessions: RetentionMode,
) -> Result<(), String> {
    let path = state.root.join("server-connection.json");
    if let Some(url) = url.filter(|v| !v.trim().is_empty()) {
        let parsed = reqwest::Url::parse(url.trim()).map_err(|e| e.to_string())?;
        if !matches!(parsed.scheme(), "http" | "https")
            || parsed.host_str().is_none()
            || !parsed.username().is_empty()
            || parsed.password().is_some()
        {
            return Err("Enter an HTTP(S) server address without embedded credentials".into());
        }
        let token = token
            .filter(|t| !t.trim().is_empty())
            .or_else(|| {
                device_connection::load(&state.root)
                    .ok()
                    .flatten()
                    .map(|c| c.token)
            })
            .ok_or("Enter the service token")?;
        let connection = ServerConnection {
            url: url.trim().trim_end_matches('/').into(),
            token,
        };
        let temporary = state.root.join("server-connection.json.tmp");
        std::fs::write(
            &temporary,
            serde_json::to_vec_pretty(&connection).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?;
        std::fs::rename(temporary, path).map_err(|e| e.to_string())?;
    } else if path.exists() {
        std::fs::remove_file(path).map_err(|e| e.to_string())?;
    }
    state.replicas.set_retention("sessions", sessions)
}
#[tauri::command]
pub(crate) fn evict_cached_session_copies(
    state: State<'_, SyncSettingsState>,
) -> Result<(), String> {
    state.replicas.evict_cached_sessions()
}
