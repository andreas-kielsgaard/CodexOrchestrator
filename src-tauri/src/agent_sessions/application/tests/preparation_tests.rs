use super::*;
use crate::agent_sessions::application::preparation::PreparedMessageInput;
use crate::agent_sessions::domain::ExternalRuntimeContextId;
use crate::agent_sessions::ports::RuntimeInvocationReady;
use crate::agent_sessions::preparation::{PreparationPhase, SessionPreparation};
use crate::execution_configuration::{
    CapabilityProfileService, InMemoryCapabilityProfileRepository, NativeCapabilityInventory,
};
use crate::execution_targets::{domain::*, endpoints::ExecutionEndpoints, ExecutionTargetService};
use std::{sync::Condvar, time::Instant};

#[derive(Default)]
struct BlockingPreparationRuntime {
    state: Mutex<BlockedState>,
    changed: Condvar,
}
#[derive(Default)]
struct BlockedState {
    requests: Vec<RuntimeInvocationRequest>,
    outcome: Option<bool>,
    delivered: Vec<AgentInvocationId>,
    canceled: Vec<AgentInvocationId>,
    sink: Option<Arc<dyn AgentRuntimeUpdateSink>>,
}
impl BlockingPreparationRuntime {
    fn release(&self, succeeds: bool) {
        self.state.lock().unwrap().outcome = Some(succeeds);
        self.changed.notify_all();
    }
    fn attempts(&self) -> usize {
        self.state.lock().unwrap().requests.len()
    }
}
impl AgentRuntime for BlockingPreparationRuntime {
    fn preflight_invocation(
        &self,
        _: RuntimeInvocationMode,
        options: &AgentRuntimeOptions,
    ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
        Ok(RuntimeInvocationPreflight {
            effective_options: options.clone(),
        })
    }
    fn prepare_invocation(
        &self,
        request: RuntimeInvocationRequest,
        external: Option<ExternalRuntimeContextId>,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<RuntimeInvocationReady, RuntimePortError> {
        let mut state = self.state.lock().unwrap();
        state.requests.push(request.clone());
        state.sink = Some(sink);
        self.changed.notify_all();
        while state.outcome.is_none() {
            state = self.changed.wait(state).unwrap();
        }
        if !state.outcome.take().unwrap() {
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::Unavailable,
                "native readiness failed",
            ));
        }
        // Return late successful readiness even after cancellation; application must refuse delivery.
        Ok(RuntimeInvocationReady {
            external_context_id: external
                .unwrap_or_else(|| ExternalRuntimeContextId::new("native-prepared").unwrap()),
            working_directory: request.working_directory.unwrap(),
        })
    }
    fn deliver_prepared_invocation(&self, id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        let sink = {
            let mut state = self.state.lock().unwrap();
            state.delivered.push(id.clone());
            state.sink.clone().unwrap()
        };
        sink.emit_update(
            id,
            RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                status: AgentInvocationTerminalStatus::Completed,
                exit_code: None,
                signal: None,
                runtime_error: None,
            }),
        )
    }
    fn cancel_invocation(&self, id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        let mut state = self.state.lock().unwrap();
        state.canceled.push(id.clone());
        state.outcome = Some(true);
        self.changed.notify_all();
        Ok(())
    }
    fn start_invocation(
        &self,
        _: RuntimeInvocationRequest,
        _: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        panic!("Preparation must not use immediate start")
    }
    fn resume_invocation(
        &self,
        _: RuntimeInvocationRequest,
        _: ExternalRuntimeContextId,
        _: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        panic!("Preparation must not use immediate resume")
    }
}
struct PreparationProfileSource;
impl ProviderConfigurationSource for PreparationProfileSource {
    fn profile_for_configuration(
        &self,
        _reference: &str,
        _cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError> {
        Ok(test_selected_runtime_profile())
    }
    fn inventory_for_configuration(
        &self,
        _reference: &str,
        _cwd: Option<&str>,
    ) -> Result<NativeCapabilityInventory, ProviderConfigurationSourceError> {
        Ok(Default::default())
    }
}
struct PreparationFixture {
    _directory: tempfile::TempDir,
    app: AgentSessionApplication,
    repository: Arc<SqliteAgentSessionRepository>,
    runtime: Arc<BlockingPreparationRuntime>,
    old_runtime: Arc<FakeRuntime>,
    old_target: SessionExecutionTarget,
    selection: SessionExecutionSelection,
    session: AgentSession,
}
impl PreparationFixture {
    fn new() -> Self {
        let directory = tempfile::tempdir().unwrap();
        let database = Arc::new(
            crate::persistence::ActiveDatabase::open(
                directory.path().join("sessions.sqlite"),
                crate::storage::initialize_active_database,
            )
            .unwrap(),
        );
        let repository = Arc::new(SqliteAgentSessionRepository::from_database(
            database.clone(),
        ));
        let source = Arc::new(PreparationProfileSource);
        let runtime = Arc::new(BlockingPreparationRuntime::default());
        let old_runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::SpawnFailure));
        let old_execution = ExecutionBinding {
            device_id: "old-device".into(),
            device_name: "Previous server".into(),
            provider: "codex".into(),
            configuration_ref: "codex-default".into(),
            connection: ExecutionConnection::Ssh {
                target: "unused-test-host".into(),
                host_executable: "/opt/orchid-host".into(),
            },
        };
        let endpoints = Arc::new(
            ExecutionEndpoints::new(crate::runtime::providers::registrations::ProviderRegistrations::single("codex", source.clone(), runtime.clone()))
                .with_runtime(&old_execution, old_runtime.clone()),
        );
        let profiles = Arc::new(
            CapabilityProfileService::new(Arc::new(InMemoryCapabilityProfileRepository::default())).with_configuration_source(source.clone())
            .with_endpoints(endpoints.clone()),
        );
        let profile = profiles
            .create_with_defaults(
                "laptop".into(),
                "Laptop".into(),
                test_selected_runtime_profile().exposure,
                RuntimeSelections {
                    model: Some("node-default".into()),
                    reasoning_mode: Some("high".into()),
                    ..Default::default()
                },
            )
            .unwrap();
        let old_target = SessionExecutionTarget {
            capability_profile_id: "previous-profile".into(),
            capability_profile_revision: 1,
            execution: old_execution,
            repository_id: String::new(),
            branch_ref: String::new(),
            worktree_id: "old-instance".into(),
            path: "/previous/worktree".into(),
            head: None,
        };
        let destination = directory.path().join("destination");
        std::fs::create_dir(&destination).unwrap();
        let target = SessionExecutionTarget {
            capability_profile_id: profile.capability_profile_id.clone(),
            capability_profile_revision: profile.revision,
            execution: profile.execution.clone(),
            repository_id: String::new(),
            branch_ref: String::new(),
            worktree_id: "destination-instance".into(),
            path: destination.to_string_lossy().into_owned(),
            head: None,
        };
        let selection = SessionExecutionSelection {
            capability_profile_id: profile.capability_profile_id,
            capability_profile_revision: profile.revision,
            execution: profile.execution,
            workspace: SessionWorkspaceSelection::Existing { target },
        };
        let providers = Arc::new(DeterministicProviders::default());
        let targets = Arc::new(ExecutionTargetService::new(
            database,
            endpoints.clone(),
            profiles.clone(),
        ));
        let app = AgentSessionApplication::new(
            repository.clone(),
            runtime.clone(),
            Arc::new(RecordingNotifier::new(repository.clone())),
            providers.clone(),
            providers,
            None,
        )
        .with_capability_profiles(profiles)
        .with_profile_source(source)
        .with_execution_endpoints(endpoints)
        .with_execution_target_service(targets);
        let session = app
            .create_session_with_ownership(
                CreateAgentSessionCommand {
                    title: Some("Prepared session".into()),
                    working_directory: None,
                    requested_options: Default::default(),
                },
                AgentSessionOwnership {
                    execution_target: Some(old_target.clone()),
                    ..Default::default()
                },
            )
            .unwrap();
        Self {
            _directory: directory,
            app,
            repository,
            runtime,
            old_runtime,
            old_target,
            selection,
            session,
        }
    }
    fn input(&self, id: &str) -> PreparedMessageInput {
        PreparedMessageInput {
            session_id: Some(self.session.id.clone()),
            submission_id: AgentInvocationId::new(id).unwrap(),
            submitted_text: "Frozen submitted text".into(),
            title: None,
            working_directory: None,
            execution_selection: Some(self.selection.clone()),
            model: Some("user-only".into()),
            reasoning_mode: Some("medium".into()),
            sandbox_mode: None,
            folder_target: None,
        }
    }
    fn preparation(&self, id: &AgentInvocationId) -> SessionPreparation {
        self.repository.preparation(id).unwrap().unwrap()
    }
    fn wait(&self, mut condition: impl FnMut() -> bool) {
        let deadline = Instant::now() + std::time::Duration::from_secs(5);
        while !condition() {
            assert!(
                Instant::now() < deadline,
                "Preparation did not reach expected state; latest={:?}",
                self.app.load_preparation(&self.session.id).unwrap()
            );
            std::thread::sleep(std::time::Duration::from_millis(5));
        }
    }
}
impl Drop for PreparationFixture {
    fn drop(&mut self) {
        self.runtime.release(false);
    }
}

