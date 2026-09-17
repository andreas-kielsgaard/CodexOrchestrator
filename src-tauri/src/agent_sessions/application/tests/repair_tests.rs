use super::*;
use crate::harness_engine::{
    domain::SidecarBindingRegistration,
    proxy::{run_proxy_listener, ProxyBindings},
    repository::SqliteHarnessBindingRepository,
    service::{HarnessEngineService, ManagedMcpUpstreamRegistry},
    session_binding::SessionProfileHarnessAuthority,
    sidecar::HarnessSidecarClient,
};
use crate::{
    execution_configuration::{CapabilityProfileService, SqliteCapabilityProfileRepository},
    workflows::{
        authoring::{WorkflowAuthoringConnection, WorkflowAuthoringNode},
        authoring_repository::SqliteWorkflowAuthoringRepository,
        authoring_service::WorkflowAuthoringService,
        compiled_plan::WorkflowConnectionPromptInput,
        execution::WorkflowExecutionService,
        instance_domain::*,
        instances::{RecipeInstance, WorkflowInstanceStore},
    },
};
use std::sync::RwLock;
use tokio_util::sync::CancellationToken;

struct WorkflowNotifier {
    recording: RecordingNotifier,
    execution: Mutex<Option<std::sync::Weak<WorkflowExecutionService>>>,
    errors: Mutex<Vec<String>>,
}
impl AgentSessionNotifier for WorkflowNotifier {
    fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
        self.recording.notify(notification.clone())?;
        let execution = self
            .execution
            .lock()
            .unwrap()
            .clone()
            .and_then(|service| service.upgrade());
        if let Some(execution) = execution {
            if let Err(error) = execution.on_agent_notification(&notification) {
                self.errors.lock().unwrap().push(error);
            }
        }
        Ok(())
    }
}

struct Fixture {
    folder: tempfile::TempDir,
    repository: Arc<SqliteAgentSessionRepository>,
    runtime: Arc<FakeRuntime>,
    notifier: Arc<WorkflowNotifier>,
    sessions: Arc<AgentSessionApplication>,
    direct: AgentSessionApplication,
    authoring: Arc<WorkflowAuthoringService>,
    execution: Arc<WorkflowExecutionService>,
    engine: Option<Arc<HarnessEngineService>>,
}

use crate::execution_configuration::RuntimeQuickFeatures;
struct QuickSource {
    profile_ref: String,
    contexts: Mutex<Vec<Option<String>>>,
}
impl SelectedRuntimeProfileSource for QuickSource {
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        Ok(test_selected_runtime_profile())
    }
    fn quick_features_at(
        &self,
        cwd: Option<&str>,
    ) -> Result<RuntimeQuickFeatures, SelectedRuntimeProfileSourceError> {
        self.contexts.lock().unwrap().push(cwd.map(String::from));
        Ok(RuntimeQuickFeatures {
            profile_ref: self.profile_ref.clone(),
            ..Default::default()
        })
    }
}

#[test]
fn quick_features_use_session_context_and_reject_a_different_provider_profile() {
    let fixture = Fixture::new();
    let source = Arc::new(QuickSource {
        profile_ref: test_selected_runtime_profile().profile_ref,
        contexts: Mutex::new(Vec::new()),
    });
    let application = fixture.direct.clone().with_profile_source(source.clone());
    application
        .load_quick_features(None, Some("preview-directory"), None)
        .unwrap();
    assert!(fixture.launches().is_empty());
    let cwd = fixture.folder.path().to_string_lossy().into_owned();
    let session = application
        .create_default_session(
            CreateAgentSessionCommand {
                title: Some("Quick features".into()),
                working_directory: Some(cwd.clone()),
                requested_options: Default::default(),
            },
            Default::default(),
        )
        .unwrap();
    application
        .load_quick_features(
            Some(&session.id),
            Some("caller-cannot-replace-session-context"),
            None,
        )
        .unwrap();
    assert_eq!(
        *source.contexts.lock().unwrap(),
        vec![Some("preview-directory".into()), Some(cwd)]
    );
    assert!(application
        .load_session(&session.id)
        .unwrap()
        .invocations
        .is_empty());
    let changed = application.with_profile_source(Arc::new(QuickSource {
        profile_ref: "different-profile".into(),
        contexts: Mutex::new(Vec::new()),
    }));
    assert!(changed
        .load_quick_features(Some(&session.id), None, None)
        .unwrap_err()
        .contains("no longer matches"));
}

