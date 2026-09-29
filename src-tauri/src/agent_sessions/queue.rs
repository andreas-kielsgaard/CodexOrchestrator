use super::domain::{AgentInvocationId, AgentSessionId};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionQueueEntryState {
    Pending,
    Executing,
    Blocked,
    Completed,
    Canceled,
}

impl SessionQueueEntryState {
    pub(crate) fn is_unfinished(self) -> bool {
        matches!(self, Self::Pending | Self::Executing | Self::Blocked)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum SessionQueueEntryItem {
    Message { invocation_id: AgentInvocationId },
    Compaction { invocation_id: AgentInvocationId },
    TargetOperation { operation_id: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionQueueEntry {
    pub(crate) id: String,
    pub(crate) session_id: AgentSessionId,
    pub(crate) ordinal: i64,
    pub(crate) state: SessionQueueEntryState,
    pub(crate) item: SessionQueueEntryItem,
    pub(crate) blocked_reason: Option<String>,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionQueueSnapshot {
    pub(crate) session_id: AgentSessionId,
    pub(crate) revision: u64,
    pub(crate) entries: Vec<SessionQueueEntry>,
}

pub(crate) fn validate_queue_entry(entry: &SessionQueueEntry) -> Result<(), String> {
    if entry.id.trim().is_empty() {
        return Err("Queue entry ID cannot be blank".into());
    }
    if entry.ordinal < 0 {
        return Err("Queue entry ordinal cannot be negative".into());
    }
    if entry.updated_at < entry.created_at {
        return Err("Queue entry update time cannot precede creation time".into());
    }
    match &entry.item {
        SessionQueueEntryItem::Message { invocation_id } => {
            if invocation_id.as_str().trim().is_empty() {
                return Err("Queued message requires an invocation".into());
            }
        }
        SessionQueueEntryItem::Compaction { invocation_id } => {
            if invocation_id.as_str().trim().is_empty() {
                return Err("Queued compaction requires an invocation".into());
            }
        }
        SessionQueueEntryItem::TargetOperation { operation_id } => {
            if operation_id.trim().is_empty() {
                return Err("Queued target operation requires an operation".into());
            }
        }
    }
    Ok(())
}
