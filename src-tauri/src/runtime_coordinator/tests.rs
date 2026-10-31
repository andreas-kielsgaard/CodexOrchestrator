use super::*;
use crate::{
    execution_devices::{ExecutionDeviceService, SaveExecutionDeviceInput, SCHEMA},
    persistence::ActiveDatabase,
};
use rusqlite::Connection;

fn runtime(root: PathBuf) -> RuntimeCoordinator {
    let database = Arc::new(
        ActiveDatabase::from_connection(Connection::open_in_memory().unwrap(), |db| {
            db.execute_batch(SCHEMA).map_err(|e| e.to_string())
        })
        .unwrap(),
    );
    let devices = Arc::new(ExecutionDeviceService::new(database.clone()));
    RuntimeCoordinator::new(root, database, devices).unwrap()
}

#[tokio::test]
async fn configured_external_failure_does_not_start_local_hosting() {
    let root = tempfile::tempdir().unwrap();
    let credentials = Credentials::new(&root.path().join("client-credentials")).unwrap();
    let credential = credentials.save("isolated-test-access-token").unwrap();
    let settings = Settings {
        external_url: Some("http://127.0.0.1:1".into()),
        credential_ref: Some(credential),
    };
    fs::write(
        root.path().join("connection.json"),
        serde_json::to_vec(&settings).unwrap(),
    )
    .unwrap();
    let runtime = runtime(root.path().into());
    let overview = runtime.overview().await.unwrap();
    assert_eq!(overview.mode, "external");
    assert!(overview.error.is_some());
    assert!(!root.path().join("local").exists());
}

#[tokio::test]
async fn active_associated_work_blocks_the_coordinator_stop_route() {
    let root = tempfile::tempdir().unwrap();
    let runtime = runtime(root.path().join("desktop"));
    let service = orchid_coordinator::service::Service::open(
        &root.path().join("server"),
        "http://127.0.0.1:1",
    )
    .unwrap();
    let identity = service.health().unwrap().coordinator_id;
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let router = orchid_coordinator::http::router(service.clone(), "test-access-token".into());
    let task = tokio::spawn(async move {
        axum::serve(listener, router).await.unwrap();
    });
    runtime
        .settings(SettingsInput {
            external_url: Some(url.clone()),
            token: Some("test-access-token".into()),
        })
        .await
        .unwrap();
    runtime
        .settings(SettingsInput {
            external_url: Some(url.clone()),
            token: None,
        })
        .await
        .unwrap();
    let saved: Settings =
        serde_json::from_slice(&fs::read(root.path().join("desktop/connection.json")).unwrap())
            .unwrap();
    assert_eq!(saved.external_url.as_deref(), Some(url.as_str()));
    assert!(saved.credential_ref.is_some());
    let input:SaveExecutionDeviceInput=serde_json::from_value(serde_json::json!({"deviceId":"worker","displayName":"Worker","lifecycle":{"start":null,"stop":null,"idleShutdownSeconds":null}})).unwrap();
    runtime.devices.save(input).unwrap();
    runtime
        .bindings
        .associate(&repository::Binding {
            coordinator_id: identity,
            device_id: "cloud-device".into(),
            execution_device_id: "worker".into(),
        })
        .unwrap();
    runtime
        .devices
        .acquire_activity("worker", "agent_invocation", "invocation")
        .unwrap();
    let error = runtime
        .power(
            "cloud-device".into(),
            PowerRequest {
                operation_id: uuid::Uuid::new_v4().to_string(),
                command: PowerCommand::Stop,
            },
        )
        .await
        .unwrap_err();
    assert!(error.contains("Orchid-managed work is active"));
    assert!(service.repository.list().unwrap().is_empty());
    task.abort();
}
