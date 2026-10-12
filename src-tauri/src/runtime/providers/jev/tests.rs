use super::{contracts::*, *};
use crate::agent_sessions::{
    application::{
        jev::{CreateJevApiSessionInput, JevApiInvocationView},
        AgentSessionApplication, AgentSessionNotification, AgentSessionNotifier,
        CancelAgentInvocationCommand, CreateAgentSessionCommand, SystemAgentSessionProviders,
    },
    domain::*,
    ports::*,
    repository::SqliteAgentSessionRepository,
};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde_json::{json, Value};
use std::{
    net::TcpListener,
    path::Path,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio_util::sync::CancellationToken;

#[derive(Clone)]
struct MockResponse {
    status: StatusCode,
    body: String,
    delay: Duration,
    calls: Arc<Mutex<Vec<Value>>>,
}

struct MockApi {
    url: String,
    calls: Arc<Mutex<Vec<Value>>>,
    stop: CancellationToken,
    thread: Option<std::thread::JoinHandle<()>>,
}

impl MockApi {
    fn new(status: StatusCode, body: Value, delay: Duration) -> Self {
        let listener = TcpListener::bind("127.0.0.1:0").unwrap();
        let url = format!("http://{}/evaluate", listener.local_addr().unwrap());
        listener.set_nonblocking(true).unwrap();
        let calls = Arc::new(Mutex::new(vec![]));
        let response = MockResponse {
            status,
            body: body.to_string(),
            delay,
            calls: calls.clone(),
        };
        let stop = CancellationToken::new();
        let worker_stop = stop.clone();
        let thread = std::thread::spawn(move || {
            tokio::runtime::Builder::new_current_thread()
                .enable_all()
                .build()
                .unwrap()
                .block_on(async move {
                    let listener = tokio::net::TcpListener::from_std(listener).unwrap();
                    let app = Router::new()
                        .route("/evaluate", post(mock_evaluate))
                        .with_state(response);
                    tokio::select! {
                        _ = worker_stop.cancelled() => {},
                        _ = axum::serve(listener, app) => {},
                    }
                });
        });
        Self {
            url,
            calls,
            stop,
            thread: Some(thread),
        }
    }

    fn runtime(&self, timeout: Duration) -> Arc<JevRuntime> {
        Arc::new(JevRuntime::new(
            self.url.clone(),
            Arc::new(|| Ok("test-key".into())),
            timeout,
        ))
    }
}

impl Drop for MockApi {
    fn drop(&mut self) {
        self.stop.cancel();
        self.thread.take().unwrap().join().unwrap();
    }
}

async fn mock_evaluate(
    State(response): State<MockResponse>,
    headers: HeaderMap,
    Json(input): Json<Value>,
) -> (StatusCode, String) {
    assert_eq!(headers["authorization"], "Bearer test-key");
    response.calls.lock().unwrap().push(input);
    tokio::time::sleep(response.delay).await;
    (response.status, response.body)
}

fn evaluation() -> JevEvaluation {
    serde_json::from_value(json!({"state": {"voice":"Please investigate this"}, "questions": {
        "urgent": {"type":"noul", "instructions":"Is this urgent?"},
        "route": {"type":"choice", "instructions":"Where should this go?", "criteria":{"work":"Work", "chat":null}},
        "effort": {"type":"score", "instructions":"How much effort?", "criteria":["Small","Large"]}
    }})).unwrap()
}

fn result() -> Value {
    json!({"model":"jev-1.13.0", "answers": {
        "urgent":{"type":"noul", "noul":0.1},
        "route":{"type":"choice", "choice":"work", "probabilities":{"work":0.8,"chat":0.2}, "confidence":0.6},
        "effort":{"type":"score", "score":0.9, "legend":{"0":"Small","1":"Large"}, "probabilities":{"0":0.1,"1":0.9}, "confidence":0.8}
    }, "usage":{"input_tokens":20,"output_tokens":8}})
}

#[derive(Default)]
struct Notifications(Mutex<Vec<AgentSessionNotification>>);
impl AgentSessionNotifier for Notifications {
    fn notify(&self, notification: AgentSessionNotification) -> Result<(), String> {
        self.0.lock().unwrap().push(notification);
        Ok(())
    }
}

fn application(
    path: &Path,
    runtime: Arc<JevRuntime>,
) -> (AgentSessionApplication, Arc<Notifications>) {
    let repo = Arc::new(SqliteAgentSessionRepository::open(path).unwrap());
    let mut providers = crate::runtime::providers::registrations::ProviderRegistrations::default();
    providers
        .register(
            PROVIDER,
            crate::runtime::providers::registrations::ProviderRegistration::new(
                Arc::new(configuration::JevConfigurationSource),
                runtime.clone(),
                None,
            ),
        )
        .unwrap();
    let notifications = Arc::new(Notifications::default());
    let system = Arc::new(SystemAgentSessionProviders);
    (
        AgentSessionApplication::new(
            repo,
            runtime,
            notifications.clone(),
            system.clone(),
            system,
            None,
        )
        .with_execution_endpoints(Arc::new(
            crate::execution_targets::endpoints::ExecutionEndpoints::new(providers),
        )),
        notifications,
    )
}

fn create(app: &AgentSessionApplication) -> AgentSession {
    app.create_jev_api_session(CreateJevApiSessionInput {
        title: None,
        model: None,
    })
    .unwrap()
}

fn wait(
    app: &AgentSessionApplication,
    session: &AgentSessionId,
    invocation: &AgentInvocationId,
) -> JevApiInvocationView {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let view = app.load_jev_api_invocation(session, invocation).unwrap();
        if view.invocation.status.is_terminal() {
            return view;
        }
        assert!(Instant::now() < deadline, "Jev invocation did not finish");
        std::thread::sleep(Duration::from_millis(5));
    }
}

