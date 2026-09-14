mod execution_configuration;
mod session_notifications;
mod sessions;

use session_notifications::SessionNotificationFanout;

use std::{
    fs,
    sync::{Arc, Mutex},
};

use tauri::{Emitter, Manager};

fn worktree_review_root(app_data_dir: &std::path::Path) -> std::path::PathBuf {
    app_data_dir.join("worktree-review")
}

pub(crate) fn run() {
    let app = tauri::Builder::default()
        .plugin(tauri_plugin_single_instance::init(|app, _, _| {
            if let Some(window) = app.get_webview_window("main") {
                let _ = window.show();
                let _ = window.set_focus();
            }
        }))
        .plugin(tauri_plugin_deep_link::init())
        .setup(|app| {
            let app_data_dir = crate::runtime::instance::app_data_dir(|| {
                app.path()
                    .app_data_dir()
                    .map_err(|error| format!("Unable to resolve app data directory: {error}"))
            })?;
            fs::create_dir_all(&app_data_dir)
                .map_err(|error| format!("Unable to create app data directory: {error}"))?;
            let database_path = crate::storage::active_database_path(&app_data_dir);
            let database = crate::product_database::open(&database_path)?;
            let repository_catalog = Arc::new(crate::repository_catalog::RepositoryCatalog::new(
                database.clone(),
            ));
            app.manage(
                crate::repository_catalog::transport::RepositoryCatalogTauriState::new(
                    repository_catalog.clone(),
                ),
            );
            let native_profiles = Arc::new(crate::native_profiles::NativeProfileService::new(
                database.clone(),
                app_data_dir.clone(),
            ));
            let repository = Arc::new(
                crate::agent_sessions::repository::SqliteAgentSessionRepository::from_database(
                    database.clone(),
                ),
            );
            let orchestration_repository = Arc::new(
                crate::orchestration::repository::SqliteOrchestrationRepository::from_database_with_harness_revision_repository(
                    database.clone(),
                    crate::storage::harness_revision_repository_path(&app_data_dir),
                )
                .map_err(|error| error.to_string())?,
            );
            let product_decisions = Arc::new(
                crate::product_decisions::ProductDecisionRepository::new(database.clone()),
            );
            let managed_mcp_upstreams = Arc::new(
                crate::harness_engine::ManagedMcpUpstreamRegistry::default(),
            );
            let harness_catalog =
                crate::harness_engine::catalog_service::HarnessCatalogService::from_database(
                    database.clone(),
                );
            let identities =
                crate::identities::service::IdentityService::from_database(database.clone());
            let harness_engine = crate::harness_engine::HarnessEngineService::open_system(
                database.clone(),
                managed_mcp_upstreams.clone(),
            )?;
            // This product-native seam resolves only durable application-owned attempt authority.
            let execution_support = crate::orchestration::execution_support::ProductExecutionSupportState::new(
                database.clone(),
                app_data_dir.join("execution-workspaces"),
                orchestration_repository.clone(),
            )
            .map_err(|error| error.to_string())?;
            let execution_support_service = execution_support.service();
            app.manage(execution_support);
            let registry =
                Arc::new(crate::orchestration::application::ManagedPlanBuilderRegistry::default());
            let transition_notification = Arc::new(Mutex::new(None));
            let sprint_transition_notification = Arc::new(Mutex::new(None));
            let workflow_execution_notification = Arc::new(Mutex::new(None));
            let notifier: Arc<dyn crate::agent_sessions::application::AgentSessionNotifier> =
                Arc::new(SessionNotificationFanout {
                    inner: Arc::new(
                        crate::agent_sessions::transport::TauriAgentSessionNotifier::new(
                            app.handle().clone(),
                        ),
                    ),
                    registry: registry.clone(),
                    transition: transition_notification.clone(),
                    sprint_transition: sprint_transition_notification.clone(),
                    workflow_execution: workflow_execution_notification.clone(),
                });
            let sessions::SessionServices { application, imports, selected_runtime_profile, capability_profiles, endpoints } = sessions::compose(
