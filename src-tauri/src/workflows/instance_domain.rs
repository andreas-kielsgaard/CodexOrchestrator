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
