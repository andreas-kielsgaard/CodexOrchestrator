use super::target_operation::{TargetOperationAction, TargetSwitchIntent};
use crate::execution_targets::domain::SessionExecutionTarget;
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkspaceTransitionReceipt {
    pub(crate) operation_id: String,
    pub(crate) intent: TargetSwitchIntent,
    pub(crate) source: Option<SessionExecutionTarget>,
    pub(crate) destination: SessionExecutionTarget,
    pub(crate) actions: Vec<TargetOperationAction>,
    pub(crate) completed_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionHistoryPosition {
    pub(crate) ordinal: u64,
    pub(crate) invocation_id: Option<String>,
    pub(crate) operation_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InvocationProviderIdentity {
    pub(crate) invocation_id: String,
    pub(crate) provider: String,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionTimeline {
    pub(crate) positions: Vec<SessionHistoryPosition>,
    pub(crate) transitions: Vec<WorkspaceTransitionReceipt>,
    pub(crate) providers: Vec<InvocationProviderIdentity>,
}
