pub(crate) struct TaskRow {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) repo_id: Option<String>,
    pub(crate) branch_id: Option<String>,
    pub(crate) worktree_id: Option<String>,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) execution_state: String,
    pub(crate) attention_state: String,
    pub(crate) priority: String,
    pub(crate) updated_at: String,
}

#[derive(Debug)]
pub(crate) struct ProjectRow {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Debug)]
pub(crate) struct RepoRow {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) name: String,
    pub(crate) root_path: String,
}

#[derive(Debug)]
pub(crate) struct BranchRow {
    pub(crate) id: String,
    pub(crate) name: String,
}

#[derive(Debug)]
pub(crate) struct WorktreeRow {
    pub(crate) id: String,
    pub(crate) path: String,
}
