use orchid_core::synchronization::ServerConnection;
use orchid_services::{
    host::{connect_once, CommandHandler},
    server::Server,
};
use serde_json::json;
use std::{
    sync::{
        atomic::{AtomicBool, AtomicUsize, Ordering},
        Arc,
    },
    time::Duration,
};

async fn start(server: Server) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    (
        url,
        tokio::spawn(async move { axum::serve(listener, server.router()).await.unwrap() }),
    )
}
async fn wait_connected(client: &reqwest::Client, url: &str) {
    tokio::time::timeout(Duration::from_secs(10), async {
        loop {
            let devices: Vec<String> = client
                .get(format!("{url}/v1/devices"))
                .bearer_auth("test-token")
                .send()
                .await
                .unwrap()
                .json()
                .await
                .unwrap();
            if devices.contains(&"work-host".into()) {
                break;
            }
            tokio::time::sleep(Duration::from_millis(20)).await;
        }
    })
    .await
    .unwrap();
}
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn router_disconnect_does_not_cancel_accepted_host_work_and_reconnect_can_observe_it() {
    let root = tempfile::tempdir().unwrap();
    let server = Server::open(root.path(), "test-token".into()).unwrap();
    let (url, listener) = start(server.clone()).await;
    let client = reqwest::Client::new();
    assert_eq!(
        client
            .get(format!("{url}/v1/sessions"))
            .send()
            .await
            .unwrap()
            .status(),
        reqwest::StatusCode::UNAUTHORIZED
    );
    let started = Arc::new(AtomicBool::new(false));
    let release = Arc::new(AtomicBool::new(false));
    let finished = Arc::new(AtomicUsize::new(0));
    let handler: Arc<CommandHandler> = {
        let started = started.clone();
        let release = release.clone();
        let finished = finished.clone();
        Arc::new(move |command| {
            if command.method == "status" {
                return Ok(json!(finished.load(Ordering::SeqCst)));
            }
            started.store(true, Ordering::SeqCst);
            while !release.load(Ordering::SeqCst) {
                std::thread::sleep(Duration::from_millis(5));
            }
            finished.fetch_add(1, Ordering::SeqCst);
            Ok(json!({"accepted":true}))
        })
    };
    let connection = ServerConnection {
        url: url.clone(),
        token: "test-token".into(),
    };
    let host = {
        let connection = connection.clone();
        let handler = handler.clone();
        tokio::spawn(async move { connect_once("work-host", handler, &connection).await })
    };
    wait_connected(&client, &url).await;
    let request = {
        let client = client.clone();
        let url = url.clone();
        tokio::spawn(async move {
            client
                .post(format!("{url}/v1/devices/work-host/commands"))
                .bearer_auth("test-token")
                .json(&json!({"method":"work","input":{}}))
                .send()
                .await
                .unwrap()
        })
    };
    tokio::time::timeout(Duration::from_secs(5), async {
        while !started.load(Ordering::SeqCst) {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    server.disconnect_hosts();
    assert!(!request.await.unwrap().status().is_success());
    release.store(true, Ordering::SeqCst);
    tokio::time::timeout(Duration::from_secs(5), async {
        while finished.load(Ordering::SeqCst) == 0 {
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let _ = tokio::time::timeout(Duration::from_secs(5), host)
        .await
        .unwrap();
    let host = tokio::spawn(async move { connect_once("work-host", handler, &connection).await });
    wait_connected(&client, &url).await;
    let count: usize = client
        .post(format!("{url}/v1/devices/work-host/commands"))
        .bearer_auth("test-token")
        .json(&json!({"method":"status","input":{}}))
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(count, 1);
    server.disconnect_hosts();
    host.abort();
    listener.abort();
}

#[tokio::test]
async fn local_commit_survives_offline_sync_and_is_readable_after_server_restart() {
    use orchid_core::agent_sessions::{
        domain::*, ports::AgentSessionRepository, repository::SqliteAgentSessionRepository,
    };
    use orchid_core::synchronization::repository::ReplicaStore;
    let host_root = tempfile::tempdir().unwrap();
    let db = orchid_core::product_database::open(&host_root.path().join("host.sqlite")).unwrap();
    let repository = SqliteAgentSessionRepository::from_database(db.clone());
    let store = ReplicaStore::new(db);
    let now = chrono_time();
    let session:AgentSession=serde_json::from_value(json!({"id":"session","title":"offline work","availability":"available","runtimeBinding":{"externalContextId":null,"runtimeVersion":null},"workingDirectory":null,"workspaceOrigin":null,"executionTarget":null,"requestedOptions":{},"sessionProfile":null,"harnessVersion":null,"assignedIdentity":null,"createdAt":now,"updatedAt":now})).unwrap();
    repository.create_session(session).unwrap();
    let unavailable = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let offline_url = format!("http://{}", unavailable.local_addr().unwrap());
    drop(unavailable);
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(3))
        .build()
        .unwrap();
    assert!(orchid_services::sync::synchronize_once(
        &client,
        &ServerConnection {
            url: offline_url,
            token: "test-token".into()
        },
        &store,
        &repository
    )
    .await
    .is_err());
    assert_eq!(
        repository
            .list_session_summaries(Default::default())
            .unwrap()
            .len(),
        1
    );
    let server_root = tempfile::tempdir().unwrap();
    let server = Server::open(server_root.path(), "test-token".into()).unwrap();
    let (url, listener) = start(server.clone()).await;
    let connection = ServerConnection {
        url: url.clone(),
        token: "test-token".into(),
    };
    orchid_services::sync::synchronize_once(&client, &connection, &store, &repository)
        .await
        .unwrap();
    orchid_services::sync::synchronize_once(&client, &connection, &store, &repository)
        .await
        .unwrap();
    assert_eq!(server.replicas.list().unwrap().len(), 1);
    listener.abort();
    let _ = listener.await;
    drop(server);
    let restarted = Server::open(server_root.path(), "test-token".into()).unwrap();
    let (url, listener) = start(restarted).await;
    let response: serde_json::Value = client
        .get(format!("{url}/v1/sessions/session"))
        .bearer_auth("test-token")
        .send()
        .await
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(response["session"]["title"], "offline work");
    let continuation: serde_json::Value = client
        .get(format!("{url}/v1/sessions/session/continuation"))
        .bearer_auth("test-token")
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(continuation["online"], false);
    assert_eq!(continuation["requiresFork"], true);
    let queue: serde_json::Value = client
        .post(format!("{url}/v1/sessions/session/commands"))
        .bearer_auth("test-token")
        .json(&json!({"method":"queue","input":{}}))
        .send()
        .await
        .unwrap()
        .error_for_status()
        .unwrap()
        .json()
        .await
        .unwrap();
    assert_eq!(queue["sessionId"], "session");
    let rejected = client
        .post(format!("{url}/v1/sessions/session/commands"))
        .bearer_auth("test-token")
        .json(&json!({"method":"queue","input":{"sessionId":"another"}}))
        .send()
        .await
        .unwrap();
    assert!(!rejected.status().is_success());
    listener.abort();
}
fn chrono_time() -> &'static str {
    "2026-10-16T12:00:00Z"
}
