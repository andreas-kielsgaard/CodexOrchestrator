use super::*;
use crate::agent_sessions::domain::{
    AgentDiagnosticSeverity, AgentDiagnosticSource, AgentInvocationInputProvenance,
    AgentInvocationTerminalStatus, AgentRuntimeEventId, AgentRuntimeEventSource,
    AgentRuntimeFailure, ExternalRuntimeContextId, NormalizedRuntimeEvent,
    NormalizedRuntimeEventKind,
};
use serde_json::json;
use std::{
    fs,
    path::PathBuf,
    sync::{Arc, Barrier},
};
use uuid::Uuid;

#[test]
fn survives_close_and_reopen_with_complete_multi_invocation_history() {
    let path = temporary_database_path();
    let connection = initialized_file_database(&path);
    let repository = SqliteAgentSessionRepository::new(connection).expect("construct repository");
    let session = repository
        .create_session(test_session("session-b", at(0)))
        .expect("create session");
    repository
        .update_runtime_binding(
            &session.id,
            AgentRuntimeBinding {
                external_context_id: Some(
                    ExternalRuntimeContextId::new("provider-thread").expect("external ID"),
                ),
                runtime_version: Some("codex-test".into()),
            },
            at(1),
        )
        .expect("bind runtime context");

    let first = repository
        .create_pending_invocation(test_invocation("invocation-b", &session.id, at(2)))
        .expect("create first invocation");
    repository
        .mark_invocation_running(&first.id, at(3), options(), at(3))
        .expect("start first invocation");
    repository
        .record_invocation_launch_accepted(&first.id, at(4))
        .expect("record launch acceptance");
    repository
        .append_event(unknown_event(
            "event-b2",
            &first.id,
            2,
            json!({"type":"future.event","nested":{"unchanged":[1,true,null]}}),
            at(4),
        ))
        .expect("append first unknown event");
    repository
        .append_event(normalized_event("event-b9", &first.id, 9, at(5)))
        .expect("append first normalized event");
    let first_completion = InvocationCompletion {
        status: AgentInvocationTerminalStatus::Completed,
        completed_at: at(6),
        exit_code: Some(0),
        signal: None,
        runtime_error: None,
    };
    repository
        .finish_invocation(&first.id, first_completion.clone(), at(6))
        .expect("finish first invocation");
    repository
        .finish_invocation(&first.id, first_completion, at(30))
        .expect("repeat identical completion idempotently");

    let mut application_invocation = test_invocation("invocation-a", &session.id, at(7));
    application_invocation.input_provenance = AgentInvocationInputProvenance::Application;
    let second = repository
        .create_pending_invocation(application_invocation)
        .expect("create second invocation");
    repository
        .append_invocation_diagnostic(
            &second.id,
            AgentDiagnostic {
                source: AgentDiagnosticSource::Repository,
                severity: AgentDiagnosticSeverity::Warning,
                code: "persist_warning".into(),
                message: "Retained independently of runtime outcome".into(),
                details: Some(json!({"attempt": 1})),
                recorded_at: at(8),
            },
        )
        .expect("append diagnostic");
    repository
        .append_event(unknown_event(
            "event-a",
            &second.id,
            0,
            json!({"unknown":"raw-only","large":"x".repeat(16_384)}),
            at(9),
        ))
        .expect("append second event");
    repository
        .finish_invocation(
            &second.id,
            InvocationCompletion {
                status: AgentInvocationTerminalStatus::Interrupted,
                completed_at: at(10),
                exit_code: None,
                signal: Some("shutdown".into()),
                runtime_error: Some(AgentRuntimeFailure {
                    code: "app_shutdown".into(),
                    message: "Application closed".into(),
                    details: Some(json!({"recoverable": true})),
                }),
            },
            at(10),
        )
        .expect("interrupt second invocation");
    drop(repository);

    let reopened = SqliteAgentSessionRepository::open(&path).expect("reopen repository");
    assert_eq!(
        reopened
            .invocation_launch_accepted_at(&first.id)
            .expect("load launch acceptance"),
        Some(at(4))
    );
    let history = reopened
        .load_session_history(&session.id)
        .expect("load complete history")
        .expect("session history");

    assert_eq!(
        history
            .session
            .runtime_binding
            .external_context_id
            .as_ref()
            .map(ExternalRuntimeContextId::as_str),
        Some("provider-thread")
    );
    assert_eq!(
        history
            .invocations
            .iter()
            .map(|entry| entry.invocation.id.as_str())
            .collect::<Vec<_>>(),
        vec!["invocation-b", "invocation-a"]
    );
    assert_eq!(
        history.invocations[0]
            .events
            .iter()
            .map(|event| event.sequence)
            .collect::<Vec<_>>(),
        vec![2, 9]
    );
    assert_eq!(history.invocations[0].events[0].normalized, None);
    assert_eq!(history.invocations[0].launch_accepted_at, Some(at(4)));
    let observation =
        crate::agent_sessions::application::project_invocation_observation(&history.invocations[0]);
    assert_eq!(observation.launch_accepted_at, Some(at(4)));
    assert!(observation.provider_activity.is_none());
    assert!(observation.provider_terminal.is_none());
    assert_eq!(
        observation
            .process_terminal
            .as_ref()
            .map(|terminal| terminal.status),
        Some(AgentInvocationStatus::Completed)
    );
    assert_eq!(
        history.invocations[0].events[0].raw_payload,
        json!({"type":"future.event","nested":{"unchanged":[1,true,null]}})
    );
    assert_eq!(history.invocations[1].invocation.diagnostics.len(), 1);
    assert_eq!(
        history.invocations[0].invocation.input_provenance,
        AgentInvocationInputProvenance::User
    );
    assert_eq!(
        history.invocations[1].invocation.input_provenance,
        AgentInvocationInputProvenance::Application
    );
    assert_eq!(
        history.invocations[1].events[0].raw_payload["large"]
            .as_str()
            .expect("large raw payload")
            .len(),
        16_384
    );

    let summaries = reopened
        .list_session_summaries(ListAgentSessionsQuery::default())
        .expect("session summaries");
    assert_eq!(summaries.len(), 1);
    assert_eq!(summaries[0].invocation_count, 2);
    assert_eq!(
        summaries[0].latest_invocation_status,
        Some(AgentInvocationStatus::Interrupted)
    );
    assert_eq!(
        summaries[0].latest_submitted_text.as_deref(),
        Some("input for invocation-a")
    );

    drop(reopened);
    fs::remove_file(path).expect("remove test database");
}