#[test]
fn prepared_ack_is_durable_while_native_setup_is_blocked_and_submission_is_frozen() {
    let fixture = PreparationFixture::new();
    let mut input = fixture.input("accepted");
    let ack = fixture.app.accept_prepared_message(input.clone()).unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 1);
    let invocation = fixture
        .repository
        .get_invocation(&ack.invocation_id)
        .unwrap()
        .unwrap();
    assert_eq!(invocation.status, AgentInvocationStatus::Pending);
    assert_eq!(invocation.submitted_text, "Frozen submitted text");
    assert_eq!(
        fixture
            .app
            .load_session(&ack.session_id)
            .unwrap()
            .session
            .execution_target,
        Some(fixture.old_target.clone())
    );
    assert!(fixture.runtime.state.lock().unwrap().delivered.is_empty());
    input.submitted_text = "Next draft changed".into();
    input.model = Some("node-default".into());
    input.reasoning_mode = Some("high".into());
    assert!(fixture.app.accept_prepared_message(input).is_err());
    let requests = fixture.runtime.state.lock().unwrap().requests.clone();
    assert_eq!(requests[0].options.model.as_deref(), Some("user-only"));
    assert_eq!(
        requests[0]
            .launch_extension
            .as_ref()
            .unwrap()
            .reasoning_mode
            .as_deref(),
        Some("medium")
    );
    assert_eq!(requests[0].submitted_text, "Frozen submitted text");
    fixture.runtime.release(true);
    fixture.wait(|| fixture.preparation(&ack.invocation_id).delivery_started);
    fixture.wait(|| {
        fixture
            .repository
            .get_invocation(&ack.invocation_id)
            .unwrap()
            .unwrap()
            .status
            .is_terminal()
    });
    assert_eq!(
        fixture.runtime.state.lock().unwrap().delivered,
        vec![ack.invocation_id]
    );
}

