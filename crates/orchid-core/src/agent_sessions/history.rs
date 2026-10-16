use super::target_operation::{TargetOperationAction, TargetSwitchIntent};
use crate::execution_targets::domain::SessionExecutionTarget;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkspaceTransitionReceipt {
    pub operation_id: String,
    pub intent: TargetSwitchIntent,
    pub source: Option<SessionExecutionTarget>,
    pub destination: SessionExecutionTarget,
    pub actions: Vec<TargetOperationAction>,
    pub completed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionHistoryPosition {
    pub ordinal: u64,
    pub invocation_id: Option<String>,
    pub operation_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationProviderIdentity {
    pub invocation_id: String,
    pub provider: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTimeline {
    pub positions: Vec<SessionHistoryPosition>,
    pub transitions: Vec<WorkspaceTransitionReceipt>,
    pub providers: Vec<InvocationProviderIdentity>,
}