#[test]
fn enforces_one_active_invocation_and_rolls_back_rejected_create() {
    let repository = memory_repository();
    let session = repository
        .create_session(test_session("session", at(0)))
        .expect("create session");
    repository
        .create_pending_invocation(test_invocation("first", &session.id, at(1)))
        .expect("create active invocation");

    let error = repository
        .create_pending_invocation(test_invocation("second", &session.id, at(2)))
        .expect_err("reject second active invocation");

    assert_eq!(error.kind, RepositoryErrorKind::Conflict);
    assert!(repository
        .get_invocation(&AgentInvocationId::new("second").expect("ID"))
        .expect("query rejected invocation")
        .is_none());
    assert_eq!(
        repository
            .list_invocations(&session.id)
            .expect("invocations")
            .len(),
        1
    );
}

#[test]
fn concurrent_typed_and_generic_launch_claims_converge_on_the_bound_transport() {
    let repository = Arc::new(memory_repository());
    let session = repository
        .create_session(test_session("session", at(0)))
        .expect("create session");
    let mut pending = test_invocation("application-invocation", &session.id, at(1));
    pending.input_provenance = AgentInvocationInputProvenance::Application;
    let invocation = repository
        .create_pending_invocation(pending)
        .expect("create application invocation");
    let binding = ApplicationInvocationTransportBinding {
        kind: ApplicationInvocationTransportKind::WorkUnitHandlerReview,
        extension_fingerprint: "exact-extension-fingerprint".into(),
        accepted_effective_extension_fingerprint: None,
        bound_at: at(2),
    };
    repository
        .reserve_application_invocation_transport(
            &invocation.id,
            binding.kind,
            binding.bound_at,
        )
        .expect("reserve exact transport");
    let unbound_generic = repository
        .mark_invocation_running(&invocation.id, at(2), options(), at(2))
        .expect_err("reserved invocation rejects an unbound generic child");
    assert_eq!(unbound_generic.kind, RepositoryErrorKind::Conflict);
    let unbound_typed = repository
        .mark_invocation_running_with_transport(
            &invocation.id,
            &binding,
            at(2),
            options(),
            at(2),
        )
        .expect_err("reserved invocation rejects a child before exact binding");
    assert_eq!(unbound_typed.kind, RepositoryErrorKind::Conflict);
    repository
        .bind_application_invocation_transport(&invocation.id, binding.clone())
        .expect("bind exact transport");

    let barrier = Arc::new(Barrier::new(4));
    let typed_repository = repository.clone();
    let typed_invocation = invocation.id.clone();
    let typed_binding = binding.clone();
    let typed_barrier = barrier.clone();
    let typed = std::thread::spawn(move || {
        typed_barrier.wait();
        typed_repository.mark_invocation_running_with_transport(
            &typed_invocation,
            &typed_binding,
            at(3),
            options(),
            at(3),
        )
    });
    let generic_repository = repository.clone();
    let generic_invocation = invocation.id.clone();
    let generic_barrier = barrier.clone();
    let generic = std::thread::spawn(move || {
        generic_barrier.wait();
        generic_repository.mark_invocation_running(
            &generic_invocation,
            at(3),
            options(),
            at(3),
        )
    });
    let differently_bound_repository = repository.clone();
    let differently_bound_invocation = invocation.id.clone();
    let differently_bound_barrier = barrier.clone();
    let mut different_binding = binding.clone();
    different_binding.extension_fingerprint = "different-extension-fingerprint".into();
    let differently_bound = std::thread::spawn(move || {
        differently_bound_barrier.wait();
        differently_bound_repository.mark_invocation_running_with_transport(
            &differently_bound_invocation,
            &different_binding,
            at(3),
            options(),
            at(3),
        )
    });
    barrier.wait();

    typed
        .join()
        .expect("typed caller thread")
        .expect("typed caller wins contract");
    let generic_error = generic
        .join()
        .expect("generic caller thread")
        .expect_err("generic caller fails closed");
    assert_eq!(generic_error.kind, RepositoryErrorKind::Conflict);
    let differently_bound_error = differently_bound
        .join()
        .expect("differently bound caller thread")
        .expect_err("differently bound caller fails closed");
    assert_eq!(differently_bound_error.kind, RepositoryErrorKind::Conflict);
    assert_eq!(
        repository
            .application_invocation_transport_binding(&invocation.id)
            .expect("read exact binding"),
        Some(binding)
    );
}

