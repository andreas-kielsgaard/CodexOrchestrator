use serde::Serialize;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppMetadata {
    app_name: &'static str,
    storage_mode: &'static str,
    codex_runtime: &'static str,
}

#[tauri::command]
fn app_metadata() -> AppMetadata {
    AppMetadata {
        app_name: "Codex Orchestrator",
        storage_mode: "local-first",
        codex_runtime: "adapter-pending",
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![app_metadata])
        .run(tauri::generate_context!())
        .expect("error while running Codex Orchestrator");
}
