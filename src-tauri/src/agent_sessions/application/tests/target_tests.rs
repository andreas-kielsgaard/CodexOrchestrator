use super::*;
use crate::execution_configuration::{
    CapabilityProfileService, InMemoryCapabilityProfileRepository, NativeCapabilityInventory,
};
use crate::execution_targets::{domain::*, endpoints::ExecutionEndpoints};

fn remote_target() -> SessionExecutionTarget {
    SessionExecutionTarget {
        capability_profile_id: "server".into(),
        capability_profile_revision: 1,
        execution: ExecutionBinding {
            device_id: "server".into(),
            device_name: "Server".into(),
            provider: "codex".into(),
            configuration_ref: "codex-default".into(),
            connection: ExecutionConnection::Ssh {
                target: "unused-test-host".into(),
                host_executable: "/opt/orchid-host".into(),
            },
        },
        repository_id: "repository-1".into(),
        branch_ref: "refs/heads/main".into(),
        worktree_id: "remote-worktree".into(),
        path: "/root/repositories/project/worktrees/main".into(),
        head: Some("published-commit".into()),
    }
}

#[test]
fn remote_target_preserves_linux_path_and_routes_continuation_without_local_launch_authority() {
    let harness = Harness::new(RuntimeBehavior::SpawnFailure);
    let remote = Arc::new(FakeRuntime::new(RuntimeBehavior::CompleteWithBinding));
    let source = Arc::new(FixedSelectedRuntimeProfileSource(
        test_selected_runtime_profile(),
    ));
    let target = remote_target();
    let endpoints = Arc::new(
        ExecutionEndpoints::new("codex", source, harness.runtime.clone())
            .unwrap()
            .with_runtime(&target.execution, remote.clone()),
    );
    let folder = tempfile::tempdir().unwrap();
    let app = harness
        .application
        .with_execution_endpoints(endpoints)
        .with_workspaces(
            super::super::SessionWorkspaces::new(folder.path().to_path_buf(), "test".into())
                .unwrap(),
        )
        .with_native_profile_launch_authority(Arc::new(RejectingProfileAuthority));
    let session = app
        .create_session_with_ownership(
            CreateAgentSessionCommand {
                title: None,
                working_directory: None,
                requested_options: Default::default(),
            },
            AgentSessionOwnership {
                execution_target: Some(target.clone()),
                ..Default::default()
            },
        )
        .unwrap();
    assert_eq!(
        session.working_directory.as_deref(),
        Some(target.path.as_str())
    );
    assert_eq!(session.workspace_origin.as_deref(), Some("explicit"));
    assert_eq!(
        app.load_quick_features(Some(&session.id), None, None)
            .unwrap_err(),
        "Native quick-feature discovery is unavailable for remote sessions."
    );
    app.send_message(message(&session.id, "First remote turn"))
        .unwrap();
    app.send_message(message(&session.id, "Continue remotely"))
        .unwrap();
    let stored = app.load_session(&session.id).unwrap();
    assert_eq!(stored.session.execution_target, Some(target.clone()));
    assert_eq!(stored.invocations.len(), 2);
    assert!(harness.runtime.calls.lock().unwrap().is_empty());
    let calls = remote.calls.lock().unwrap();
    assert!(calls.iter().any(|call| matches!(call, RuntimeCall::Start(request) if request.working_directory.as_deref() == Some(target.path.as_str()) && request.launch_extension.is_none())));
    assert!(calls.iter().any(|call| matches!(call, RuntimeCall::Resume(request, external) if external == "codex-thread-1" && request.working_directory.as_deref() == Some(target.path.as_str()))));
    assert!(app
        .resolve_working_directory(&session.id, folder.path().to_string_lossy().into_owned())
        .is_err());
}

#[test]
fn cancellation_uses_the_bound_remote_runtime() {
    let harness = Harness::new(RuntimeBehavior::SpawnFailure);
    let remote = Arc::new(FakeRuntime::new(RuntimeBehavior::StayRunning));
    let target = remote_target();
    let endpoints = Arc::new(
        ExecutionEndpoints::new(
            "codex",
            Arc::new(FixedSelectedRuntimeProfileSource(
                test_selected_runtime_profile(),
            )),
            harness.runtime.clone(),
        )
        .unwrap()
        .with_runtime(&target.execution, remote.clone()),
    );
    let app = harness.application.with_execution_endpoints(endpoints);
    let session = app
        .create_session_with_ownership(
            CreateAgentSessionCommand {
                title: None,
                working_directory: None,
                requested_options: Default::default(),
            },
            AgentSessionOwnership {
                execution_target: Some(target),
                ..Default::default()
            },
        )
        .unwrap();
    let sent = app
        .send_message(message(&session.id, "Run remotely"))
        .unwrap();
    let canceled = app
        .cancel_invocation(CancelAgentInvocationCommand {
            invocation_id: sent.invocation_id.clone(),
        })
        .unwrap();
    assert_eq!(canceled.status, AgentInvocationStatus::Canceled);
    assert!(harness.runtime.calls.lock().unwrap().is_empty());
    assert!(remote
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|call| matches!(call, RuntimeCall::Cancel(id) if id == &sent.invocation_id)));
}