#[test]
fn accepted_typed_transport_binding_survives_close_and_reopen() {
    let path = temporary_database_path();
    let connection = initialized_file_database(&path);
    let repository = SqliteAgentSessionRepository::new(connection).expect("construct repository");
    let session = repository
        .create_session(test_session("session", at(0)))
        .expect("create session");
    let mut pending = test_invocation("application-invocation", &session.id, at(1));
    pending.input_provenance = AgentInvocationInputProvenance::Application;
    let invocation = repository
        .create_pending_invocation(pending)
        .expect("create application invocation");
    let binding = ApplicationInvocationTransportBinding {
        kind: ApplicationInvocationTransportKind::WorkUnitImplementerReporting,
        extension_fingerprint: "persisted-extension-fingerprint".into(),
        accepted_effective_extension_fingerprint: None,
        bound_at: at(2),
    };
    repository
        .reserve_application_invocation_transport(
            &invocation.id,
            binding.kind,
            binding.bound_at,
        )
        .expect("reserve exact transport");
    repository
        .bind_application_invocation_transport(&invocation.id, binding.clone())
        .expect("bind exact transport");
    repository
        .mark_invocation_running_with_transport(
            &invocation.id,
            &binding,
            at(3),
            options(),
            at(3),
        )
        .expect("start exact transport");
    let mismatched_acceptance = repository
        .record_invocation_launch_accepted_with_transport(
            &invocation.id,
            &binding,
            "different-effective-extension",
            at(4),
        )
        .expect_err("differently bound provider child cannot be accepted");
    assert_eq!(mismatched_acceptance.kind, RepositoryErrorKind::Conflict);
    assert_eq!(
        repository
            .invocation_launch_accepted_at(&invocation.id)
            .expect("unaccepted mismatched child"),
        None
    );
    repository
        .record_invocation_launch_accepted_with_transport(
            &invocation.id,
            &binding,
            &binding.extension_fingerprint,
            at(4),
        )
        .expect("record exact acceptance");
    let accepted_binding = ApplicationInvocationTransportBinding {
        accepted_effective_extension_fingerprint: Some(binding.extension_fingerprint.clone()),
        ..binding
    };
    drop(repository);

    let reopened = SqliteAgentSessionRepository::open(&path).expect("reopen repository");
    assert_eq!(
        reopened
            .application_invocation_transport_binding(&invocation.id)
            .expect("read reopened binding"),
        Some(accepted_binding)
    );
    assert_eq!(
        reopened
            .invocation_launch_accepted_at(&invocation.id)
            .expect("read reopened acceptance"),
        Some(at(4))
    );
    drop(reopened);
    fs::remove_file(path).expect("remove test database");
}

