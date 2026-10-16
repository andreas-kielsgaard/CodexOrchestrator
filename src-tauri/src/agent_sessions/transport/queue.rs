use super::*;
use crate::agent_sessions::target_operation::TargetSwitchRequest;
use crate::agent_sessions::{
    application::queue::{SessionQueueView, UpdateQueuedMessageInput, UpdateTargetOperationInput},
    domain::AgentSessionId,
};
use crate::execution_configuration::SandboxMode;
use serde::Deserialize;

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionQueueQuery {
    session_id: AgentSessionId,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReorderSessionQueueInput {
    session_id: AgentSessionId,
    expected_revision: u64,
    entry_ids: Vec<String>,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateQueuedMessageCommand {
    session_id: AgentSessionId,
    entry_id: String,
    expected_revision: u64,
    submitted_text: String,
    model: Option<String>,
    reasoning_mode: Option<String>,
    sandbox_mode: Option<SandboxMode>,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateTargetOperationCommand {
    session_id: AgentSessionId,
    entry_id: String,
    expected_revision: u64,
    request: TargetSwitchRequest,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct QueueEntryAction {
    session_id: AgentSessionId,
    entry_id: String,
}

#[tauri::command]
pub(crate) async fn run_agent_session_target_operation(
    state: State<'_, AgentSessionTauriState>,
    input: QueueEntryAction,
) -> Result<serde_json::Value, String> {
    let command=state.command("runTargetOperation",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SendQueuedMessageNowInput {
    session_id: AgentSessionId,
    entry_id: String,
    expected_revision: u64,
    message: Option<QueuedMessageEdits>,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct QueuedMessageEdits {
    submitted_text: String,
    model: Option<String>,
    reasoning_mode: Option<String>,
    sandbox_mode: Option<SandboxMode>,
}

#[tauri::command]
pub(crate) async fn send_queued_agent_session_message_now(
    state: State<'_, AgentSessionTauriState>,
    input: SendQueuedMessageNowInput,
) -> Result<serde_json::Value, String> {
    let command=state.command("sendQueueEntryNow",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EnqueueTargetOperationCommand {
    operation_id: String,
    session_id: AgentSessionId,
    request: TargetSwitchRequest,
    before_entry_id: Option<String>,
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EnqueueCompactionCommand {
    submission_id: crate::agent_sessions::domain::AgentInvocationId,
    session_id: AgentSessionId,
}

#[tauri::command]
pub(crate) async fn load_agent_session_queue(
    state: State<'_, AgentSessionTauriState>,
    input: SessionQueueQuery,
) -> Result<serde_json::Value, String> {
    let command=state.command("queue",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub(crate) async fn enqueue_agent_session_target_operation(
    state: State<'_, AgentSessionTauriState>,
    input: EnqueueTargetOperationCommand,
) -> Result<serde_json::Value, String> {
    let command=state.command("enqueueTargetOperation",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub(crate) async fn enqueue_agent_session_compaction(
    state: State<'_, AgentSessionTauriState>,
    input: EnqueueCompactionCommand,
) -> Result<serde_json::Value, String> {
    let command=state.command("enqueueCompaction",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub(crate) async fn reorder_agent_session_queue(
    state: State<'_, AgentSessionTauriState>,
    input: ReorderSessionQueueInput,
) -> Result<serde_json::Value, String> {
    let command=state.command("reorderQueue",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub(crate) async fn update_queued_agent_session_message(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateQueuedMessageCommand,
) -> Result<serde_json::Value, String> {
    let command=state.command("updateQueuedMessage",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub(crate) async fn update_agent_session_target_operation(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateTargetOperationCommand,
) -> Result<serde_json::Value, String> {
    let command=state.command("updateTargetOperation",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub(crate) async fn retry_agent_session_queue_entry(
    state: State<'_, AgentSessionTauriState>,
    input: QueueEntryAction,
) -> Result<serde_json::Value, String> {
    let command=state.command("retryQueueEntry",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[tauri::command]
pub(crate) async fn cancel_agent_session_queue_entry(
    state: State<'_, AgentSessionTauriState>,
    input: QueueEntryAction,
) -> Result<serde_json::Value, String> {
    let command=state.command("cancelQueueEntry",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}

#[derive(Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RecalculateTargetProjectionCommand {
    session_id: AgentSessionId,
    entry_id: String,
    expected_revision: u64,
}
#[tauri::command]
pub(crate) async fn recalculate_agent_session_target_projection(
    state: State<'_, AgentSessionTauriState>,
    input: RecalculateTargetProjectionCommand,
) -> Result<serde_json::Value, String> {
    let command=state.command("recalculateTargetProjection",input)?;
    tauri::async_runtime::spawn_blocking(command).await.map_err(|e|e.to_string())?
}
