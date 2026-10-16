//! Optional network attachment. Local execution is composed before this starts.
use orchid_core::{
    agent_sessions::{
        application::AgentSessionApplication, client::AgentSessionCommandClient,
        repository::SqliteAgentSessionRepository,
    },
    persistence::ActiveDatabase,
    synchronization::{repository::ReplicaStore, ServerConnection},
};
use std::sync::Arc;
use tauri::Emitter;

pub(crate) fn compose(
    root: &std::path::Path,
    database: Arc<ActiveDatabase>,
    repository: Arc<SqliteAgentSessionRepository>,
    application: Arc<AgentSessionApplication>,
    app: tauri::AppHandle,
) -> Result<Arc<AgentSessionCommandClient>, String> {
    let replicas = ReplicaStore::new(database.clone());
    let connection = match load(root) {
        Ok(value) => value,
        Err(error) => {
            eprintln!("Server configuration unavailable; local operation continues: {error}");
            None
        }
    };
    let remote = connection.clone().map(|connection| {
        Arc::new(orchid_services::host::RemoteHttpTransport(connection))
            as Arc<dyn orchid_core::agent_sessions::client::DeviceCommandTransport>
    });
    if let Some(remote) = &remote {
        application
            .execution_targets()?
            .endpoints
            .attach_device_transport(remote.clone());
    }
    let commands = Arc::new(AgentSessionCommandClient {
        local_id: replicas.device_id()?,
        application: application.clone(),
        repository: repository.clone(),
        replicas,
        remote,
    });
    if let Some(connection) = connection {
        tauri::async_runtime::spawn(orchid_services::host::run_connections_with_observer(
            database,
            repository,
            application,
            connection,
            Arc::new(move |ids| {
                let _ = app.emit("agent-session-replicas-updated", ids);
            }),
        ));
    }
    Ok(commands)
}

pub(crate) fn load(root: &std::path::Path) -> Result<Option<ServerConnection>, String> {
    let path = root.join("server-connection.json");
    if !path.exists() {
        return Ok(None);
    }
    let connection: ServerConnection =
        serde_json::from_slice(&std::fs::read(path).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
    let url = reqwest::Url::parse(&connection.url).map_err(|e| e.to_string())?;
    if !matches!(url.scheme(), "http" | "https") || connection.token.trim().is_empty() {
        return Err("Server connection needs an HTTP(S) URL and service token".into());
    }
    Ok(Some(connection))
}

pub(crate) struct DefinitionSyncState {
    pub store: orchid_core::synchronization::definitions::DefinitionStore,
    pub connection: Option<ServerConnection>,
}
impl DefinitionSyncState {
    pub(crate) fn changed(&self, kind: &str, id: &str, content: serde_json::Value) {
        let Some(connection) = self.connection.clone() else {
            return;
        };
        let Ok(Some(proposal)) = self.store.proposal(kind, id, content) else {
            return;
        };
        let store = self.store.clone();
        tauri::async_runtime::spawn_blocking(move || {
            // One immediate attempt. An offline non-master edit is never queued for replay.
            let result = (|| -> Result<(), String> {
                let response = reqwest::blocking::Client::builder()
                    .timeout(std::time::Duration::from_secs(5))
                    .build()
                    .map_err(|e| e.to_string())?
                    .post(format!(
                        "{}/v1/definition-edits",
                        connection.url.trim_end_matches('/')
                    ))
                    .bearer_auth(connection.token)
                    .json(&proposal)
                    .send()
                    .map_err(|e| e.to_string())?
                    .error_for_status()
                    .map_err(|e| e.to_string())?;
                let accepted = response.json().map_err(|e| e.to_string())?;
                store.receive(&accepted)
            })();
            if let Err(error) = result {
                eprintln!("Definition edit remains local until the next canonical synchronization: {error}");
            }
        });
    }
}
