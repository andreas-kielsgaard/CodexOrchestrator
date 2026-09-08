use super::*;
use crate::agent_sessions::domain::{
    AgentDiagnosticSeverity, AgentDiagnosticSource, AgentInvocationInputProvenance,
    AgentInvocationTerminalStatus, AgentRuntimeEventId, AgentRuntimeEventSource,
    AgentRuntimeFailure, ExternalRuntimeContextId, NormalizedRuntimeEvent,
    NormalizedRuntimeEventKind, RuntimeSandboxMode,
};
use crate::{
    harness_engine::domain::{HarnessId, HarnessVersionNumber, HarnessVersionRef},
    identities::{AssignedAgentIdentity, IdentityId, IdentityShape},
};
use serde_json::json;
use std::{fs, path::PathBuf};
use uuid::Uuid;

#[test]
fn model_override_persists_without_replacing_the_session_sandbox() {
    let path = temporary_database_path();
    let connection = initialized_file_database(&path);
    let repository = SqliteAgentSessionRepository::new(connection).expect("construct repository");
    let mut session = test_session("model-session", at(0));
    session.requested_options.model = Some("initial-model".into());
    session.requested_options.sandbox = Some(RuntimeSandboxMode::ReadOnly);
    let session = repository.create_session(session).expect("create session");

    let updated = repository
        .update_session_model_override(&session.id, Some("replacement-model".into()), at(1))
        .expect("update model override");
    assert_eq!(
        updated.requested_options.model.as_deref(),
        Some("replacement-model")
    );
    assert_eq!(
        updated.requested_options.sandbox,
        Some(RuntimeSandboxMode::ReadOnly)
    );
    let cleared = repository
        .update_session_model_override(&session.id, None, at(2))
        .expect("clear model override");
    assert!(cleared.requested_options.model.is_none());
    assert_eq!(
        cleared.requested_options.sandbox,
        Some(RuntimeSandboxMode::ReadOnly)
    );
    drop(repository);

    let reopened = SqliteAgentSessionRepository::open(&path).expect("reopen repository");
    let persisted = reopened
        .get_session(&session.id)
        .unwrap()
        .expect("persisted session");
    assert!(persisted.requested_options.model.is_none());
    assert_eq!(
        persisted.requested_options.sandbox,
        Some(RuntimeSandboxMode::ReadOnly)
    );
    drop(reopened);
    fs::remove_file(path).expect("remove test database");
}

#[test]
fn persists_and_updates_session_owned_harness_and_identity() {
    let path = temporary_database_path();
    let connection = initialized_file_database(&path);
    let repository = SqliteAgentSessionRepository::new(connection).expect("construct repository");
    let mut session = test_session("owned-session", at(0));
    session.harness_version = Some(harness_ref("planning", 1));
    session.assigned_identity = Some(assigned_identity("avery", "Avery", "#39745a"));
    let session = repository.create_session(session).expect("create session");

    let migrated = repository
        .update_harness_version(&session.id, Some(harness_ref("planning", 2)), at(1))
        .expect("migrate Harness reference");
    assert_eq!(
        migrated
            .harness_version
            .as_ref()
            .map(|value| value.version().get()),
        Some(2)
    );
    let reassigned = repository
        .update_assigned_identity(
            &session.id,
            Some(assigned_identity("morgan", "Morgan", "#224466")),
            at(2),
        )
        .expect("replace assigned identity");
    assert_eq!(
        reassigned
            .assigned_identity
            .as_ref()
            .map(|identity| identity.display_name.as_str()),
        Some("Morgan")
    );
    drop(repository);

    let reopened = SqliteAgentSessionRepository::open(&path).expect("reopen repository");
    let persisted = reopened
        .get_session(&session.id)
        .expect("load session")
        .expect("persisted session");
    assert_eq!(persisted.harness_version, Some(harness_ref("planning", 2)));
    assert_eq!(
        persisted.assigned_identity,
        Some(assigned_identity("morgan", "Morgan", "#224466"))
    );

    drop(reopened);
    fs::remove_file(path).expect("remove test database");
}

#[test]
fn repository_open_adds_nullable_ownership_to_pre_ownership_session_schema() {
    let connection = Connection::open_in_memory().expect("memory database");
    connection
        .execute_batch(
            "PRAGMA foreign_keys = ON;
             CREATE TABLE agent_sessions (
               id TEXT PRIMARY KEY,
               title TEXT NOT NULL,
               availability TEXT NOT NULL,
               external_context_id TEXT,
               runtime_version TEXT,
               working_directory TEXT,
               requested_options_json TEXT NOT NULL,
               created_at TEXT NOT NULL,
               updated_at TEXT NOT NULL
             );
             INSERT INTO agent_sessions
               (id,title,availability,requested_options_json,created_at,updated_at)
             VALUES ('legacy','Legacy','available','{}','2026-07-10T12:00:00Z','2026-07-10T12:00:00Z');",
        )
        .expect("legacy schema");

    let repository = SqliteAgentSessionRepository::new(connection).expect("upgrade repository");
    let session = repository
        .get_session(&AgentSessionId::new("legacy").unwrap())
        .expect("read upgraded session")
        .expect("legacy session");

    assert!(session.harness_version.is_none());
    assert!(session.assigned_identity.is_none());
}

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

#[test]
fn invalid_file_history_rolls_back_the_runtime_event() {
    let repository = memory_repository();
    let session = repository
        .create_session(test_session("author", at(0)))
        .unwrap();
    let invocation = repository
        .create_pending_invocation(test_invocation("write", &session.id, at(1)))
        .unwrap();
    let mut event = normalized_event("edit", &invocation.id, 1, at(2));
    event.normalized.as_mut().unwrap().details = Some(json!({"fileChanges":[
        {"path":"valid.md","operation":"create"},{"path":"","operation":"edit"}
    ]}));
    assert!(repository.append_event(event).is_err());
    assert!(repository.list_events(&invocation.id).unwrap().is_empty());
    let count: i64 = repository
        .lock()
        .unwrap()
        .query_row(
            "SELECT count(*) FROM agent_session_file_changes",
            [],
            |row| row.get(0),
        )
        .unwrap();
    assert_eq!(count, 0);
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
        session_profile: None,
        harness_version: None,
        assigned_identity: None,
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

fn harness_ref(id: &str, version: u64) -> HarnessVersionRef {
    HarnessVersionRef::new(
        HarnessId::new(id).expect("Harness ID"),
        HarnessVersionNumber::new(version).expect("Harness version"),
    )
}

fn assigned_identity(id: &str, name: &str, color: &str) -> AssignedAgentIdentity {
    AssignedAgentIdentity::new(
        Some(IdentityId::new(id).expect("identity ID")),
        name,
        color,
        IdentityShape::Circle,
    )
    .expect("assigned identity")
}

fn at(second: u32) -> DateTime<Utc> {
    format!("2026-07-10T12:00:{second:02}Z")
        .parse()
        .expect("timestamp")
}