#[test]
fn steering_is_durable_and_does_not_create_a_workflow_delivery() {
    let fixture = Fixture::with_runtime(RuntimeBehavior::StayRunning, false);
    let instance = fixture.instance(None);
    fixture
        .execution
        .dispatch_user_request(&instance.recipe.recipe_id, &instance.id, "Plan".into())
        .unwrap();
    let launch = fixture.launches().remove(0);
    let attempts = fixture.execution.instances.attempts(&instance.id).unwrap();
    let command = super::super::SteerAgentSessionCommand {
        session_id: launch.session_id.clone(),
        invocation_id: launch.invocation_id.clone(),
        input_id: "steering-input".into(),
        text: "Use the revised requirements".into(),
    };
    let accepted = fixture.sessions.steer_session(command.clone()).unwrap();
    assert_eq!(accepted.state, "accepted");
    assert_eq!(
        fixture.sessions.steer_session(command.clone()).unwrap(),
        accepted
    );
    assert!(fixture
        .sessions
        .steer_session(super::super::SteerAgentSessionCommand {
            text: "Different content with the same identity".into(),
            ..command
        })
        .is_err());
    assert_eq!(fixture.launches().len(), 1);
    assert_eq!(fixture.runtime.calls.lock().unwrap().iter().filter(|call| matches!(call,
        RuntimeCall::Steer(id, input) if *id == launch.invocation_id && input == "steering-input"
    )).count(), 1);
    assert_eq!(
        fixture
            .notifier
            .recording
            .notifications
            .lock()
            .unwrap()
            .iter()
            .filter(|event| matches!(event, AgentSessionNotification::SteeringAccepted { .. }))
            .count(),
        1
    );
    assert!(fixture
        .notifier
        .recording
        .persisted_before_notify
        .load(Ordering::SeqCst));
    assert!(fixture.notifier.errors.lock().unwrap().is_empty());
    assert_eq!(
        serde_json::to_value(fixture.execution.instances.attempts(&instance.id).unwrap()).unwrap(),
        serde_json::to_value(attempts).unwrap()
    );
    let reopened =
        SqliteAgentSessionRepository::open(fixture.folder.path().join("repair.sqlite")).unwrap();
    let history = reopened
        .load_session_history(&launch.session_id)
        .unwrap()
        .unwrap();
    assert_eq!(history.invocations.len(), 1);
    assert_eq!(super::super::project_interactions(&history), vec![accepted]);
}

impl Fixture {
    fn new() -> Self {
        Self::with_runtime(RuntimeBehavior::CompleteWithFinalOutput, false)
    }

    fn with_runtime(behavior: RuntimeBehavior, mediated: bool) -> Self {
        let folder = tempfile::tempdir().unwrap();
        let database_path = folder.path().join("repair.sqlite");
        let database = crate::product_database::open(&database_path).unwrap();
        let identities = IdentityService::from_database(database.clone());
        let repository = Arc::new(SqliteAgentSessionRepository::from_database(
            database.clone(),
        ));
        let runtime = Arc::new(FakeRuntime::new(behavior));
        let notifier = Arc::new(WorkflowNotifier {
            recording: RecordingNotifier::new(repository.clone()),
            execution: Mutex::new(None),
            errors: Mutex::new(Vec::new()),
        });
        let providers = Arc::new(DeterministicProviders::default());
        let registry = Arc::new(ManagedMcpUpstreamRegistry::default());
        let engine = mediated.then(|| {
            HarnessEngineService::new(
                Arc::new(SqliteHarnessBindingRepository::from_database(
                    database.clone(),
                )),
                Arc::new(LocalProxy::new()),
                registry.clone(),
            )
            .unwrap()
        });
        let mut sessions = AgentSessionApplication::new(
            repository.clone(),
            runtime.clone(),
            notifier.clone(),
            providers.clone(),
            providers,
            None,
        );
        if let Some(engine) = &engine {
            sessions = sessions.with_session_harness_launch_authority(Arc::new(
                SessionProfileHarnessAuthority {
                    engine: engine.clone(),
                    sessions: repository.clone(),
                },
            ));
        }
        let sessions = Arc::new(sessions);
        let mut snapshot = test_selected_runtime_profile();
        if mediated {
            snapshot.exposure.mcp_tools.insert(
                "workflow_handoff".into(),
                ["handoff_to_agent".into()].into_iter().collect(),
            );
        }
        let source = Arc::new(FixedSelectedRuntimeProfileSource(snapshot.clone()));
        let profiles = Arc::new(CapabilityProfileService::new(
            Arc::new(SqliteCapabilityProfileRepository::from_database(
                database.clone(),
            )),
            source.clone(),
        ));
        let definition = test_session_creation_request().capability_profile;
        profiles
            .create(
                definition.capability_profile_id.clone(),
                definition.name,
                snapshot.exposure,
            )
            .unwrap();
        profiles
            .set_default_profile(&definition.capability_profile_id)
            .unwrap();
        let sessions = Arc::new(
            sessions
                .as_ref()
                .clone()
                .with_profile_source(source.clone())
                .with_capability_profiles(profiles.clone()),
        );
        let adapter = Arc::new(
            AgentSessionEventAdapter::new(
                sessions.clone(),
                repository.clone(),
                source.clone(),
                identities,
            )
            .with_capability_profiles(profiles.clone()),
        );
        let events = Arc::new(SessionEventApplication::new(
            adapter.clone(),
            adapter.clone(),
            Arc::new(SqliteSessionEventStore::from_database(database.clone())),
        ));
        let authoring = Arc::new(WorkflowAuthoringService::new(
            Arc::new(SqliteWorkflowAuthoringRepository::from_database(
                database.clone(),
            )),
            profiles.clone(),
            crate::otp_host::OtpRegistry::import(&["workflow"]).unwrap(),
        ));
        let execution = Arc::new(WorkflowExecutionService::new(
            authoring.clone(),
            events,
            Arc::new(WorkflowInstanceStore::from_database(database)),
            adapter,
            repository.clone(),
        ));
        *notifier.execution.lock().unwrap() = Some(Arc::downgrade(&execution));
        if mediated {
            let (mut descriptors, owner) = crate::otp_host::mcp::start_server(
                crate::otp_host::OtpRegistry::import(&["workflow"]).unwrap(),
                Arc::downgrade(&execution),
            )
            .unwrap();
            let registration = registry.register(descriptors.remove(0)).unwrap();
            assert!(registry.retain_owner(&registration, owner).is_ok());
        }
        let direct = sessions.as_ref().clone();
        Self {
            folder,
            repository,
            runtime,
            notifier,
            sessions,
            direct,
            authoring,
            execution,
            engine,
        }
    }