#[test]
fn native_failure_preserves_previous_binding_and_retry_reuses_resolved_workspace() {
    let fixture = PreparationFixture::new();
    let ack = fixture
        .app
        .accept_prepared_message(fixture.input("retry"))
        .unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 1);
    let target = fixture
        .preparation(&ack.invocation_id)
        .resolved_target
        .unwrap();
    fixture.runtime.release(false);
    fixture.wait(|| fixture.preparation(&ack.invocation_id).phase == PreparationPhase::Failed);
    fixture.wait(|| {
        fixture
            .repository
            .get_invocation(&ack.invocation_id)
            .unwrap()
            .unwrap()
            .status
            .is_terminal()
    });
    assert_eq!(
        fixture
            .app
            .load_session(&ack.session_id)
            .unwrap()
            .session
            .execution_target,
        Some(fixture.old_target.clone())
    );
    assert!(fixture.preparation(&ack.invocation_id).can_retry);
    assert!(fixture.runtime.state.lock().unwrap().delivered.is_empty());
    // The accepted destination stays fixed when the editable selection changes elsewhere.
    fixture.runtime.state.lock().unwrap().outcome = None;
    let deadline = Instant::now() + std::time::Duration::from_secs(5);
    loop {
        match fixture.app.retry_preparation(&ack.invocation_id) {
            Ok(()) => break,
            Err(error) => {
                assert!(Instant::now() < deadline, "retry stayed blocked: {error}");
                std::thread::sleep(std::time::Duration::from_millis(5));
            }
        }
    }
    fixture.wait(|| fixture.runtime.attempts() == 2);
    assert_eq!(
        fixture.preparation(&ack.invocation_id).resolved_target,
        Some(target.clone())
    );
    let paths = fixture
        .runtime
        .state
        .lock()
        .unwrap()
        .requests
        .iter()
        .map(|r| r.working_directory.clone())
        .collect::<Vec<_>>();
    assert_eq!(paths, vec![Some(target.path.clone()), Some(target.path)]);
    fixture.runtime.release(true);
    fixture.wait(|| fixture.preparation(&ack.invocation_id).delivery_started);
    fixture.wait(|| {
        fixture
            .repository
            .get_invocation(&ack.invocation_id)
            .unwrap()
            .unwrap()
            .status
            .is_terminal()
    });
    assert_eq!(fixture.runtime.state.lock().unwrap().delivered.len(), 1);
}

#[test]
fn cancel_routes_to_preparing_destination_and_late_readiness_cannot_deliver() {
    let fixture = PreparationFixture::new();
    let ack = fixture
        .app
        .accept_prepared_message(fixture.input("cancel"))
        .unwrap();
    fixture.wait(|| fixture.runtime.attempts() == 1);
    fixture.app.cancel_preparation(&ack.invocation_id).unwrap();
    fixture.wait(|| fixture.preparation(&ack.invocation_id).phase == PreparationPhase::Canceled);
    assert!(fixture.runtime.state.lock().unwrap().delivered.is_empty());
    assert!(fixture
        .runtime
        .state
        .lock()
        .unwrap()
        .canceled
        .contains(&ack.invocation_id));
    assert!(fixture.old_runtime.calls.lock().unwrap().is_empty());
    assert_eq!(
        fixture
            .app
            .load_session(&ack.session_id)
            .unwrap()
            .session
            .execution_target,
        Some(fixture.old_target.clone())
    );
}

mod persistence;
