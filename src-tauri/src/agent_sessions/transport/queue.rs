use super::*;
use crate::agent_sessions::{
    application::queue::{SessionQueueView, UpdateQueuedMessageInput, UpdateTargetOperationInput},
    domain::AgentSessionId,
};
use crate::execution_configuration::SandboxMode;
use crate::execution_targets::domain::SessionExecutionSelection;
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
    destination: SessionExecutionSelection,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct QueueEntryAction {
    session_id: AgentSessionId,
    entry_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct EnqueueTargetOperationCommand {
    session_id: AgentSessionId,
    destination: SessionExecutionSelection,
}

#[tauri::command]
pub(crate) fn load_agent_session_queue(
    state: State<'_, AgentSessionTauriState>,
    input: SessionQueueQuery,
) -> Result<SessionQueueView, String> {
    state
        .application
        .load_session_queue_view(&input.session_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn enqueue_agent_session_target_operation(
    state: State<'_, AgentSessionTauriState>,
    input: EnqueueTargetOperationCommand,
) -> Result<SessionQueueView, String> {
    let session_id = input.session_id.clone();
    state
        .application
        .enqueue_session_target_operation(&input.session_id, input.destination)
        .and_then(|_| state.application.load_session_queue_view(&session_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn reorder_agent_session_queue(
    state: State<'_, AgentSessionTauriState>,
    input: ReorderSessionQueueInput,
) -> Result<SessionQueueView, String> {
    state
        .application
        .reorder_session_queue(&input.session_id, input.expected_revision, &input.entry_ids)
        .and_then(|_| state.application.load_session_queue_view(&input.session_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn update_queued_agent_session_message(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateQueuedMessageCommand,
) -> Result<SessionQueueView, String> {
    let session_id = input.session_id.clone();
    state
        .application
        .update_queued_message(UpdateQueuedMessageInput {
            session_id: input.session_id,
            entry_id: input.entry_id,
            expected_revision: input.expected_revision,
            submitted_text: input.submitted_text,
            model: input.model,
            reasoning_mode: input.reasoning_mode,
            sandbox_mode: input.sandbox_mode,
        })
        .and_then(|_| state.application.load_session_queue_view(&session_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn update_agent_session_target_operation(
    state: State<'_, AgentSessionTauriState>,
    input: UpdateTargetOperationCommand,
) -> Result<SessionQueueView, String> {
    let session_id = input.session_id.clone();
    state
        .application
        .update_target_operation(UpdateTargetOperationInput {
            session_id: input.session_id,
            entry_id: input.entry_id,
            expected_revision: input.expected_revision,
            destination: input.destination,
        })
        .and_then(|_| state.application.load_session_queue_view(&session_id))
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn start_agent_session_queue_entry(
    state: State<'_, AgentSessionTauriState>,
    input: QueueEntryAction,
) -> Result<(), String> {
    state
        .application
        .start_queue_entry(&input.session_id, &input.entry_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn retry_agent_session_queue_entry(
    state: State<'_, AgentSessionTauriState>,
    input: QueueEntryAction,
) -> Result<(), String> {
    state
        .application
        .retry_queue_entry(&input.session_id, &input.entry_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn cancel_agent_session_queue_entry(
    state: State<'_, AgentSessionTauriState>,
    input: QueueEntryAction,
) -> Result<(), String> {
    state
        .application
        .cancel_queue_entry(&input.session_id, &input.entry_id)
        .map_err(|error| error.to_string())
}
