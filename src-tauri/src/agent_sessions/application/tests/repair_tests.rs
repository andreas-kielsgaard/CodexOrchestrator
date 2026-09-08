use super::*;
use crate::harness_engine::{
    domain::SidecarBindingRegistration,
    proxy::{run_proxy_listener, ProxyBindings},
    repository::SqliteHarnessBindingRepository,
    service::{HarnessEngineService, ManagedMcpUpstreamRegistry},
    session_binding::SessionProfileHarnessAuthority,
    sidecar::HarnessSidecarClient,
};
use crate::session_events::SessionEventStore;
use crate::{
    execution_configuration::{CapabilityProfileService, SqliteCapabilityProfileRepository},
    workflows::{
        authoring::{WorkflowAuthoringConnection, WorkflowAuthoringNode},
        authoring_repository::SqliteWorkflowAuthoringRepository,
        authoring_service::WorkflowAuthoringService,
        compiled_plan::{
            WorkflowConnectionPromptInput, WorkflowConnectionTargetPlan, WorkflowConnectionTrigger,
        },
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
    profiles: Arc<CapabilityProfileService>,
    direct: AgentSessionProfileApplication,
    authoring: Arc<WorkflowAuthoringService>,
    execution: Arc<WorkflowExecutionService>,
    engine: Option<Arc<HarnessEngineService>>,
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
        let repository = Arc::new(SqliteAgentSessionRepository::from_database(database.clone()));
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
                Arc::new(SqliteHarnessBindingRepository::from_database(database.clone())),
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
                definition.capability_profile_id,
                definition.name,
                snapshot.exposure,
            )
            .unwrap();
        let adapter = Arc::new(
            AgentSessionEventAdapter::from_database(
                database.clone(),
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
            let (descriptor, owner) =
                crate::workflows::mcp::start_session_event_server(Arc::downgrade(&execution))
                    .unwrap();
            let registration = registry.register(descriptor).unwrap();
            assert!(registry.retain_owner(&registration, owner).is_ok());
        }
        let direct = AgentSessionProfileApplication::new(sessions.clone(), source);
        Self {
            folder,
            repository,
            runtime,
            notifier,
            sessions,
            profiles,
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
            })
            .collect();
        let mut prompt_inputs = vec![WorkflowConnectionPromptInput::InvocationOutput];
        if let Some(file) = file {
            prompt_inputs.push(WorkflowConnectionPromptInput::ReferencedContent {
                reference: ReferenceIdentity::new("file", "path", file).unwrap(),
            });
        }
        state.draft.connections = vec![WorkflowAuthoringConnection {
            connection_id: "a-to-b".into(),
            name: "Review the plan".into(),
            source_node_id: "a".into(),
            destination_node_id: "b".into(),
            trigger: WorkflowConnectionTrigger::InvocationCompleted,
            prompt_inputs,
            prompt_text: "Review the result".into(),
            target: WorkflowConnectionTargetPlan::default(),
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
        "application:standalone"
    );
    assert_eq!(pinned.session_profile().pinned_defaults().model, None);
    fixture
        .direct
        .send_direct_user_message(SendDirectUserAgentSessionMessageCommand {
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
        .start_direct_user_session("First".into(), None, None, Some("not-exposed".into()), None)
        .is_err());
    assert!(fixture
        .repository
        .list_sessions(Default::default())
        .unwrap()
        .is_empty());
    assert!(fixture.launches().is_empty());
}

#[test]
fn stored_instance_drives_normal_completion_and_existing_sessions_ignore_deleted_profiles() {
    let fixture = Fixture::new();
    let instance = fixture.instance(None);
    assert!(fixture.launches().is_empty());
    let reopened =
        WorkflowInstanceStore::open(&fixture.folder.path().join("repair.sqlite")).unwrap();
    assert_eq!(reopened.load(&instance.id).unwrap(), instance);
    let first = fixture
        .execution
        .dispatch_user_request(
            &instance.recipe.recipe_id,
            &instance.id,
            "Make a plan".into(),
        )
        .unwrap();
    assert_eq!(
        fixture.launches().len(),
        2,
        "normal runtime notification must launch B"
    );
    assert!(fixture.notifier.errors.lock().unwrap().is_empty());
    let entries = fixture.execution.instance_sessions(&instance).unwrap();
    assert_eq!(entries.len(), 2);
    let b = entries
        .iter()
        .find(|entry| entry.logical_address.as_ref().unwrap().subject.id() == "b")
        .unwrap();
    let b_id = AgentSessionId::new(b.session.id()).unwrap();
    let b_history = fixture.sessions.load_session(&b_id).unwrap();
    assert_eq!(
        b_history.invocations[0].invocation.submitted_text,
        "Plan from this invocation\n\nReview the result"
    );
    assert_eq!(
        b_history.session.working_directory.as_deref(),
        Some(instance.target.worktree.path.as_str())
    );
    let before = b_history.session.session_profile.clone();
    let a_id = AgentSessionId::new(first.group.created_session.unwrap().id()).unwrap();
    fixture.profiles.delete("test-capabilities").unwrap();
    fixture
        .direct
        .send_direct_user_message(SendDirectUserAgentSessionMessageCommand {
            session_id: a_id.clone(),
            submitted_text: "User follow-up".into(),
            model: Some("user-only".into()),
            reasoning_mode: Some("medium".into()),
        })
        .unwrap();
    fixture
        .execution
        .dispatch_user_request(
            &instance.recipe.recipe_id,
            &instance.id,
            "Workflow follow-up".into(),
        )
        .unwrap();
    assert_eq!(fixture.launches().len(), 6);
    assert!(fixture.notifier.errors.lock().unwrap().is_empty());
    let after = fixture.sessions.load_session(&b_id).unwrap();
    assert_eq!(after.session.session_profile, before);
    assert!(after.invocations.iter().all(|turn| turn
        .invocation
        .requested_options
        .model
        .as_deref()
        == Some("node-default")));
    let launches = fixture.launches();
    assert!(launches.iter().skip(2).all(|launch| launch
        .launch_extension
        .as_ref()
        .is_none_or(|extension| extension.initial_prompt_prefix.is_none())));
    let a_history = fixture.sessions.load_session(&a_id).unwrap();
    let terminal = a_history.invocations.last().unwrap().invocation.clone();
    fixture
        .execution
        .on_agent_notification(&AgentSessionNotification::InvocationTerminal {
            session_id: a_id,
            invocation: terminal,
        })
        .unwrap();
    assert_eq!(
        fixture.launches().len(),
        6,
        "duplicate completion must not dispatch again"
    );
    let new_instance = fixture
        .execution
        .create_instance(
            &instance.recipe.recipe_id,
            instance.recipe.revision,
            "Another".into(),
            fixture.target(),
        )
        .unwrap();
    assert!(fixture
        .execution
        .dispatch_user_request(
            &instance.recipe.recipe_id,
            &new_instance.id,
            "New birth".into()
        )
        .is_err());
    assert!(fixture
        .execution
        .instance_sessions(&new_instance)
        .unwrap()
        .is_empty());
}

#[test]
fn missing_prompt_file_records_handoff_failure_without_changing_sender_completion() {
    let fixture = Fixture::new();
    let instance = fixture.instance(Some("missing.txt"));
    let first = fixture
        .execution
        .dispatch_user_request(
            &instance.recipe.recipe_id,
            &instance.id,
            "Make a plan".into(),
        )
        .unwrap();
    assert_eq!(fixture.launches().len(), 1);
    let id = AgentSessionId::new(first.group.created_session.unwrap().id()).unwrap();
    assert_eq!(
        fixture.sessions.load_session(&id).unwrap().invocations[0]
            .invocation
            .status,
        AgentInvocationStatus::Completed
    );
    assert!(fixture
        .execution
        .instances
        .attempts(&instance.id)
        .unwrap()
        .iter()
        .any(|attempt| attempt
            .error
            .as_ref()
            .is_some_and(|error| error.contains("missing.txt"))));
}

#[test]
fn existing_instance_keeps_recipe_after_reactivation_and_second_instance_is_isolated() {
    let fixture = Fixture::new();
    let original = fixture.instance(None);
    let mut changed = fixture
        .authoring
        .load(&original.recipe.recipe_id)
        .unwrap()
        .draft;
    changed.connections.clear();
    changed.nodes[0].initial_prompt = Some("Changed initial prompt".into());
    changed = fixture.authoring.save_draft(changed).unwrap().draft;
    fixture.authoring.activate(&changed.recipe_id).unwrap();
    let next = fixture
        .execution
        .create_instance(
            &changed.recipe_id,
            changed.revision,
            "Next".into(),
            fixture.target(),
        )
        .unwrap();
    fixture
        .execution
        .dispatch_user_request(&changed.recipe_id, &original.id, "Old run".into())
        .unwrap();
    fixture
        .execution
        .dispatch_user_request(&changed.recipe_id, &next.id, "New run".into())
        .unwrap();
    assert_eq!(
        fixture
            .execution
            .instance_sessions(&original)
            .unwrap()
            .len(),
        2
    );
    assert_eq!(fixture.execution.instance_sessions(&next).unwrap().len(), 1);
    let launches = fixture.launches();
    assert_eq!(
        launches[0]
            .launch_extension
            .as_ref()
            .unwrap()
            .initial_prompt_prefix
            .as_ref()
            .unwrap()
            .content,
        "Initial for a"
    );
    assert_eq!(
        launches[2]
            .launch_extension
            .as_ref()
            .unwrap()
            .initial_prompt_prefix
            .as_ref()
            .unwrap()
            .content,
        "Changed initial prompt"
    );
}

#[test]
fn prompt_files_are_read_and_fixed_prompt_references_survive_in_delivery_records() {
    let fixture = Fixture::new();
    std::fs::write(
        fixture.folder.path().join("plan.md"),
        "Actual file contents",
    )
    .unwrap();
    let instance = fixture.instance(Some("plan.md"));
    fixture
        .execution
        .dispatch_user_request(&instance.recipe.recipe_id, &instance.id, "Plan".into())
        .unwrap();
    assert!(fixture.notifier.errors.lock().unwrap().is_empty());
    let launches = fixture.launches();
    assert!(launches[1].submitted_text.contains("Actual file contents"));
    let store =
        SqliteSessionEventStore::open(&fixture.folder.path().join("repair.sqlite")).unwrap();
    let attempts = fixture.execution.instances.attempts(&instance.id).unwrap();
    let group = attempts
        .iter()
        .find(|attempt| attempt.source_session_id.is_some())
        .unwrap()
        .event_group
        .as_ref()
        .unwrap();
    let records = store.deliveries_for_group(group).unwrap();
    assert!(records[0].prompt_contributions.iter().any(|source| matches!(source, crate::session_events::PromptSource::ReferencedContent { reference, text } if reference.kind() == "prompt_field" && text == "Review the result")));
    assert!(records[0].included_created_session_contributions.iter().any(|source| matches!(source, crate::session_events::PromptSource::ReferencedContent { reference, text } if reference.kind() == "prompt_field" && text == "Initial for b")));
}

#[test]
fn invalid_source_pairs_and_escaping_file_references_do_not_launch_receiver() {
    let fixture = Fixture::new();
    let instance = fixture.instance(None);
    let mut draft = instance.recipe.clone();
    draft.connections[0].prompt_inputs = vec![WorkflowConnectionPromptInput::McpArgument {
        name: "promptText".into(),
    }];
    let saved = fixture.authoring.save_draft(draft).unwrap();
    assert!(fixture
        .authoring
        .activate_revision(&saved.draft.recipe_id, saved.draft.revision)
        .is_err());
    let outside = tempfile::NamedTempFile::new().unwrap();
    let escape = fixture.instance(Some(outside.path().to_str().unwrap()));
    fixture
        .execution
        .dispatch_user_request(&escape.recipe.recipe_id, &escape.id, "Plan".into())
        .unwrap();
    assert_eq!(fixture.launches().len(), 1);
    assert!(fixture
        .execution
        .instances
        .attempts(&escape.id)
        .unwrap()
        .iter()
        .any(|attempt| attempt
            .error
            .as_ref()
            .is_some_and(|error| error.contains("outside"))));
}

#[tokio::test]
async fn pinned_mcp_binding_reaches_the_real_event_receiver_and_rejects_injected_routing() {
    let fixture = Fixture::with_runtime(RuntimeBehavior::StayRunning, true);
    let baseline = fixture.instance(None);
    let mut draft = baseline.recipe.clone();
    let tools = [(
        "workflow_handoff".into(),
        ["handoff_to_agent".into()].into_iter().collect(),
    )]
    .into_iter()
    .collect();
    draft.nodes[0].node_profile.allowed_capabilities.mcp_tools = tools;
    draft.connections[0].trigger = WorkflowConnectionTrigger::McpCall {
        server: ReferenceIdentity::new("mcp", "server", "workflow_handoff").unwrap(),
        tool: ReferenceIdentity::new("mcp", "tool", "handoff_to_agent").unwrap(),
    };
    draft.connections[0].prompt_inputs = vec![
        WorkflowConnectionPromptInput::McpArgument {
            name: "promptText".into(),
        },
        WorkflowConnectionPromptInput::McpArgument {
            name: "filePaths".into(),
        },
    ];
    draft = fixture.authoring.save_draft(draft).unwrap().draft;
    fixture
        .authoring
        .activate_revision(&draft.recipe_id, draft.revision)
        .unwrap();
    let instance = fixture
        .execution
        .create_instance(
            &draft.recipe_id,
            draft.revision,
            "MCP run".into(),
            fixture.target(),
        )
        .unwrap();
    fixture
        .execution
        .dispatch_user_request(&draft.recipe_id, &instance.id, "Plan".into())
        .unwrap();
    assert_eq!(fixture.launches().len(), 1);
    let launch = fixture.launches().remove(0);
    let args = &launch.launch_extension.as_ref().unwrap().additional_args;
    assert!(args.contains(&"mcp_servers={}".to_string()));
    let url: String = serde_json::from_str(
        args.iter()
            .find_map(|arg| arg.strip_prefix("mcp_servers.session_capability_1.url="))
            .unwrap(),
    )
    .unwrap();
    let rpc = |name: &str, arguments: serde_json::Value| {
        reqwest::Client::new().post(&url).json(&json!({"jsonrpc":"2.0","id":1,"method":"tools/call","params":{"name":name,"arguments":arguments}})).send()
    };
    let denied: serde_json::Value = rpc("not_allowed", json!({}))
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert!(
        denied.get("error").is_some() || denied.pointer("/result/isError") == Some(&json!(true))
    );
    let injected: serde_json::Value = rpc("handoff_to_agent", json!({"promptText":"Wrong route","filePaths":[],"_workflowInvocation":{"sourceInvocationId":"other-instance"}})).await.unwrap().json().await.unwrap();
    assert_eq!(injected["result"]["isError"], true);
    assert_eq!(fixture.launches().len(), 1);
    let response: serde_json::Value = rpc(
        "handoff_to_agent",
        json!({"promptText":"Review this","filePaths":["plan.md"]}),
    )
    .await
    .unwrap()
    .json()
    .await
    .unwrap();
    assert_eq!(response["result"]["isError"], false, "{response}");
    assert_eq!(
        response["result"]["structuredContent"]["filePaths"],
        json!(["plan.md"])
    );
    assert_eq!(fixture.launches().len(), 2);
    let receiver = fixture.launches()[1].session_id.clone();
    assert_eq!(
        fixture
            .sessions
            .load_session(&receiver)
            .unwrap()
            .invocations[0]
            .invocation
            .submitted_text,
        "Review this\n\nplan.md\n\nReview the result"
    );
    assert_eq!(
        fixture
            .execution
            .instance_sessions(&instance)
            .unwrap()
            .len(),
        2
    );
    assert!(fixture
        .execution
        .instance_sessions(&baseline)
        .unwrap()
        .is_empty());
}

#[test]
fn application_event_entry_uses_the_source_instance_and_configured_field() {
    let fixture = Fixture::new();
    let baseline = fixture.instance(None);
    let mut draft = baseline.recipe.clone();
    let event_kind = ReferenceIdentity::new("application", "event_kind", "plan_ready").unwrap();
    draft.connections[0].trigger = WorkflowConnectionTrigger::ApplicationEvent {
        event_kind: event_kind.clone(),
    };
    draft.connections[0].prompt_inputs =
        vec![WorkflowConnectionPromptInput::ApplicationEventField {
            field: "context".into(),
        }];
    draft = fixture.authoring.save_draft(draft).unwrap().draft;
    fixture
        .authoring
        .activate_revision(&draft.recipe_id, draft.revision)
        .unwrap();
    let instance = fixture
        .execution
        .create_instance(
            &draft.recipe_id,
            draft.revision,
            "Event run".into(),
            fixture.target(),
        )
        .unwrap();
    let first = fixture
        .execution
        .dispatch_user_request(&draft.recipe_id, &instance.id, "Plan".into())
        .unwrap();
    let source = first.group.created_session.unwrap();
    assert_eq!(fixture.launches().len(), 1);
    let trigger = crate::session_events::SessionEventOccurrenceTrigger::ApplicationEvent {
        event: ReferenceIdentity::new("application", "event", "ready-1").unwrap(),
        event_kind,
        fields: [("context".into(), "Ready to review".into())]
            .into_iter()
            .collect(),
    };
    assert_eq!(
        fixture
            .execution
            .receive_source_event(&source, "ready-1", trigger.clone())
            .unwrap(),
        1
    );
    assert_eq!(
        fixture
            .execution
            .receive_source_event(&source, "ready-1", trigger)
            .unwrap(),
        0
    );
    assert_eq!(fixture.launches().len(), 2);
    assert!(fixture
        .execution
        .instance_sessions(&baseline)
        .unwrap()
        .is_empty());
    let receiver = &fixture.launches()[1].session_id;
    assert_eq!(
        fixture.sessions.load_session(receiver).unwrap().invocations[0]
            .invocation
            .submitted_text,
        "Ready to review\n\nReview the result"
    );
}
