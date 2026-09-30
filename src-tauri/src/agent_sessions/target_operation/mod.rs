use super::domain::AgentSessionId;
use crate::execution_targets::domain::{SessionExecutionSelection, SessionExecutionTarget};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod planning;

pub(crate) use planning::{
    plan_target_switch, same_workspace_identity, selection_target_relation, SelectionTargetRelation,
};

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
    ActivateTarget,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum TargetSwitchIntent {
    ChangeConversationTarget,
    ContinueWork,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum MigrationStrategy {
    ExistingSister,
    LinkExisting,
    CreateLinked,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct MigrationResolution {
    pub(crate) strategy: MigrationStrategy,
    pub(crate) repository_id: String,
    pub(crate) branch_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetSwitchRequest {
    pub(crate) destination: SessionExecutionSelection,
    pub(crate) intent: TargetSwitchIntent,
    pub(crate) migration_resolution: Option<MigrationResolution>,
}

impl TargetSwitchRequest {
    pub(crate) fn retarget(destination: SessionExecutionSelection) -> Self {
        Self {
            destination,
            intent: TargetSwitchIntent::ChangeConversationTarget,
            migration_resolution: None,
        }
    }
    pub(crate) fn validate(&self) -> Result<(), String> {
        match (self.intent, self.migration_resolution.as_ref()) {
            (TargetSwitchIntent::ContinueWork, None) => {
                Err("Continuing work requires an explicit migration resolution".into())
            }
            (TargetSwitchIntent::ChangeConversationTarget, Some(_)) => {
                Err("Changing conversation target cannot migrate work".into())
            }
            (TargetSwitchIntent::ContinueWork, Some(resolution))
                if resolution.repository_id.is_empty() || resolution.branch_ref.is_empty() =>
            {
                Err("Migration requires a repository and branch lineage".into())
            }
            _ => Ok(()),
        }
    }
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetSwitchFacts {
    pub(crate) destination: Option<SessionExecutionTarget>,
    pub(crate) destination_ready: Option<bool>,
    pub(crate) unknown: Vec<String>,
    pub(crate) blockers: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TargetSwitchProjection {
    pub(crate) calculated_at: DateTime<Utc>,
    pub(crate) request_revision: u64,
    pub(crate) stale: bool,
    pub(crate) source: Option<SessionExecutionTarget>,
    pub(crate) facts: TargetSwitchFacts,
    pub(crate) steps: Vec<TargetOperationAction>,
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
    pub(crate) request: TargetSwitchRequest,
    pub(crate) needs_resolution: bool,
    pub(crate) projected_plan: TargetSwitchProjection,
    pub(crate) phase: TargetOperationPhase,
    pub(crate) actions: Vec<TargetOperationAction>,
    pub(crate) staged_target: Option<SessionExecutionTarget>,
    pub(crate) error: Option<String>,
    pub(crate) receipt: Option<crate::agent_sessions::history::WorkspaceTransitionReceipt>,
    pub(crate) generation: u64,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

impl SessionTargetOperation {
    pub(crate) fn new(
        id: String,
        session_id: AgentSessionId,
        source_target: Option<SessionExecutionTarget>,
        request: TargetSwitchRequest,
        now: DateTime<Utc>,
    ) -> Self {
        let facts = TargetSwitchFacts::default();
        let steps = plan_target_switch(source_target.as_ref(), &request, &facts);
        let projected_plan = TargetSwitchProjection {
            calculated_at: now,
            request_revision: 0,
            stale: false,
            source: source_target.clone(),
            facts,
            steps,
        };
        Self {
            id,
            session_id,
            source_target,
            request,
            needs_resolution: false,
            projected_plan,
            phase: TargetOperationPhase::Pending,
            actions: Vec::new(),
            staged_target: None,
            error: None,
            receipt: None,
            generation: 0,
            created_at: now,
            updated_at: now,
        }
    }
}

pub(crate) fn validate_target_operation(operation: &SessionTargetOperation) -> Result<(), String> {
    if operation.id.trim().is_empty() {
        return Err("Target operation ID cannot be blank".into());
    }
    operation
        .request
        .destination
        .execution
        .validate()
        .map_err(|error| format!("Invalid target operation route: {error}"))?;
    if operation
        .request
        .destination
        .capability_profile_id
        .trim()
        .is_empty()
    {
        return Err("Target operation requires a Capability Profile".into());
    }
    operation.request.validate()?;
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

#[cfg(test)]
mod planning_tests;
