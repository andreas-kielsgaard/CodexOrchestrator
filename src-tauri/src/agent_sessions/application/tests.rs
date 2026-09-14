use super::{
    AgentSessionApplication, AgentSessionClock, AgentSessionIdProvider, AgentSessionNotification,
    AgentSessionNotifier, AgentSessionOwnership, ApplicationInvocationLaunchEvidence,
    CancelAgentInvocationCommand, CreateAgentSessionCommand, NativeProfileLaunchAuthority,
    SendAgentSessionMessageCommand, SendIdempotentApplicationAgentSessionMessageCommand,
    SessionHarnessLaunchAuthority, SessionHarnessVersionResolver, UpdateAgentSessionHarnessCommand,
    UpdateAgentSessionIdentityCommand, UpdateAgentSessionModelOverrideCommand,
};
mod repair_tests;

#[test]
fn addressed_creation_retry_uses_its_original_native_evidence() {
    struct OneReadSource(AtomicU64);
    impl SelectedRuntimeProfileSource for OneReadSource {
        fn selected_runtime_profile(
            &self,
        ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
            if self.0.fetch_add(1, Ordering::SeqCst) > 0 {
                return Err(SelectedRuntimeProfileSourceError::unavailable(
                    "native discovery changed",
                ));
            }
            Ok(test_selected_runtime_profile())
        }
    }
    let directory = tempfile::tempdir().unwrap();
    let repository = Arc::new(
        SqliteAgentSessionRepository::new(
            crate::storage::open_active_database(&directory.path().join("retry.sqlite")).unwrap(),
        )
        .unwrap(),
    );
    let providers = Arc::new(DeterministicProviders::default());
    let source = Arc::new(OneReadSource(AtomicU64::new(0)));
    let application = AgentSessionApplication::new(
        repository.clone(),
        Arc::new(FakeRuntime::new(RuntimeBehavior::CompleteWithBinding)),
        Arc::new(RecordingNotifier::new(repository.clone())),
        providers.clone(),
        providers,
        None,
    )
    .with_profile_source(source.clone());
    let creation = test_session_creation_request();
    let request = crate::session_events::SessionCreationSpec {
        logical_address: SessionLogicalAddress::new(
            reference("workflow_instance", "run"),
            reference("workflow_node", "node"),
        ),
        event_group_id: reference("event_group", "create"),
        created_by_event: reference("user_request", "create"),
        created_by_session: None,
        configuration: SessionCreationConfiguration {
            contract: reference("session_creation_request", "v1"),
            payload: serde_json::to_value(&creation).unwrap(),
            assigned_identity: None,
        },
    };
    let id = AgentSessionId::new("retry-session").unwrap();
    let first = application
        .create_addressed_profiled_session(
            repository.as_ref(),
            request.clone(),
            id.clone(),
            creation.clone(),
            None,
            None,
            None,
        )
        .unwrap();
    let retry = application
        .create_addressed_profiled_session(
            repository.as_ref(),
            request,
            id,
            creation,
            None,
            None,
            None,
        )
        .unwrap();
    assert_eq!(first, retry);
    assert_eq!(source.0.load(Ordering::SeqCst), 1);
}
use super::configuration::{
    LoadPinnedSessionProfileQuery, SendDirectUserAgentSessionMessageCommand,
    SessionConfigurationErrorKind,
};
use crate::agent_sessions::{
    domain::{
        AgentDiagnostic, AgentInvocation, AgentInvocationId, AgentInvocationInputProvenance,
        AgentInvocationStatus, AgentInvocationTerminalStatus, AgentRuntimeBinding,
        AgentRuntimeEvent, AgentRuntimeEventId, AgentRuntimeEventSource, AgentRuntimeFailure,
        AgentRuntimeOptions, AgentSession, AgentSessionAvailability, AgentSessionId,
        InvocationCompletion, NormalizedRuntimeEvent, NormalizedRuntimeEventKind,
        RuntimeSandboxMode,
    },
    ports::{
        AgentRuntime, AgentRuntimeUpdateSink, AgentSessionHistory, AgentSessionRepository,
        AgentSessionSummary, ListAgentSessionsQuery, RepositoryError, RepositoryErrorKind,
        RuntimeEventDraft, RuntimeInvocationMode, RuntimeInvocationOutcome,
        RuntimeInvocationPreflight, RuntimeInvocationRequest, RuntimeLaunchExtension,
        RuntimePortError, RuntimePortErrorKind, RuntimeUpdate, RuntimeUpdateDeliveryFailure,
    },
    repository::{SqliteAgentSessionRepository, AGENT_SESSION_SCHEMA},
};
use crate::{
    agent_sessions::session_event_adapter::AgentSessionEventAdapter,
    execution_configuration::{
        CapabilityProfile, CapabilitySet, NodeProfile, RuntimeProfileSnapshot, RuntimeSelections,
        SandboxMode as ExecutionSandboxMode, SelectedRuntimeProfileSource,
        SelectedRuntimeProfileSourceError, SessionCreationRequest,
    },
    harness_engine::domain::{HarnessId, HarnessVersionNumber, HarnessVersionRef},
    identities::{service::IdentityService, AssignedAgentIdentity, IdentityId, IdentityShape},
    session_events::{
        MissingTargetPolicy, PromptSource, ReferenceIdentity, RunningFilter,
        SessionCreationConfiguration, SessionEventApplication, SessionEventCommand,
        SessionEventQueryApplication, SessionEventSource, SessionEventTrigger,
        SessionLogicalAddress, SessionTarget, SqliteSessionEventStore, TargetCardinality,
        TargetOrdering, TargetSelection,
    },
};
use chrono::{DateTime, Duration, TimeZone, Utc};
use rusqlite::Connection;
use serde_json::json;
use std::sync::{
    atomic::{AtomicBool, AtomicU64, Ordering},
    Arc, Mutex,
};