#[derive(Default)]
struct ConfiguredSource(Mutex<Vec<String>>);
impl SelectedRuntimeProfileSource for ConfiguredSource {
    fn quick_features_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<
        crate::execution_configuration::RuntimeQuickFeatures,
        SelectedRuntimeProfileSourceError,
    > {
        self.0
            .lock()
            .unwrap()
            .push(format!("quick:{reference}:{}", cwd.unwrap_or_default()));
        Ok(crate::execution_configuration::RuntimeQuickFeatures {
            profile_ref: test_selected_runtime_profile().profile_ref,
            defaults: RuntimeSelections {
                model: Some("runtime-default".into()),
                reasoning_mode: Some("runtime-effort".into()),
                ..Default::default()
            },
            ..Default::default()
        })
    }
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        Ok(test_selected_runtime_profile())
    }
    fn resolve_configuration_ref(
        &self,
        _: &str,
    ) -> Result<String, SelectedRuntimeProfileSourceError> {
        Ok("frozen-native-home".into())
    }
    fn profile_for_configuration(
        &self,
        reference: &str,
        _: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        self.0.lock().unwrap().push(reference.into());
        self.selected_runtime_profile()
    }
    fn native_inventory(
        &self,
    ) -> Result<NativeCapabilityInventory, SelectedRuntimeProfileSourceError> {
        Ok(Default::default())
    }
}

#[test]
fn profiled_first_send_freezes_target_and_followup_uses_its_endpoint() {
    let harness = Harness::new(RuntimeBehavior::SpawnFailure);
    let selected_runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::CompleteWithBinding));
    let source = Arc::new(ConfiguredSource::default());
    let endpoints = Arc::new(ExecutionEndpoints::new(
        "codex",
        source.clone(),
        selected_runtime.clone(),
    ).unwrap());
    let profiles = Arc::new(
        CapabilityProfileService::new(
            Arc::new(InMemoryCapabilityProfileRepository::default()),
            source.clone(),
        )
        .with_endpoints(endpoints.clone()),
    );
    let saved = profiles
        .create_with_defaults(
            "laptop".into(),
            "Laptop".into(),
            test_session_creation_request()
                .capability_profile
                .allowed_capabilities,
            RuntimeSelections {
                model: Some("node-default".into()),
                reasoning_mode: Some("high".into()),
                ..Default::default()
            },
        )
        .unwrap();
    let folder = tempfile::tempdir().unwrap();
    let target = SessionExecutionTarget {
        capability_profile_id: saved.capability_profile_id.clone(),
        capability_profile_revision: saved.revision,
        execution: saved.execution,
        repository_id: "repo".into(),
        branch_ref: "refs/heads/main".into(),
        worktree_id: "worktree".into(),
        path: folder.path().to_string_lossy().into_owned(),
        head: None,
    };
    let app = harness
        .application
        .with_capability_profiles(profiles.clone())
        .with_execution_endpoints(endpoints)
        .with_profile_source(source.clone());
    let preview = app
        .load_quick_features(None, Some("unrelated-cwd"), Some(&target))
        .unwrap();
    assert_eq!(preview.defaults.model.as_deref(), Some("node-default"));
    assert_eq!(preview.defaults.reasoning_mode.as_deref(), Some("high"));
    assert_eq!(
        source.0.lock().unwrap().last().unwrap(),
        &format!("quick:selected:{}", target.path)
    );
    let first = app
        .start_direct_user_session_with_target(
            "Start on target".into(),
            None,
            None,
            None,
            None,
            None,
            Some(target),
            None,
        )
        .unwrap();
    let session_id = first.acknowledgement.session_id;
    let pinned = app.load_session(&session_id).unwrap().session;
    assert_eq!(
        pinned
            .execution_target
            .as_ref()
            .unwrap()
            .execution
            .configuration_ref,
        "frozen-native-home"
    );
    profiles
        .update_with_defaults(
            "laptop",
            "Renamed profile".into(),
            saved.allowed_capabilities,
            RuntimeSelections {
                model: Some("user-only".into()),
                reasoning_mode: Some("medium".into()),
                ..Default::default()
            },
        )
        .unwrap();
    app.send_direct_user_message(SendDirectUserAgentSessionMessageCommand {
        session_id: session_id.clone(),
        submitted_text: "Continue".into(),
        model: None,
        reasoning_mode: None,
        sandbox_mode: None,
    })
    .unwrap();
    assert_eq!(
        app.load_session(&session_id)
            .unwrap()
            .session
            .execution_target,
        pinned.execution_target
    );
    assert!(harness.runtime.calls.lock().unwrap().is_empty());
    assert_eq!(
        source.0.lock().unwrap().last().map(String::as_str),
        Some("frozen-native-home")
    );
    assert!(selected_runtime.calls.lock().unwrap().iter().any(
        |call| matches!(call, RuntimeCall::Resume(_, external) if external == "codex-thread-1")
    ));
    let quick = app
        .load_quick_features(Some(&session_id), Some("unrelated-cwd"), None)
        .unwrap();
    assert_eq!(quick.defaults.model.as_deref(), Some("node-default"));
    assert_eq!(quick.defaults.reasoning_mode.as_deref(), Some("high"));
    assert_eq!(
        source.0.lock().unwrap().last().unwrap(),
        &format!(
            "quick:frozen-native-home:{}",
            pinned.working_directory.unwrap()
        )
    );
}

#[test]
fn workflow_creation_rejects_remote_profile_without_local_discovery() {
    let mut creation = test_session_creation_request();
    creation.capability_profile.execution = remote_target().execution;
    let result = crate::execution_configuration::SessionProfileResolver::resolve_creation(
        &ConfiguredSource::default(),
        creation,
    );
    assert!(result
        .unwrap_err()
        .to_string()
        .contains("Workflow execution is local-only"));
}
