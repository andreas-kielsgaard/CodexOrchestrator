use super::*;

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskDashboardSnapshot {
    pub(crate) groups: Vec<DashboardGroup>,
    pub(crate) projects: Vec<TaskDashboardProject>,
    pub(crate) repos: Vec<TaskDashboardRepo>,
    pub(crate) worktree_anchors: Vec<TaskDashboardWorktreeAnchor>,
    pub(crate) total_open_tasks: usize,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskDashboardProject {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskDashboardRepo {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) project: String,
    pub(crate) name: String,
    pub(crate) root_path: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskDashboardWorktreeAnchor {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) project: String,
    pub(crate) repo_id: String,
    pub(crate) repo: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) branch: Option<String>,
    pub(crate) path: String,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardGroup {
    pub(crate) id: &'static str,
    pub(crate) title: &'static str,
    pub(crate) tasks: Vec<DashboardTask>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DashboardTask {
    pub(crate) id: String,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) project: String,
    pub(crate) execution_state: String,
    pub(crate) attention_state: String,
    pub(crate) priority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repo: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) worktree_path: Option<String>,
    pub(crate) updated_at: String,
}
