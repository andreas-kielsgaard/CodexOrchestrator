use super::*;

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskRunDetailSnapshot {
    pub(crate) task: TaskRunDetailTaskAnchor,
    pub(crate) runs: Vec<TaskRunDetailRun>,
    pub(crate) unlinked_artifacts: TaskRunDetailArtifactGroups,
    pub(crate) unlinked_validation_runs: Vec<TaskRunDetailValidationRun>,
    pub(crate) event_timeline: Vec<DetailEvent>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskRunDetailTaskAnchor {
    pub(crate) record: DetailTask,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) project: Option<DetailProject>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repo: Option<DetailRepo>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) branch: Option<DetailBranch>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) worktree: Option<DetailWorktree>,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskRunDetailRun {
    pub(crate) run: DetailTaskRun,
    pub(crate) artifacts: TaskRunDetailArtifactGroups,
    pub(crate) validation_runs: Vec<TaskRunDetailValidationRun>,
    pub(crate) events: Vec<DetailEvent>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskRunDetailValidationRun {
    pub(crate) run: DetailValidationRun,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output_artifact: Option<DetailArtifact>,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TaskRunDetailArtifactGroups {
    pub(crate) final_responses: Vec<DetailArtifact>,
    pub(crate) raw_event_streams: Vec<DetailArtifact>,
    pub(crate) diffs: Vec<DetailArtifact>,
    pub(crate) validation_logs: Vec<DetailArtifact>,
    pub(crate) notes: Vec<DetailArtifact>,
    pub(crate) screenshots: Vec<DetailArtifact>,
    pub(crate) handoffs: Vec<DetailArtifact>,
    pub(crate) summaries: Vec<DetailArtifact>,
    pub(crate) other: Vec<DetailArtifact>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetailProject {
    pub(crate) id: String,
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) description: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetailRepo {
    pub(crate) id: String,
    pub(crate) project_id: String,
    pub(crate) name: String,
    pub(crate) root_path: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) default_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) remote_url: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetailBranch {
    pub(crate) id: String,
    pub(crate) repo_id: String,
    pub(crate) name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) base_branch: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) head_sha: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) intent: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetailWorktree {
    pub(crate) id: String,
    pub(crate) repo_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) branch_id: Option<String>,
    pub(crate) path: String,
    pub(crate) is_main: bool,
    pub(crate) is_dirty: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) lock_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) last_scanned_at: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetailTask {
    pub(crate) id: String,
    pub(crate) project_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repo_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) worktree_id: Option<String>,
    pub(crate) conversation_ids: Vec<String>,
    pub(crate) title: String,
    pub(crate) summary: String,
    pub(crate) execution_state: String,
    pub(crate) attention_state: String,
    pub(crate) priority: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) due_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) snoozed_until: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetailTaskRun {
    pub(crate) id: String,
    pub(crate) task_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) worktree_id: Option<String>,
    pub(crate) execution_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exit_code: Option<i64>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetailArtifact {
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) task_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) conversation_id: Option<String>,
    pub(crate) kind: String,
    pub(crate) title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) uri: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) content: Option<String>,
    pub(crate) created_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetailValidationRun {
    pub(crate) id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) task_run_id: Option<String>,
    pub(crate) command: String,
    pub(crate) status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exit_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output_artifact_id: Option<String>,
    pub(crate) created_at: String,
    pub(crate) updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct DetailEvent {
    pub(crate) id: String,
    pub(crate) kind: String,
    pub(crate) occurred_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) project_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) task_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) task_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) validation_run_id: Option<String>,
    pub(crate) payload: Map<String, Value>,
}
