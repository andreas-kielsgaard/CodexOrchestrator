use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedRepoBranchWorktreeTarget {
    pub repository: WorkflowRepositoryTarget,
    pub branch: WorkflowBranchTarget,
    pub worktree: WorkflowWorktreeTarget,
}

impl ResolvedRepoBranchWorktreeTarget {
    pub fn validate(&self) -> Result<(), String> {
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
pub struct WorkflowRepositoryTarget {
    pub id: String,
    pub name: String,
    pub git_common_directory: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowBranchTarget {
    pub id: String,
    pub name: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WorkflowWorktreeTarget {
    pub id: String,
    pub path: String,
}
