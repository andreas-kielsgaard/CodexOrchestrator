use super::*;

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCodexTaskRunCommandResult {
    pub(crate) status: String,
    pub(crate) task_id: String,
    pub(crate) task_run_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) raw_event_stream_artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) final_response_artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exit_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) status_reason: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) post_run_capture: Option<StartCodexTaskRunPostRunCaptureResult>,
    pub(crate) task: StartCodexTaskRunTaskState,
    pub(crate) task_run: StartCodexTaskRunTaskRunState,
}

#[derive(Serialize, Clone, Debug, Default, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCodexTaskRunPostRunCaptureResult {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) diff: Option<StartCodexTaskRunDiffCaptureResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) validation: Option<StartCodexTaskRunValidationCaptureResult>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) skipped_reason: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase", tag = "status")]
pub(crate) enum StartCodexTaskRunDiffCaptureResult {
    #[serde(rename = "captured")]
    Captured {
        artifact_id: String,
        event_id: String,
        diff_length: i64,
        is_empty_diff: bool,
        worktree_path: String,
    },
    #[serde(rename = "failed")]
    Failed { error: String },
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCodexTaskRunValidationCaptureResult {
    pub(crate) status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) validation_run_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) output_artifact_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) started_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) artifact_created_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) completed_event_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exit_code: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) signal: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) error: Option<String>,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCodexTaskRunTaskState {
    pub(crate) id: String,
    pub(crate) execution_state: String,
    pub(crate) attention_state: String,
    pub(crate) conversation_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repo_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) branch_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) worktree_id: Option<String>,
    pub(crate) updated_at: String,
}

#[derive(Serialize, Clone, Debug, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct StartCodexTaskRunTaskRunState {
    pub(crate) id: String,
    pub(crate) execution_state: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) conversation_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) worktree_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) started_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) exit_code: Option<i64>,
    pub(crate) updated_at: String,
}