#[test]
fn pinned_profile_query_and_direct_user_message_preserve_session_configuration() {
    let directory = tempfile::tempdir().expect("temporary directory");
    let database_path = directory.path().join("session-events.sqlite");
    let identities = IdentityService::open(&database_path).expect("Identity service");
    let repository = Arc::new(
        SqliteAgentSessionRepository::open(&database_path).expect("Agent Session repository"),
    );
    let runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::CompleteWithBinding));
    let notifier = Arc::new(RecordingNotifier::new(repository.clone()));
    let providers = Arc::new(DeterministicProviders::default());
    let application = Arc::new(AgentSessionApplication::new(
        repository.clone(),
        runtime.clone(),
        notifier,
        providers.clone(),
        providers,
        Some("codex-test".into()),
    ));
    let runtime_profile = test_selected_runtime_profile();
    let profile_source = Arc::new(FixedSelectedRuntimeProfileSource(runtime_profile.clone()));
    let assigned_definition = identities
        .create("Avery".into(), "#39745a".into(), IdentityShape::Circle)
        .expect("Identity definition");
    let adapter = Arc::new(AgentSessionEventAdapter::new(
        application.clone(),
        repository,
        profile_source.clone(),
        identities,
    ));
    let event_store =
        Arc::new(SqliteSessionEventStore::open(&database_path).expect("Session Event store"));
    let event_queries = SessionEventQueryApplication::new(event_store.clone());
    let events = SessionEventApplication::new(adapter.clone(), adapter.clone(), event_store);
    let profile_application = application
        .as_ref()
        .clone()
        .with_profile_source(profile_source);
    let logical_address = SessionLogicalAddress::new(
        reference("workflow_instance", "run-1"),
        reference("workflow_node", "review"),
    );
    let event_group_id = reference("event_group", "create-review");
    let user_request = reference("user_request", "request-review");
    let created = events
        .dispatch(SessionEventCommand {
            event_group_id: event_group_id.clone(),
            definition_ref: reference("event_definition", "review-user-entry"),
            trigger: SessionEventTrigger::UserRequest {
                request: user_request.clone(),
            },
            source: SessionEventSource::UserRequest {
                request: user_request.clone(),
            },
            prompt_sources: vec![PromptSource::UserRequestText {
                request: user_request,
                text: "Review this plan".into(),
            }],
            created_session_prompt_sources: vec![PromptSource::literal(
                "You are the plan reviewer.",
            )],
            creation_configuration: Some(SessionCreationConfiguration {
                contract: ReferenceIdentity::new(
                    "orchestrator.execution_configuration",
                    "session_creation_request",
                    "v1",
                )
                .unwrap(),
                payload: serde_json::to_value(test_session_creation_request())
                    .expect("creation request"),
                assigned_identity: Some(
                    ReferenceIdentity::new(
                        "orchestrator.identities",
                        "identity",
                        assigned_definition.definition.id.as_str(),
                    )
                    .unwrap(),
                ),
            }),
            target: TargetSelection {
                target: SessionTarget::Logical {
                    address: logical_address.clone(),
                },
                cardinality: TargetCardinality::First,
                ordering: TargetOrdering::Newest,
                running: RunningFilter::Any,
                created_by: None,
                missing: MissingTargetPolicy::Create,
            },
            created_by_session: None,
            direct_user_options: None,
        })
        .expect("creation Session Event");
    assert_eq!(created.deliveries.len(), 1);
    assert_eq!(
        created.group.created_session,
        Some(created.deliveries[0].target_session.clone())
    );
    let created_session = created.deliveries[0].target_session.clone();
    let session_id = AgentSessionId::new(created_session.id()).unwrap();
    let persisted = application.load_session(&session_id).unwrap().session;
    assert_eq!(
        persisted
            .assigned_identity
            .as_ref()
            .map(|identity| identity.display_name.as_str()),
        Some("Avery")
    );
    let pinned = persisted.session_profile.as_ref().expect("pinned profile");
    assert_eq!(
        pinned.session_profile().node_capabilities().models,
        ["node-default".to_string()].into_iter().collect()
    );
    assert_eq!(
        pinned.session_profile().attached_runtime_capabilities(),
        &runtime_profile.exposure
    );
    let pinned_before_message = pinned.clone();

    let follow_up_group = reference("event_group", "workflow-delivery");
    let application_event = reference("application_event", "plan-ready");
    let follow_up = events
        .dispatch(SessionEventCommand {
            event_group_id: follow_up_group.clone(),
            definition_ref: reference("event_definition", "review-follow-up"),
            trigger: SessionEventTrigger::ApplicationEvent {
                event: application_event.clone(),
            },
            source: SessionEventSource::ApplicationEvent {
                event: application_event.clone(),
            },
            prompt_sources: vec![PromptSource::ApplicationEventField {
                event: application_event,
                field: "plan".into(),
                text: "Review the completed plan".into(),
            }],
            created_session_prompt_sources: Vec::new(),
            creation_configuration: None,
            target: TargetSelection {
                target: SessionTarget::Logical {
                    address: logical_address,
                },
                cardinality: TargetCardinality::First,
                ordering: TargetOrdering::Newest,
                running: RunningFilter::Any,
                created_by: None,
                missing: MissingTargetPolicy::Fail,
            },
            created_by_session: None,
            direct_user_options: None,
        })
        .expect("existing Session Event");
    assert_eq!(follow_up.group.created_session, None);
    assert_eq!(follow_up.deliveries[0].target_session, created_session);
    assert!(follow_up.deliveries[0]
        .included_created_session_contributions
        .is_empty());
    assert_eq!(
        event_queries
            .recorded_event(&event_group_id)
            .unwrap()
            .unwrap(),
        created
    );
    assert_eq!(
        event_queries
            .deliveries_for_session(&created_session)
            .unwrap()
            .len(),
        2
    );

    let queried = profile_application
        .load_pinned_session_profile(LoadPinnedSessionProfileQuery {
            session_id: session_id.clone(),
        })
        .expect("query pinned Session Profile");
    assert_eq!(queried.session_id, session_id);
    assert_eq!(queried.creation_resolution, pinned_before_message);

    let direct_user = profile_application
        .send_direct_user_message(SendDirectUserAgentSessionMessageCommand {
            session_id: session_id.clone(),
            submitted_text: "User prompt".into(),
            model: Some("user-only".into()),
            reasoning_mode: Some("medium".into()),
            sandbox_mode: None,
        })
        .expect("direct-user message");
    assert_eq!(
        direct_user
            .invocation_resolution
            .selections
            .model
            .as_deref(),
        Some("user-only")
    );
    assert_eq!(
        direct_user
            .invocation_resolution
            .selections
            .reasoning_mode
            .as_deref(),
        Some("medium")
    );
    let invalid = profile_application
        .send_direct_user_message(SendDirectUserAgentSessionMessageCommand {
            session_id: session_id.clone(),
            submitted_text: "Invalid selection must not launch".into(),
            model: Some("not-exposed".into()),
            reasoning_mode: None,
            sandbox_mode: None,
        })
        .expect_err("unavailable direct-user model must fail");
    assert_eq!(
        invalid.kind,
        SessionConfigurationErrorKind::InvalidInvocationSelection
    );

    let calls = runtime.calls.lock().unwrap();
    let launches = calls
        .iter()
        .filter_map(|call| match call {
            RuntimeCall::Start(request) | RuntimeCall::Resume(request, _) => Some(request),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(launches.len(), 3);
    assert_eq!(
        launches[0]
            .launch_extension
            .as_ref()
            .and_then(|extension| extension.initial_prompt_prefix.as_ref())
            .map(|prefix| prefix.content.as_str()),
        Some("You are the plan reviewer.")
    );
    assert!(launches[1]
        .launch_extension
        .as_ref()
        .is_none_or(|extension| extension.initial_prompt_prefix.is_none()));
    assert!(launches[2]
        .launch_extension
        .as_ref()
        .is_some_and(|extension| extension.reasoning_mode.as_deref() == Some("medium")));
    drop(calls);
    let history = application.load_session(&session_id).unwrap();
    assert_eq!(
        history.session.session_profile.as_ref(),
        Some(&pinned_before_message)
    );
    assert_eq!(history.invocations.len(), 3);
    assert_eq!(
        history.invocations[0]
            .invocation
            .requested_options
            .model
            .as_deref(),
        Some("node-default")
    );
    assert_eq!(
        history.invocations[2]
            .invocation
            .requested_options
            .model
            .as_deref(),
        Some("user-only")
    );

    Connection::open(&database_path)
        .unwrap()
        .execute(
            "UPDATE agent_sessions SET session_profile_json=json_set(session_profile_json, '$.digest', 'tampered') WHERE id=?1",
            [session_id.as_str()],
        )
        .unwrap();
    assert!(application.load_session(&session_id).is_err());
}

#[derive(Clone)]
struct FixedSelectedRuntimeProfileSource(RuntimeProfileSnapshot);

impl SelectedRuntimeProfileSource for FixedSelectedRuntimeProfileSource {
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        Ok(self.0.clone())
    }
}

fn test_selected_runtime_profile() -> RuntimeProfileSnapshot {
    RuntimeProfileSnapshot {
        contract_version: 1,
        profile_ref: "native-codex:selected".into(),
        exposure: CapabilitySet {
            models: ["node-default".to_string(), "user-only".to_string()]
                .into_iter()
                .collect(),
            reasoning_modes: ["high".to_string(), "medium".to_string()]
                .into_iter()
                .collect(),
            sandbox_modes: [ExecutionSandboxMode::WorkspaceWrite].into_iter().collect(),
            ..CapabilitySet::default()
        },
        locked: RuntimeSelections {
            model: None,
            reasoning_mode: None,
            sandbox_mode: Some(ExecutionSandboxMode::WorkspaceWrite),
        },
    }
}

fn test_session_creation_request() -> SessionCreationRequest {
    let runtime = test_selected_runtime_profile();
    let node_capabilities = CapabilitySet {
        models: ["node-default".to_string()].into_iter().collect(),
        reasoning_modes: ["high".to_string()].into_iter().collect(),
        sandbox_modes: [ExecutionSandboxMode::WorkspaceWrite].into_iter().collect(),
        ..CapabilitySet::default()
    };
    SessionCreationRequest {
        contract_version: 1,
        capability_profile: CapabilityProfile {
            contract_version: 1,
            defaults: Default::default(),
            capability_profile_id: "test-capabilities".into(),
            name: "Test capabilities".into(),
            revision: 1,
            allowed_capabilities: runtime.exposure,
        },
        node_profile: NodeProfile {
            contract_version: 1,
            allowed_capabilities: node_capabilities,
            pinned_defaults: RuntimeSelections {
                model: Some("node-default".into()),
                reasoning_mode: Some("high".into()),
                sandbox_mode: None,
            },
        },
    }
}

fn reference(kind: &str, id: &str) -> ReferenceIdentity {
    ReferenceIdentity::new("test", kind, id).unwrap()
}

#[test]
fn application_creates_and_updates_session_owned_harness_and_identity() {
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(AGENT_SESSION_SCHEMA)
        .expect("schema");
    let repository = Arc::new(SqliteAgentSessionRepository::new(connection).expect("repository"));
    let runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::StayRunning));
    let notifier = Arc::new(RecordingNotifier::new(repository.clone()));
    let providers = Arc::new(DeterministicProviders::default());
    let application = AgentSessionApplication::new(
        repository,
        runtime,
        notifier,
        providers.clone(),
        providers,
        Some("codex-test".into()),
    );
    let first_ref = HarnessVersionRef::new(
        HarnessId::new("planning").unwrap(),
        HarnessVersionNumber::new(1).unwrap(),
    );
    let first_identity = AssignedAgentIdentity::new(
        Some(IdentityId::new("avery").unwrap()),
        "Avery",
        "#39745a",
        IdentityShape::Circle,
    )
    .unwrap();
    let session = application
        .create_session_with_ownership(
            CreateAgentSessionCommand {
                title: Some("Owned session".into()),
                working_directory: None,
                requested_options: AgentRuntimeOptions {
                    model: Some("session-model".into()),
                    sandbox: None,
                },
            },
            AgentSessionOwnership {
                harness_version: Some(first_ref),
                assigned_identity: Some(first_identity),
                session_profile: None,
            },
        )
        .expect("create owned Session");
    assert_eq!(
        session.requested_options.model.as_deref(),
        Some("session-model")
    );

    let replacement = HarnessVersionRef::new(
        HarnessId::new("planning").unwrap(),
        HarnessVersionNumber::new(2).unwrap(),
    );
    let migrated = application
        .update_session_harness(UpdateAgentSessionHarnessCommand {
            session_id: session.id.clone(),
            harness_version: Some(replacement.clone()),
        })
        .expect("update Harness reference");
    assert_eq!(migrated.harness_version, Some(replacement));
    let cleared = application
        .update_session_identity(UpdateAgentSessionIdentityCommand {
            session_id: session.id,
            assigned_identity: None,
        })
        .expect("clear Session identity");
    assert!(cleared.assigned_identity.is_none());
}

#[test]
fn application_updates_and_clears_the_session_model_without_changing_sandbox() {
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(AGENT_SESSION_SCHEMA)
        .expect("schema");
    let repository = Arc::new(SqliteAgentSessionRepository::new(connection).expect("repository"));
    let runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::StayRunning));
    let notifier = Arc::new(RecordingNotifier::new(repository.clone()));
    let providers = Arc::new(DeterministicProviders::default());
    let application = AgentSessionApplication::new(
        repository,
        runtime,
        notifier,
        providers.clone(),
        providers,
        None,
    );
    let session = application
        .create_session(CreateAgentSessionCommand {
            title: None,
            working_directory: None,
            requested_options: AgentRuntimeOptions {
                model: None,
                sandbox: Some(RuntimeSandboxMode::WorkspaceWrite),
            },
        })
        .unwrap();

    let updated = application
        .update_session_model_override(UpdateAgentSessionModelOverrideCommand {
            session_id: session.id.clone(),
            model: Some("  user-selected-model  ".into()),
        })
        .unwrap();
    assert_eq!(
        updated.requested_options.model.as_deref(),
        Some("user-selected-model")
    );
    assert_eq!(
        updated.requested_options.sandbox,
        Some(RuntimeSandboxMode::WorkspaceWrite)
    );

    let error = application
        .update_session_model_override(UpdateAgentSessionModelOverrideCommand {
            session_id: session.id.clone(),
            model: Some("  ".into()),
        })
        .expect_err("blank override is not a model identifier");
    assert!(error.to_string().contains("cannot be blank"));

    let cleared = application
        .update_session_model_override(UpdateAgentSessionModelOverrideCommand {
            session_id: session.id,
            model: None,
        })
        .unwrap();
    assert!(cleared.requested_options.model.is_none());
    assert_eq!(
        cleared.requested_options.sandbox,
        Some(RuntimeSandboxMode::WorkspaceWrite)
    );
}

