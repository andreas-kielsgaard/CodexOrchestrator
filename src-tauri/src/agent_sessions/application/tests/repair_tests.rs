use super::*;
mod continuation_tests;
#[cfg(feature = "live-tests")]
mod live_continuation;
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
        let database = folder.path().join("repair.sqlite");
        let identities = IdentityService::open(&database).unwrap();
        let repository = Arc::new(SqliteAgentSessionRepository::open(&database).unwrap());
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
                Arc::new(SqliteHarnessBindingRepository::open(&database).unwrap()),
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
                "workflow".into(),
                [
                    "handoff_to_agent".into(),
                    "trigger_workflow_continuation".into(),
                ]
                .into_iter()
                .collect(),
            );
        }
        let source = Arc::new(FixedSelectedRuntimeProfileSource(snapshot.clone()));
        let profiles = Arc::new(CapabilityProfileService::new(
            Arc::new(SqliteCapabilityProfileRepository::open(&database).unwrap()),
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
            AgentSessionEventAdapter::open(
                &database,
                sessions.clone(),
                repository.clone(),
                source.clone(),
                identities,
            )
            .unwrap()
            .with_capability_profiles(profiles.clone()),
        );
        let events = Arc::new(SessionEventApplication::new(
            adapter.clone(),
            adapter.clone(),
            Arc::new(SqliteSessionEventStore::open(&database).unwrap()),
        ));
        let authoring = Arc::new(WorkflowAuthoringService::new(
            Arc::new(SqliteWorkflowAuthoringRepository::open(&database).unwrap()),
            profiles.clone(),
            crate::otp_host::OtpRegistry::import(&["workflow"]).unwrap(),
        ));
        let execution = Arc::new(WorkflowExecutionService::new(
            authoring.clone(),
            events,
            Arc::new(WorkflowInstanceStore::open(&database).unwrap()),
            adapter,
            repository.clone(),
        ));
        *notifier.execution.lock().unwrap() = Some(Arc::downgrade(&execution));
        if mediated {
            let (mut descriptors, owner) = crate::otp_host::mcp::start_server(
                execution.registry.clone(),
                Arc::downgrade(&execution),
            )
            .unwrap();
            let registration = registry.register(descriptors.remove(0)).unwrap();
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
            trigger: otp_output("on_invocation_completed", "completed"),
            prompt_inputs,
            prompt_text: "Review the result".into(),
            action: crate::otp_api::CapabilityRef {
                package: "workflow".into(),
                tool: "prompt_agent".into(),
            },
            configuration: json!({}),
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
        .find(|attempt| attempt.context.source.is_some())
        .unwrap()
        .event_groups
        .first()
        .unwrap();
    let records = store.deliveries_for_group(group).unwrap();
    assert!(records[0].prompt_contributions.iter().any(|source| matches!(source, crate::session_events::PromptSource::ReferencedContent { reference, text } if reference.kind() == "prompt_input" && text == "Review the result")));
    assert!(records[0].included_created_session_contributions.iter().any(|source| matches!(source, crate::session_events::PromptSource::ReferencedContent { reference, text } if reference.kind() == "initial_prompt" && text == "Initial for b")));
}

#[test]
fn invalid_source_pairs_and_escaping_file_references_do_not_launch_receiver() {
    let fixture = Fixture::new();
    let instance = fixture.instance(None);
    let mut draft = instance.recipe.clone();
    draft.connections[0].prompt_inputs = vec![WorkflowConnectionPromptInput::OutputField {
        field: "promptText".into(),
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
        "workflow".into(),
        ["handoff_to_agent".into()].into_iter().collect(),
    )]
    .into_iter()
    .collect();
    draft.nodes[0].node_profile.allowed_capabilities.mcp_tools = tools;
    draft.connections[0].trigger = otp_output("handoff_to_agent", "handoff");
    draft.connections[0].prompt_inputs = vec![
        WorkflowConnectionPromptInput::OutputField {
            field: "promptText".into(),
        },
        WorkflowConnectionPromptInput::OutputField {
            field: "filePaths".into(),
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
    assert!(response["result"]["content"][0]["text"]
        .as_str()
        .unwrap()
        .contains("1 delivery"));
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
        "Review this\n\n[\n  \"plan.md\"\n]\n\nReview the result"
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

fn otp_output(tool: &str, output: &str) -> crate::otp_api::OutputRef {
    crate::otp_api::OutputRef {
        capability: crate::otp_api::CapabilityRef {
            package: "workflow".into(),
            tool: tool.into(),
        },
        output: output.into(),
    }
}

#[test]
fn otp_node_handles_and_new_exact_requests_use_one_dispatcher() {
    use crate::otp_api::*;
    let fixture = Fixture::new();
    let instance = fixture.instance(None);
    let mut context = InvocationContext {
        instance_id: instance.id.clone(),
        occurrence_id: "fresh-one".into(),
        capability: instance.recipe.entry_action.clone(),
        source: None,
        connection_id: Some("a-to-b".into()),
        output_node_id: Some("b".into()),
    };
    let host = crate::otp_host::workflow::WorkflowHost {
        execution: &fixture.execution,
        instance: &instance,
        context: &context,
    };
    let node = host.node(&context, "b").unwrap();
    assert_eq!(node.initial_prompt.as_deref(), Some("Initial for b"));
    assert_eq!(
        node.configuration["capabilityProfileId"],
        "test-capabilities"
    );
    let connection = host.connection(&context).unwrap().unwrap();
    assert_eq!(connection.id, "a-to-b");
    assert_eq!(connection.configuration["destinationNodeId"], "b");
    assert!(host.sessions(&context, "a").is_err());
    let mut forged = context.clone();
    forged.instance_id = "foreign".into();
    assert!(host.node(&forged, "b").is_err());
    let run = |context: &InvocationContext, config| {
        fixture
            .execution
            .dispatch_otp_action(
                &instance,
                context,
                None,
                json!({}),
                config,
                Ok(vec![ResolvedInput {
                    reference: context.occurrence_id.clone(),
                    value: json!("Review"),
                }]),
            )
            .unwrap()
    };
    let first = run(&context, json!({"mode":"new"}));
    context.occurrence_id = "fresh-two".into();
    let second = run(&context, json!({"mode":"new"}));
    assert_ne!(
        first[0].group.created_session,
        second[0].group.created_session
    );
    context.occurrence_id = "select-one".into();
    let exact = run(&context, json!({}));
    assert_eq!(
        exact[0].deliveries[0].target_session,
        second[0].deliveries[0].target_session
    );
    assert!(exact[0].group.created_session.is_none());
    context.occurrence_id = "fresh-three".into();
    run(&context, json!({"mode":"new"}));
    assert_eq!(
        fixture
            .execution
            .instance_sessions(&instance)
            .unwrap()
            .len(),
        3
    );
    let launches = fixture.launches();
    assert_eq!(launches.len(), 4);
    for index in [0, 1, 3] {
        assert_eq!(
            launches[index]
                .launch_extension
                .as_ref()
                .unwrap()
                .initial_prompt_prefix
                .as_ref()
                .unwrap()
                .content,
            "Initial for b"
        )
    }
    assert!(launches[2]
        .launch_extension
        .as_ref()
        .is_none_or(|e| e.initial_prompt_prefix.is_none()));
    assert!(run(&context, json!({"mode":"new"})).is_empty());
    assert_eq!(
        fixture.launches().len(),
        4,
        "an occurrence is not dispatched twice"
    );
    let attempts = fixture.execution.instances.attempts(&instance.id).unwrap();
    assert_eq!(attempts.len(), 4);
    assert!(attempts
        .iter()
        .all(|a| a.session_requests.len() == 1 && a.event_groups.len() == 1));
    assert!(matches!(
        attempts
            .iter()
            .find(|a| a.context.occurrence_id == "select-one")
            .unwrap()
            .session_requests[0]
            .target,
        SessionRequestTarget::Exact { .. }
    ));
}

#[test]
fn otp_selection_keeps_busy_session_failure_visible_without_relaunch() {
    use crate::otp_api::*;
    let fixture = Fixture::with_runtime(RuntimeBehavior::StayRunning, false);
    let instance = fixture.instance(None);
    let mut context = InvocationContext {
        instance_id: instance.id.clone(),
        occurrence_id: "new".into(),
        capability: instance.recipe.entry_action.clone(),
        source: None,
        connection_id: None,
        output_node_id: Some("b".into()),
    };
    let run = |ctx: &InvocationContext| {
        fixture
            .execution
            .dispatch_otp_action(
                &instance,
                ctx,
                None,
                json!({}),
                json!({}),
                Ok(vec![ResolvedInput {
                    reference: ctx.occurrence_id.clone(),
                    value: json!("Review"),
                }]),
            )
            .unwrap()
    };
    run(&context);
    context.occurrence_id = "busy".into();
    let failed = run(&context);
    assert!(matches!(
        failed[0].deliveries[0].outcome,
        crate::session_events::DeliveryOutcome::Failed { .. }
    ));
    assert_eq!(fixture.launches().len(), 1);
    assert_eq!(
        fixture.execution.instances.attempts(&instance.id).unwrap()[0]
            .event_groups
            .len(),
        1
    );
}

#[test]
fn otp_compilation_requires_imported_outputs_actions_and_valid_configuration() {
    let fixture = Fixture::new();
    let instance = fixture.instance(None);
    let plan = fixture
        .execution
        .compile_instance(&instance.id, None)
        .unwrap();
    let empty = crate::otp_host::OtpRegistry::import(&[]).unwrap();
    assert!(
        crate::workflows::compiler::WorkflowCompiler::compile(plan.clone(), &empty)
            .unwrap_err()
            .contains("not imported")
    );
    let check = |plan| {
        crate::workflows::compiler::WorkflowCompiler::compile(plan, &fixture.execution.registry)
    };
    let mut invalid = plan.clone();
    invalid.connections[0].trigger.output = "absent".into();
    assert!(check(invalid).is_err());
    let mut invalid = plan.clone();
    invalid.connections[0].action = invalid.connections[0].trigger.capability.clone();
    assert!(check(invalid).is_err());
    let mut invalid = plan.clone();
    invalid.connections[0].configuration = json!({"mode":"random"});
    assert!(check(invalid).is_err());
    let mut invalid = plan.clone();
    invalid.connections[0].prompt_inputs = vec![WorkflowConnectionPromptInput::OutputField {
        field: "undeclared".into(),
    }];
    assert!(check(invalid).is_err());
    let mut invalid = plan;
    invalid.connections[0].destination_node =
        crate::workflows::address_references::WorkflowNodeReference::new("absent").unwrap();
    assert!(check(invalid).is_err());
}

#[test]
fn unsupported_recipes_and_instances_remain_stored_but_are_not_executed() {
    let fixture = Fixture::new();
    let instance = fixture.instance(None);
    let database = rusqlite::Connection::open(fixture.folder.path().join("repair.sqlite")).unwrap();
    let recipe = r#"{"contractVersion":1,"old":"recipe bytes"}"#;
    let record = r#"{"recipe":{"contractVersion":1},"old":"instance bytes"}"#;
    database
        .execute(
            "UPDATE workflow_recipe_authoring SET draft_json=?1,active_json=?1 WHERE recipe_id=?2",
            rusqlite::params![recipe, instance.recipe.recipe_id],
        )
        .unwrap();
    database
        .execute(
            "UPDATE workflow_recipe_instances SET record_json=?1 WHERE id=?2",
            rusqlite::params![record, instance.id],
        )
        .unwrap();
    assert!(fixture.authoring.list().unwrap().is_empty());
    assert!(fixture.execution.instances.list().unwrap().is_empty());
    assert!(fixture
        .authoring
        .load(&instance.recipe.recipe_id)
        .unwrap_err()
        .contains("Unsupported"));
    assert!(fixture
        .execution
        .instances
        .load(&instance.id)
        .unwrap_err()
        .contains("Unsupported"));
    assert_eq!(
        database
            .query_row(
                "SELECT record_json FROM workflow_recipe_instances",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        record
    );
    assert_eq!(
        database
            .query_row(
                "SELECT draft_json FROM workflow_recipe_authoring",
                [],
                |r| r.get::<_, String>(0)
            )
            .unwrap(),
        recipe
    );
}