#[test]
fn rejects_duplicate_or_reordered_event_sequences_without_partial_write() {
    let repository = memory_repository();
    let session = repository
        .create_session(test_session("session", at(0)))
        .expect("create session");
    let invocation = repository
        .create_pending_invocation(test_invocation("invocation", &session.id, at(1)))
        .expect("create invocation");
    repository
        .append_event(unknown_event(
            "event-10",
            &invocation.id,
            10,
            json!({"first": true}),
            at(2),
        ))
        .expect("append event");

    let error = repository
        .append_event(unknown_event(
            "event-9",
            &invocation.id,
            9,
            json!({"late": true}),
            at(3),
        ))
        .expect_err("reject decreasing event sequence");

    assert_eq!(error.kind, RepositoryErrorKind::Conflict);
    assert_eq!(
        repository
            .list_events(&invocation.id)
            .expect("events")
            .len(),
        1
    );
}

#[test]
fn orders_session_lists_by_update_time_then_id_and_filters_availability() {
    let repository = memory_repository();
    repository
        .create_session(test_session("z-session", at(0)))
        .expect("create z session");
    repository
        .create_session(test_session("a-session", at(0)))
        .expect("create a session");
    let archived = repository
        .create_session(test_session("archived", at(1)))
        .expect("create archived session");
    repository
        .set_session_availability(&archived.id, AgentSessionAvailability::Archived, at(2))
        .expect("archive session");

    assert_eq!(
        repository
            .list_sessions(ListAgentSessionsQuery {
                availability: Some(AgentSessionAvailability::Available),
                limit: None,
            })
            .expect("available sessions")
            .iter()
            .map(|session| session.id.as_str())
            .collect::<Vec<_>>(),
        vec!["a-session", "z-session"]
    );
    assert_eq!(
        repository
            .list_sessions(ListAgentSessionsQuery {
                availability: None,
                limit: Some(1),
            })
            .expect("limited sessions")[0]
            .id
            .as_str(),
        "archived"
    );
}