#[test]
fn unchanged_owned_harness_is_resolved_before_launch_without_rewriting_the_reference() {
    let requested = harness_version("planning", 1);
    let resolver = Arc::new(RecordingHarnessVersionResolver::succeed(requested.clone()));
    let (application, repository, runtime, native_authority, harness_authority) =
        harness_resolution_application(resolver.clone());
    let session = create_harness_owned_session(&application, requested.clone());

    application
        .send_message(message(&session.id, "resolve current Harness"))
        .expect("launch with current Harness");

    assert_eq!(
        resolver.requests.lock().unwrap().as_slice(),
        &[(session.id.clone(), requested.clone())]
    );
    assert_eq!(
        repository
            .get_session(&session.id)
            .unwrap()
            .unwrap()
            .harness_version,
        Some(requested.clone())
    );
    assert_eq!(native_authority.calls.lock().unwrap().len(), 1);
    assert_eq!(
        native_authority
            .observed_harness_versions
            .lock()
            .unwrap()
            .as_slice(),
        &[Some(requested)]
    );
    assert_eq!(harness_authority.invocations.lock().unwrap().len(), 1);
    assert!(runtime
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|call| matches!(call, RuntimeCall::Preflight(_))));
}

#[test]
fn replacement_resolution_is_persisted_before_launch_preparation() {
    let requested = harness_version("planning", 1);
    let replacement = harness_version("planning", 2);
    let resolver = Arc::new(RecordingHarnessVersionResolver::succeed(
        replacement.clone(),
    ));
    let (application, repository, runtime, native_authority, harness_authority) =
        harness_resolution_application(resolver);
    let session = create_harness_owned_session(&application, requested);

    application
        .send_message(message(&session.id, "migrate then launch"))
        .expect("launch with replacement Harness");

    assert_eq!(
        repository
            .get_session(&session.id)
            .unwrap()
            .unwrap()
            .harness_version,
        Some(replacement.clone())
    );
    assert_eq!(native_authority.calls.lock().unwrap().len(), 1);
    assert_eq!(
        native_authority
            .observed_harness_versions
            .lock()
            .unwrap()
            .as_slice(),
        &[Some(replacement)]
    );
    assert_eq!(harness_authority.invocations.lock().unwrap().len(), 1);
    assert!(runtime
        .calls
        .lock()
        .unwrap()
        .iter()
        .any(|call| matches!(call, RuntimeCall::Start(_))));
}

#[test]
fn harness_resolution_failure_is_a_durable_preflight_failure_before_launch_preparation() {
    let requested = harness_version("planning", 1);
    let resolver = Arc::new(RecordingHarnessVersionResolver::fail(
        "replacement chain is unavailable",
    ));
    let (application, repository, runtime, native_authority, harness_authority) =
        harness_resolution_application(resolver);
    let session = create_harness_owned_session(&application, requested.clone());

    let result = application
        .send_message(message(&session.id, "must not launch"))
        .expect("resolution failure is terminalized");

    let invocation = repository
        .get_invocation(&result.invocation_id)
        .unwrap()
        .unwrap();
    assert_eq!(invocation.status, AgentInvocationStatus::Failed);
    assert_eq!(
        invocation
            .runtime_error
            .as_ref()
            .map(|error| error.code.as_str()),
        Some("runtime_preflight_failed")
    );
    assert_eq!(
        repository
            .get_session(&session.id)
            .unwrap()
            .unwrap()
            .harness_version,
        Some(requested)
    );
    assert!(native_authority.calls.lock().unwrap().is_empty());
    assert!(harness_authority.invocations.lock().unwrap().is_empty());
    assert!(runtime.calls.lock().unwrap().is_empty());
}

#[test]
fn session_harness_authority_is_consulted_for_every_fresh_and_resumed_invocation() {
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(AGENT_SESSION_SCHEMA)
        .expect("schema");
    let repository = Arc::new(SqliteAgentSessionRepository::new(connection).expect("repository"));
    let runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::CompleteWithBinding));
    let notifier = Arc::new(RecordingNotifier::new(repository.clone()));
    let providers = Arc::new(DeterministicProviders::default());
    let authority = Arc::new(RecordingSessionHarnessAuthority::default());
    let application = AgentSessionApplication::new(
        repository,
        runtime.clone(),
        notifier,
        providers.clone(),
        providers,
        Some("codex-test".into()),
    )
    .with_session_harness_launch_authority(authority.clone());
    let session = application
        .create_session(CreateAgentSessionCommand {
            title: None,
            working_directory: None,
            requested_options: AgentRuntimeOptions::default(),
        })
        .unwrap();

    application
        .send_message(message(&session.id, "fresh"))
        .unwrap();
    application
        .send_message(message(&session.id, "resume"))
        .unwrap();

    assert_eq!(authority.invocations.lock().unwrap().len(), 2);
    let requests = runtime
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter_map(|call| match call {
            RuntimeCall::Start(request) | RuntimeCall::Resume(request, _) => Some(request.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(requests.len(), 2);
    assert!(requests
        .iter()
        .all(|request| request
            .launch_extension
            .as_ref()
            .is_some_and(|extension| extension
                .config_overrides
                .contains(&"mcp_servers={}".to_string()))));
}

#[test]
fn managed_profile_authority_prepares_fresh_and_resume_launches_without_replacing_role_environment()
{
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(AGENT_SESSION_SCHEMA)
        .expect("schema");
    let repository = Arc::new(SqliteAgentSessionRepository::new(connection).expect("repository"));
    let runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::CompleteWithBinding));
    let notifier = Arc::new(RecordingNotifier::new(repository.clone()));
    let providers = Arc::new(DeterministicProviders::default());
    let authority = Arc::new(RecordingProfileAuthority::default());
    let application = AgentSessionApplication::new(
        repository,
        runtime.clone(),
        notifier,
        providers.clone(),
        providers,
        Some("codex-test".into()),
    )
    .with_native_profile_launch_authority(authority.clone());
    let session = application
        .create_session(CreateAgentSessionCommand {
            title: None,
            working_directory: None,
            requested_options: AgentRuntimeOptions::default(),
        })
        .expect("session");

    application
        .send_message_with_launch_extension(
            message(&session.id, "fresh"),
            Some(RuntimeLaunchExtension {
                managed_mcp_servers: Vec::new(),
                skill_roots: Vec::new(),
                ignore_user_rules: false,
                reasoning_mode: None,
                config_overrides: vec![],
                environment: vec![("ROLE_CONFIG".into(), "present".into())],
                initial_prompt_prefix: None,
            }),
        )
        .expect("fresh launch");
    application
        .send_message(message(&session.id, "resume"))
        .expect("resume launch");

    assert_eq!(
        *authority.calls.lock().expect("authority calls"),
        vec![false, true]
    );
    let requests = runtime
        .calls
        .lock()
        .expect("runtime calls")
        .iter()
        .filter_map(|call| match call {
            RuntimeCall::Start(request) | RuntimeCall::Resume(request, _) => Some(request.clone()),
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(requests.len(), 2);
    assert!(requests
        .iter()
        .all(|request| request
            .launch_extension
            .as_ref()
            .is_some_and(|extension| extension
                .environment
                .iter()
                .any(|(key, value)| key == "CODEX_HOME" && value == "native-home"))));
    assert!(requests[0]
        .launch_extension
        .as_ref()
        .unwrap()
        .environment
        .iter()
        .any(|(key, value)| key == "ROLE_CONFIG" && value == "present"));
}

#[test]
fn managed_profile_authority_failure_is_durable_and_prevents_provider_preflight_and_spawn() {
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(AGENT_SESSION_SCHEMA)
        .expect("schema");
    let repository = Arc::new(SqliteAgentSessionRepository::new(connection).expect("repository"));
    let runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::StayRunning));
    let notifier = Arc::new(RecordingNotifier::new(repository.clone()));
    let providers = Arc::new(DeterministicProviders::default());
    let application = AgentSessionApplication::new(
        repository.clone(),
        runtime.clone(),
        notifier,
        providers.clone(),
        providers,
        None,
    )
    .with_native_profile_launch_authority(Arc::new(RejectingProfileAuthority));
    let session = application
        .create_session(CreateAgentSessionCommand {
            title: None,
            working_directory: None,
            requested_options: AgentRuntimeOptions::default(),
        })
        .expect("session");

    let result = application
        .send_message(message(&session.id, "must not launch"))
        .expect("durable failure");
    let invocation = repository
        .get_invocation(&result.invocation_id)
        .expect("read invocation")
        .expect("invocation");
    assert_eq!(invocation.status, AgentInvocationStatus::Failed);
    assert_eq!(
        invocation
            .runtime_error
            .as_ref()
            .map(|error| error.code.as_str()),
        Some("runtime_preflight_failed")
    );
    assert!(runtime.calls.lock().expect("runtime calls").is_empty());
}