#[test]
fn jev_api_sessions_persist_typed_results_and_repeated_evaluations_are_independent() {
    let api = MockApi::new(StatusCode::OK, result(), Duration::ZERO);
    let directory = tempfile::tempdir().unwrap();
    let path = directory.path().join("jev.sqlite");
    let (app, notifications) = application(&path, api.runtime(Duration::from_secs(2)));
    let session = create(&app);
    assert_eq!(
        session
            .session_profile
            .as_ref()
            .unwrap()
            .session_profile()
            .configuration()
            .provider,
        PROVIDER
    );
    let id = AgentInvocationId::new("voice-query-1").unwrap();
    assert!(
        app.invoke_jev_api_session(session.id.clone(), id.clone(), evaluation())
            .unwrap()
            .launch_accepted
    );
    let finished = wait(&app, &session.id, &id);
    assert_eq!(finished.invocation.status, AgentInvocationStatus::Completed);
    assert_eq!(finished.evaluation, evaluation());
    let query_list = serde_json::to_value(app.list_jev_api_invocations(&session.id).unwrap()).unwrap();
    assert_eq!(query_list[0]["invocationId"], "voice-query-1");
    assert_eq!(query_list[0]["questionCount"], 3);
    assert_eq!(query_list[0]["status"], "completed");
    assert_eq!(query_list[0]["answers"].as_array().unwrap().len(), 3);
    assert_eq!(serde_json::to_value(finished.result).unwrap(), result());
    assert_eq!(
        finished.invocation.input_provenance,
        AgentInvocationInputProvenance::Application
    );
    assert!(
        app.invoke_jev_api_session(session.id.clone(), id.clone(), evaluation())
            .unwrap()
            .launch_accepted
    );
    let mut second = evaluation();
    second.state = json!({"voice":"This is a separate query"});
    assert!(app
        .invoke_jev_api_session(session.id.clone(), id.clone(), second.clone())
        .is_err());
    let second_id = AgentInvocationId::new("voice-query-2").unwrap();
    app.invoke_jev_api_session(session.id.clone(), second_id.clone(), second.clone())
        .unwrap();
    wait(&app, &session.id, &second_id);
    let summaries = serde_json::to_value(app.list_jev_api_invocations(&session.id).unwrap()).unwrap();
    assert_eq!(summaries.as_array().unwrap().len(), 2);
    assert_eq!(summaries[0]["invocationId"], "voice-query-2");
    assert_eq!(summaries[1]["invocationId"], "voice-query-1");
    let calls = api.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 2);
    assert_eq!(calls[0]["state"], evaluation().state);
    assert_eq!(calls[1]["state"], second.state);
    assert_eq!(calls[0]["model"], DEFAULT_MODEL);
    assert!(!calls[1].to_string().contains("Please investigate this"));
    assert!(notifications
        .0
        .lock()
        .unwrap()
        .iter()
        .any(|n| matches!(n, AgentSessionNotification::InvocationTerminal { .. })));
    let history = app.load_session(&session.id).unwrap();
    assert!(history
        .session
        .runtime_binding
        .external_context_id
        .is_none());
    assert!(
        crate::agent_sessions::application::project_invocation_observation(&history.invocations[0])
            .provider_activity
            .is_some()
    );
    assert_eq!(
        history.invocations[0]
            .events
            .iter()
            .find_map(|e| e.normalized.as_ref()?.usage.as_ref())
            .unwrap()
            .input_tokens,
        Some(20)
    );
    drop(app);
    let (reloaded, _) = application(&path, api.runtime(Duration::from_secs(2)));
    assert_eq!(
        reloaded
            .load_jev_api_invocation(&session.id, &id)
            .unwrap()
            .invocation
            .status,
        AgentInvocationStatus::Completed
    );
    let third_id = AgentInvocationId::new("voice-query-after-restart").unwrap();
    reloaded
        .invoke_jev_api_session(session.id.clone(), third_id.clone(), evaluation())
        .unwrap();
    assert!(wait(&reloaded, &session.id, &third_id).result.is_some());
    assert_eq!(api.calls.lock().unwrap().len(), 3);
}

