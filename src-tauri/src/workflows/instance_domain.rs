use super::domain::{EffectiveRecipe, WorkflowConnectionActivation};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedRepoBranchWorktreeTarget {
    pub(crate) repository: WorkflowRepositoryTarget,
    pub(crate) branch: WorkflowBranchTarget,
    pub(crate) worktree: WorkflowWorktreeTarget,
}

impl ResolvedRepoBranchWorktreeTarget {
    pub(crate) fn validate(&self) -> Result<(), String> {
        for (value, label) in [
            (&self.repository.id, "repository ID"),
            (&self.repository.name, "repository name"),
            (
                &self.repository.git_common_directory,
                "repository Git identity",
            ),
            (&self.branch.id, "branch ID"),
            (&self.branch.name, "branch name"),
            (&self.worktree.id, "worktree ID"),
            (&self.worktree.path, "worktree path"),
        ] {
            if value.trim().is_empty() {
                return Err(format!("A Workflow instance target {label} is required."));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowRepositoryTarget {
    pub(crate) id: String,
    pub(crate) name: String,
    pub(crate) git_common_directory: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowBranchTarget {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorkflowWorktreeTarget {
    pub(crate) id: String,
    pub(crate) path: String,
}

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
