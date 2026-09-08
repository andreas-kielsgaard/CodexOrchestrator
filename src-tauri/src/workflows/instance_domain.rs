use super::domain::{EffectiveRecipe, WorkflowConnectionActivation};
use serde::Serialize;

pub(crate) use crate::repository_catalog::{
    ResolvedBranchTarget as WorkflowBranchTarget, ResolvedRepoBranchWorktreeTarget,
    ResolvedRepositoryTarget as WorkflowRepositoryTarget,
    ResolvedWorktreeTarget as WorkflowWorktreeTarget,
};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkflowSessionActivity {
    Active,
    Idle,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowInstanceSession {
    pub(crate) node_id: String,
    pub(crate) session_id: String,
    pub(crate) title: String,
    pub(crate) activity: WorkflowSessionActivity,
    pub(crate) latest_turn_summary: Option<String>,
    pub(crate) associated_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowInstanceSummary {
    pub(crate) id: String,
    pub(crate) workflow_type_id: String,
    pub(crate) workflow_type_name: String,
    pub(crate) recipe_id: String,
    pub(crate) name: String,
    pub(crate) session_count: u32,
    pub(crate) active_session_count: u32,
    pub(crate) idle_session_count: u32,
    pub(crate) created_at: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowInstance {
    pub(crate) summary: WorkflowInstanceSummary,
    pub(crate) target: ResolvedRepoBranchWorktreeTarget,
    pub(crate) recipe: EffectiveRecipe,
    pub(crate) sessions: Vec<WorkflowInstanceSession>,
    pub(crate) connection_activations: Vec<WorkflowConnectionActivation>,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkflowInstanceRecord {
    pub(crate) id: String,
    pub(crate) workflow_type_id: String,
    pub(crate) workflow_type_name: String,
    pub(crate) recipe: EffectiveRecipe,
    pub(crate) name: String,
    pub(crate) target: ResolvedRepoBranchWorktreeTarget,
    pub(crate) created_at: String,
    pub(crate) session_associations: Vec<WorkflowSessionAssociationRecord>,
}

#[derive(Clone, Debug)]
pub(crate) struct WorkflowSessionAssociationRecord {
    pub(crate) node_id: String,
    pub(crate) session_id: String,
    pub(crate) associated_at: String,
}

#[derive(Clone, Debug)]
pub(crate) struct CreateWorkflowInstancePreparation {
    pub(crate) instance_id: String,
    pub(crate) workflow_type_id: String,
    pub(crate) recipe_id: String,
    pub(crate) name: String,
    pub(crate) target: ResolvedRepoBranchWorktreeTarget,
    pub(crate) created_at: String,
}