#[test]
fn jev_api_http_errors_are_durable_and_do_not_echo_native_error_bodies() {
    for status in [
        StatusCode::UNAUTHORIZED,
        StatusCode::TOO_MANY_REQUESTS,
        StatusCode::from_u16(529).unwrap(),
    ] {
        let api = MockApi::new(
            status,
            json!({"error":"test-key private request body"}),
            Duration::ZERO,
        );
        let directory = tempfile::tempdir().unwrap();
        let (app, _) = application(
            &directory.path().join("error.sqlite"),
            api.runtime(Duration::from_secs(2)),
        );
        let session = create(&app);
        let id = AgentInvocationId::new("error").unwrap();
        app.invoke_jev_api_session(session.id.clone(), id.clone(), evaluation())
            .unwrap();
        let view = wait(&app, &session.id, &id);
        assert_eq!(view.invocation.status, AgentInvocationStatus::Failed);
        assert!(view.result.is_none());
        assert!(view
            .invocation
            .runtime_error
            .unwrap()
            .message
            .contains(&status.as_u16().to_string()));
        assert_eq!(api.calls.lock().unwrap().len(), 1);
        let evidence = format!("{:?}", app.load_session(&session.id).unwrap());
        assert!(!evidence.contains("test-key"));
        assert!(!evidence.contains("private request body"));
    }
}

#[test]
fn jev_api_rejects_missing_credentials_before_sending() {
    let api = MockApi::new(StatusCode::OK, result(), Duration::ZERO);
    let runtime = Arc::new(JevRuntime::new(
        api.url.clone(),
        Arc::new(|| Err("TYPESAFE_API_KEY is missing".into())),
        Duration::from_secs(2),
    ));
    let directory = tempfile::tempdir().unwrap();
    let (app, _) = application(&directory.path().join("missing.sqlite"), runtime);
    let session = create(&app);
    let id = AgentInvocationId::new("missing-key").unwrap();
    assert!(
        !app.invoke_jev_api_session(session.id.clone(), id.clone(), evaluation())
            .unwrap()
            .launch_accepted
    );
    assert_eq!(
        wait(&app, &session.id, &id).invocation.status,
        AgentInvocationStatus::Failed
    );
    assert!(api.calls.lock().unwrap().is_empty());
}

