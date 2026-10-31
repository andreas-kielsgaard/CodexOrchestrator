use axum::{
    extract::{Path, State},
    http::{HeaderMap, StatusCode},
    routing::{get, post},
    Json, Router,
};
use orchid_coordinator::{client::Client, http, protocol::*, service::Service};
use serde_json::{json, Value};
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    Arc, Mutex,
};

#[derive(Default)]
struct Cloud {
    calls: AtomicUsize,
    power: Mutex<String>,
    action: Mutex<String>,
    uncertain: AtomicUsize,
}
fn server(cloud: &Cloud) -> Value {
    json!({"id":42,"name":"worker","status":*cloud.power.lock().unwrap(),"public_net":{"ipv4":{"ip":"192.0.2.42"}},"location":{"name":"hel1"}})
}
fn authorized(headers: &HeaderMap) -> bool {
    headers.get("authorization").and_then(|v| v.to_str().ok()) == Some("Bearer test-cloud-token")
}
async fn servers(
    State(cloud): State<Arc<Cloud>>,
    headers: HeaderMap,
) -> Result<Json<Value>, StatusCode> {
    if !authorized(&headers) {
        return Err(StatusCode::UNAUTHORIZED);
    }
    Ok(Json(
        json!({"servers":[server(&cloud)],"meta":{"pagination":{"next_page":null}}}),
    ))
}
async fn detail(State(cloud): State<Arc<Cloud>>) -> Json<Value> {
    Json(json!({"server":server(&cloud)}))
}
async fn submit(
    State(cloud): State<Arc<Cloud>>,
    Path((_id, command)): Path<(u64, String)>,
) -> Result<Json<Value>, StatusCode> {
    cloud.calls.fetch_add(1, Ordering::SeqCst);
    *cloud.power.lock().unwrap() = if command == "poweron" {
        "running"
    } else {
        "off"
    }
    .into();
    if cloud.uncertain.load(Ordering::SeqCst) == 2 {
        Err(StatusCode::SERVICE_UNAVAILABLE)
    } else if cloud.uncertain.load(Ordering::SeqCst) == 1 {
        Ok(Json(json!({"action":{}})))
    } else {
        Ok(Json(json!({"action":{"id":99}})))
    }
}
async fn action(State(cloud): State<Arc<Cloud>>) -> Json<Value> {
    Json(json!({"action":{"status":*cloud.action.lock().unwrap()}}))
}
async fn serve(router: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    (url, task)
}
async fn fixture() -> (
    tempfile::TempDir,
    Arc<Cloud>,
    String,
    tokio::task::JoinHandle<()>,
    Arc<Service>,
    Device,
) {
    let root = tempfile::tempdir().unwrap();
    let cloud = Arc::new(Cloud::default());
    *cloud.power.lock().unwrap() = "off".into();
    *cloud.action.lock().unwrap() = "running".into();
    let router = Router::new()
        .route("/servers", get(servers))
        .route("/servers/{id}", get(detail))
        .route("/servers/{id}/actions/{command}", post(submit))
        .route("/actions/{id}", get(action))
        .with_state(cloud.clone());
    let (url, task) = serve(router).await;
    let service = Service::open(root.path(), &url).unwrap();
    let found = service
        .connect(Connect {
            token: "test-cloud-token".into(),
        })
        .await
        .unwrap();
    assert_eq!(found.servers[0].id, 42);
    let device = service
        .register(Register {
            credential_id: found.credential_id,
            server_id: 42,
            display_name: "Linux worker".into(),
        })
        .await
        .unwrap();
    (root, cloud, url, task, service, device)
}
fn request(command: PowerCommand) -> PowerRequest {
    PowerRequest {
        operation_id: uuid::Uuid::new_v4().to_string(),
        command,
    }
}

#[tokio::test]
async fn restart_observes_receipt_and_duplicate_never_resubmits() {
    let (root, cloud, url, task, service, device) = fixture().await;
    let identity = service.health().unwrap().coordinator_id;
    let input = request(PowerCommand::Start);
    let operation = service.power(&device.id, input.clone()).await.unwrap();
    assert_eq!(operation.status, "running");
    assert_eq!(operation.provider_action_id, Some(99));
    drop(service);
    let resumed = Service::open(root.path(), &url).unwrap();
    assert_eq!(resumed.health().unwrap().coordinator_id, identity);
    assert_eq!(
        resumed.power(&device.id, input.clone()).await.unwrap().id,
        input.operation_id
    );
    assert!(resumed
        .power(&device.id, request(PowerCommand::Stop))
        .await
        .is_err());
    *cloud.action.lock().unwrap() = "success".into();
    let refreshed = resumed.refresh(&device.id).await.unwrap();
    assert_eq!(refreshed.operation.unwrap().status, "succeeded");
    assert_eq!(refreshed.server.status, "running");
    assert_eq!(cloud.calls.load(Ordering::SeqCst), 1);
    let mut wrong = input;
    wrong.command = PowerCommand::Stop;
    assert!(resumed.power(&device.id, wrong).await.is_err());
    task.abort();
}
#[tokio::test]
async fn uncertain_submission_is_observed_without_replay() {
    let (_root, cloud, _url, task, service, device) = fixture().await;
    cloud.uncertain.store(1, Ordering::SeqCst);
    let input = request(PowerCommand::Start);
    assert_eq!(
        service
            .power(&device.id, input.clone())
            .await
            .unwrap()
            .status,
        "unconfirmed"
    );
    assert!(service
        .power(&device.id, request(PowerCommand::Start))
        .await
        .is_err());
    let refreshed = service.refresh(&device.id).await.unwrap();
    assert_eq!(refreshed.operation.unwrap().status, "observed");
    assert_eq!(
        service.power(&device.id, input).await.unwrap().status,
        "observed"
    );
    assert_eq!(cloud.calls.load(Ordering::SeqCst), 1);
    task.abort();
}

