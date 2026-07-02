use serde::{Deserialize, Serialize};

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct AppMetadata {
    app_name: &'static str,
    storage_mode: &'static str,
    codex_runtime: &'static str,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct CreateOpenTaskCommandInput {
    project_id: String,
    title: String,
    summary: String,
    execution_state: Option<String>,
    attention_state: Option<String>,
    priority: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct UpdateOpenTaskCommandInput {
    title: Option<String>,
    summary: Option<String>,
    execution_state: Option<String>,
    attention_state: Option<String>,
    priority: Option<String>,
}

const TASK_DASHBOARD_BACKEND_PENDING: &str =
    "Persisted open task dashboard commands require the pending Rust SQLite backend adapter.";

#[tauri::command]
fn app_metadata() -> AppMetadata {
    AppMetadata {
        app_name: "Codex Orchestrator",
        storage_mode: "local-first",
        codex_runtime: "adapter-pending",
    }
}

#[tauri::command]
fn load_open_task_dashboard() -> Result<(), String> {
    Err(TASK_DASHBOARD_BACKEND_PENDING.to_string())
}

#[tauri::command]
fn create_open_task(_input: CreateOpenTaskCommandInput) -> Result<(), String> {
    Err(TASK_DASHBOARD_BACKEND_PENDING.to_string())
}

#[tauri::command]
fn update_open_task(_task_id: String, _input: UpdateOpenTaskCommandInput) -> Result<(), String> {
    Err(TASK_DASHBOARD_BACKEND_PENDING.to_string())
}

#[tauri::command]
fn archive_open_task(_task_id: String) -> Result<(), String> {
    Err(TASK_DASHBOARD_BACKEND_PENDING.to_string())
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .invoke_handler(tauri::generate_handler![
            app_metadata,
            load_open_task_dashboard,
            create_open_task,
            update_open_task,
            archive_open_task
        ])
        .run(tauri::generate_context!())
        .expect("error while running Codex Orchestrator");
}
