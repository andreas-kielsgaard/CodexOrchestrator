pub mod host;
pub mod options;
pub mod server;
pub mod sync;
use orchid_core::agent_sessions::client::HostCommand;
use serde::{Deserialize, Serialize};
#[derive(Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum HostMessage {
    Command {
        id: String,
        command: HostCommand,
    },
    Reply {
        id: String,
        result: Result<serde_json::Value, String>,
    },
}
pub mod host_api;

pub async fn shutdown_signal() {
    #[cfg(unix)]
    {
        let mut terminate =
            tokio::signal::unix::signal(tokio::signal::unix::SignalKind::terminate())
                .expect("SIGTERM handler");
        tokio::select! { _ = tokio::signal::ctrl_c() => {}, _ = terminate.recv() => {} }
    }
    #[cfg(not(unix))]
    let _ = tokio::signal::ctrl_c().await;
}