#[tokio::test]
async fn provider_server_error_is_unconfirmed_and_stale_power_controls_do_not_resubmit() {
    let (_root, cloud, _url, task, service, device) = fixture().await;
    cloud.uncertain.store(2, Ordering::SeqCst);
    assert_eq!(
        service
            .power(&device.id, request(PowerCommand::Start))
            .await
            .unwrap()
            .status,
        "unconfirmed"
    );
    assert_eq!(
        service
            .refresh(&device.id)
            .await
            .unwrap()
            .operation
            .unwrap()
            .status,
        "observed"
    );
    assert!(service
        .power(&device.id, request(PowerCommand::Start))
        .await
        .is_err());
    assert_eq!(cloud.calls.load(Ordering::SeqCst), 1);
    task.abort();
}
#[tokio::test]
async fn discovery_rejects_bad_tokens_without_leaking_them_and_registration_does_not_power() {
    let (root, cloud, _url, task, service, device) = fixture().await;
    let error = service
        .connect(Connect {
            token: "private-rejected-secret".into(),
        })
        .await
        .unwrap_err();
    assert!(!error.contains("private-rejected-secret"));
    assert!(error.contains("401"));
    assert_eq!(cloud.calls.load(Ordering::SeqCst), 0);
    let discovery = service
        .connect(Connect {
            token: "test-cloud-token".into(),
        })
        .await
        .unwrap();
    assert!(service
        .register(Register {
            credential_id: discovery.credential_id,
            server_id: 42,
            display_name: "duplicate".into()
        })
        .await
        .is_err());
    assert!(!serde_json::to_string(&device)
        .unwrap()
        .contains("test-cloud-token"));
    let bytes = std::fs::read(root.path().join("coordinator.sqlite")).unwrap();
    assert!(!bytes.windows(16).any(|part| part == b"test-cloud-token"));
    task.abort();
}
#[tokio::test]
async fn authenticated_http_contract_and_offline_endpoint_do_not_fall_back() {
    let (_root, _cloud, _url, cloud_task, service, _device) = fixture().await;
    let (url, task) = serve(http::router(service, "coordinator-access-token".into())).await;
    assert_eq!(
        reqwest::get(format!("{url}/api/v1/health"))
            .await
            .unwrap()
            .status(),
        StatusCode::UNAUTHORIZED
    );
    let client = Client::new(&url, "coordinator-access-token").unwrap();
    assert_eq!(client.health().await.unwrap().contract_version, VERSION);
    assert_eq!(client.list().await.unwrap().len(), 1);
    assert!(Client::new("http://example.com", "token").is_err());
    assert!(Client::new("https://user:secret@example.com", "token").is_err());
    task.abort();
    cloud_task.abort();
    assert!(client.health().await.is_err());
}
#[tokio::test]
async fn concurrent_observation_does_not_overwrite_a_submission() {
    let (_root, cloud, _url, task, service, device) = fixture().await;
    let (power, observation) = tokio::join!(
        service.power(&device.id, request(PowerCommand::Start)),
        service.refresh(&device.id)
    );
    assert_eq!(power.unwrap().provider_action_id, Some(99));
    assert_eq!(observation.unwrap().operation.unwrap().status, "running");
    assert_eq!(cloud.calls.load(Ordering::SeqCst), 1);
    task.abort();
}

#[tokio::test]
async fn explicit_stop_requests_shutdown_and_observes_off() {
    let (_root, cloud, _url, task, service, device) = fixture().await;
    *cloud.power.lock().unwrap() = "running".into();
    let operation = service.power(&device.id, request(PowerCommand::Stop)).await.unwrap();
    assert_eq!(operation.command, PowerCommand::Stop);
    assert_eq!(operation.status, "running");
    *cloud.action.lock().unwrap() = "success".into();
    let observed = service.refresh(&device.id).await.unwrap();
    assert_eq!(observed.server.status, "off");
    assert_eq!(observed.operation.unwrap().status, "succeeded");
    assert_eq!(cloud.calls.load(Ordering::SeqCst), 1);
    task.abort();
}
