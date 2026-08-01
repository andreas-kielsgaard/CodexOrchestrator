#[cfg(debug_assertions)]
use std::path::PathBuf;
use std::{
    fs,
    sync::{Arc, Mutex, Weak},
};

use tauri::Manager;

struct ManagedPlanBuilderNotifier {
    inner: Arc<dyn crate::agent_sessions::application::AgentSessionNotifier>,
    registry: Arc<crate::orchestration::application::ManagedPlanBuilderRegistry>,
    transition: Arc<
        Mutex<
            Option<
                Weak<crate::orchestration::bootstrap_transition::PostConfirmationTransitionService>,
            >,
        >,
    >,
}
impl crate::agent_sessions::application::AgentSessionNotifier for ManagedPlanBuilderNotifier {
    fn notify(
        &self,
        notification: crate::agent_sessions::application::AgentSessionNotification,
    ) -> Result<(), String> {
        if let crate::agent_sessions::application::AgentSessionNotification::InvocationTerminal {
            invocation,
            ..
        } = &notification
        {
            self.registry.on_terminal(invocation);
        }
        let transition_error = self
            .transition
            .lock()
            .map_err(|_| "post-confirmation notification registry is unavailable".to_string())?
            .clone()
            .and_then(|service| service.upgrade())
            .map(|service| service.on_agent_notification(&notification))
            .transpose()
            .err()
            .map(|error| error.to_string());
        let inner_error = self.inner.notify(notification).err();
        match (transition_error, inner_error) {
            (None, None) => Ok(()),
            (Some(error), None) | (None, Some(error)) => Err(error),
            (Some(transition), Some(inner)) => Err(format!("{transition}; {inner}")),
        }
    }
}