#[test]
fn first_turn_captures_binding_and_second_turn_resumes_with_effective_options() {
    let harness = Harness::new(RuntimeBehavior::CompleteWithBinding);
    let session = harness.create_session();

    let first = harness
        .application
        .send_message(message(&session.id, "First"))
        .expect("first send");
    let loaded = harness
        .application
        .load_session(&session.id)
        .expect("load first");
    assert_eq!(loaded.session.id, session.id);
    assert_eq!(
        loaded
            .session
            .runtime_binding
            .external_context_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("codex-thread-1")
    );
    assert_eq!(
        loaded.invocations[0].invocation.status,
        AgentInvocationStatus::Completed
    );

    let second = harness
        .application
        .send_message(message(&session.id, "Second"))
        .expect("resume send");
    assert_ne!(first.invocation_id, second.invocation_id);
    let calls = harness.runtime.calls.lock().expect("runtime calls");
    assert!(matches!(
        calls[0],
        RuntimeCall::Preflight(RuntimeInvocationMode::Start)
    ));
    assert!(matches!(calls[1], RuntimeCall::Start(_)));
    assert!(matches!(
        calls[2],
        RuntimeCall::Preflight(RuntimeInvocationMode::Resume)
    ));
    assert!(matches!(
        &calls[3],
        RuntimeCall::Resume(_, external) if external == "codex-thread-1"
    ));
    for request in calls.iter().filter_map(|call| match call {
        RuntimeCall::Start(request) | RuntimeCall::Resume(request, _) => Some(request),
        _ => None,
    }) {
        assert_eq!(request.options.model.as_deref(), Some("confirmed-model"));
        assert_eq!(
            request.options.sandbox,
            Some(RuntimeSandboxMode::WorkspaceWrite)
        );
    }
    assert!(harness
        .notifier
        .persisted_before_notify
        .load(Ordering::Acquire));
}

#[test]
fn fast_concurrent_updates_are_sequenced_persisted_and_then_notified() {
    let harness = Harness::new(RuntimeBehavior::ConcurrentFastCompletion);
    let session = harness.create_session();
    let result = harness
        .application
        .send_message(message(&session.id, "Concurrent"))
        .expect("send");
    let events = harness
        .repository
        .list_events(&result.invocation_id)
        .expect("events");

    assert_eq!(events.len(), 2);
    assert_eq!(
        events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![0, 1]
    );
    assert_eq!(
        harness
            .repository
            .get_invocation(&result.invocation_id)
            .expect("invocation")
            .expect("present")
            .status,
        AgentInvocationStatus::Completed
    );
    assert!(harness
        .notifier
        .persisted_before_notify
        .load(Ordering::Acquire));
}

#[test]
fn missed_notifications_reload_from_durable_history_and_record_delivery_diagnostics() {
    let harness = Harness::new(RuntimeBehavior::CompleteWithBinding);
    harness.notifier.fail.store(true, Ordering::Release);
    let session = harness.create_session();
    let result = harness
        .application
        .send_message(message(&session.id, "Miss every event"))
        .expect("send remains acknowledged");

    let history = harness
        .application
        .load_session(&session.id)
        .expect("durable reload");
    assert_eq!(
        history.invocations[0].invocation.status,
        AgentInvocationStatus::Completed
    );
    assert!(!history.invocations[0].events.is_empty());
    let persisted = harness
        .repository
        .get_invocation(&result.invocation_id)
        .expect("invocation")
        .expect("present");
    assert!(persisted.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "runtime_update_delivery_failed"
            && diagnostic
                .details
                .as_ref()
                .is_some_and(|details| details["failedUpdate"]["kind"] == "event")
    }));
    assert_eq!(persisted.runtime_error, None);
    assert_eq!(
        harness
            .application
            .reconcile_startup()
            .expect("already terminal remains idempotent"),
        0
    );
}

#[test]
fn spawn_failure_callback_owns_the_single_terminal_outcome_despite_launch_error_return() {
    let harness = Harness::new(RuntimeBehavior::SpawnFailure);
    let session = harness.create_session();
    let result = harness
        .application
        .send_message(message(&session.id, "Fail spawn"))
        .expect("durable acknowledgement");
    let invocation = harness
        .repository
        .get_invocation(&result.invocation_id)
        .expect("invocation")
        .expect("present");

    assert_eq!(invocation.status, AgentInvocationStatus::Failed);
    assert_eq!(harness.application.update_lane_count(), 0);
    assert_eq!(
        invocation
            .runtime_error
            .as_ref()
            .map(|error| error.code.as_str()),
        Some("spawn")
    );
    assert!(invocation
        .diagnostics
        .iter()
        .any(|diagnostic| diagnostic.code == "runtime_launch_returned_error"));
    assert_eq!(
        harness
            .notifier
            .notifications
            .lock()
            .expect("notifications")
            .iter()
            .filter(|notification| matches!(
                notification,
                AgentSessionNotification::InvocationTerminal { .. }
            ))
            .count(),
        1
    );
}

#[test]
fn launch_error_without_terminal_callback_is_durably_failed_once() {
    let harness = Harness::new(RuntimeBehavior::LaunchErrorWithoutCallback);
    let session = harness.create_session();
    let result = harness
        .application
        .send_message(message(&session.id, "Fail before child"))
        .expect("durable acknowledgement");
    let invocation = harness
        .repository
        .get_invocation(&result.invocation_id)
        .expect("invocation")
        .expect("present");

    assert_eq!(invocation.status, AgentInvocationStatus::Failed);
    assert_eq!(harness.application.update_lane_count(), 0);
    assert_eq!(
        invocation
            .runtime_error
            .as_ref()
            .map(|error| error.code.as_str()),
        Some("runtime_launch_failed")
    );
    assert_eq!(
        harness
            .notifier
            .notifications
            .lock()
            .expect("notifications")
            .iter()
            .filter(|notification| matches!(
                notification,
                AgentSessionNotification::InvocationTerminal { .. }
            ))
            .count(),
        1
    );
}

#[test]
fn runtime_error_never_records_application_launch_acceptance() {
    let harness = Harness::new(RuntimeBehavior::LaunchErrorWithoutCallback);
    let session = harness.create_session();
    let invocation_id = harness.application.allocate_application_invocation_id();
    let result = harness
        .application
        .send_idempotent_application_message_with_launch_observation(
            SendIdempotentApplicationAgentSessionMessageCommand {
                invocation_id: invocation_id.clone(),
                message: message(&session.id, "Application launch error"),
            },
            None,
        )
        .expect("durable failed acknowledgement");

    assert!(!result.launch_accepted);
    assert_eq!(
        harness
            .application
            .application_invocation_launch_evidence(&invocation_id, &session.id)
            .expect("launch evidence"),
        ApplicationInvocationLaunchEvidence::PersistedNotAccepted
    );
    assert_eq!(
        harness
            .repository
            .invocation_launch_accepted_at(&invocation_id)
            .expect("acceptance lookup"),
        None
    );
}

#[test]
fn runtime_ok_records_durable_application_launch_acceptance() {
    let harness = Harness::new(RuntimeBehavior::StayRunning);
    let session = harness.create_session();
    let invocation_id = harness.application.allocate_application_invocation_id();
    let result = harness
        .application
        .send_idempotent_application_message_with_launch_observation(
            SendIdempotentApplicationAgentSessionMessageCommand {
                invocation_id: invocation_id.clone(),
                message: message(&session.id, "Application launch accepted"),
            },
            None,
        )
        .expect("accepted launch");

    assert!(result.launch_accepted);
    assert_eq!(
        harness
            .application
            .application_invocation_launch_evidence(&invocation_id, &session.id)
            .expect("launch evidence"),
        ApplicationInvocationLaunchEvidence::LaunchAccepted
    );
    assert!(harness
        .repository
        .invocation_launch_accepted_at(&invocation_id)
        .expect("acceptance lookup")
        .is_some());
}

#[test]
fn prepared_application_invocation_launches_once_without_allocating_a_replacement() {
    let harness = Harness::new(RuntimeBehavior::StayRunning);
    let session = harness.create_session();
    let invocation_id = harness.application.allocate_application_invocation_id();
    let mut prepared_message = message(&session.id, "Prepared application launch");
    prepared_message.working_directory = session.working_directory.clone();
    let command = SendIdempotentApplicationAgentSessionMessageCommand {
        invocation_id: invocation_id.clone(),
        message: prepared_message,
    };

    harness
        .application
        .prepare_idempotent_application_invocation(command.clone())
        .expect("prepare one invocation");
    let first = harness
        .application
        .launch_prepared_application_invocation_with_launch_observation(command.clone(), None)
        .expect("launch prepared invocation");
    let second = harness
        .application
        .launch_prepared_application_invocation_with_launch_observation(command, None)
        .expect("idempotent accepted launch");

    assert!(first.launch_accepted);
    assert!(second.launch_accepted);
    assert_eq!(
        harness
            .repository
            .list_invocations(&session.id)
            .expect("invocations")
            .len(),
        1
    );
    assert_eq!(
        harness
            .runtime
            .calls
            .lock()
            .expect("runtime calls")
            .iter()
            .filter(|call| matches!(call, RuntimeCall::Start(request) if request.invocation_id == invocation_id))
            .count(),
        1
    );
}

#[test]
fn prepared_launch_refuses_to_allocate_a_missing_invocation() {
    let harness = Harness::new(RuntimeBehavior::StayRunning);
    let session = harness.create_session();
    let invocation_id = harness.application.allocate_application_invocation_id();
    let mut missing_message = message(&session.id, "Missing prepared invocation");
    missing_message.working_directory = session.working_directory.clone();

    let result = harness
        .application
        .launch_prepared_application_invocation_with_launch_observation(
            SendIdempotentApplicationAgentSessionMessageCommand {
                invocation_id: invocation_id.clone(),
                message: missing_message,
            },
            None,
        );
    let error = match result {
        Ok(_) => panic!("missing invocation must not be created at launch time"),
        Err(error) => error,
    };

    assert!(error
        .to_string()
        .contains("prepared application invocation not found"));
    assert_eq!(
        harness
            .repository
            .get_invocation(&invocation_id)
            .expect("invocation lookup"),
        None
    );
    assert!(harness
        .runtime
        .calls
        .lock()
        .expect("runtime calls")
        .is_empty());
}

