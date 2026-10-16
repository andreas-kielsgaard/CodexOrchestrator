use super::domain::AgentSessionId;
use crate::execution_targets::domain::{SessionExecutionSelection, SessionExecutionTarget};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

mod planning;

pub use planning::{
    plan_target_switch, same_workspace_identity, selection_target_relation, SelectionTargetRelation,
};

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetOperationPhase {
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
pub enum TargetOperationActionKind {
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
pub enum TargetSwitchIntent {
    ChangeConversationTarget,
    ContinueWork,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum MigrationStrategy {
    ExistingSister,
    LinkExisting,
    CreateLinked,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct MigrationResolution {
    pub strategy: MigrationStrategy,
    pub repository_id: String,
    pub branch_ref: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSwitchRequest {
    pub destination: SessionExecutionSelection,
    pub intent: TargetSwitchIntent,
    pub migration_resolution: Option<MigrationResolution>,
}

impl TargetSwitchRequest {
    pub fn retarget(destination: SessionExecutionSelection) -> Self {
        Self {
            destination,
            intent: TargetSwitchIntent::ChangeConversationTarget,
            migration_resolution: None,
        }
    }
    pub fn validate(&self) -> Result<(), String> {
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
pub struct TargetSwitchFacts {
    pub destination: Option<SessionExecutionTarget>,
    pub destination_ready: Option<bool>,
    pub unknown: Vec<String>,
    pub blockers: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetSwitchProjection {
    pub calculated_at: DateTime<Utc>,
    pub request_revision: u64,
    pub stale: bool,
    pub source: Option<SessionExecutionTarget>,
    pub facts: TargetSwitchFacts,
    pub steps: Vec<TargetOperationAction>,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum TargetOperationActionStatus {
    Pending,
    Running,
    Completed,
    Failed,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TargetOperationAction {
    pub kind: TargetOperationActionKind,
    pub status: TargetOperationActionStatus,
    pub detail: Option<String>,
    pub error: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionTargetOperation {
    pub id: String,
    pub session_id: AgentSessionId,
    pub source_target: Option<SessionExecutionTarget>,
    pub request: TargetSwitchRequest,
    pub needs_resolution: bool,
    pub projected_plan: TargetSwitchProjection,
    pub phase: TargetOperationPhase,
    pub actions: Vec<TargetOperationAction>,
    pub staged_target: Option<SessionExecutionTarget>,
    pub error: Option<String>,
    pub receipt: Option<crate::agent_sessions::history::WorkspaceTransitionReceipt>,
    pub generation: u64,
    pub created_at: DateTime<Utc>,
    pub updated_at: DateTime<Utc>,
}

impl SessionTargetOperation {
    pub fn new(
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

pub fn validate_target_operation(operation: &SessionTargetOperation) -> Result<(), String> {
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
