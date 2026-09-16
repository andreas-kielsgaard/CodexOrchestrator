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
            let sessions::SessionServices { application, imports, selected_runtime_profile, capability_profiles, execution_targets } = sessions::compose(
                database.clone(), &database_path, native_profiles.clone(), repository.clone(), harness_catalog.clone(), harness_engine.clone(), notifier,
            )?;
            app.manage(crate::execution_targets::transport::ExecutionTargetTauriState(execution_targets));
            let session_event_adapter = Arc::new(
                crate::agent_sessions::session_event_adapter::AgentSessionEventAdapter::new(
                    application.clone(),
                    repository.clone(),
                    selected_runtime_profile.clone(),
                    identities.clone(),
                ).with_capability_profiles(capability_profiles.clone()),
            );
            let session_event_store = Arc::new(
                crate::session_events::SqliteSessionEventStore::from_database(database.clone()),
            );
            let event_app_handle = app.handle().clone();
            let session_events = Arc::new(crate::session_events::SessionEventApplication::new(
                session_event_adapter.clone(),
                session_event_adapter.clone(),
                session_event_store.clone(),
            ).with_record_observer(Arc::new(move |result| {
                let _ = event_app_handle.emit("session-event-recorded", &result.group.event_group_id);
            })));
            let session_event_queries = Arc::new(
                crate::session_events::SessionEventQueryApplication::new(session_event_store),
            );
            app.manage(session_events.clone());
            app.manage(crate::session_events::transport::SessionEventQueryTauriState::new(
                session_event_queries,
            ));
            app.manage(
                crate::execution_configuration::transport::CapabilityProfileTauriState::new(
                    capability_profiles.clone(),
                ),
            );
            app.manage(imports);
            app.manage(
                crate::agent_sessions::transport::AgentSessionTauriState::new(application.clone()),
            );
            let workflow_authoring = Arc::new(
                crate::workflows::authoring_service::WorkflowAuthoringService::new(
                    Arc::new(
                        crate::workflows::authoring_repository::SqliteWorkflowAuthoringRepository::from_database(
                            database.clone(),
                        ),
                    ),
                    capability_profiles,
                ),
            );
            app.manage(
                crate::workflows::authoring_transport::WorkflowAuthoringTauriState::new(
                    workflow_authoring.clone(),
                ),
            );
            let instance_app_handle = app.handle().clone();
            let workflow_execution = Arc::new(crate::workflows::execution::WorkflowExecutionService::new(
                workflow_authoring, session_events,
                Arc::new(crate::workflows::instances::WorkflowInstanceStore::from_database(
                    database.clone(),
                )),
                session_event_adapter, repository.clone(),
            ).with_record_observer(Arc::new(move |instance_id| {
                let _ = instance_app_handle.emit("workflow-instance-updated", instance_id);
            })));
            *workflow_execution_notification.lock().map_err(|_| "Workflow notification registry is unavailable")? = Some(Arc::downgrade(&workflow_execution));
            app.manage(crate::session_navigation::transport::SessionNavigationTauriState(Arc::new(
                crate::session_navigation::application::SessionNavigationService::new(repository_catalog.clone(), workflow_execution.instances.clone(), repository.clone(), application.clone(), crate::session_navigation::order_repository::NavigationOrderRepository::new(database.clone()))
            )));
            app.manage(crate::workflows::execution_transport::WorkflowExecutionTauriState::new(workflow_execution.clone()));
            let (workflow_mcp, workflow_mcp_owner) =
                crate::workflows::mcp::start_session_event_server(Arc::downgrade(&workflow_execution))?;
            let workflow_mcp_registration = managed_mcp_upstreams.register(workflow_mcp)?;
            if let Err(workflow_mcp_owner) = managed_mcp_upstreams
                .retain_owner(&workflow_mcp_registration, workflow_mcp_owner)
            {
                workflow_mcp_owner.stop();
                managed_mcp_upstreams.unregister(&workflow_mcp_registration);
                return Err("Unable to retain the Workflow MCP server.".into());
            }
            app.manage(crate::harness_engine::HarnessEngineTauriState::new(
                harness_engine,
            ));
            app.manage(
                crate::harness_engine::transport::HarnessCatalogTauriState::new(harness_catalog),
            );
            app.manage(crate::identities::transport::IdentityTauriState::new(
                identities,
            ));
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
                crate::orchestration::bootstrap_transition::SqliteBootstrapTransitionRepository::from_database(
                    database.clone(),
                ),
            );
            let transition =
                crate::orchestration::bootstrap_transition::PostConfirmationTransitionService::new(
                    transition_repository,
                    application.clone(),
                    app_data_dir.join("orchestration-materials"),
                );
            let sprint_runners = crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService::from_database_with_application_git_authority(
                database.clone(),
                application.clone(),
            )
            .map_err(|error| error.to_string())?;
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
            // All terminal observers and managed MCP upstreams are ready before recovery can notify.
            application.reconcile_startup().map_err(|error| error.to_string())?;
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
                    crate::orchestration::application::ManagedPlanBuilderService::new_with_managed_mcp_upstreams(
                        orchestration.clone(),
                        application,
                        registry,
                        initiation_confirmations,
                        Some(managed_mcp_upstreams),
                    ),
                ),
            );
            let review = Arc::new(crate::worktree_review::WorktreeReviewApplication::open(
                worktree_review_root(&app_data_dir),
                repository_catalog,
            ));
            app.manage(crate::worktree_review::transport::WorktreeReviewTauriState::new(
                review,
            ));
            app.manage(
                crate::orchestration::transport::ContextualFileReviewTauriState::unavailable(
                    orchestration.clone(),
                ),
            );
            crate::session_navigation::agent_access::start(app.handle(), &app_data_dir)?;
            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            crate::session_navigation::agent_access::complete_session_navigation_command,
            crate::agent_sessions::transport::create_agent_session,
            crate::agent_sessions::transport::import::preview_codex_import,
            crate::agent_sessions::transport::import::import_codex_conversation,
            crate::agent_sessions::transport::list_agent_sessions,
            crate::agent_sessions::transport::load_agent_session,
            crate::agent_sessions::transport::send_agent_session_message,
            crate::execution_configuration::transport::load_default_capability_profile,
            crate::execution_configuration::transport::set_default_capability_profile,
            crate::agent_sessions::transport::interactions::steer_agent_session,
            crate::agent_sessions::transport::interactions::respond_to_agent_runtime_request,
            crate::agent_sessions::transport::interactions::list_agent_session_interactions,
            crate::agent_sessions::transport::interactions::resolve_agent_session_working_directory,
            crate::agent_sessions::transport::selections::load_pinned_agent_session_profile,
            crate::agent_sessions::transport::selections::load_current_agent_session_profile,
            crate::agent_sessions::transport::quick_features::load_agent_session_quick_features,
            crate::agent_sessions::transport::selections::send_direct_user_agent_session_message,
            crate::execution_targets::transport::resolve_published_worktree_commit,
            crate::agent_sessions::transport::preparation::send_prepared_agent_session_message,
            crate::agent_sessions::transport::preparation::load_agent_session_preparation,
            crate::agent_sessions::transport::preparation::cancel_agent_session_preparation,
            crate::agent_sessions::transport::preparation::retry_agent_session_preparation,
            crate::agent_sessions::transport::cancel_agent_invocation,
            crate::agent_sessions::transport::update_agent_session_harness,
            crate::agent_sessions::transport::update_agent_session_identity,
            crate::agent_sessions::transport::update_agent_session_model_override,
            crate::execution_configuration::transport::load_selected_runtime_profile,
            crate::execution_configuration::transport::load_native_capability_inventory,
            crate::execution_configuration::transport::list_capability_profiles,
            crate::execution_targets::transport::list_session_execution_targets,
            crate::execution_targets::transport::list_execution_target_devices,
            crate::execution_targets::transport::list_execution_worktree_choices,
            crate::execution_targets::transport::load_execution_target_runtime,
            crate::execution_targets::transport::list_repository_device_locations,
            crate::execution_targets::transport::save_repository_device_location,
            crate::execution_configuration::transport::load_capability_profile,
            crate::execution_configuration::transport::create_capability_profile,
            crate::execution_configuration::transport::update_capability_profile,
            crate::execution_configuration::transport::delete_capability_profile,
            crate::session_events::transport::load_session_event_group,
            crate::session_navigation::transport::start_direct_user_agent_session,
            crate::session_navigation::transport::load_agent_session_navigation,
            crate::session_navigation::transport::move_agent_session,
            crate::session_navigation::transport::pin_agent_session,
            crate::session_navigation::transport::reorder_session_navigation,
            crate::workflows::execution_transport::create_workflow_recipe_instance,
            crate::workflows::execution_transport::list_workflow_recipe_instances,
            crate::workflows::execution_transport::load_workflow_recipe_instance,
            crate::session_events::transport::load_recorded_session_event,
            crate::session_events::transport::list_session_event_deliveries_for_group,
            crate::session_events::transport::list_session_event_deliveries_for_session,
            crate::identities::transport::list_identities,
            crate::identities::transport::create_identity,
            crate::identities::transport::update_identity,
            crate::identities::transport::delete_identity,
            crate::harness_engine::transport::list_harnesses,
            crate::harness_engine::transport::load_harness,
            crate::harness_engine::transport::create_harness,
            crate::harness_engine::transport::rename_harness,
            crate::harness_engine::transport::save_harness_draft,
            crate::harness_engine::transport::publish_harness_draft,
            crate::harness_engine::transport::publish_session_harness_override,
            crate::harness_engine::transport::order_harness_version_replacement,
            crate::harness_engine::transport::resolve_harness_version,
            crate::workflows::authoring_transport::list_workflow_recipes,
            crate::workflows::authoring_transport::load_workflow_recipe,
            crate::workflows::authoring_transport::create_workflow_recipe,
            crate::workflows::authoring_transport::save_workflow_recipe_draft,
            crate::workflows::authoring_transport::copy_workflow_node_configuration,
            crate::workflows::authoring_transport::activate_workflow_recipe,
            crate::workflows::authoring_transport::compile_workflow_recipe_instance,
            crate::workflows::execution_transport::dispatch_workflow_user_request,
            crate::repository_catalog::transport::repository_catalog_overview,
            crate::repository_catalog::transport::register_repository_directory,
            crate::repository_catalog::transport::register_codex_repository,
            crate::repository_catalog::transport::list_registered_repository_worktree_targets,
            crate::native_profiles::load_native_profile_query,
            crate::native_profiles::discover_native_codex_homes,
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
            crate::worktree_review::transport::worktree_review_overview,
            crate::worktree_review::transport::select_worktree_review_repository,
            crate::worktree_review::transport::worktree_review_target_detail,
            crate::worktree_review::transport::worktree_review_commit_history,
            crate::worktree_review::transport::worktree_review_branch_graph,
            crate::worktree_review::transport::worktree_review_worktree_activity,
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
                if let Err(error) = state.application().shutdown_runtime() {
                    // Runtime shutdown retains ownership through direct-child reap. If that
                    // authoritative path reports an error, keep the application alive so a later
                    // exit request can retry instead of silently accepting an uncertain cleanup.
                    eprintln!(
                        "Agent runtime shutdown failed; application exit was prevented: {error}"
                    );
                    api.prevent_exit();
                    return;
                }
                // Release invocation ownership without stopping any upstream retained by the
                // application-lifetime Harness registry.
                if let Some(managed) = app_handle
                    .try_state::<crate::orchestration::transport::ManagedPlanBuilderTauriState>(
                ) {
                    managed.service().shutdown();
                }
                // Agent runtimes stop first, followed by the Harness proxy, then its retained
                // managed upstreams.
                if let Some(harness) =
                    app_handle.try_state::<crate::harness_engine::HarnessEngineTauriState>()
                {
                    if let Err(error) = harness.service().shutdown() {
                        eprintln!("Harness sidecar shutdown failed: {error}");
                        api.prevent_exit();
                        return;
                    }
                }
                if let Some(transition) = app_handle
                    .try_state::<crate::orchestration::transport::BootstrapTransitionTauriState>()
                {
                    transition.service().shutdown();
                }
            }
        }
    });
}