pub(crate) fn run() {
    let app = tauri::Builder::default()
        .setup(|app| {
            let app_data_dir = crate::runtime::instance::app_data_dir(|| {
                app.path()
                    .app_data_dir()
                    .map_err(|error| format!("Unable to resolve app data directory: {error}"))
            })?;
            fs::create_dir_all(&app_data_dir)
                .map_err(|error| format!("Unable to create app data directory: {error}"))?;
            let database_path = crate::storage::active_database_path(&app_data_dir);
            let connection = crate::storage::open_active_database(&database_path)?;
            let repository = Arc::new(
                crate::agent_sessions::repository::SqliteAgentSessionRepository::new(connection)
                    .map_err(|error| error.to_string())?,
            );
            let orchestration_repository = Arc::new(
                crate::orchestration::repository::SqliteOrchestrationRepository::open(
                    &database_path,
                )
                .map_err(|error| error.to_string())?,
            );
            // Startup never probes the provider. Capability failures are handled per invocation.
            let runtime = Arc::new(crate::runtime::codex::CodexCliRuntime::system(
                "codex", None,
            ));
            let registry =
                Arc::new(crate::orchestration::application::ManagedPlanBuilderRegistry::default());
            let transition_notification = Arc::new(Mutex::new(None));
            let notifier: Arc<dyn crate::agent_sessions::application::AgentSessionNotifier> =
                Arc::new(ManagedPlanBuilderNotifier {
                    inner: Arc::new(
                        crate::agent_sessions::transport::TauriAgentSessionNotifier::new(
                            app.handle().clone(),
                        ),
                    ),
                    registry: registry.clone(),
                    transition: transition_notification.clone(),
                });
            let providers =
                Arc::new(crate::agent_sessions::application::SystemAgentSessionProviders);
            let application = Arc::new(
                crate::agent_sessions::application::AgentSessionApplication::new(
                    repository,
                    runtime,
                    notifier,
                    providers.clone(),
                    providers,
                    None,
                ),
            );
            application
                .reconcile_startup()
                .map_err(|error| error.to_string())?;
            app.manage(
                crate::agent_sessions::transport::AgentSessionTauriState::new(application.clone()),
            );
            let orchestration = Arc::new(
                crate::orchestration::application::OrchestrationApplication::new(
                    orchestration_repository,
                ),
            );
            app.manage(
                crate::orchestration::transport::OrchestrationTauriState::new(
                    orchestration.clone(),
                ),
            );
            let initiation_confirmations =
                crate::orchestration::confirmation::InitiationConfirmationCoordinator::new(
                    orchestration.clone(),
                    Arc::new(
                        crate::orchestration::transport::TauriInitiationConfirmationNotifier::new(
                            app.handle().clone(),
                        ),
                    ),
                );
            let transition_repository = Arc::new(
                crate::orchestration::bootstrap_transition::SqliteBootstrapTransitionRepository::open(
                    &database_path,
                )
                .map_err(|error| error.to_string())?,
            );
            let transition =
                crate::orchestration::bootstrap_transition::PostConfirmationTransitionService::new(
                    transition_repository,
                    application.clone(),
                    app_data_dir.join("orchestration-materials"),
                );
            *transition_notification
                .lock()
                .map_err(|_| "post-confirmation notification registry is unavailable")? =
                Some(Arc::downgrade(&transition));
            initiation_confirmations
                .set_persisted_observer(transition.persisted_initiation_observer())?;
            initiation_confirmations
                .set_button_context_scheduler(orchestration.clone())?;
            transition
                .reconcile_startup()
                .map_err(|error| error.to_string())?;
            app.manage(
                crate::orchestration::transport::BootstrapTransitionTauriState::new(
                    transition,
                ),
            );
            app.manage(
                crate::orchestration::transport::InitiationConfirmationTauriState::new(
                    initiation_confirmations.clone(),
                ),
            );
            app.manage(
                crate::orchestration::transport::ManagedPlanBuilderTauriState::new(
                    crate::orchestration::application::ManagedPlanBuilderService::new(
                        orchestration,
                        application,
                        registry,
                        initiation_confirmations,
                    ),
                ),
            );
            #[cfg(debug_assertions)]
            {
                let source = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
                    .parent()
                    .ok_or("Unable to resolve launcher source")?
                    .to_path_buf();
                let review_root = std::env::var_os("CODEX_ORCHESTRATOR_REVIEW_RUNTIME_DIR")
                    .map(PathBuf::from)
                    .unwrap_or_else(|| {
                        app_data_dir
                            .parent()
                            .unwrap_or(&app_data_dir)
                            .join("dev.codex-orchestrator.human-review")
                    });
                let review = Arc::new(crate::worktree_review::compose(&source, &review_root)?);
                if let Some(controller) =
                    crate::worktree_review::debug_controller::start_if_enabled(
                        review.clone(),
                        &review_root,
                    )?
                {
                    app.manage(controller);
                }
                app.manage(
                    crate::worktree_review::transport::HumanReviewLauncherTauriState::new(
                        review.clone(),
                    ),
                );
                if std::env::var_os("CODEX_ORCHESTRATOR_REVIEW_RUNTIME_DIR").is_some() {
                    app.get_webview_window("main")
                        .ok_or("Unable to identify the isolated review launcher window")?
                        .set_title("Codex Orchestrator - Worktree Review Launcher")
                        .map_err(|_| "Unable to identify the isolated review launcher window")?;
                }
            }
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::app_metadata,
            crate::load_open_task_dashboard,
            crate::register_task_worktree,
            crate::register_task_repo,
            crate::discover_task_repos,
            crate::create_open_task,
            crate::update_open_task,
            crate::archive_open_task,
            crate::load_task_run_detail,
            crate::start_codex_task_run,
            crate::agent_sessions::transport::create_agent_session,
            crate::agent_sessions::transport::list_agent_sessions,
            crate::agent_sessions::transport::load_agent_session,
            crate::agent_sessions::transport::send_agent_session_message,
            crate::agent_sessions::transport::cancel_agent_invocation,
            crate::orchestration::transport::send_managed_plan_builder_message,
            crate::orchestration::transport::request_managed_plan_builder_action,
            crate::orchestration::transport::reconcile_managed_plan_builder_session,
            crate::orchestration::transport::load_managed_plan_builder_harness_inspection,
            crate::orchestration::transport::update_epic_planning_draft_title,
            crate::orchestration::transport::cancel_epic_planning_draft,
            crate::orchestration::transport::request_epic_initiation_confirmation,
            crate::orchestration::transport::resolve_epic_initiation_confirmation,
            crate::orchestration::transport::load_orchestration_native_query,
            crate::orchestration::transport::load_scoped_file_review,
            crate::orchestration::transport::load_epic_bootstrap_transition_query,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::list_human_review_worktrees,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::list_human_review_instances,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::prepare_human_review_instance,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::human_review_operation_progress,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::list_human_review_operation_progress,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::human_review_instance_detail,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::human_review_instance_comparison,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::human_review_launcher_proof_navigation,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::human_review_launcher_detail_navigation,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::human_review_launcher_proof_presentation,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::mark_worktree_build_ready,
            #[cfg(debug_assertions)]
            crate::worktree_review::debug_controller::worktree_review_proof_navigation,
            #[cfg(debug_assertions)]
            crate::worktree_review::worktree_build::worktree_build_context,
            #[cfg(debug_assertions)]
            crate::worktree_review::detail::worktree_build_detail,
            #[cfg(debug_assertions)]
            crate::worktree_review::comparison::worktree_build_comparison,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::build_human_review_instance,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::start_human_review_instance,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::status_human_review_instance,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::focus_human_review_instance,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::stop_human_review_instance,
            #[cfg(debug_assertions)]
            crate::worktree_review::transport::recover_human_review_instance
        ])
        .build(tauri::generate_context!())
        .expect("error while building Codex Orchestrator");

    app.run(|app_handle, event| {
        if let tauri::RunEvent::ExitRequested { api, .. } = event {
            if let Some(state) =
                app_handle.try_state::<crate::agent_sessions::transport::AgentSessionTauriState>()
            {
                if let Some(managed) = app_handle
                    .try_state::<crate::orchestration::transport::ManagedPlanBuilderTauriState>(
                ) {
                    managed.service().shutdown();
                }
                if let Some(transition) = app_handle
                    .try_state::<crate::orchestration::transport::BootstrapTransitionTauriState>()
                {
                    transition.service().shutdown();
                }
                if let Err(error) = state.application().shutdown_runtime() {
                    // Runtime shutdown retains ownership through direct-child reap. If that
                    // authoritative path reports an error, keep the application alive so a later
                    // exit request can retry instead of silently accepting an uncertain cleanup.
                    eprintln!(
                        "Agent runtime shutdown failed; application exit was prevented: {error}"
                    );
                    api.prevent_exit();
                }
            }
        }
    });
}
