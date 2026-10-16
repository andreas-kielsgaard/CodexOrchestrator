use crate::HostMessage;
use axum::{
    extract::{
        ws::{Message, WebSocket},
        Path, Query, State, WebSocketUpgrade,
    },
    http::{HeaderMap, StatusCode},
    response::Response,
    routing::{get, post},
    Json, Router,
};
use futures_util::{SinkExt, StreamExt};
use orchid_core::{
    agent_sessions::{
        api::{AgentSessionDetailsDto, AgentSessionSummaryDto},
        client::HostCommand,
        domain::AgentSessionId,
        ports::ListAgentSessionsQuery,
        queries::AgentSessionQueries,
        repository::SqliteAgentSessionRepository,
    },
    synchronization::{repository::ReplicaStore, SessionChange},
};
use serde::Deserialize;
use serde_json::Value;
use std::{
    collections::HashMap,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::sync::{mpsc, oneshot};

type Pending = Arc<Mutex<HashMap<String, oneshot::Sender<Result<Value, String>>>>>;
#[derive(Clone)]
struct Connection {
    id: String,
    send: mpsc::Sender<HostMessage>,
    pending: Pending,
    close: tokio::sync::watch::Sender<bool>,
}
#[derive(Clone)]
pub struct Server {
    _lease: Arc<orchid_core::instance::InstanceLease>,
    token: Arc<str>,
    hosts: Arc<Mutex<HashMap<String, Connection>>>,
    pub replicas: ReplicaStore,
    queries: AgentSessionQueries,
    definitions: orchid_core::synchronization::definitions::DefinitionStore,
    directory: orchid_core::devices::DeviceDirectory,
}
type Failure = (StatusCode, String);
fn failure(e: impl ToString) -> Failure {
    (StatusCode::BAD_REQUEST, e.to_string())
}
impl Server {
    pub fn disconnect_hosts(&self) {
        for connection in self.hosts.lock().unwrap().values() {
            let _ = connection.close.send(true);
        }
    }
    pub fn open(root: &std::path::Path, token: String) -> Result<Self, String> {
        let lease = Arc::new(orchid_core::instance::InstanceLease::acquire(
            root, "server",
        )?);
        std::fs::create_dir_all(root).map_err(|e| e.to_string())?;
        let db = orchid_core::product_database::open(&root.join("orchid.sqlite"))?;
        let replicas = ReplicaStore::new(db.clone());
        let definitions =
            orchid_core::synchronization::definitions::DefinitionStore::new(db.clone());
        let directory = orchid_core::devices::DeviceDirectory(db.clone());
        let queries =
            AgentSessionQueries::new(Arc::new(SqliteAgentSessionRepository::from_database(db)))
                .with_replicas(replicas.clone());
        Ok(Self {
            _lease: lease,
            token: token.into(),
            hosts: Default::default(),
            replicas,
            queries,
            definitions,
            directory,
        })
    }
    fn authorize(&self, headers: &HeaderMap) -> Result<(), Failure> {
        if headers.get("authorization").and_then(|v| v.to_str().ok())
            == Some(&format!("Bearer {}", self.token))
        {
            Ok(())
        } else {
            Err((
                StatusCode::UNAUTHORIZED,
                "Service authentication required".into(),
            ))
        }
    }
    pub fn router(self) -> Router {
        Router::new()
            .route("/v1/hosts/{device}", get(connect))
            .route("/v1/devices", get(devices))
            .route("/v1/device-directory", get(directory))
            .route("/v1/devices/{device}/description", post(describe))
            .route("/v1/devices/{device}/commands", post(command))
            .route("/v1/sync/{device}", post(push))
            .route("/v1/changes", get(changes))
            .route("/v1/sessions", get(sessions))
            .route("/v1/sessions/{session}", get(history))
            .route("/v1/sessions/{session}/commands", post(session_command))
            .route("/v1/sessions/{session}/continuation", get(continuation))
            .route("/v1/sessions/{session}/events/{event}", get(event_details))
            .route("/v1/replicas/{session}", get(replica))
            .route("/v1/forks", post(fork))
            .route("/v1/definitions", get(definitions::list))
            .route("/v1/definitions/{device}", post(definitions::publish))
            .route("/v1/definition-edits", post(definitions::edit))
            .route("/v1/skills", get(definitions::skills))
            .route("/v1/skills/{id}", get(definitions::skill))
            .layer(axum::extract::DefaultBodyLimit::max(64 * 1024 * 1024))
            .with_state(self)
    }
}
async fn connect(
    State(server): State<Server>,
    Path(device): Path<String>,
    headers: HeaderMap,
    ws: WebSocketUpgrade,
) -> Result<Response, Failure> {
    server.authorize(&headers)?;
    Ok(ws
        .max_message_size(64 * 1024 * 1024)
        .on_upgrade(move |socket| connection(server, device, socket)))
}
async fn connection(server: Server, device: String, socket: WebSocket) {
    let (mut writer, mut reader) = socket.split();
    let (send, mut receiver) = mpsc::channel(64);
    let id = uuid::Uuid::new_v4().to_string();
    let pending: Pending = Default::default();
    let (close, mut closed) = tokio::sync::watch::channel(false);
    if let Some(previous) = server.hosts.lock().unwrap().insert(
        device.clone(),
        Connection {
            id: id.clone(),
            send,
            pending: pending.clone(),
            close,
        },
    ) {
        let _ = previous.close.send(true);
    }
    let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
    let mut last_received = tokio::time::Instant::now();
    loop {
        tokio::select! {
            _=closed.changed()=>break,
            _=heartbeat.tick()=>{
                if last_received.elapsed()>Duration::from_secs(45){break;}
                if !matches!(tokio::time::timeout(Duration::from_secs(5),writer.send(Message::Ping(Vec::new().into()))).await,Ok(Ok(()))){break;}
            },
            message=receiver.recv()=>match message {
                Some(message)=>{let Ok(text)=serde_json::to_string(&message)else{break};if !matches!(tokio::time::timeout(Duration::from_secs(5),writer.send(Message::Text(text.into()))).await,Ok(Ok(()))){break;}}
                None=>break,
            },
            message=reader.next()=>match message {
                Some(Ok(Message::Text(text)))=>{
                    last_received=tokio::time::Instant::now();
                    if let Ok(HostMessage::Reply{id,result})=serde_json::from_str(&text){if let Some(reply)=pending.lock().unwrap().remove(&id){let _=reply.send(result);}}
                },
                Some(Ok(Message::Ping(data)))=>{last_received=tokio::time::Instant::now();if !matches!(tokio::time::timeout(Duration::from_secs(5),writer.send(Message::Pong(data))).await,Ok(Ok(()))){break;}},
                Some(Ok(Message::Pong(_)))=>{last_received=tokio::time::Instant::now();},
                Some(Ok(Message::Close(_)))|None|Some(Err(_))=>break,
                _=>{},
            }
        }
    }
    let _ = tokio::time::timeout(Duration::from_secs(1), writer.send(Message::Close(None))).await;
    let mut hosts = server.hosts.lock().unwrap();
    if hosts
        .get(&device)
        .is_some_and(|connection| connection.id == id)
    {
        hosts.remove(&device);
    }
    for (_, reply) in pending.lock().unwrap().drain() {
        let _=reply.send(Err("Host disconnected; command outcome may be unknown. Check or retry with the original submission identity.".into()));
    }
}
async fn command(
    State(server): State<Server>,
    Path(device): Path<String>,
    headers: HeaderMap,
    Json(command): Json<HostCommand>,
) -> Result<Json<Value>, Failure> {
    server.authorize(&headers)?;
    let connection = server.hosts.lock().unwrap().get(&device).cloned().ok_or((
        StatusCode::SERVICE_UNAVAILABLE,
        "Execution device is offline; fork to another target to continue elsewhere".into(),
    ))?;
    let id = uuid::Uuid::new_v4().to_string();
    let (reply, result) = oneshot::channel();
    connection.pending.lock().unwrap().insert(id.clone(), reply);
    if connection
        .send
        .try_send(HostMessage::Command {
            id: id.clone(),
            command,
        })
        .is_err()
    {
        connection.pending.lock().unwrap().remove(&id);
        return Err((
            StatusCode::SERVICE_UNAVAILABLE,
            "Host disconnected or busy before dispatch".into(),
        ));
    }
    let outcome = tokio::time::timeout(Duration::from_secs(60), result).await;
    connection.pending.lock().unwrap().remove(&id);
    outcome
        .map_err(|_| {
            (
                StatusCode::GATEWAY_TIMEOUT,
                "Host acknowledgement timed out; outcome is unknown".into(),
            )
        })?
        .map_err(|_| failure("Host disconnected"))?
        .map(Json)
        .map_err(failure)
}
async fn devices(
    State(server): State<Server>,
    headers: HeaderMap,
) -> Result<Json<Vec<String>>, Failure> {
    server.authorize(&headers)?;
    Ok(Json(server.hosts.lock().unwrap().keys().cloned().collect()))
}

async fn continuation(
    State(server): State<Server>,
    Path(session): Path<String>,
    headers: HeaderMap,
) -> Result<Json<Value>, Failure> {
    server.authorize(&headers)?;
    let replica = server
        .replicas
        .get(&session)
        .map_err(failure)?
        .ok_or((StatusCode::NOT_FOUND, "Session is not synchronized".into()))?;
    let online = server
        .hosts
        .lock()
        .unwrap()
        .contains_key(&replica.device_id);
    let interrupted = replica.session.history.invocations.last().is_some_and(|t| {
        t.invocation.status
            == orchid_core::agent_sessions::domain::AgentInvocationStatus::Interrupted
    });
    Ok(Json(
        serde_json::json!({"deviceId":replica.device_id,"online":online,"requiresFork":!online||interrupted,"fork":replica.session.fork}),
    ))
}

async fn session_command(
    State(server): State<Server>,
    Path(session): Path<String>,
    headers: HeaderMap,
    Json(mut request): Json<HostCommand>,
) -> Result<Json<Value>, Failure> {
    server.authorize(&headers)?;
    let input = request
        .input
        .as_object_mut()
        .ok_or_else(|| failure("Command input must be an object"))?;
    if input
        .get("sessionId")
        .is_some_and(|id| id.as_str() != Some(&session))
    {
        return Err(failure("Command addresses a different session"));
    }
    input.insert("sessionId".into(), serde_json::json!(session));
    let replica = server
        .replicas
        .get(&session)
        .map_err(failure)?
        .ok_or((StatusCode::NOT_FOUND, "Session is not synchronized".into()))?;
    // Observation reads continue to work while the execution device is offline.
    let read = match request.method.as_str() {
        "queue" => Some(serde_json::to_value(replica.session.queue_view())),
        "preparation" => Some(serde_json::to_value(replica.session.preparations.last())),
        "interactions" => Some(serde_json::to_value(
            orchid_core::agent_sessions::interactions::project_interactions(
                &replica.session.history,
            ),
        )),
        _ => None,
    };
    if let Some(read) = read {
        return read.map(Json).map_err(failure);
    }
    command(
        State(server),
        Path(replica.device_id),
        headers,
        Json(request),
    )
    .await
}
async fn push(
    State(server): State<Server>,
    Path(device): Path<String>,
    headers: HeaderMap,
    Json(change): Json<SessionChange>,
) -> Result<Json<bool>, Failure> {
    server.authorize(&headers)?;
    server
        .replicas
        .accept(&device, &change)
        .map(Json)
        .map_err(failure)
}
#[derive(Deserialize)]
struct Cursor {
    #[serde(default)]
    after: u64,
}
async fn changes(
    State(server): State<Server>,
    headers: HeaderMap,
    Query(cursor): Query<Cursor>,
) -> Result<Json<Vec<(u64, SessionChange)>>, Failure> {
    server.authorize(&headers)?;
    server
        .replicas
        .changes(cursor.after)
        .map(Json)
        .map_err(failure)
}
async fn sessions(
    State(server): State<Server>,
    headers: HeaderMap,
) -> Result<Json<Vec<AgentSessionSummaryDto>>, Failure> {
    server.authorize(&headers)?;
    server
        .queries
        .list_sessions(ListAgentSessionsQuery::default())
        .map(|rows| Json(rows.into_iter().map(Into::into).collect()))
        .map_err(failure)
}
async fn history(
    State(server): State<Server>,
    Path(session): Path<String>,
    headers: HeaderMap,
) -> Result<Json<AgentSessionDetailsDto>, Failure> {
    server.authorize(&headers)?;
    server
        .queries
        .load_session(&AgentSessionId::new(session).map_err(failure)?)
        .map(AgentSessionDetailsDto::from_history)
        .map(Json)
        .map_err(failure)
}
async fn event_details(
    State(server): State<Server>,
    Path((session, event)): Path<(String, String)>,
    headers: HeaderMap,
) -> Result<Json<orchid_core::agent_sessions::domain::AgentRuntimeEvent>, Failure> {
    server.authorize(&headers)?;
    server
        .queries
        .event_details(&AgentSessionId::new(session).map_err(failure)?, &event)
        .map(Json)
        .map_err(failure)
}
async fn replica(
    State(server): State<Server>,
    Path(session): Path<String>,
    headers: HeaderMap,
) -> Result<Json<SessionChange>, Failure> {
    server.authorize(&headers)?;
    server
        .replicas
        .get(&session)
        .map_err(failure)?
        .map(Json)
        .ok_or((StatusCode::NOT_FOUND, "Retained session not found".into()))
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ForkInput {
    source_session_id: String,
    fork_id: String,
    target: orchid_core::execution_targets::domain::SessionExecutionTarget,
}
async fn fork(
    State(server): State<Server>,
    headers: HeaderMap,
    Json(input): Json<ForkInput>,
) -> Result<Json<Value>, Failure> {
    server.authorize(&headers)?;
    let prefix = server
        .replicas
        .get(&input.source_session_id)
        .map_err(failure)?
        .and_then(|r| r.session.initial_prompt_prefix);
    let history = server
        .queries
        .load_session(&AgentSessionId::new(input.source_session_id).map_err(failure)?)
        .map_err(failure)?;
    let device = input.target.execution.device_id.clone();
    let request = HostCommand {
        method: "fork".into(),
        input: serde_json::json!({"forkId":input.fork_id,"history":history,"target":input.target,"initialPromptPrefix":prefix}),
    };
    command(State(server), Path(device), headers, Json(request)).await
}
mod definitions;
async fn directory(
    State(server): State<Server>,
    headers: HeaderMap,
) -> Result<Json<Vec<Value>>, Failure> {
    server.authorize(&headers)?;
    let descriptions = server.directory.list().map_err(failure)?;
    let hosts = server.hosts.lock().unwrap();
    Ok(Json(descriptions.into_iter().map(|description|serde_json::json!({"online":hosts.contains_key(&description.id),"description":description})).collect()))
}
async fn describe(
    State(server): State<Server>,
    Path(device): Path<String>,
    headers: HeaderMap,
    Json(description): Json<orchid_core::synchronization::DeviceDescription>,
) -> Result<Json<()>, Failure> {
    server.authorize(&headers)?;
    if description.id != device {
        return Err(failure("Device identity mismatch"));
    }
    server
        .directory
        .record(&description)
        .map(Json)
        .map_err(failure)
}