#[test]
fn jev_api_invalid_results_fail_without_publishing_an_answer() {
    let mut bodies = vec![json!({"answers":{}})];
    let mut mismatch = result();
    mismatch["answers"]["urgent"] =
        json!({"type":"choice","choice":"yes","probabilities":{"yes":1.0},"confidence":1.0});
    bodies.push(mismatch);
    let mut invalid_distribution = result();
    invalid_distribution["answers"]["route"]["probabilities"]["work"] = json!(1.5);
    bodies.push(invalid_distribution);
    for body in bodies {
        let api = MockApi::new(StatusCode::OK, body, Duration::ZERO);
        let directory = tempfile::tempdir().unwrap();
        let (app, _) = application(
            &directory.path().join("malformed.sqlite"),
            api.runtime(Duration::from_secs(2)),
        );
        let session = create(&app);
        let id = AgentInvocationId::new("malformed").unwrap();
        app.invoke_jev_api_session(session.id.clone(), id.clone(), evaluation())
            .unwrap();
        let view = wait(&app, &session.id, &id);
        assert_eq!(view.invocation.status, AgentInvocationStatus::Failed);
        assert!(view.result.is_none());
    }
}

#[test]
fn jev_api_timeout_and_cancellation_use_session_terminal_states() {
    for cancel in [false, true] {
        let api = MockApi::new(StatusCode::OK, result(), Duration::from_secs(1));
        let directory = tempfile::tempdir().unwrap();
        let (app, notifications) = application(
            &directory.path().join("cancel.sqlite"),
            api.runtime(Duration::from_millis(100)),
        );
        let session = create(&app);
        let id = AgentInvocationId::new("cancel").unwrap();
        app.invoke_jev_api_session(session.id.clone(), id.clone(), evaluation())
            .unwrap();
        if cancel {
            app.cancel_invocation(CancelAgentInvocationCommand {
                invocation_id: id.clone(),
            })
            .unwrap();
        }
        let view = wait(&app, &session.id, &id);
        assert_eq!(
            view.invocation.status,
            if cancel {
                AgentInvocationStatus::Canceled
            } else {
                AgentInvocationStatus::Failed
            }
        );
        if !cancel {
            assert_eq!(
                view.invocation.runtime_error.unwrap().message,
                "Jev request timed out"
            );
        }
        assert!(view.result.is_none());
        assert_eq!(
            notifications
                .0
                .lock()
                .unwrap()
                .iter()
                .filter(|n| matches!(n, AgentSessionNotification::InvocationTerminal { .. }))
                .count(),
            1
        );
    }
}

#[test]
fn jev_api_shutdown_cancels_requests_and_prevents_more_launches() {
    let api = MockApi::new(StatusCode::OK, result(), Duration::from_secs(1));
    let directory = tempfile::tempdir().unwrap();
    let (app, _) = application(
        &directory.path().join("shutdown.sqlite"),
        api.runtime(Duration::from_secs(2)),
    );
    let session = create(&app);
    let id = AgentInvocationId::new("shutdown").unwrap();
    app.invoke_jev_api_session(session.id.clone(), id.clone(), evaluation())
        .unwrap();
    app.shutdown_runtime().unwrap();
    assert_eq!(
        wait(&app, &session.id, &id).invocation.status,
        AgentInvocationStatus::Canceled
    );
    let next = AgentInvocationId::new("after-shutdown").unwrap();
    assert!(
        !app.invoke_jev_api_session(session.id.clone(), next.clone(), evaluation())
            .unwrap()
            .launch_accepted
    );
    assert_eq!(
        wait(&app, &session.id, &next).invocation.status,
        AgentInvocationStatus::Failed
    );
}

