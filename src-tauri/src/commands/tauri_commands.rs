use super::*;

#[tauri::command]
pub(crate) fn app_metadata() -> AppMetadata {
    AppMetadata {
        app_name: "Codex Orchestrator",
        storage_mode: "local-first",
        codex_runtime: "tauri-codex-exec",
    }
}

#[tauri::command]
pub(crate) fn load_open_task_dashboard(app: AppHandle) -> Result<TaskDashboardSnapshot, String> {
    with_app_database(&app, |conn| load_dashboard_snapshot(conn))
}

#[tauri::command]
pub(crate) fn register_task_worktree(
    app: AppHandle,
    input: RegisterTaskWorktreeCommandInput,
) -> Result<TaskDashboardSnapshot, String> {
    with_app_database(&app, |conn| {
        register_task_worktree_anchor(conn, input)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
pub(crate) fn register_task_repo(
    app: AppHandle,
    input: RegisterTaskRepoCommandInput,
) -> Result<TaskDashboardSnapshot, String> {
    with_app_database(&app, |conn| {
        register_task_repo_anchor(conn, input)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
pub(crate) fn discover_task_repos(
    input: DiscoverTaskReposCommandInput,
) -> Result<Vec<DiscoveredTaskRepo>, String> {
    discover_git_repos(input)
}

#[tauri::command]
pub(crate) fn create_open_task(
    app: AppHandle,
    input: CreateOpenTaskCommandInput,
) -> Result<TaskDashboardSnapshot, String> {
    with_app_database(&app, |conn| {
        create_task(conn, input)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
pub(crate) fn update_open_task(
    app: AppHandle,
    task_id: String,
    input: UpdateOpenTaskCommandInput,
) -> Result<TaskDashboardSnapshot, String> {
    with_app_database(&app, |conn| {
        update_task(conn, &task_id, input)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
pub(crate) fn archive_open_task(
    app: AppHandle,
    task_id: String,
) -> Result<TaskDashboardSnapshot, String> {
    with_app_database(&app, |conn| {
        archive_task(conn, &task_id)?;
        load_dashboard_snapshot(conn)
    })
}

#[tauri::command]
pub(crate) fn load_task_run_detail(
    app: AppHandle,
    task_id: String,
) -> Result<TaskRunDetailSnapshot, String> {
    with_app_database(&app, |conn| load_task_run_detail_snapshot(conn, &task_id))
}

#[tauri::command]
pub(crate) fn start_codex_task_run(
    app: AppHandle,
    input: StartCodexTaskRunCommandInput,
) -> Result<StartCodexTaskRunCommandResult, String> {
    with_app_database(&app, |conn| {
        start_codex_task_run_with_runners(
            conn,
            input,
            &SystemCodexCommandRunner,
            &SystemGitDiffRunner,
            &SystemValidationCommandRunner,
        )
    })
}

#[tauri::command]
pub(crate) fn check_and_reopen_rust_backend(
    app: AppHandle,
) -> Result<BackendMaintenanceResult, String> {
    check_and_reopen_backend(app)
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(icon) = app.default_window_icon().cloned() {
                let tray_menu = tauri::menu::MenuBuilder::new(app)
                    .text("show", "Show Codex Orchestrator")
                    .separator()
                    .text("quit", "Quit")
                    .build()?;

                tauri::tray::TrayIconBuilder::with_id("codex-orchestrator-backend")
                    .icon(icon)
                    .menu(&tray_menu)
                    .tooltip("Codex Orchestrator backend is running")
                    .show_menu_on_left_click(false)
                    .on_menu_event(|app, event| match event.id().as_ref() {
                        "show" => show_main_window(app),
                        "quit" => app.exit(0),
                        _ => {}
                    })
                    .on_tray_icon_event(|tray, event| match event {
                        tauri::tray::TrayIconEvent::Click { .. }
                        | tauri::tray::TrayIconEvent::DoubleClick { .. } => {
                            show_main_window(tray.app_handle());
                        }
                        _ => {}
                    })
                    .build(app)?;
            }

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            app_metadata,
            load_open_task_dashboard,
            register_task_worktree,
            register_task_repo,
            discover_task_repos,
            create_open_task,
            update_open_task,
            archive_open_task,
            load_task_run_detail,
            start_codex_task_run,
            check_and_reopen_rust_backend
        ])
        .on_window_event(|window, event| {
            if window.label() == "main" {
                if let tauri::WindowEvent::CloseRequested { api, .. } = event {
                    api.prevent_close();
                    let _ = window.hide();
                }
            }
        })
        .run(tauri::generate_context!())
        .expect("error while running Codex Orchestrator");
}

fn show_main_window(app: &AppHandle) {
    if let Some(window) = app.get_webview_window("main") {
        let _ = window.show();
        let _ = window.set_focus();
    }
}
