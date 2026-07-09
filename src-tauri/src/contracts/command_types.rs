use super::*;

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AppMetadata {
    pub(crate) app_name: &'static str,
    pub(crate) storage_mode: &'static str,
    pub(crate) codex_runtime: &'static str,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BackendMaintenanceResult {
    pub(crate) status: String,
    pub(crate) stale: bool,
    pub(crate) checked_at: String,
    pub(crate) newest_source_path: Option<String>,
    pub(crate) newest_source_modified_at: Option<String>,
    pub(crate) executable_modified_at: Option<String>,
    pub(crate) message: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateOpenTaskCommandInput {
    pub(crate) project_id: String,
    pub(crate) repo_id: Option<String>,
    pub(crate) branch_id: Option<String>,
    pub(crate) worktree_id: Option<String>,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) execution_state: Option<String>,
    pub(crate) attention_state: Option<String>,
    pub(crate) priority: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterTaskWorktreeCommandInput {
    pub(crate) project_name: String,
    pub(crate) repo_name: Option<String>,
    pub(crate) repo_root_path: String,
    pub(crate) branch_name: Option<String>,
    pub(crate) worktree_path: String,
    pub(crate) is_main: Option<bool>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterTaskRepoCommandInput {
    pub(crate) repo_root_path: String,
    pub(crate) project_name: Option<String>,
    pub(crate) repo_name: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiscoverTaskReposCommandInput {
    pub(crate) root_path: String,
    pub(crate) max_depth: Option<usize>,
}

#[derive(Serialize, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DiscoveredTaskRepo {
    pub(crate) name: String,
    pub(crate) path: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct UpdateOpenTaskCommandInput {
    pub(crate) title: Option<String>,
    pub(crate) summary: Option<String>,
    pub(crate) execution_state: Option<String>,
    pub(crate) attention_state: Option<String>,
    pub(crate) priority: Option<String>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCodexTaskRunCommandInput {
    pub(crate) task_id: String,
    pub(crate) prompt: String,
    pub(crate) cwd: Option<String>,
    pub(crate) worktree_id: Option<String>,
    pub(crate) conversation_title: Option<String>,
    pub(crate) conversation_summary: Option<String>,
    pub(crate) additional_args: Option<Vec<String>>,
    pub(crate) env: Option<BTreeMap<String, Option<String>>>,
    pub(crate) post_run_capture: Option<StartCodexTaskRunPostRunCaptureInput>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCodexTaskRunPostRunCaptureInput {
    pub(crate) collect_diff: Option<bool>,
    pub(crate) validation_command: Option<StartCodexTaskRunValidationCommandInput>,
}

#[derive(Deserialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCodexTaskRunValidationCommandInput {
    pub(crate) command: String,
    pub(crate) args: Option<Vec<String>>,
    pub(crate) cwd: Option<String>,
    pub(crate) env: Option<BTreeMap<String, Option<String>>>,
}
