//! Exercise the real session lifecycle and provider HTTP adapter with a controlled API.
use orchid_core::{
    agent_sessions::{
        application::*, domain::*, ports::*, repository::SqliteAgentSessionRepository,
    },
    execution_targets::endpoints::ExecutionEndpoints,
    runtime::providers::{
        jev::{configuration::JevConfigurationSource, JevRuntime},
        registrations::*,
    },
    synchronization::{repository::ReplicaStore, ServerConnection},
};
use orchid_services::{
    host::{connect_once, CommandHandler},
    server::Server,
};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc,
};
use std::time::Duration;

struct Notifications;
impl AgentSessionNotifier for Notifications {
    fn notify(&self, _: AgentSessionNotification) -> Result<(), String> {
        Ok(())
    }
}

async fn serve(server: Server, address: &str) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    let address = listener.local_addr().unwrap().to_string();
    (
        address,
        tokio::spawn(async move { axum::serve(listener, server.router()).await.unwrap() }),
    )
}
async fn online(client: &reqwest::Client, url: &str, id: &str) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let rows: Vec<String> = client
                .get(format!("{url}/v1/devices"))
                .bearer_auth("test-token")
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if rows.iter().any(|d| d == id) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn accepted_runtime_finishes_during_router_restart_and_replays_history_once() {
    let calls = Arc::new(AtomicUsize::new(0));
    let release = Arc::new(tokio::sync::Notify::new());
    let endpoint = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let provider_url = format!("http://{}/evaluate", endpoint.local_addr().unwrap());
    let provider = {
        let calls = calls.clone();
        let release = release.clone();
        tokio::spawn(async move {
            axum::serve(endpoint,axum::Router::new().route("/evaluate",axum::routing::post(move||{let calls=calls.clone();let release=release.clone();async move{
            calls.fetch_add(1,Ordering::SeqCst);release.notified().await;
            axum::Json(json!({"model":"jev-1.13.0","answers":{"ready":{"type":"noul","noul":0.9}},"usage":{"input_tokens":3,"output_tokens":2}}))
        }}))).await.unwrap();
        })
    };
    let root = tempfile::tempdir().unwrap();
    let db = orchid_core::product_database::open(&root.path().join("host.sqlite")).unwrap();
    let repo = Arc::new(SqliteAgentSessionRepository::from_database(db.clone()));
    let store = ReplicaStore::new(db);
    let device = store.device_id().unwrap();
    let runtime = Arc::new(JevRuntime::new(
        provider_url,
        Arc::new(|| Ok("fixture-key".into())),
        Duration::from_secs(30),
    ));
    let mut registrations = ProviderRegistrations::default();
    registrations
        .register(
            "jev",
            ProviderRegistration::new(Arc::new(JevConfigurationSource), runtime.clone(), None),
        )
        .unwrap();
    let system = Arc::new(SystemAgentSessionProviders);
    let app = Arc::new(
        AgentSessionApplication::new(
            repo.clone(),
            runtime,
            Arc::new(Notifications),
            system.clone(),
            system,
            None,
        )
        .with_execution_endpoints(Arc::new(ExecutionEndpoints::new(registrations))),
    );
    let (app, session) = tokio::task::spawn_blocking(move || {
        let session = app
            .create_jev_api_session(jev::CreateJevApiSessionInput {
                title: Some("router outage".into()),
                model: None,
            })
            .unwrap();
        (app, session)
    })
    .await
    .unwrap();
    let handler: Arc<CommandHandler> = {
        let app = app.clone();
        let session = session.id.clone();
        Arc::new(move |_| {
            let receipt=app.invoke_jev_api_session(session.clone(),AgentInvocationId::new("stable-request").unwrap(),serde_json::from_value(json!({"state":{"work":"test"},"questions":{"ready":{"type":"noul","instructions":"Is this ready?"}}})).unwrap()).map_err(|e|e.to_string())?;
            serde_json::to_value(receipt).map_err(|e| e.to_string())
        })
    };
    let server_root = tempfile::tempdir().unwrap();
    let server = Server::open(server_root.path(), "test-token".into()).unwrap();
    let (address, listener) = serve(server.clone(), "127.0.0.1:0").await;
    let connection = ServerConnection {
        url: format!("http://{address}"),
        token: "test-token".into(),
    };
    let client = reqwest::Client::new();
    let host = {
        let device = device.clone();
        let handler = handler.clone();
        let connection = connection.clone();
        tokio::spawn(async move { connect_once(&device, handler, &connection).await })
    };
    online(&client, &connection.url, &device).await;
    let submit = || {
        client
            .post(format!("{}/v1/devices/{device}/commands", connection.url))
            .bearer_auth("test-token")
            .json(&json!({"method":"evaluate","input":{}}))
    };
    assert!(submit().send().await.unwrap().status().is_success());
    tokio::time::timeout(Duration::from_secs(10), async {
        while calls.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(10)).await
        }
    })
    .await
    .unwrap();
    orchid_services::sync::synchronize_once(&client, &connection, &store, &repo)
        .await
        .unwrap();
    server.disconnect_hosts();
    let _ = tokio::time::timeout(Duration::from_secs(10), host)
        .await
        .unwrap();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let rows: Vec<String> = client
                .get(format!("{}/v1/devices", connection.url))
                .bearer_auth("test-token")
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if rows.is_empty() {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    listener.abort();
    let _ = listener.await;
    drop(server);
    release.notify_one();
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let history = repo.load_session_history(&session.id).unwrap().unwrap();
            if history.invocations[0].invocation.status == AgentInvocationStatus::Completed {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
    let restarted = Server::open(server_root.path(), "test-token".into()).unwrap();
    let (_, listener) = serve(restarted.clone(), &address).await;
    let host = {
        let device = device.clone();
        let connection = connection.clone();
        tokio::spawn(async move { connect_once(&device, handler, &connection).await })
    };
    online(&client, &connection.url, &device).await;
    assert!(submit().send().await.unwrap().status().is_success());
    orchid_services::sync::synchronize_once(&client, &connection, &store, &repo)
        .await
        .unwrap();
    let replica = restarted
        .replicas
        .get(session.id.as_str())
        .unwrap()
        .unwrap();
    assert_eq!(calls.load(Ordering::SeqCst), 1);
    assert_eq!(replica.session.history.invocations.len(), 1);
    assert_eq!(
        replica.session.history.invocations[0].invocation.status,
        AgentInvocationStatus::Completed
    );
    assert!(!replica.session.history.invocations[0].events.is_empty());
    // A second independent device can retain the finished conversation without a runtime.
    let backup = tempfile::tempdir().unwrap();
    let backup_db =
        orchid_core::product_database::open(&backup.path().join("backup.sqlite")).unwrap();
    let backup_store = ReplicaStore::new(backup_db.clone());
    let backup_repo = SqliteAgentSessionRepository::from_database(backup_db);
    orchid_services::sync::synchronize_once(&client, &connection, &backup_store, &backup_repo)
        .await
        .unwrap();
    assert!(backup_store.get(session.id.as_str()).unwrap().is_some());
    assert!(backup_repo.get_session(&session.id).unwrap().is_none());
    restarted.disconnect_hosts();
    host.abort();
    listener.abort();
    provider.abort();
    tokio::task::spawn_blocking(move || {
        app.shutdown_runtime().unwrap();
        drop(app)
    })
    .await
    .unwrap();
}