#[test]
fn construction_enables_and_verifies_foreign_keys_for_injected_connections() {
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(&format!(
            "PRAGMA foreign_keys = OFF; {AGENT_SESSION_SCHEMA}"
        ))
        .expect("initialize Agent Session schema");
    assert_eq!(
        connection
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .expect("initial foreign key state"),
        0
    );

    let repository = SqliteAgentSessionRepository::new(connection).expect("construct repository");

    assert_eq!(
        repository
            .lock()
            .expect("repository connection")
            .query_row("PRAGMA foreign_keys", [], |row| row.get::<_, i64>(0))
            .expect("repository foreign key state"),
        1
    );
}

fn memory_repository() -> SqliteAgentSessionRepository {
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(&format!("PRAGMA foreign_keys = ON; {AGENT_SESSION_SCHEMA}"))
        .expect("initialize Agent Session schema");
    SqliteAgentSessionRepository::new(connection).expect("construct repository")
}

fn initialized_file_database(path: &PathBuf) -> Connection {
    let connection = Connection::open(path).expect("file database");
    connection
        .execute_batch(&format!("PRAGMA foreign_keys = ON; {AGENT_SESSION_SCHEMA}"))
        .expect("initialize Agent Session schema");
    connection
}

fn temporary_database_path() -> PathBuf {
    std::env::temp_dir().join(format!("codex-agent-session-{}.sqlite", Uuid::new_v4()))
}

fn test_session(id: &str, created_at: DateTime<Utc>) -> AgentSession {
    AgentSession {
        id: AgentSessionId::new(id).expect("session ID"),
        title: format!("Session {id}"),
        availability: AgentSessionAvailability::Available,
        runtime_binding: AgentRuntimeBinding {
            external_context_id: None,
            runtime_version: None,
        },
        working_directory: Some("C:/work".into()),
        requested_options: options(),
        created_at,
        updated_at: created_at,
    }
}

fn test_invocation(
    id: &str,
    session_id: &AgentSessionId,
    created_at: DateTime<Utc>,
) -> AgentInvocation {
    AgentInvocation {
        id: AgentInvocationId::new(id).expect("invocation ID"),
        session_id: session_id.clone(),
        submitted_text: format!("input for {id}"),
        input_provenance: AgentInvocationInputProvenance::User,
        status: AgentInvocationStatus::Pending,
        requested_options: options(),
        effective_options: None,
        started_at: None,
        completed_at: None,
        exit_code: None,
        signal: None,
        runtime_error: None,
        diagnostics: Vec::new(),
        created_at,
        updated_at: created_at,
    }
}

fn unknown_event(
    id: &str,
    invocation_id: &AgentInvocationId,
    sequence: u64,
    raw_payload: serde_json::Value,
    recorded_at: DateTime<Utc>,
) -> AgentRuntimeEvent {
    AgentRuntimeEvent {
        id: AgentRuntimeEventId::new(id).expect("event ID"),
        invocation_id: invocation_id.clone(),
        sequence,
        source: AgentRuntimeEventSource::Stdout,
        raw_payload,
        normalized: None,
        recorded_at,
    }
}

fn normalized_event(
    id: &str,
    invocation_id: &AgentInvocationId,
    sequence: u64,
    recorded_at: DateTime<Utc>,
) -> AgentRuntimeEvent {
    AgentRuntimeEvent {
        id: AgentRuntimeEventId::new(id).expect("event ID"),
        invocation_id: invocation_id.clone(),
        sequence,
        source: AgentRuntimeEventSource::Runtime,
        raw_payload: json!({"type":"agent_message","text":"done"}),
        normalized: Some(NormalizedRuntimeEvent {
            kind: NormalizedRuntimeEventKind::AgentMessage,
            text: Some("done".into()),
            external_context_id: None,
            usage: None,
            details: Some(json!({"final": true})),
            tool_activity: None,
        }),
        recorded_at,
    }
}

fn options() -> AgentRuntimeOptions {
    AgentRuntimeOptions {
        model: Some("test-model".into()),
        sandbox: None,
    }
}

fn at(second: u32) -> DateTime<Utc> {
    format!("2026-07-10T12:00:{second:02}Z")
        .parse()
        .expect("timestamp")
}
