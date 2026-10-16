pub mod definitions;
pub mod repository;
mod schema;
#[cfg(test)]
mod tests;
use crate::agent_sessions::{
    ports::AgentSessionHistory, preparation::SessionPreparation, queue::SessionQueueSnapshot,
};
pub use schema::initialize;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionReplica {
    #[serde(default)]
    pub fork: Option<ConversationFork>,
    #[serde(default)]
    pub address: Option<crate::agent_sessions::repository::StoredAddressDomain>,
    #[serde(default)]
    pub provider_conversations: Vec<crate::agent_sessions::domain::ProviderSessionBinding>,
    #[serde(default)]
    pub current_profile: Option<crate::execution_configuration::SessionCreationResolution>,
    #[serde(default)]
    pub invocation_devices: std::collections::BTreeMap<String, String>,
    pub history: AgentSessionHistory,
    pub queue: SessionQueueSnapshot,
    pub preparations: Vec<SessionPreparation>,
    #[serde(default)]
    pub target_operations: Vec<crate::agent_sessions::target_operation::SessionTargetOperation>,
    #[serde(default)]
    pub initial_prompt_prefix: Option<crate::agent_sessions::ports::InitialPromptPrefix>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ConversationFork {
    pub source_session_id: String,
    pub source_invocation_id: Option<String>,
    pub created_at: String,
}

impl SessionReplica {
    pub fn queue_view(&self) -> crate::agent_sessions::application::queue::SessionQueueView {
        use crate::agent_sessions::{application::queue::*, queue::SessionQueueEntryItem};
        SessionQueueView {
            session_id: self.history.session.id.clone(),
            revision: self.queue.revision,
            active_target: self.history.session.execution_target.clone(),
            entries: self
                .queue
                .entries
                .iter()
                .cloned()
                .map(|entry| {
                    let (message, target_operation) = match &entry.item {
                        SessionQueueEntryItem::Message { invocation_id }
                        | SessionQueueEntryItem::Compaction { invocation_id } => {
                            let message = self
                                .history
                                .invocations
                                .iter()
                                .find(|t| t.invocation.id == *invocation_id)
                                .zip(
                                    self.preparations
                                        .iter()
                                        .find(|p| p.invocation_id == *invocation_id),
                                )
                                .map(|(turn, preparation)| QueuedMessageView {
                                    invocation: turn.invocation.clone(),
                                    preparation: preparation.clone(),
                                });
                            (message, None)
                        }
                        SessionQueueEntryItem::TargetOperation { operation_id } => (
                            None,
                            self.target_operations
                                .iter()
                                .find(|op| op.id == *operation_id)
                                .cloned(),
                        ),
                    };
                    SessionQueueEntryView {
                        entry,
                        message,
                        target_operation,
                    }
                })
                .collect(),
        }
    }
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionChange {
    pub device_id: String,
    pub sequence: u64,
    pub session: SessionReplica,
    #[serde(default)]
    pub transfers: Vec<OwnershipTransfer>,
}
#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct OwnershipTransfer {
    pub operation_id: String,
    pub source: String,
    pub destination: String,
}
pub trait SessionReplicaSource: Send + Sync {
    fn fetch_session(&self, id: &str) -> Result<SessionChange, String>;
}

pub fn summarize(
    history: &AgentSessionHistory,
) -> crate::agent_sessions::ports::AgentSessionSummary {
    let latest = history.invocations.last().map(|turn| &turn.invocation);
    let completed = history
        .invocations
        .iter()
        .rev()
        .find(|turn| turn.invocation.status.is_terminal());
    crate::agent_sessions::ports::AgentSessionSummary {
        session: history.session.clone(),
        invocation_count: history.invocations.len() as u64,
        latest_completed_id: completed.map(|t| t.invocation.id.to_string()),
        completion_state: completed.map(|t| format!("{:?}", t.invocation.status).to_lowercase()),
        pending_request_count: crate::agent_sessions::interactions::pending_request_count(
            &history.invocations,
        ),
        latest_invocation_status: latest.map(|t| t.status),
        latest_submitted_text: latest.map(|t| t.submitted_text.clone()),
    }
}

#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum RetentionMode {
    Retained,
    Cache,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ServerConnection {
    pub url: String,
    pub token: String,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct DeviceDescription {
    pub id: String,
    pub name: String,
    pub providers: Vec<ProviderObservation>,
}

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderObservation {
    pub provider: String,
    pub configuration_ref: String,
    pub configured: bool,
    pub state: String,
    pub selected: bool,
    pub observed_at: chrono::DateTime<chrono::Utc>,
}