    fn target(&self) -> ResolvedRepoBranchWorktreeTarget {
        ResolvedRepoBranchWorktreeTarget {
            repository: WorkflowRepositoryTarget {
                id: "repo".into(),
                name: "Repo".into(),
                git_common_directory: "git".into(),
            },
            branch: WorkflowBranchTarget {
                id: "branch".into(),
                name: "main".into(),
            },
            worktree: WorkflowWorktreeTarget {
                id: "worktree".into(),
                path: self.folder.path().to_string_lossy().into_owned(),
            },
        }
    }

    fn instance(&self, file: Option<&str>) -> RecipeInstance {
        let mut state = self.authoring.create("Review".into()).unwrap();
        state.draft.starting_node_id = Some("a".into());
        state.draft.nodes = ["a", "b"]
            .into_iter()
            .map(|id| WorkflowAuthoringNode {
                node_id: id.into(),
                name: id.into(),
                position_x: 50.0,
                position_y: 50.0,
                capability_profile_id: "test-capabilities".into(),
                node_profile: test_session_creation_request().node_profile,
                initial_prompt: Some(format!("Initial for {id}")),
                agent_identity_id: None,
                agent_mcp_configuration: Default::default(),
            })
            .collect();
        let mut prompt_inputs = vec![WorkflowConnectionPromptInput::OutputField {
            field: "output".into(),
        }];
        if let Some(file) = file {
            prompt_inputs.push(WorkflowConnectionPromptInput::FileContent { path: file.into() });
        }
        state.draft.connections = vec![WorkflowAuthoringConnection {
            connection_id: "a-to-b".into(),
            name: "Review the plan".into(),
            source_node_id: "a".into(),
            destination_node_id: "b".into(),
            trigger: crate::otp_api::OutputRef {
                capability: crate::otp_api::CapabilityRef {
                    package: "workflow".into(),
                    tool: "on_invocation_completed".into(),
                },
                output: "completed".into(),
            },
            prompt_inputs,
            prompt_text: "Review the result".into(),
            action: crate::otp_api::CapabilityRef {
                package: "workflow".into(),
                tool: "prompt_agent".into(),
            },
            configuration: serde_json::json!({}),
        }];
        state = self.authoring.save_draft(state.draft).unwrap();
        self.authoring.activate(&state.draft.recipe_id).unwrap();
        self.execution
            .create_instance(
                &state.draft.recipe_id,
                state.draft.revision,
                "Run".into(),
                self.target(),
            )
            .unwrap()
    }

    fn launches(&self) -> Vec<RuntimeInvocationRequest> {
        self.runtime
            .calls
            .lock()
            .unwrap()
            .iter()
            .filter_map(|call| match call {
                RuntimeCall::Start(request) | RuntimeCall::Resume(request, _) => {
                    Some(request.clone())
                }
                _ => None,
            })
            .collect()
    }
}