#[test]
fn launch_acceptance_persistence_failure_does_not_invent_durable_acceptance() {
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(AGENT_SESSION_SCHEMA)
        .expect("Agent Session schema");
    let inner =
        Arc::new(SqliteAgentSessionRepository::new(connection).expect("Agent Session repository"));
    let repository = Arc::new(FaultInjectingRepository {
        inner: inner.clone(),
        fail_finish: AtomicBool::new(false),
        fail_binding: AtomicBool::new(false),
        fail_launch_acceptance: AtomicBool::new(true),
    });
    let runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::StayRunning));
    let notifier = Arc::new(RecordingNotifier::new(inner.clone()));
    let providers = Arc::new(DeterministicProviders::default());
    let application = AgentSessionApplication::new(
        repository,
        runtime.clone(),
        notifier,
        providers.clone(),
        providers,
        Some("codex-test".to_string()),
    );
    let session = application
        .create_session(CreateAgentSessionCommand {
            title: Some("Acceptance persistence fault".to_string()),
            working_directory: None,
            requested_options: AgentRuntimeOptions::default(),
        })
        .expect("session");
    let invocation_id = application.allocate_application_invocation_id();
    let result = application
        .send_idempotent_application_message_with_launch_observation(
            SendIdempotentApplicationAgentSessionMessageCommand {
                invocation_id: invocation_id.clone(),
                message: message(&session.id, "Accepted externally, marker fails"),
            },
            None,
        )
        .expect("marker persistence failure is terminalized truthfully");

    assert!(!result.launch_accepted);
    let invocation = inner
        .get_invocation(&invocation_id)
        .expect("invocation")
        .expect("persisted");
    assert_eq!(invocation.status, AgentInvocationStatus::Failed);
    assert_eq!(
        invocation
            .runtime_error
            .as_ref()
            .map(|error| error.code.as_str()),
        Some("runtime_launch_acceptance_persistence_failed")
    );
    assert_eq!(
        inner
            .invocation_launch_accepted_at(&invocation_id)
            .expect("acceptance lookup"),
        None
    );
    assert!(runtime.calls.lock().expect("runtime calls").iter().any(
        |call| matches!(call, RuntimeCall::Start(request) if request.invocation_id == invocation_id)
    ));
    assert_eq!(
        application
            .application_invocation_launch_evidence(&invocation_id, &session.id)
            .expect("conservative launch evidence"),
        ApplicationInvocationLaunchEvidence::PersistedNotAccepted
    );
    assert!(runtime
        .calls
        .lock()
        .expect("runtime calls")
        .iter()
        .any(|call| matches!(call, RuntimeCall::Cancel(id) if id == &invocation_id)));
}

#[test]
fn repeated_completion_is_idempotent_and_does_not_duplicate_terminal_notification() {
    let harness = Harness::new(RuntimeBehavior::DuplicateCompletion);
    let session = harness.create_session();
    let result = harness
        .application
        .send_message(message(&session.id, "Complete twice"))
        .expect("send");
    let invocation = harness
        .repository
        .get_invocation(&result.invocation_id)
        .expect("invocation")
        .expect("present");

    assert_eq!(invocation.status, AgentInvocationStatus::Completed);
    assert_eq!(harness.application.update_lane_count(), 0);
    assert_eq!(
        harness
            .notifier
            .notifications
            .lock()
            .expect("notifications")
            .iter()
            .filter(|notification| matches!(
                notification,
                AgentSessionNotification::InvocationTerminal { .. }
            ))
            .count(),
        1
    );
}

#[test]
fn late_event_after_terminal_is_diagnosed_without_append_and_lane_is_removed() {
    let harness = Harness::new(RuntimeBehavior::LateEventAfterCompletion);
    let session = harness.create_session();
    let result = harness
        .application
        .send_message(message(&session.id, "Finish before late output"))
        .expect("send");
    let invocation = harness
        .repository
        .get_invocation(&result.invocation_id)
        .expect("invocation")
        .expect("present");

    assert_eq!(invocation.status, AgentInvocationStatus::Completed);
    assert!(harness
        .repository
        .list_events(&result.invocation_id)
        .expect("events")
        .is_empty());
    assert!(invocation.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "runtime_update_delivery_failed"
            && diagnostic.message == "ignored late runtime event for terminal invocation"
    }));
    assert_eq!(harness.application.update_lane_count(), 0);
}

#[test]
fn cancellation_uses_runtime_and_terminal_persistence_remains_in_update_sink() {
    let harness = Harness::new(RuntimeBehavior::StayRunning);
    let session = harness.create_session();
    let sent = harness
        .application
        .send_message(message(&session.id, "Keep running"))
        .expect("send");
    let canceled = harness
        .application
        .cancel_invocation(CancelAgentInvocationCommand {
            invocation_id: sent.invocation_id.clone(),
        })
        .expect("cancel");

    assert_eq!(canceled.status, AgentInvocationStatus::Canceled);
    assert!(harness
        .runtime
        .calls
        .lock()
        .expect("calls")
        .iter()
        .any(|call| matches!(call, RuntimeCall::Cancel(id) if id == &sent.invocation_id)));
}

#[test]
fn startup_reconciliation_interrupts_pending_and_running_once() {
    let harness = Harness::new(RuntimeBehavior::StayRunning);
    let running_session = harness.create_session();
    let running = harness
        .application
        .send_message(message(&running_session.id, "Running"))
        .expect("running send");
    let pending_session = harness
        .application
        .create_session(CreateAgentSessionCommand {
            title: Some("Pending".to_string()),
            working_directory: None,
            requested_options: AgentRuntimeOptions::default(),
        })
        .expect("pending session");
    let created_at = Utc.with_ymd_and_hms(2026, 7, 10, 12, 0, 0).unwrap();
    let pending = AgentInvocation {
        id: AgentInvocationId::new("pending-manual").expect("ID"),
        session_id: pending_session.id,
        submitted_text: "Pending".to_string(),
        input_provenance: AgentInvocationInputProvenance::User,
        status: AgentInvocationStatus::Pending,
        requested_options: AgentRuntimeOptions::default(),
        effective_options: None,
        started_at: None,
        completed_at: None,
        exit_code: None,
        signal: None,
        runtime_error: None,
        diagnostics: Vec::new(),
        created_at,
        updated_at: created_at,
    };
    harness
        .repository
        .create_pending_invocation(pending.clone())
        .expect("pending invocation");

    assert_eq!(
        harness.application.reconcile_startup().expect("reconcile"),
        2
    );
    assert_eq!(harness.application.reconcile_startup().expect("repeat"), 0);
    for invocation_id in [running.invocation_id, pending.id] {
        assert_eq!(
            harness
                .repository
                .get_invocation(&invocation_id)
                .expect("invocation")
                .expect("present")
                .status,
            AgentInvocationStatus::Interrupted
        );
    }
}

#[test]
fn classified_pre_acceptance_interruption_recovers_the_exact_application_invocation_once() {
    let harness = Harness::new(RuntimeBehavior::StayRunning);
    let session = harness.create_session();
    let invocation_id = harness.application.allocate_application_invocation_id();
    let mut prepared_message = message(&session.id, "Recover exact invocation");
    prepared_message.working_directory = session.working_directory.clone();
    let command = SendIdempotentApplicationAgentSessionMessageCommand {
        invocation_id: invocation_id.clone(),
        message: prepared_message,
    };
    harness
        .application
        .prepare_idempotent_application_invocation(command.clone())
        .expect("prepare exact invocation");
    let started_at = harness
        .repository
        .get_invocation(&invocation_id)
        .expect("read prepared invocation")
        .expect("prepared invocation")
        .created_at
        + Duration::milliseconds(1);
    harness
        .repository
        .mark_invocation_running(
            &invocation_id,
            started_at,
            AgentRuntimeOptions::default(),
            started_at,
        )
        .expect("simulate crash after running persistence");

    assert_eq!(
        harness
            .application
            .reconcile_startup()
            .expect("classify gap"),
        1
    );
    let interrupted = harness
        .repository
        .get_invocation(&invocation_id)
        .expect("read interruption")
        .expect("persisted interruption");
    assert_eq!(interrupted.status, AgentInvocationStatus::Interrupted);
    assert_eq!(
        interrupted
            .runtime_error
            .as_ref()
            .map(|error| error.code.as_str()),
        Some("runtime_startup_without_launch_acceptance")
    );

    harness
        .application
        .recover_pre_acceptance_application_invocation(&invocation_id, &session.id)
        .expect("recover exact invocation");
    let first = harness
        .application
        .launch_prepared_application_invocation_with_launch_observation(command.clone(), None)
        .expect("launch recovered invocation");
    let second = harness
        .application
        .launch_prepared_application_invocation_with_launch_observation(command, None)
        .expect("replay accepted launch");

    assert!(first.launch_accepted && second.launch_accepted);
    assert_eq!(
        harness
            .repository
            .list_invocations(&session.id)
            .expect("exact invocation count")
            .len(),
        1
    );
    assert_eq!(
        harness
            .runtime
            .calls
            .lock()
            .expect("runtime calls")
            .iter()
            .filter(|call| matches!(call, RuntimeCall::Start(request) if request.invocation_id == invocation_id))
            .count(),
        1
    );
}

#[test]
fn preflight_failure_is_durable_and_never_launches() {
    let harness = Harness::new(RuntimeBehavior::PreflightFailure);
    let session = harness.create_session();
    let sent = harness
        .application
        .send_message(message(&session.id, "Unsupported options"))
        .expect("acknowledge durable failed invocation");
    let invocation = harness
        .repository
        .get_invocation(&sent.invocation_id)
        .expect("invocation")
        .expect("present");

    assert_eq!(invocation.status, AgentInvocationStatus::Failed);
    assert_eq!(
        invocation
            .runtime_error
            .as_ref()
            .map(|error| error.code.as_str()),
        Some("runtime_preflight_failed")
    );
    assert_eq!(
        harness.runtime.calls.lock().expect("calls").len(),
        1,
        "preflight failure must not launch"
    );
}

