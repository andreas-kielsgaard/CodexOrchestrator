use super::*;
use crate::agent_sessions::target_operation::TargetSwitchRequest;
use crate::agent_sessions::{
    application::queue::{SessionQueueView, UpdateQueuedMessageInput, UpdateTargetOperationInput},
    domain::AgentSessionId,
};
use crate::execution_configuration::SandboxMode;
use serde::Deserialize;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionQueueQuery {
    session_id: AgentSessionId,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ReorderSessionQueueInput {
    session_id: AgentSessionId,
    expected_revision: u64,
    entry_ids: Vec<String>,
}

#[derive(Deserialize)]
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateTargetOperationCommand {
    session_id: AgentSessionId,
    entry_id: String,
    expected_revision: u64,
    request: TargetSwitchRequest,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct QueueEntryAction {
    session_id: AgentSessionId,
    entry_id: String,
}

#[tauri::command]
pub(crate) async fn run_agent_session_target_operation(
    state: State<'_, AgentSessionTauriState>,
    input: QueueEntryAction,
) -> Result<SessionQueueView, String> {
    let app = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        app.run_target_operation_now(&input.session_id, &input.entry_id)
            .and_then(|_| app.load_session_queue_view(&input.session_id))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|e| e.to_string())?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SendQueuedMessageNowInput {
    session_id: AgentSessionId,
    entry_id: String,
    expected_revision: u64,
    message: Option<QueuedMessageEdits>,
}

#[derive(Deserialize)]
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
) -> Result<SessionQueueView, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let edits = input.message.map(|message| UpdateQueuedMessageInput {
            session_id: input.session_id.clone(),
            entry_id: input.entry_id.clone(),
            expected_revision: input.expected_revision,
            submitted_text: message.submitted_text,
            model: message.model,
            reasoning_mode: message.reasoning_mode,
            sandbox_mode: message.sandbox_mode,
        });
        application
            .send_queued_message_now(
                &input.session_id,
                &input.entry_id,
                input.expected_revision,
                edits,
            )
            .and_then(|_| application.load_session_queue_view(&input.session_id))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EnqueueTargetOperationCommand {
    session_id: AgentSessionId,
    request: TargetSwitchRequest,
    before_entry_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EnqueueCompactionCommand {
    session_id: AgentSessionId,
}

#[tauri::command]
pub(crate) async fn load_agent_session_queue(
    state: State<'_, AgentSessionTauriState>,
    input: SessionQueueQuery,
) -> Result<SessionQueueView, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .load_session_queue_view(&input.session_id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn enqueue_agent_session_target_operation(
    state: State<'_, AgentSessionTauriState>,
    input: EnqueueTargetOperationCommand,
) -> Result<SessionQueueView, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let session_id = input.session_id.clone();
        application
            .enqueue_session_target_operation(
                &input.session_id,
                input.request,
                input.before_entry_id.as_deref(),
            )
            .and_then(|_| application.load_session_queue_view(&session_id))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn enqueue_agent_session_compaction(
    state: State<'_, AgentSessionTauriState>,
    input: EnqueueCompactionCommand,
) -> Result<SessionQueueView, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let session_id = input.session_id.clone();
        application
            .enqueue_session_compaction(&input.session_id)
            .and_then(|_| application.load_session_queue_view(&session_id))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn reorder_agent_session_queue(
    state: State<'_, AgentSessionTauriState>,
    input: ReorderSessionQueueInput,
) -> Result<SessionQueueView, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .reorder_session_queue(&input.session_id, input.expected_revision, &input.entry_ids)
            .and_then(|_| application.load_session_queue_view(&input.session_id))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn update_queued_agent_session_message(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateQueuedMessageCommand,
) -> Result<SessionQueueView, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let session_id = input.session_id.clone();
        application
            .update_queued_message(UpdateQueuedMessageInput {
                session_id: input.session_id,
                entry_id: input.entry_id,
                expected_revision: input.expected_revision,
                submitted_text: input.submitted_text,
                model: input.model,
                reasoning_mode: input.reasoning_mode,
                sandbox_mode: input.sandbox_mode,
            })
            .and_then(|_| application.load_session_queue_view(&session_id))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn update_agent_session_target_operation(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateTargetOperationCommand,
) -> Result<SessionQueueView, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let session_id = input.session_id.clone();
        application
            .update_target_operation(UpdateTargetOperationInput {
                session_id: input.session_id,
                entry_id: input.entry_id,
                expected_revision: input.expected_revision,
                request: input.request,
            })
            .and_then(|_| application.load_session_queue_view(&session_id))
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn retry_agent_session_queue_entry(
    state: State<'_, AgentSessionTauriState>,
    input: QueueEntryAction,
) -> Result<(), String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .retry_queue_entry(&input.session_id, &input.entry_id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn cancel_agent_session_queue_entry(
    state: State<'_, AgentSessionTauriState>,
    input: QueueEntryAction,
) -> Result<(), String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .cancel_queue_entry(&input.session_id, &input.entry_id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[derive(Deserialize)]
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
) -> Result<SessionQueueView, String> {
    let application = state.application.clone();
    tauri::async_runtime::spawn_blocking(move || {
        application
            .recalculate_target_operation_projection(
                &input.session_id,
                &input.entry_id,
                input.expected_revision,
            )
            .and_then(|_| application.load_session_queue_view(&input.session_id))
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}