impl Drop for Fixture {
    fn drop(&mut self) {
        if let Some(engine) = &self.engine {
            let _ = engine.shutdown();
        }
    }
}

/// Existing proxy implementation in-process; only the child-process transport is replaced.
struct LocalProxy {
    bindings: Arc<RwLock<ProxyBindings>>,
    address: std::net::SocketAddr,
    cancellation: CancellationToken,
    thread: Mutex<Option<std::thread::JoinHandle<()>>>,
}
impl LocalProxy {
    fn new() -> Self {
        let listener = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
        listener.set_nonblocking(true).unwrap();
        let address = listener.local_addr().unwrap();
        let bindings = Arc::new(RwLock::new(ProxyBindings::default()));
        let cancellation = CancellationToken::new();
        let state = bindings.clone();
        let cancel = cancellation.clone();
        let thread = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    run_proxy_listener(
                        tokio::net::TcpListener::from_std(listener).unwrap(),
                        state,
                        cancel,
                    )
                    .await;
                });
        });
        Self {
            bindings,
            address,
            cancellation,
            thread: Mutex::new(Some(thread)),
        }
    }
}
impl HarnessSidecarClient for LocalProxy {
    fn register_binding(&self, registration: SidecarBindingRegistration) -> Result<String, String> {
        self.bindings.write().unwrap().register(registration)
    }
    fn ensure_binding(&self, registration: SidecarBindingRegistration) -> Result<(), String> {
        self.register_binding(registration).map(|_| ())
    }
    fn retire_binding(&self, id: &str) -> Result<(), String> {
        self.bindings.write().unwrap().retire(id);
        Ok(())
    }
    fn prepare_invocation(&self, id: &str, invocation: &str) -> Result<(), String> {
        self.bindings
            .write()
            .unwrap()
            .prepare_invocation(id, invocation)
    }
    fn proxy_address(&self) -> Result<std::net::SocketAddr, String> {
        Ok(self.address)
    }
    fn shutdown(&self) -> Result<(), String> {
        self.cancellation.cancel();
        if let Some(thread) = self.thread.lock().unwrap().take() {
            thread.join().unwrap();
        }
        Ok(())
    }
}
impl Drop for LocalProxy {
    fn drop(&mut self) {
        let _ = self.shutdown();
    }
}

#[test]
fn standalone_first_and_second_message_use_pinned_profile_without_pinning_first_override() {
    let fixture = Fixture::new();
    let created = fixture
        .direct
        .start_direct_user_session(
            "First".into(),
            None,
            Some(fixture.target().worktree.path.clone()),
            Some("user-only".into()),
            Some("medium".into()),
            None,
            None,
        )
        .unwrap();
    let session_id = created.acknowledgement.session_id;
    let pinned = fixture
        .sessions
        .load_session(&session_id)
        .unwrap()
        .session
        .session_profile
        .unwrap();
    assert_eq!(
        pinned.session_profile().capability_profile_id(),
        test_session_creation_request()
            .capability_profile
            .capability_profile_id
    );
    assert_eq!(pinned.session_profile().pinned_defaults().model, None);
    fixture
        .direct
        .send_direct_user_message(SendDirectUserAgentSessionMessageCommand {
            sandbox_mode: None,
            session_id: session_id.clone(),
            submitted_text: "Second".into(),
            model: None,
            reasoning_mode: None,
        })
        .unwrap();
    let history = fixture.sessions.load_session(&session_id).unwrap();
    assert_eq!(history.session.session_profile, Some(pinned));
    assert_eq!(history.invocations.len(), 2);
    assert_eq!(
        history.invocations[0]
            .invocation
            .requested_options
            .model
            .as_deref(),
        Some("user-only")
    );
    assert_eq!(
        history.invocations[1].invocation.requested_options.model,
        None
    );
    assert!(fixture
        .launches()
        .iter()
        .all(|launch| launch.working_directory.as_ref() == Some(&fixture.target().worktree.path)));
}

#[test]
fn invalid_standalone_override_leaves_no_session_or_launch() {
    let fixture = Fixture::new();
    assert!(fixture
        .direct
        .start_direct_user_session(
            "First".into(),
            None,
            None,
            Some("not-exposed".into()),
            None,
            None,
            None
        )
        .is_err());
    assert!(fixture
        .repository
        .list_sessions(Default::default())
        .unwrap()
        .is_empty());
    assert!(fixture.launches().is_empty());
}

// Workflow routing is covered by otp_packages::workflow tests.