#[test]
fn persistence_failure_retains_known_runtime_success_as_a_delivery_diagnostic() {
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(AGENT_SESSION_SCHEMA)
        .expect("Agent Session schema");
    let inner =
        Arc::new(SqliteAgentSessionRepository::new(connection).expect("Agent Session repository"));
    let repository = Arc::new(FaultInjectingRepository {
        inner: inner.clone(),
        fail_finish: AtomicBool::new(true),
        fail_binding: AtomicBool::new(false),
        fail_launch_acceptance: AtomicBool::new(false),
    });
    let runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::CompleteWithBinding));
    let notifier = Arc::new(RecordingNotifier::new(inner.clone()));
    let providers = Arc::new(DeterministicProviders::default());
    let application = AgentSessionApplication::new(
        repository,
        runtime,
        notifier,
        providers.clone(),
        providers,
        Some("codex-test".to_string()),
    );
    let session = application
        .create_session(CreateAgentSessionCommand {
            title: Some("Persistence fault".to_string()),
            working_directory: None,
            requested_options: AgentRuntimeOptions::default(),
        })
        .expect("session");
    let sent = application
        .send_message(message(&session.id, "Complete successfully"))
        .expect("send acknowledgement");
    let invocation = inner
        .get_invocation(&sent.invocation_id)
        .expect("invocation")
        .expect("present");

    assert_eq!(invocation.status, AgentInvocationStatus::Running);
    assert_eq!(invocation.runtime_error, None);
    assert!(invocation.diagnostics.iter().any(|diagnostic| {
        diagnostic.code == "runtime_update_delivery_failed"
            && diagnostic.details.as_ref().is_some_and(|details| {
                details["failedUpdate"]["kind"] == "finished"
                    && details["failedUpdate"]["payload"]["status"] == "completed"
            })
    }));

    let restarted_runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::StayRunning));
    let restarted_notifier = Arc::new(RecordingNotifier::new(inner.clone()));
    let restarted_providers = Arc::new(DeterministicProviders::default());
    let restarted = AgentSessionApplication::new(
        inner.clone(),
        restarted_runtime,
        restarted_notifier,
        restarted_providers.clone(),
        restarted_providers,
        Some("codex-test".to_string()),
    );
    assert_eq!(restarted.reconcile_startup().expect("terminal replay"), 1);
    assert_eq!(restarted.reconcile_startup().expect("idempotent replay"), 0);
    let replayed = inner
        .get_invocation(&sent.invocation_id)
        .expect("invocation")
        .expect("present");
    assert_eq!(replayed.status, AgentInvocationStatus::Completed);
    assert_eq!(replayed.exit_code, Some(0));
}

#[test]
fn missing_binding_is_repaired_from_durable_context_event_before_resume() {
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(AGENT_SESSION_SCHEMA)
        .expect("Agent Session schema");
    let inner =
        Arc::new(SqliteAgentSessionRepository::new(connection).expect("Agent Session repository"));
    let repository = Arc::new(FaultInjectingRepository {
        inner: inner.clone(),
        fail_finish: AtomicBool::new(false),
        fail_binding: AtomicBool::new(true),
        fail_launch_acceptance: AtomicBool::new(false),
    });
    let runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::CompleteWithBinding));
    let notifier = Arc::new(RecordingNotifier::new(inner.clone()));
    let providers = Arc::new(DeterministicProviders::default());
    let application = AgentSessionApplication::new(
        repository,
        runtime.clone(),
        notifier,
        providers.clone(),
        providers,
        Some("codex-test".to_string()),
    );
    let session = application
        .create_session(CreateAgentSessionCommand {
            title: Some("Binding repair".to_string()),
            working_directory: None,
            requested_options: AgentRuntimeOptions::default(),
        })
        .expect("session");
    application
        .send_message(message(&session.id, "Establish context"))
        .expect("first send");
    assert!(inner
        .get_session(&session.id)
        .expect("session")
        .expect("present")
        .runtime_binding
        .external_context_id
        .is_none());

    application
        .send_message(message(&session.id, "Resume repaired context"))
        .expect("second send");
    assert!(runtime.calls.lock().expect("calls").iter().any(|call| {
        matches!(call, RuntimeCall::Resume(_, external) if external == "codex-thread-1")
    }));
    assert_eq!(
        inner
            .get_session(&session.id)
            .expect("session")
            .expect("present")
            .runtime_binding
            .external_context_id
            .as_ref()
            .map(|id| id.as_str()),
        Some("codex-thread-1")
    );
}

struct Harness {
    application: AgentSessionApplication,
    repository: Arc<SqliteAgentSessionRepository>,
    runtime: Arc<FakeRuntime>,
    notifier: Arc<RecordingNotifier>,
}

impl Harness {
    fn new(behavior: RuntimeBehavior) -> Self {
        let connection = Connection::open_in_memory().expect("memory database");
        connection
            .execute_batch(AGENT_SESSION_SCHEMA)
            .expect("Agent Session schema");
        let repository = Arc::new(
            SqliteAgentSessionRepository::new(connection).expect("Agent Session repository"),
        );
        let runtime = Arc::new(FakeRuntime::new(behavior));
        let notifier = Arc::new(RecordingNotifier::new(repository.clone()));
        let providers = Arc::new(DeterministicProviders::default());
        let application = AgentSessionApplication::new(
            repository.clone(),
            runtime.clone(),
            notifier.clone(),
            providers.clone(),
            providers,
            Some("codex-test".to_string()),
        );
        Self {
            application,
            repository,
            runtime,
            notifier,
        }
    }

    fn create_session(&self) -> crate::agent_sessions::domain::AgentSession {
        self.application
            .create_session(CreateAgentSessionCommand {
                title: Some("Test session".to_string()),
                working_directory: Some("C:/work".to_string()),
                requested_options: AgentRuntimeOptions {
                    model: Some("requested-model".to_string()),
                    sandbox: Some(RuntimeSandboxMode::WorkspaceWrite),
                },
            })
            .expect("create session")
    }
}

fn message(session_id: &AgentSessionId, text: &str) -> SendAgentSessionMessageCommand {
    SendAgentSessionMessageCommand {
        session_id: Some(session_id.clone()),
        submitted_text: text.to_string(),
        title: None,
        working_directory: None,
        requested_options: None,
    }
}

fn harness_version(harness_id: &str, version: u64) -> HarnessVersionRef {
    HarnessVersionRef::new(
        HarnessId::new(harness_id).unwrap(),
        HarnessVersionNumber::new(version).unwrap(),
    )
}

fn create_harness_owned_session(
    application: &AgentSessionApplication,
    harness_version: HarnessVersionRef,
) -> AgentSession {
    application
        .create_session_with_ownership(
            CreateAgentSessionCommand {
                title: Some("Harness-owned Session".into()),
                working_directory: None,
                requested_options: AgentRuntimeOptions::default(),
            },
            AgentSessionOwnership {
                harness_version: Some(harness_version),
                assigned_identity: None,
                session_profile: None,
            },
        )
        .unwrap()
}

fn harness_resolution_application(
    resolver: Arc<dyn SessionHarnessVersionResolver>,
) -> (
    AgentSessionApplication,
    Arc<SqliteAgentSessionRepository>,
    Arc<FakeRuntime>,
    Arc<RecordingProfileAuthority>,
    Arc<RecordingSessionHarnessAuthority>,
) {
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(AGENT_SESSION_SCHEMA)
        .expect("schema");
    let repository = Arc::new(SqliteAgentSessionRepository::new(connection).unwrap());
    let runtime = Arc::new(FakeRuntime::new(RuntimeBehavior::StayRunning));
    let notifier = Arc::new(RecordingNotifier::new(repository.clone()));
    let providers = Arc::new(DeterministicProviders::default());
    let native_authority = Arc::new(RecordingProfileAuthority::observing(repository.clone()));
    let harness_authority = Arc::new(RecordingSessionHarnessAuthority::default());
    let application = AgentSessionApplication::new(
        repository.clone(),
        runtime.clone(),
        notifier,
        providers.clone(),
        providers,
        Some("codex-test".into()),
    )
    .with_session_harness_version_resolver(resolver)
    .with_native_profile_launch_authority(native_authority.clone())
    .with_session_harness_launch_authority(harness_authority.clone());
    (
        application,
        repository,
        runtime,
        native_authority,
        harness_authority,
    )
}

#[derive(Clone, Copy)]
enum RuntimeBehavior {
    CompleteWithBinding,
    CompleteWithFinalOutput,
    ConcurrentFastCompletion,
    SpawnFailure,
    LaunchErrorWithoutCallback,
    DuplicateCompletion,
    LateEventAfterCompletion,
    StayRunning,
    PreflightFailure,
}

#[derive(Clone, Debug)]
enum RuntimeCall {
    Preflight(RuntimeInvocationMode),
    Start(RuntimeInvocationRequest),
    Resume(RuntimeInvocationRequest, String),
    Cancel(AgentInvocationId),
    Steer(AgentInvocationId, String),
}

#[derive(Default)]
struct RecordingProfileAuthority {
    calls: Mutex<Vec<bool>>,
    repository: Option<Arc<SqliteAgentSessionRepository>>,
    observed_harness_versions: Mutex<Vec<Option<HarnessVersionRef>>>,
}

impl RecordingProfileAuthority {
    fn observing(repository: Arc<SqliteAgentSessionRepository>) -> Self {
        Self {
            repository: Some(repository),
            ..Self::default()
        }
    }
}

struct RecordingHarnessVersionResolver {
    result: Result<HarnessVersionRef, String>,
    requests: Mutex<Vec<(AgentSessionId, HarnessVersionRef)>>,
}

impl RecordingHarnessVersionResolver {
    fn succeed(reference: HarnessVersionRef) -> Self {
        Self {
            result: Ok(reference),
            requests: Mutex::new(Vec::new()),
        }
    }

