use super::domain::{AgentRuntimeBinding, AgentSessionId};
use crate::execution_targets::domain::{SessionExecutionSelection, SessionExecutionTarget};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetOperationPhase {
    Pending,
    Staging,
    Staged,
    Activating,
    Completed,
    Blocked,
    Canceled,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetOperationActionKind {
    PrepareDevice,
    ResolveRevision,
    MaterializeWorktree,
    VerifyWorktree,
    InspectSource,
    InspectDestination,
    CaptureSnapshot,
    TransferSnapshot,
    ApplySnapshot,
    VerifyMigratedState,
    ActivateSister,
    SelectProviderContext,
    ActivateTarget,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetOperationActionStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetOperationAction {
    pub(crate) kind: TargetOperationActionKind,
    pub(crate) status: TargetOperationActionStatus,
    pub(crate) detail: Option<String>,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionTargetOperation {
    pub(crate) id: String,
    pub(crate) session_id: AgentSessionId,
    pub(crate) source_target: Option<SessionExecutionTarget>,
    pub(crate) destination_selection: SessionExecutionSelection,
    pub(crate) phase: TargetOperationPhase,
    pub(crate) actions: Vec<TargetOperationAction>,
    pub(crate) staged_target: Option<SessionExecutionTarget>,
    pub(crate) activated_binding: Option<AgentRuntimeBinding>,
    pub(crate) error: Option<String>,
    pub(crate) generation: u64,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl SessionTargetOperation {
    pub(crate) fn new(
        id: String,
        session_id: AgentSessionId,
        source_target: Option<SessionExecutionTarget>,
        destination_selection: SessionExecutionSelection,
        now: DateTime<Utc>,
    ) -> Self {
        Self {
            id,
            session_id,
            source_target,
            destination_selection,
            phase: TargetOperationPhase::Pending,
            actions: planned_actions(),
            staged_target: None,
            activated_binding: None,
            error: None,
            generation: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

pub(crate) fn planned_actions() -> Vec<TargetOperationAction> {
    [
        TargetOperationActionKind::PrepareDevice,
        TargetOperationActionKind::ResolveRevision,
        TargetOperationActionKind::MaterializeWorktree,
        TargetOperationActionKind::VerifyWorktree,
        TargetOperationActionKind::InspectSource,
        TargetOperationActionKind::InspectDestination,
        TargetOperationActionKind::CaptureSnapshot,
        TargetOperationActionKind::TransferSnapshot,
        TargetOperationActionKind::ApplySnapshot,
        TargetOperationActionKind::VerifyMigratedState,
        TargetOperationActionKind::ActivateSister,
        TargetOperationActionKind::SelectProviderContext,
        TargetOperationActionKind::ActivateTarget,
    ]
    .into_iter()
    .map(|kind| TargetOperationAction {
        kind,
        status: TargetOperationActionStatus::Pending,
        detail: None,
        error: None,
    })
    .collect()
}

pub(crate) fn validate_target_operation(operation: &SessionTargetOperation) -> Result<(), String> {
    if operation.id.trim().is_empty() {
        return Err("Target operation ID cannot be blank".into());
    }
    operation
        .destination_selection
        .execution
        .validate()
        .map_err(|error| format!("Invalid target operation route: {error}"))?;
    if operation
        .destination_selection
        .capability_profile_id
        .trim()
        .is_empty()
    {
        return Err("Target operation requires a Capability Profile".into());
    }
    if operation.actions.is_empty() {
        return Err("Target operation requires actions".into());
    }
    if operation.updated_at < operation.created_at {
        return Err("Target operation update time cannot precede creation time".into());
    }
    if matches!(
        operation.phase,
        TargetOperationPhase::Staged
            | TargetOperationPhase::Activating
            | TargetOperationPhase::Completed
    ) && operation.staged_target.is_none()
    {
        return Err("Staged target operation requires a resolved target".into());
    }
    if operation.phase == TargetOperationPhase::Blocked
        && operation.error.as_deref().is_none_or(str::is_empty)
    {
        return Err("Blocked target operation requires an error".into());
    }
    Ok(())
}
