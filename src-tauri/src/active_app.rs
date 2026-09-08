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
    sprint_transition: Arc<
        Mutex<
            Option<
                Weak<crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService>,
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
        // Runtime launch provenance is persisted synchronously before the process start returns.
        // A Bootstrap-terminal transition can therefore launch the Runner and re-enter this
        // notifier before the outer notification completes. Never retain a registry lock while
        // dispatching that callback.
        let transition = {
            self.transition
                .lock()
                .map_err(|_| "post-confirmation notification registry is unavailable".to_string())?
                .clone()
        };
        let transition_error = transition
            .and_then(|service| service.upgrade())
            .map(|service| service.on_agent_notification(&notification))
            .transpose()
            .err()
            .map(|error| error.to_string());
        let sprint_transition = {
            self.sprint_transition
                .lock()
                .map_err(|_| "Sprint Runner notification registry is unavailable".to_string())?
                .clone()
        };
        let sprint_transition_error = sprint_transition
            .and_then(|service| service.upgrade())
            .map(|service| service.on_agent_notification(&notification))
            .transpose()
            .err()
            .map(|error| error.to_string());
        let inner_error = self.inner.notify(notification).err();
        match (transition_error, sprint_transition_error, inner_error) {
            (None, None, None) => Ok(()),
            (Some(error), None, None) | (None, Some(error), None) | (None, None, Some(error)) => {
                Err(error)
            }
            (transition, sprint, inner) => Err([transition, sprint, inner]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("; ")),
        }
    }
}