    fn fail(message: &str) -> Self {
        Self {
            result: Err(message.into()),
            requests: Mutex::new(Vec::new()),
        }
    }
}

impl SessionHarnessVersionResolver for RecordingHarnessVersionResolver {
    fn resolve_session_harness_version(
        &self,
        session_id: &AgentSessionId,
        requested: &HarnessVersionRef,
    ) -> Result<HarnessVersionRef, String> {
        self.requests
            .lock()
            .unwrap()
            .push((session_id.clone(), requested.clone()));
        self.result.clone()
    }
}

impl NativeProfileLaunchAuthority for RecordingProfileAuthority {
    fn prepare_launch(
        &self,
        session_id: &AgentSessionId,
        _: &AgentInvocationId,
        resuming: bool,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<RuntimeLaunchExtension, String> {
        self.calls.lock().expect("authority calls").push(resuming);
        if let Some(repository) = self.repository.as_ref() {
            let harness_version = repository
                .get_session(session_id)
                .map_err(|error| error.to_string())?
                .ok_or_else(|| "Session disappeared before native launch preparation".to_string())?
                .harness_version;
            self.observed_harness_versions
                .lock()
                .unwrap()
                .push(harness_version);
        }
        let mut extension = extension.unwrap_or_default();
        extension
            .environment
            .push(("CODEX_HOME".into(), "native-home".into()));
        Ok(extension)
    }
}

struct RejectingProfileAuthority;

impl NativeProfileLaunchAuthority for RejectingProfileAuthority {
    fn prepare_launch(
        &self,
        _: &AgentSessionId,
        _: &AgentInvocationId,
        _: bool,
        _: Option<RuntimeLaunchExtension>,
    ) -> Result<RuntimeLaunchExtension, String> {
        Err("selected native profile is not ready".into())
    }
}

#[derive(Default)]
struct RecordingSessionHarnessAuthority {
    invocations: Mutex<Vec<AgentInvocationId>>,
}

impl SessionHarnessLaunchAuthority for RecordingSessionHarnessAuthority {
    fn prepare_launch(
        &self,
        _: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<Option<RuntimeLaunchExtension>, String> {
        self.invocations.lock().unwrap().push(invocation_id.clone());
        let mut extension = extension.unwrap_or_default();
        extension
            .config_overrides
            .extend(["-c".to_string(), "mcp_servers={}".to_string()]);
        Ok(Some(extension))
    }
}

struct FakeRuntime {
    behavior: RuntimeBehavior,
    calls: Mutex<Vec<RuntimeCall>>,
    active: Mutex<Option<(AgentInvocationId, Arc<dyn AgentRuntimeUpdateSink>)>>,
}

impl FakeRuntime {
    fn new(behavior: RuntimeBehavior) -> Self {
        Self {
            behavior,
            calls: Mutex::new(Vec::new()),
            active: Mutex::new(None),
        }
    }

    fn launch(
        &self,
        request: RuntimeInvocationRequest,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        match self.behavior {
            RuntimeBehavior::CompleteWithFinalOutput => {
                deliver(
                    &sink,
                    &request.invocation_id,
                    RuntimeUpdate::Event(context_event()),
                );
                let mut event = text_event("Plan from this invocation");
                let normalized = event.normalized.as_mut().unwrap();
                normalized.kind = NormalizedRuntimeEventKind::AgentMessage;
                normalized.details = Some(json!({"role": "final"}));
                deliver(&sink, &request.invocation_id, RuntimeUpdate::Event(event));
                deliver(&sink, &request.invocation_id, completed());
                Ok(())
            }
            RuntimeBehavior::CompleteWithBinding => {
                deliver(
                    &sink,
                    &request.invocation_id,
                    RuntimeUpdate::Event(context_event()),
                );
                deliver(&sink, &request.invocation_id, completed());
                Ok(())
            }
            RuntimeBehavior::ConcurrentFastCompletion => {
                let left_sink = sink.clone();
                let left_id = request.invocation_id.clone();
                let left = std::thread::spawn(move || {
                    deliver(
                        &left_sink,
                        &left_id,
                        RuntimeUpdate::Event(text_event("stdout")),
                    )
                });
                let right_sink = sink.clone();
                let right_id = request.invocation_id.clone();
                let right = std::thread::spawn(move || {
                    deliver(&right_sink, &right_id, RuntimeUpdate::Event(stderr_event()))
                });
                left.join().expect("stdout delivery");
                right.join().expect("stderr delivery");
                deliver(&sink, &request.invocation_id, completed());
                Ok(())
            }
            RuntimeBehavior::SpawnFailure => {
                deliver(
                    &sink,
                    &request.invocation_id,
                    RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                        status: AgentInvocationTerminalStatus::Failed,
                        exit_code: None,
                        signal: None,
                        runtime_error: Some(AgentRuntimeFailure {
                            code: "spawn".to_string(),
                            message: "spawn failed".to_string(),
                            details: None,
                        }),
                    }),
                );
                Err(RuntimePortError::new(
                    RuntimePortErrorKind::LaunchFailed,
                    "spawn returned an error",
                ))
            }
            RuntimeBehavior::LaunchErrorWithoutCallback => {
                deliver(
                    &sink,
                    &request.invocation_id,
                    RuntimeUpdate::Event(text_event("pre-launch diagnostic output")),
                );
                Err(RuntimePortError::new(
                    RuntimePortErrorKind::Unavailable,
                    "program resolution failed before child launch",
                ))
            }
            RuntimeBehavior::DuplicateCompletion => {
                deliver(&sink, &request.invocation_id, completed());
                deliver(&sink, &request.invocation_id, completed());
                Ok(())
            }
            RuntimeBehavior::LateEventAfterCompletion => {
                deliver(&sink, &request.invocation_id, completed());
                deliver(
                    &sink,
                    &request.invocation_id,
                    RuntimeUpdate::Event(text_event("late")),
                );
                Ok(())
            }
            RuntimeBehavior::StayRunning => {
                *self.active.lock().expect("active runtime") = Some((request.invocation_id, sink));
                Ok(())
            }
            RuntimeBehavior::PreflightFailure => {
                panic!("preflight failure must never reach launch")
            }
        }
    }
}

impl AgentRuntime for FakeRuntime {
    fn active_turn(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<crate::agent_sessions::ports::RuntimeTurnTarget, RuntimePortError> {
        let active = self.active.lock().unwrap();
        if !active.as_ref().is_some_and(|(id, _)| id == invocation_id) {
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::NotActive,
                "not active",
            ));
        }
        Ok(crate::agent_sessions::ports::RuntimeTurnTarget {
            thread_id: "test-thread".into(),
            turn_id: "test-turn".into(),
        })
    }
    fn steer(
        &self,
        invocation_id: &AgentInvocationId,
        target: &crate::agent_sessions::ports::RuntimeTurnTarget,
        input_id: &str,
        _text: &str,
    ) -> Result<(), RuntimePortError> {
        assert_eq!(*target, self.active_turn(invocation_id)?);
        self.calls
            .lock()
            .unwrap()
            .push(RuntimeCall::Steer(invocation_id.clone(), input_id.into()));
        Ok(())
    }

    fn preflight_invocation(
        &self,
        mode: RuntimeInvocationMode,
        _requested_options: &AgentRuntimeOptions,
    ) -> Result<RuntimeInvocationPreflight, RuntimePortError> {
        self.calls
            .lock()
            .expect("calls")
            .push(RuntimeCall::Preflight(mode));
        if matches!(self.behavior, RuntimeBehavior::PreflightFailure) {
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::UnsupportedOptions,
                "unsupported requested options",
            ));
        }
        Ok(RuntimeInvocationPreflight {
            effective_options: AgentRuntimeOptions {
                model: Some("confirmed-model".to_string()),
                sandbox: Some(RuntimeSandboxMode::WorkspaceWrite),
            },
        })
    }

    fn start_invocation(
        &self,
        request: RuntimeInvocationRequest,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        self.calls
            .lock()
            .expect("calls")
            .push(RuntimeCall::Start(request.clone()));
        self.launch(request, sink)
    }

    fn resume_invocation(
        &self,
        request: RuntimeInvocationRequest,
        external_context_id: crate::agent_sessions::domain::ExternalRuntimeContextId,
        sink: Arc<dyn AgentRuntimeUpdateSink>,
    ) -> Result<(), RuntimePortError> {
        self.calls.lock().expect("calls").push(RuntimeCall::Resume(
            request.clone(),
            external_context_id.as_str().to_string(),
        ));
        self.launch(request, sink)
    }

    fn cancel_invocation(&self, invocation_id: &AgentInvocationId) -> Result<(), RuntimePortError> {
        self.calls
            .lock()
            .expect("calls")
            .push(RuntimeCall::Cancel(invocation_id.clone()));
        let active = self.active.lock().expect("active runtime").take();
        let (_, sink) = active
            .ok_or_else(|| RuntimePortError::new(RuntimePortErrorKind::NotActive, "not active"))?;
        deliver(
            &sink,
            invocation_id,
            RuntimeUpdate::Finished(RuntimeInvocationOutcome {
                status: AgentInvocationTerminalStatus::Canceled,
                exit_code: None,
                signal: None,
                runtime_error: None,
            }),
        );
        Ok(())
    }
}

fn deliver(
    sink: &Arc<dyn AgentRuntimeUpdateSink>,
    invocation_id: &AgentInvocationId,
    update: RuntimeUpdate,
) {
    if let Err(error) = sink.emit_update(invocation_id, update.clone()) {
        sink.report_delivery_failure(
            invocation_id,
            RuntimeUpdateDeliveryFailure { update, error },
        );
    }
}

fn context_event() -> RuntimeEventDraft {
    RuntimeEventDraft {
        source: AgentRuntimeEventSource::Stdout,
        raw_payload: json!({"type": "thread.started", "thread_id": "codex-thread-1"}),
        normalized: Some(NormalizedRuntimeEvent {
            kind: NormalizedRuntimeEventKind::RuntimeContextEstablished,
            text: None,
            external_context_id: Some(
                crate::agent_sessions::domain::ExternalRuntimeContextId::new("codex-thread-1")
                    .expect("external ID"),
            ),
            usage: None,
            details: None,
            tool_activity: None,
        }),
    }
}

