use crate::HostMessage;
use futures_util::{SinkExt, StreamExt};
use orchid_core::{
    agent_sessions::{
        application::AgentSessionApplication, client::HostCommand,
        repository::SqliteAgentSessionRepository,
    },
    persistence::ActiveDatabase,
    synchronization::{repository::ReplicaStore, ServerConnection},
};
use std::{sync::Arc, time::Duration};
use tokio_tungstenite::tungstenite::{client::IntoClientRequest, Message};

pub type CommandHandler = dyn Fn(HostCommand) -> Result<serde_json::Value, String> + Send + Sync;
pub async fn connect_once(
    device: &str,
    handler: Arc<CommandHandler>,
    connection: &ServerConnection,
) -> Result<(), String> {
    let mut url = reqwest::Url::parse(&connection.url).map_err(|e| e.to_string())?;
    let scheme = match url.scheme() {
        "https" => "wss",
        "http" => "ws",
        _ => return Err("Server URL must use http or https".into()),
    };
    url.set_scheme(scheme).map_err(|_| "Invalid server URL")?;
    url.set_path(&format!("/v1/hosts/{device}"));
    let mut request = url
        .as_str()
        .into_client_request()
        .map_err(|e| e.to_string())?;
    request.headers_mut().insert(
        "authorization",
        format!("Bearer {}", connection.token)
            .parse()
            .map_err(|_| "Invalid service token")?,
    );
    let (socket, _) = tokio::time::timeout(
        Duration::from_secs(15),
        tokio_tungstenite::connect_async(request),
    )
    .await
    .map_err(|_| "Router connection timed out")?
    .map_err(|e| e.to_string())?;
    let (mut writer, mut reader) = socket.split();
    let (send, mut replies) = tokio::sync::mpsc::channel::<HostMessage>(64);
    let mut heartbeat = tokio::time::interval(Duration::from_secs(15));
    let mut last_received = tokio::time::Instant::now();
    loop {
        tokio::select! {
            _=heartbeat.tick()=>{
                if last_received.elapsed()>Duration::from_secs(45){return Err("Router heartbeat expired".into());}
                tokio::time::timeout(Duration::from_secs(5),writer.send(Message::Ping(Vec::new().into()))).await.map_err(|_|"Router write timed out")?.map_err(|e|e.to_string())?;
            },
            reply=replies.recv()=>{if let Some(reply)=reply{tokio::time::timeout(Duration::from_secs(5),writer.send(Message::Text(serde_json::to_string(&reply).map_err(|e|e.to_string())?.into()))).await.map_err(|_|"Router write timed out")?.map_err(|e|e.to_string())?;}},
            message=reader.next()=>match message {
                Some(Ok(Message::Text(text)))=>{last_received=tokio::time::Instant::now();if let Ok(HostMessage::Command{id,command})=serde_json::from_str(&text){
                    let handler=handler.clone();let send=send.clone();
                    tokio::spawn(async move {
                        let result=tokio::task::spawn_blocking(move||handler(command)).await.map_err(|e|e.to_string()).and_then(|r|r);
                        let _=send.send(HostMessage::Reply{id,result}).await;
                    });
                }},
                Some(Ok(Message::Ping(data)))=>{last_received=tokio::time::Instant::now();tokio::time::timeout(Duration::from_secs(5),writer.send(Message::Pong(data))).await.map_err(|_|"Router write timed out")?.map_err(|e|e.to_string())?;},
                Some(Ok(Message::Pong(_)))=>{last_received=tokio::time::Instant::now();},
                Some(Ok(Message::Close(_)))|None=>return Err("Router connection closed".into()),
                Some(Err(error))=>return Err(error.to_string()),
                _=>{},
            }
        }
    }
}
pub async fn run_connections(
    database: Arc<ActiveDatabase>,
    repository: Arc<SqliteAgentSessionRepository>,
    application: Arc<AgentSessionApplication>,
    connection: ServerConnection,
) {
    run_connections_with_observer(
        database,
        repository,
        application,
        connection,
        Arc::new(|_| {}),
    )
    .await
}
pub async fn run_connections_with_observer(
    database: Arc<ActiveDatabase>,
    repository: Arc<SqliteAgentSessionRepository>,
    application: Arc<AgentSessionApplication>,
    connection: ServerConnection,
    observer: Arc<dyn Fn(Vec<String>) + Send + Sync>,
) {
    if let Ok(targets) = application.execution_targets() {
        targets
            .endpoints
            .attach_device_transport(Arc::new(RemoteHttpTransport(connection.clone())));
    }
    let directory = orchid_core::devices::DeviceDirectory(database.clone());
    let store = ReplicaStore::new(database);
    let Ok(device) = store.device_id() else {
        return;
    };
    let handler_repository = repository.clone();
    let handler_device = device.clone();
    let handler: Arc<CommandHandler> = Arc::new(move |command| {
        orchid_core::agent_sessions::client::dispatch_on_device(
            &handler_device,
            &application,
            &handler_repository,
            command,
        )
    });
    let describe_handler = handler.clone();
    let sync_device = device.clone();
    let sync_connection = connection.clone();
    let sync = async move {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("HTTP client");
        let mut advertised = std::time::Instant::now() - Duration::from_secs(60);
        loop {
            if advertised.elapsed() > Duration::from_secs(30) {
                let handler = describe_handler.clone();
                let refresh = handler.clone();
                let _ = tokio::task::spawn_blocking(move || {
                    refresh(HostCommand {
                        method: "refreshSkills".into(),
                        input: serde_json::json!({}),
                    })
                })
                .await;
                if let Ok(Ok(description)) = tokio::task::spawn_blocking(move || {
                    handler(HostCommand {
                        method: "device.description".into(),
                        input: serde_json::json!({}),
                    })
                })
                .await
                {
                    if let Ok(description) = serde_json::from_value::<
                        orchid_core::synchronization::DeviceDescription,
                    >(description)
                    {
                        let _ = directory.record(&description);
                        let _ = client
                            .post(format!(
                                "{}/v1/devices/{sync_device}/description",
                                sync_connection.url.trim_end_matches('/')
                            ))
                            .bearer_auth(&sync_connection.token)
                            .json(&description)
                            .send()
                            .await;
                    }
                }
                advertised = std::time::Instant::now();
                let response = client
                    .get(format!(
                        "{}/v1/device-directory",
                        sync_connection.url.trim_end_matches('/')
                    ))
                    .bearer_auth(&sync_connection.token)
                    .send()
                    .await;
                if let Ok(response) = response {
                    if let Ok(rows) = response.json::<Vec<serde_json::Value>>().await {
                        for row in rows {
                            if let Ok(description) =
                                serde_json::from_value(row["description"].clone())
                            {
                                let _ = directory.record(&description);
                            }
                        }
                    }
                }
            }
            match crate::sync::synchronize_once(&client, &sync_connection, &store, &repository)
                .await
            {
                Ok(updated) if !updated.is_empty() => observer(updated),
                Err(error) => eprintln!("Synchronization pending: {error}"),
                _ => {}
            }
            if let Err(error) = crate::sync::synchronize_definitions_once(
                &client,
                &sync_connection,
                &repository.definitions(),
            )
            .await
            {
                eprintln!("Definition synchronization pending: {error}");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    };
    let routing = async move {
        loop {
            if let Err(error) = connect_once(&device, handler.clone(), &connection).await {
                eprintln!("Router unavailable; local execution continues: {error}");
            }
            tokio::time::sleep(Duration::from_secs(2)).await;
        }
    };
    tokio::join!(sync, routing);
}

pub struct RemoteHttpTransport(pub ServerConnection);
impl orchid_core::synchronization::SessionReplicaSource for RemoteHttpTransport {
    fn fetch_session(
        &self,
        id: &str,
    ) -> Result<orchid_core::synchronization::SessionChange, String> {
        let mut url = reqwest::Url::parse(&self.0.url).map_err(|e| e.to_string())?;
        url.path_segments_mut()
            .map_err(|_| "Invalid server URL")?
            .extend(["v1", "replicas", id]);
        reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .map_err(|e| e.to_string())?
            .get(url)
            .bearer_auth(&self.0.token)
            .send()
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?
            .json()
            .map_err(|e| e.to_string())
    }
}
impl orchid_core::agent_sessions::client::DeviceCommandTransport for RemoteHttpTransport {
    fn devices(
        &self,
    ) -> Result<Vec<(orchid_core::synchronization::DeviceDescription, bool)>, String> {
        #[derive(serde::Deserialize)]
        struct Entry {
            description: orchid_core::synchronization::DeviceDescription,
            online: bool,
        }
        let response = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(3))
            .build()
            .map_err(|e| e.to_string())?
            .get(format!(
                "{}/v1/device-directory",
                self.0.url.trim_end_matches('/')
            ))
            .bearer_auth(&self.0.token)
            .send()
            .map_err(|e| e.to_string())?
            .error_for_status()
            .map_err(|e| e.to_string())?;
        response
            .json::<Vec<Entry>>()
            .map(|rows| {
                rows.into_iter()
                    .map(|row| (row.description, row.online))
                    .collect()
            })
            .map_err(|e| e.to_string())
    }
    fn execute(&self, device_id: &str, command: HostCommand) -> Result<serde_json::Value, String> {
        let mut url = reqwest::Url::parse(&self.0.url).map_err(|e| e.to_string())?;
        url.path_segments_mut()
            .map_err(|_| "Invalid server URL")?
            .extend(["v1", "devices", device_id, "commands"]);
        let response = reqwest::blocking::Client::builder()
            .timeout(Duration::from_secs(65))
            .build()
            .map_err(|e| e.to_string())?
            .post(url)
            .bearer_auth(&self.0.token)
            .json(&command)
            .send()
            .map_err(|e| e.to_string())?;
        if !response.status().is_success() {
            return Err(response
                .text()
                .unwrap_or_else(|_| "Host command failed".into()));
        }
        response.json().map_err(|e| e.to_string())
    }
}