#[test]
fn jev_api_requires_typed_inputs_and_its_own_session_binding() {
    let api = MockApi::new(StatusCode::OK, result(), Duration::ZERO);
    let directory = tempfile::tempdir().unwrap();
    let runtime = api.runtime(Duration::from_secs(2));
    let (app, _) = application(&directory.path().join("typed.sqlite"), runtime.clone());
    let session = create(&app);
    let mut invalid = evaluation();
    invalid.state = Value::Null;
    assert!(app
        .invoke_jev_api_session(
            session.id.clone(),
            AgentInvocationId::new("bad").unwrap(),
            invalid
        )
        .is_err());
    assert!(app
        .load_session(&session.id)
        .unwrap()
        .invocations
        .is_empty());
    let ordinary = app
        .create_session(CreateAgentSessionCommand {
            title: None,
            working_directory: None,
            requested_options: Default::default(),
        })
        .unwrap();
    assert!(app
        .invoke_jev_api_session(
            ordinary.id,
            AgentInvocationId::new("wrong-provider").unwrap(),
            evaluation()
        )
        .is_err());
    assert!(runtime
        .preflight_invocation(RuntimeInvocationMode::Resume, &Default::default())
        .is_err());
    assert!(runtime
        .preflight_invocation(
            RuntimeInvocationMode::Start,
            &AgentRuntimeOptions {
                model: None,
                sandbox: Some(RuntimeSandboxMode::DangerFullAccess)
            }
        )
        .is_err());
    assert!(app
        .create_jev_api_session(CreateJevApiSessionInput {
            title: None,
            model: Some("gpt-5".into())
        })
        .is_err());
    assert!(api.calls.lock().unwrap().is_empty());
}

#[test]
fn jev_api_keeps_provider_success_when_event_delivery_fails() {
    #[derive(Default)]
    struct Sink {
        terminal: Mutex<Option<RuntimeInvocationOutcome>>,
        failures: Mutex<Vec<RuntimeUpdateDeliveryFailure>>,
    }
    impl AgentRuntimeUpdateSink for Sink {
        fn emit_update(
            &self,
            _: &AgentInvocationId,
            update: RuntimeUpdate,
        ) -> Result<(), RuntimePortError> {
            if let RuntimeUpdate::Finished(outcome) = update {
                *self.terminal.lock().unwrap() = Some(outcome);
                Ok(())
            } else {
                Err(RuntimePortError::new(
                    RuntimePortErrorKind::EventDeliveryFailed,
                    "storage unavailable",
                ))
            }
        }
        fn report_delivery_failure(
            &self,
            _: &AgentInvocationId,
            failure: RuntimeUpdateDeliveryFailure,
        ) {
            self.failures.lock().unwrap().push(failure);
        }
    }
    let api = MockApi::new(StatusCode::OK, result(), Duration::ZERO);
    let runtime = api.runtime(Duration::from_secs(2));
    let sink = Arc::new(Sink::default());
    runtime
        .start_invocation(
            RuntimeInvocationRequest {
                session_id: AgentSessionId::new("jev-session").unwrap(),
                invocation_id: AgentInvocationId::new("delivery-failure").unwrap(),
                kind: RuntimeInvocationKind::Turn,
                content: InvocationContent::query(serde_json::to_string(&evaluation()).unwrap()),
                attachments: vec![],
                working_directory: None,
                options: Default::default(),
                launch_extension: None,
            },
            sink.clone(),
        )
        .unwrap();
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        if let Some(outcome) = sink.terminal.lock().unwrap().as_ref() {
            assert_eq!(outcome.status, AgentInvocationTerminalStatus::Completed);
            break;
        }
        assert!(Instant::now() < deadline);
        std::thread::sleep(Duration::from_millis(5));
    }
    assert_eq!(sink.failures.lock().unwrap().len(), 3);
}