fn text_event(text: &str) -> RuntimeEventDraft {
    RuntimeEventDraft {
        source: AgentRuntimeEventSource::Stdout,
        raw_payload: json!({"text": text}),
        normalized: Some(NormalizedRuntimeEvent {
            kind: NormalizedRuntimeEventKind::ProcessingUpdate,
            text: Some(text.to_string()),
            external_context_id: None,
            usage: None,
            details: None,
            tool_activity: None,
        }),
    }
}

fn stderr_event() -> RuntimeEventDraft {
    RuntimeEventDraft {
        source: AgentRuntimeEventSource::Stderr,
        raw_payload: json!({"text": "stderr"}),
        normalized: None,
    }
}

fn completed() -> RuntimeUpdate {
    RuntimeUpdate::Finished(RuntimeInvocationOutcome {
        status: AgentInvocationTerminalStatus::Completed,
        exit_code: Some(0),
        signal: None,
        runtime_error: None,
    })
}

struct RecordingNotifier {
    repository: Arc<SqliteAgentSessionRepository>,
    notifications: Mutex<Vec<AgentSessionNotification>>,
    persisted_before_notify: AtomicBool,
    fail: AtomicBool,
}

impl RecordingNotifier {
    fn new(repository: Arc<SqliteAgentSessionRepository>) -> Self {
        Self {
            repository,
            notifications: Mutex::new(Vec::new()),
            persisted_before_notify: AtomicBool::new(true),
            fail: AtomicBool::new(false),
        }
    }
}

impl AgentSessionNotifier for RecordingNotifier {
    fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
        let persisted = match &notification {
            AgentSessionNotification::SteeringAccepted {
                invocation_id,
                input_id,
                ..
            } => self
                .repository
                .list_events(invocation_id)
                .is_ok_and(|events| {
                    events.iter().any(|event| {
                        event.raw_payload["kind"] == "session_steering_result"
                            && event.raw_payload["id"] == *input_id
                            && event.raw_payload["state"] == "accepted"
                    })
                }),
            AgentSessionNotification::EventPersisted { event, .. } => {
                self.repository
                    .list_events(&event.invocation_id)
                    .is_ok_and(|events| events.iter().any(|candidate| candidate.id == event.id))
                    && self
                        .repository
                        .get_invocation(&event.invocation_id)
                        .is_ok_and(|stored| {
                            stored.is_some_and(|stored| {
                                stored.status == AgentInvocationStatus::Running
                            })
                        })
            }
            AgentSessionNotification::InvocationTerminal { invocation, .. } => self
                .repository
                .get_invocation(&invocation.id)
                .is_ok_and(|stored| stored.is_some_and(|stored| stored.status.is_terminal())),
            AgentSessionNotification::DiagnosticRecorded { invocation, .. } => self
                .repository
                .get_invocation(&invocation.id)
                .is_ok_and(|stored| stored.is_some_and(|stored| !stored.diagnostics.is_empty())),
        };
        if !persisted {
            self.persisted_before_notify.store(false, Ordering::Release);
        }
        self.notifications
            .lock()
            .expect("notifications")
            .push(notification);
        if self.fail.load(Ordering::Acquire) {
            Err("fake transport failure".to_string())
        } else {
            Ok(())
        }
    }
}

#[derive(Default)]
struct DeterministicProviders {
    session: AtomicU64,
    invocation: AtomicU64,
    event: AtomicU64,
    time: AtomicU64,
}

impl AgentSessionClock for DeterministicProviders {
    fn now(&self) -> DateTime<Utc> {
        Utc.with_ymd_and_hms(2026, 7, 10, 12, 0, 0).unwrap()
            + Duration::milliseconds(self.time.fetch_add(1, Ordering::AcqRel) as i64)
    }
}

impl AgentSessionIdProvider for DeterministicProviders {
    fn session_id(&self) -> AgentSessionId {
        AgentSessionId::new(format!(
            "session-{}",
            self.session.fetch_add(1, Ordering::AcqRel)
        ))
        .expect("session ID")
    }

    fn invocation_id(&self) -> AgentInvocationId {
        AgentInvocationId::new(format!(
            "invocation-{}",
            self.invocation.fetch_add(1, Ordering::AcqRel)
        ))
        .expect("invocation ID")
    }

    fn event_id(&self) -> AgentRuntimeEventId {
        AgentRuntimeEventId::new(format!(
            "event-{}",
            self.event.fetch_add(1, Ordering::AcqRel)
        ))
        .expect("event ID")
    }
}

struct FaultInjectingRepository {
    inner: Arc<SqliteAgentSessionRepository>,
    fail_finish: AtomicBool,
    fail_binding: AtomicBool,
    fail_launch_acceptance: AtomicBool,
}

impl AgentSessionRepository for FaultInjectingRepository {
    fn create_session_with_placement(
        &self,
        session: AgentSession,
        placement: &crate::agent_sessions::organization::SessionPlacement,
    ) -> Result<AgentSession, RepositoryError> {
        self.inner.create_session_with_placement(session, placement)
    }

    fn create_session(&self, session: AgentSession) -> Result<AgentSession, RepositoryError> {
        self.inner.create_session(session)
    }

    fn get_session(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSession>, RepositoryError> {
        self.inner.get_session(session_id)
    }

    fn list_sessions(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSession>, RepositoryError> {
        self.inner.list_sessions(query)
    }

    fn load_session_history(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<AgentSessionHistory>, RepositoryError> {
        self.inner.load_session_history(session_id)
    }

    fn list_session_summaries(
        &self,
        query: ListAgentSessionsQuery,
    ) -> Result<Vec<AgentSessionSummary>, RepositoryError> {
        self.inner.list_session_summaries(query)
    }

    fn set_session_availability(
        &self,
        session_id: &AgentSessionId,
        availability: AgentSessionAvailability,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        self.inner
            .set_session_availability(session_id, availability, updated_at)
    }

    fn update_runtime_binding(
        &self,
        session_id: &AgentSessionId,
        binding: AgentRuntimeBinding,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        if self.fail_binding.swap(false, Ordering::AcqRel) {
            return Err(RepositoryError::new(
                RepositoryErrorKind::Unavailable,
                "deterministic runtime binding persistence failure",
            ));
        }
        self.inner
            .update_runtime_binding(session_id, binding, updated_at)
    }

    fn update_harness_version(
        &self,
        session_id: &AgentSessionId,
        harness_version: Option<crate::harness_engine::domain::HarnessVersionRef>,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        self.inner
            .update_harness_version(session_id, harness_version, updated_at)
    }

    fn update_assigned_identity(
        &self,
        session_id: &AgentSessionId,
        assigned_identity: Option<crate::identities::AssignedAgentIdentity>,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        self.inner
            .update_assigned_identity(session_id, assigned_identity, updated_at)
    }

    fn update_session_model_override(
        &self,
        session_id: &AgentSessionId,
        model: Option<String>,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentSession, RepositoryError> {
        self.inner
            .update_session_model_override(session_id, model, updated_at)
    }

    fn create_pending_invocation(
        &self,
        invocation: AgentInvocation,
    ) -> Result<AgentInvocation, RepositoryError> {
        self.inner.create_pending_invocation(invocation)
    }

    fn get_invocation(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<AgentInvocation>, RepositoryError> {
        self.inner.get_invocation(invocation_id)
    }

    fn list_invocations(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Vec<AgentInvocation>, RepositoryError> {
        self.inner.list_invocations(session_id)
    }

    fn mark_invocation_running(
        &self,
        invocation_id: &AgentInvocationId,
        started_at: DateTime<Utc>,
        effective_options: AgentRuntimeOptions,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError> {
        self.inner
            .mark_invocation_running(invocation_id, started_at, effective_options, updated_at)
    }

    fn record_invocation_launch_accepted(
        &self,
        invocation_id: &AgentInvocationId,
        accepted_at: DateTime<Utc>,
    ) -> Result<(), RepositoryError> {
        if self.fail_launch_acceptance.swap(false, Ordering::AcqRel) {
            return Err(RepositoryError::new(
                RepositoryErrorKind::Unavailable,
                "deterministic launch acceptance persistence failure",
            ));
        }
        self.inner
            .record_invocation_launch_accepted(invocation_id, accepted_at)
    }

    fn invocation_launch_accepted_at(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Option<DateTime<Utc>>, RepositoryError> {
        self.inner.invocation_launch_accepted_at(invocation_id)
    }

    fn recover_pre_acceptance_interruption(
        &self,
        invocation_id: &AgentInvocationId,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError> {
        self.inner
            .recover_pre_acceptance_interruption(invocation_id, updated_at)
    }

    fn finish_invocation(
        &self,
        invocation_id: &AgentInvocationId,
        completion: InvocationCompletion,
        updated_at: DateTime<Utc>,
    ) -> Result<AgentInvocation, RepositoryError> {
        if self.fail_finish.swap(false, Ordering::AcqRel) {
            return Err(RepositoryError::new(
                RepositoryErrorKind::Unavailable,
                "deterministic terminal persistence failure",
            ));
        }
        self.inner
            .finish_invocation(invocation_id, completion, updated_at)
    }

    fn append_invocation_diagnostic(
        &self,
        invocation_id: &AgentInvocationId,
        diagnostic: AgentDiagnostic,
    ) -> Result<AgentInvocation, RepositoryError> {
        self.inner
            .append_invocation_diagnostic(invocation_id, diagnostic)
    }

    fn append_event(&self, event: AgentRuntimeEvent) -> Result<AgentRuntimeEvent, RepositoryError> {
        self.inner.append_event(event)
    }

    fn list_events(
        &self,
        invocation_id: &AgentInvocationId,
    ) -> Result<Vec<AgentRuntimeEvent>, RepositoryError> {
        self.inner.list_events(invocation_id)
    }
}