fn worktree_review_root(app_data_dir: &std::path::Path) -> PathBuf {
    app_data_dir.join("worktree-review")
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
            let native_profiles = Arc::new(crate::native_profiles::NativeProfileService::open(
                database_path.clone(),
                app_data_dir.clone(),
            )?);
            let repository = Arc::new(
                crate::agent_sessions::repository::SqliteAgentSessionRepository::new(connection)
                    .map_err(|error| error.to_string())?,
            );
            let orchestration_repository = Arc::new(
                crate::orchestration::repository::SqliteOrchestrationRepository::open_with_harness_revision_repository(
                    &database_path,
                    crate::storage::harness_revision_repository_path(&app_data_dir),
                )
                .map_err(|error| error.to_string())?,
            );
            let product_decisions = Arc::new(
                crate::product_decisions::ProductDecisionRepository::open(&database_path)
                    .map_err(|_| "Unable to open Product Decision storage.".to_string())?,
            );
            // This product-native seam resolves only durable application-owned attempt authority.
            let execution_support = crate::orchestration::execution_support::ProductExecutionSupportState::new(
                &database_path,
                app_data_dir.join("execution-workspaces"),
                orchestration_repository.clone(),
            )
            .map_err(|error| error.to_string())?;
            let execution_support_service = execution_support.service();
            app.manage(execution_support);
            // Startup never probes the provider. Capability failures are handled per invocation.
            let runtime = Arc::new(crate::runtime::codex::CodexCliRuntime::system(
                "codex", None,
            ));
            let registry =
                Arc::new(crate::orchestration::application::ManagedPlanBuilderRegistry::default());
            let transition_notification = Arc::new(Mutex::new(None));
            let sprint_transition_notification = Arc::new(Mutex::new(None));
            let notifier: Arc<dyn crate::agent_sessions::application::AgentSessionNotifier> =
                Arc::new(ManagedPlanBuilderNotifier {
                    inner: Arc::new(
                        crate::agent_sessions::transport::TauriAgentSessionNotifier::new(
                            app.handle().clone(),
                        ),
                    ),
                    registry: registry.clone(),
                    transition: transition_notification.clone(),
                    sprint_transition: sprint_transition_notification.clone(),
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
                )
                .with_native_profile_launch_authority(native_profiles.clone()),
            );
            application
                .reconcile_startup()
                .map_err(|error| error.to_string())?;
            app.manage(
                crate::agent_sessions::transport::AgentSessionTauriState::new(application.clone()),
            );
            app.manage(crate::native_profiles::NativeProfileTauriState::new(
                native_profiles,
            ));
            let orchestration = Arc::new(
                crate::orchestration::application::OrchestrationApplication::new(
                    orchestration_repository.clone(),
                ),
            );
            // Composition only makes the bounded package constructible. It creates no workflow
            // Session, invocation, Work Unit, or attempt at startup.
            let work_unit_handler = Arc::new(
                crate::orchestration::work_unit_execution_harness::WorkUnitExecutionHarnessService::new(
                    execution_support_service,
                    application.clone(),
                    orchestration.clone(),
                ),
            );
            app.manage(work_unit_handler.clone());
            app.manage(
                crate::orchestration::transport::OrchestrationTauriState::new(
                    orchestration.clone(),
                ),
            );
            app.manage(crate::product_decisions::ProductDecisionTauriState::new(
                product_decisions,
                application.clone(),
            ));
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
            let sprint_runners = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::open_with_application_git_authority(
                &database_path,
                application.clone(),
            )
            .map_err(|error| error.to_string())?;
            let git_comparison = sprint_runners.git_comparison_port();
            sprint_runners
                .attach_work_unit_handler_activation(work_unit_handler)
                .map_err(|error| error.to_string())?;
            *sprint_transition_notification.lock().map_err(|_| "Sprint Runner notification registry is unavailable")? = Some(Arc::downgrade(&sprint_runners));
            transition
                .attach_sprint_runner_transition(sprint_runners.clone())
                .map_err(|error| error.to_string())?;
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
            sprint_runners
                .reconcile_startup()
                .map_err(|error| error.to_string())?;
            app.manage(
                crate::orchestration::transport::BootstrapTransitionTauriState::new(
                    transition,
                ),
            );
            app.manage(
                crate::orchestration::transport::SprintRunnerTransitionTauriState::new(
                    sprint_runners,
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
                        orchestration.clone(),
                        application,
                        registry,
                        initiation_confirmations,
                    ),
                ),
            );
            let review_root = worktree_review_root(&app_data_dir);
            let review = Arc::new(crate::worktree_review::WorktreeReviewApplication::open(
                review_root.clone(),
            ));
            app.manage(
                crate::orchestration::transport::ContextualFileReviewTauriState::available(
                    orchestration.clone(),
                    Arc::new(
                        crate::orchestration::file_review_originating_entry::FileReviewOriginatingEntryService::new(
                            orchestration_repository.clone(),
                            git_comparison,
                        ),
                    ),
                ),
            );
            app.manage(
                crate::worktree_review::transport::WorktreeReviewTauriState::new(review),
            );
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
            crate::epic_origin::choose_epic_origin_project,
            crate::epic_origin::inspect_epic_origin_project,
            crate::agent_sessions::transport::create_agent_session,
            crate::agent_sessions::transport::list_agent_sessions,
            crate::agent_sessions::transport::load_agent_session,
            crate::agent_sessions::transport::send_agent_session_message,
            crate::agent_sessions::transport::cancel_agent_invocation,
            crate::native_profiles::load_native_profile_query,
            crate::native_profiles::register_native_profile,
            crate::native_profiles::create_dedicated_native_profile,
            crate::native_profiles::select_native_profile,
            crate::native_profiles::select_native_profile_execution_mode,
            crate::native_profiles::authorize_native_profile_danger_full_access,
            crate::native_profiles::revoke_native_profile_danger_full_access,
            crate::native_profiles::request_native_profile_login,
            crate::native_profiles::refresh_native_profile_readiness,
            crate::native_profiles::request_native_profile_sandbox_initialization,
            crate::native_profiles::confirm_native_profile_sandbox_initialization,
            crate::native_profiles::verify_native_profile_preprovisioned_sandbox,
            crate::native_profiles::confirm_native_profile_preprovisioned_sandbox_adoption,
            crate::native_profiles::run_native_profile_workspace_write_canary,
            crate::native_profiles::run_native_profile_danger_full_access_canary,
            crate::native_profiles::probe_native_profile_mcp_reporting,
            crate::native_profiles::reconcile_native_profile_mcp_reporting,
            crate::product_decisions::accept_product_decision_version,
            crate::product_decisions::load_product_decision_current_query,
            crate::product_decisions::load_product_decision_history,
            crate::product_decisions::start_product_decision_correction_conversation,
            crate::product_decisions::send_product_decision_correction_message,
            crate::product_decisions::save_product_decision_correction_proposal,
            crate::product_decisions::accept_product_decision_correction_proposal,
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
            crate::orchestration::transport::request_contextual_file_review,
            crate::orchestration::transport::load_epic_bootstrap_transition_query,
            crate::orchestration::transport::load_sprint_runner_transition_query,
            crate::worktree_application::transport::create_physical_worktree,
            crate::worktree_review::transport::worktree_review_overview,
            crate::worktree_review::transport::select_worktree_review_repository,
            crate::worktree_review::transport::worktree_review_repository_registration_overview,
            crate::worktree_review::transport::register_worktree_review_repository_directory,
            crate::worktree_review::transport::register_codex_worktree_review_repository,
            crate::worktree_review::transport::worktree_review_branch_detail,
            crate::worktree_review::transport::worktree_review_branch_history,
            crate::worktree_review::transport::associate_worktree_review_worktree,
            crate::worktree_review::transport::create_worktree_review_worktree,
            crate::worktree_review::transport::create_worktree_review_build,
            crate::worktree_review::transport::worktree_review_open_build
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

#[cfg(test)]
mod tests {
    use super::worktree_review_root;

    #[test]
    fn review_storage_is_app_owned_and_override_must_be_absolute() {
        let app_data = std::env::temp_dir().join("codex-orchestrator-app-data");
        assert_eq!(
            worktree_review_root(&app_data),
            app_data.join("worktree-review")
        );
    }
}
