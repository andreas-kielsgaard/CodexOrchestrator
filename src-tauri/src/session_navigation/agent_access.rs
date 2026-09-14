//! Local agent commands round-trip through the mounted UI, not a second navigation state.
use crate::agent_sessions::organization::{SessionFolderTarget, SessionPlacement};
use axum::{
    extract::State,
    http::{HeaderMap, StatusCode},
    routing::post,
    Json, Router,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{
    collections::HashMap,
    path::Path,
    sync::{Arc, Mutex},
    time::Duration,
};
use tauri::{Emitter, Manager};
use tokio::sync::{oneshot, Mutex as AsyncMutex};

#[derive(Clone, Deserialize, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
enum NavigationCommand {
    Inspect,
    OpenSession {
        session_id: String,
    },
    NewSession {
        folder_target: Option<SessionFolderTarget>,
    },
    SetFolderExpanded {
        folder_id: String,
        expanded: bool,
    },
    ShowMore {
        folder_id: String,
    },
    MoveSession {
        session_id: String,
        placement: SessionPlacement,
    },
    PinSession {
        session_id: String,
        pinned: bool,
    },
    ReorderNavigation {
        scope: super::order::NavigationOrderScope,
        ordered_ids: Vec<String>,
    },
    GetDeeplink {
        session_id: String,
    },
}

type Reply = Result<Value, String>;
pub(crate) struct NavigationAgentBridge {
    app: tauri::AppHandle,
    token: String,
    pending: Mutex<HashMap<String, oneshot::Sender<Reply>>>,
    command_lock: AsyncMutex<()>,
}

pub(crate) fn start(app: &tauri::AppHandle, app_data: &Path) -> Result<(), String> {
    let listener = std::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, 0))
        .map_err(|error| error.to_string())?;
    listener
        .set_nonblocking(true)
        .map_err(|error| error.to_string())?;
    let endpoint = format!(
        "http://{}/commands",
        listener.local_addr().map_err(|error| error.to_string())?
    );
    let bridge = Arc::new(NavigationAgentBridge {
        app: app.clone(),
        token: uuid::Uuid::new_v4().to_string(),
        pending: Mutex::new(HashMap::new()),
        command_lock: AsyncMutex::new(()),
    });
    std::fs::write(
        app_data.join("agent-session-navigation.json"),
        serde_json::to_vec_pretty(
            &json!({"version":1,"pid":std::process::id(),"endpoint":endpoint,"token":bridge.token}),
        )
        .map_err(|error| error.to_string())?,
    )
    .map_err(|error| error.to_string())?;
    app.manage(bridge.clone());
    tauri::async_runtime::spawn(async move {
        let router = Router::new()
            .route("/commands", post(command))
            .with_state(bridge);
        match tokio::net::TcpListener::from_std(listener) {
            Ok(listener) => {
                if let Err(error) = axum::serve(listener, router).await {
                    eprintln!("Session navigation agent server: {error}");
                }
            }
            Err(error) => eprintln!("Session navigation agent listener: {error}"),
        }
    });
    Ok(())
}

async fn command(
    State(bridge): State<Arc<NavigationAgentBridge>>,
    headers: HeaderMap,
    Json(command): Json<NavigationCommand>,
) -> (StatusCode, Json<Value>) {
    if headers
        .get("authorization")
        .and_then(|value| value.to_str().ok())
        != Some(&format!("Bearer {}", bridge.token))
    {
        return (
            StatusCode::UNAUTHORIZED,
            Json(json!({"error":"Local navigation token required"})),
        );
    }
    let _guard = bridge.command_lock.lock().await;
    let id = uuid::Uuid::new_v4().to_string();
    let (send, receive) = oneshot::channel();
    bridge.pending.lock().unwrap().insert(id.clone(), send);
    if let Err(error) = bridge.app.emit_to(
        "main",
        "session-navigation-command",
        json!({"id":id,"command":command}),
    ) {
        bridge.pending.lock().unwrap().remove(&id);
        return (
            StatusCode::INTERNAL_SERVER_ERROR,
            Json(json!({"error":error.to_string()})),
        );
    }
    let result = tokio::time::timeout(Duration::from_secs(15), receive).await;
    bridge.pending.lock().unwrap().remove(&id);
    match result {
        Ok(Ok(Ok(state))) => (StatusCode::OK, Json(state)),
        Ok(Ok(Err(error))) => (StatusCode::BAD_REQUEST, Json(json!({"error":error}))),
        _ => (
            StatusCode::GATEWAY_TIMEOUT,
            Json(
                json!({"error":"The Agent Sessions UI did not respond. Wait for the app to load before issuing commands."}),
            ),
        ),
    }
}

#[tauri::command]
pub(crate) fn complete_session_navigation_command(
    bridge: tauri::State<'_, Arc<NavigationAgentBridge>>,
    request_id: String,
    state: Option<Value>,
    error: Option<String>,
) {
    if let Some(reply) = bridge.pending.lock().unwrap().remove(&request_id) {
        let _ = reply.send(match error {
            Some(error) => Err(error),
            None => Ok(state.unwrap_or(Value::Null)),
        });
    }
}
