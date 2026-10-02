use super::*;
use crate::agent_sessions::{
    domain::{AgentInvocationId, AgentInvocationStatus, AgentSessionId},
    ports::{
        AgentRuntimeUpdateSink, RuntimePortError, RuntimeUpdate, RuntimeUpdateDeliveryFailure,
    },
    queue::{SessionQueueEntryItem, SessionQueueEntryState, SessionQueueSnapshot},
    target_operation::{SessionTargetOperation, TargetOperationPhase, TargetSwitchRequest},
};
use std::{
    collections::{HashMap, HashSet},
    sync::{Arc, Mutex, Weak},
};

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionQueueView {
    pub(crate) session_id: AgentSessionId,
    pub(crate) revision: u64,
    pub(crate) active_target: Option<crate::execution_targets::domain::SessionExecutionTarget>,
    pub(crate) entries: Vec<SessionQueueEntryView>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionQueueEntryView {
    pub(crate) entry: crate::agent_sessions::queue::SessionQueueEntry,
    pub(crate) message: Option<QueuedMessageView>,
    pub(crate) target_operation: Option<SessionTargetOperation>,
}

#[derive(Clone, Debug, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct QueuedMessageView {
    pub(crate) invocation: crate::agent_sessions::domain::AgentInvocation,
    pub(crate) preparation: crate::agent_sessions::preparation::SessionPreparation,
}

#[derive(Default)]
pub(super) struct QueueDispatchLanes(Mutex<HashMap<AgentSessionId, Weak<Mutex<()>>>>);

impl QueueDispatchLanes {
    fn lane(&self, session_id: &AgentSessionId) -> Arc<Mutex<()>> {
        let mut lanes = self.0.lock().expect("queue lane registry");
        if let Some(lane) = lanes.get(session_id).and_then(Weak::upgrade) {
            return lane;
        }
        let lane = Arc::new(Mutex::new(()));
        lanes.insert(session_id.clone(), Arc::downgrade(&lane));
        lane
    }
}

#[derive(Default)]
pub(super) struct TargetOperationWorkers(Mutex<HashSet<String>>);

impl TargetOperationWorkers {
    pub(super) fn reserve(&self, id: &str) -> bool {
        self.0
            .lock()
            .map(|mut workers| workers.insert(id.to_string()))
            .unwrap_or(false)
    }

    pub(super) fn release(&self, id: &str) {
        if let Ok(mut workers) = self.0.lock() {
            workers.remove(id);
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct UpdateQueuedMessageInput {
    pub(crate) session_id: AgentSessionId,
    pub(crate) entry_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) submitted_text: String,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_mode: Option<String>,
    pub(crate) sandbox_mode: Option<crate::execution_configuration::SandboxMode>,
}

#[derive(Clone, Debug)]
pub(crate) struct UpdateTargetOperationInput {
    pub(crate) session_id: AgentSessionId,
    pub(crate) entry_id: String,
    pub(crate) expected_revision: u64,
    pub(crate) request: TargetSwitchRequest,
}

impl AgentSessionApplication {
    pub(crate) fn send_queued_message_now(
        &self,
        session_id: &AgentSessionId,
        entry_id: &str,
        expected_revision: u64,
        edits: Option<UpdateQueuedMessageInput>,
    ) -> Result<(), AgentSessionApplicationError> {
        // Serialize against automatic dispatch so a message cannot launch and steer twice.
        let lane = self.queue_dispatch_lanes.lane(session_id);
        let _guard = lane
            .lock()
            .map_err(|_| AgentSessionApplicationError::conflict("Session queue unavailable"))?;
        let mut queue = self.load_session_queue(session_id)?;
        if queue.revision != expected_revision {
            return Err(AgentSessionApplicationError::conflict(
                "The queue changed. Try again.",
            ));
        }
        if let Some(edits) = edits {
            if edits.session_id != *session_id
                || edits.entry_id != entry_id
                || edits.expected_revision != expected_revision
            {
                return Err(AgentSessionApplicationError::invalid(
                    "Queue edit does not match the selected message",
                ));
            }
            self.update_queued_message(edits)?;
            queue = self.load_session_queue(session_id)?;
        }
        let entry = queue
            .entries
            .iter()
            .find(|entry| entry.id == entry_id)
            .ok_or_else(|| AgentSessionApplicationError::not_found("Queue entry not found"))?;
        let SessionQueueEntryItem::Message { invocation_id } = &entry.item else {
            return Err(AgentSessionApplicationError::invalid(
                "Only queued messages can be sent immediately",
            ));
        };
        if entry.state != SessionQueueEntryState::Pending {
            return Err(AgentSessionApplicationError::conflict(
                "This message is not pending",
            ));
        }
        let history = self.load_session(session_id)?;
        if let Some(active) = history
            .invocations
            .iter()
            .find(|item| item.invocation.status == AgentInvocationStatus::Running)
        {
            let queued = self
                .repository
                .get_invocation(invocation_id)
                .map_err(AgentSessionApplicationError::repository)?
                .ok_or_else(|| {
                    AgentSessionApplicationError::not_found("Queued message not found")
                })?;
            self.repository
                .set_queue_entry_state(
                    entry_id,
                    SessionQueueEntryState::Executing,
                    None,
                    self.clock.now(),
                )
                .map_err(AgentSessionApplicationError::repository)?;
            self.notify_queue(session_id);
            let result = self.steer_session(super::SteerAgentSessionCommand {
                session_id: session_id.clone(),
                invocation_id: active.invocation.id.clone(),
                input_id: format!("queue-now:{entry_id}:{}", queue.revision),
                text: queued.submitted_text,
                attachments: queued.attachments,
            });
            let uncertain = result
                .as_ref()
                .is_ok_and(|outcome| outcome.state == "uncertain");
            let reason = result
                .as_ref()
                .ok()
                .and_then(|outcome| outcome.result.clone());
            self.repository
                .set_queue_entry_state(
                    entry_id,
                    if uncertain {
                        SessionQueueEntryState::Blocked
                    } else {
                        SessionQueueEntryState::Pending
                    },
                    reason,
                    self.clock.now(),
                )
                .map_err(AgentSessionApplicationError::repository)?;
            match result {
                Ok(outcome) if matches!(outcome.state.as_str(), "accepted" | "queued") => {
                    self.cancel_queue_entry(session_id, entry_id)?
                }
                Ok(outcome) => {
                    self.notify_queue(session_id);
                    return Err(AgentSessionApplicationError::conflict(
                        outcome.result.unwrap_or_else(|| {
                            format!(
                                "Steering {}. The queued message was retained.",
                                outcome.state
                            )
                        }),
                    ));
                }
                Err(error) => {
                    self.notify_queue(session_id);
                    return Err(error);
                }
            }
        } else {
            // Preserve preceding target operations: priority must not change the destination.
            let dependencies = queue
                .entries
                .iter()
                .take_while(|item| item.id != entry_id)
                .filter(|item| {
                    item.state.is_unfinished()
                        && matches!(item.item, SessionQueueEntryItem::TargetOperation { .. })
                })
                .map(|item| item.id.clone())
                .collect::<Vec<_>>();
            let mut ids = dependencies.clone();
            ids.push(entry_id.to_string());
            ids.extend(
                queue
                    .entries
                    .iter()
                    .filter(|item| {
                        item.state.is_unfinished()
                            && item.id != entry_id
                            && !dependencies.contains(&item.id)
                    })
                    .map(|item| item.id.clone()),
            );
            self.reorder_session_queue(session_id, queue.revision, &ids)?;
        }
        Ok(())
    }

    pub(crate) fn enqueue_session_compaction(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<SessionQueueSnapshot, AgentSessionApplicationError> {
        let session = self
            .repository
            .get_session(session_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent Session not found"))?;
        if session.runtime_binding.external_context_id.is_none() {
            return Err(AgentSessionApplicationError::conflict(
                "This conversation has no Codex context to compact",
            ));
        }
        let current_profile = self
            .load_current_session_profile(session_id)
            .map_err(|error| AgentSessionApplicationError::invalid(error.to_string()))?;
        let provider = &current_profile
            .creation_resolution
            .session_profile()
            .configuration()
            .provider;
        if provider != "codex" {
            return Err(AgentSessionApplicationError::invalid(
                "Conversation compaction is available only for Codex sessions",
            ));
        }
        let submission_id = self.allocate_application_invocation_id();
        self.accept_prepared_inner(
            super::preparation::PreparedMessageInput {
                session_id: Some(session.id.clone()),
                submission_id,
                submitted_text: "/compact".into(),
                attachments: Vec::new(),
                context: Vec::new(),
                title: None,
                working_directory: session.working_directory,
                execution_selection: None,
                model: None,
                reasoning_mode: None,
                sandbox_mode: None,
                folder_target: None,
            },
            crate::agent_sessions::domain::AgentInvocationInputProvenance::Application,
        )?;
        self.load_session_queue(session_id)
    }

    pub(crate) fn load_session_queue_view(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<SessionQueueView, AgentSessionApplicationError> {
        let snapshot = self.load_session_queue(session_id)?;
        let session = self
            .repository
            .get_session(session_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent Session not found"))?;
        let mut entries = Vec::with_capacity(snapshot.entries.len());
        for entry in snapshot.entries {
            let (message, target_operation) = match &entry.item {
                SessionQueueEntryItem::Message { invocation_id }
                | SessionQueueEntryItem::Compaction { invocation_id } => {
                    let invocation = self
                        .repository
                        .get_invocation(invocation_id)
                        .map_err(AgentSessionApplicationError::repository)?
                        .ok_or_else(|| {
                            AgentSessionApplicationError::not_found("Queued invocation not found")
                        })?;
                    let preparation = self
                        .repository
                        .preparation(invocation_id)
                        .map_err(AgentSessionApplicationError::repository)?
                        .ok_or_else(|| {
                            AgentSessionApplicationError::not_found("Queued preparation not found")
                        })?;
                    (
                        Some(QueuedMessageView {
                            invocation,
                            preparation,
                        }),
                        None,
                    )
                }
                SessionQueueEntryItem::TargetOperation { operation_id } => (
                    None,
                    self.repository
                        .target_operation(operation_id)
                        .map_err(AgentSessionApplicationError::repository)?,
                ),
            };
            entries.push(SessionQueueEntryView {
                entry,
                message,
                target_operation,
            });
        }
        Ok(SessionQueueView {
            session_id: snapshot.session_id,
            revision: snapshot.revision,
            active_target: session.execution_target,
            entries,
        })
    }

    pub(crate) fn load_session_queue(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<SessionQueueSnapshot, AgentSessionApplicationError> {
        self.repository
            .session_queue(session_id)
            .map_err(AgentSessionApplicationError::repository)
    }

    pub(crate) fn reorder_session_queue(
        &self,
        session_id: &AgentSessionId,
        expected_revision: u64,
        entry_ids: &[String],
    ) -> Result<SessionQueueSnapshot, AgentSessionApplicationError> {
        let snapshot = self
            .repository
            .reorder_session_queue(session_id, expected_revision, entry_ids, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(session_id);
        self.wake_session_queue(session_id.clone());
        Ok(snapshot)
    }

    pub(crate) fn update_queued_message(
        &self,
        input: UpdateQueuedMessageInput,
    ) -> Result<SessionQueueSnapshot, AgentSessionApplicationError> {
        if self
            .load_session_queue(&input.session_id)?
            .entries
            .iter()
            .any(|entry| {
                entry.id == input.entry_id
                    && matches!(entry.item, SessionQueueEntryItem::Compaction { .. })
            })
        {
            return Err(AgentSessionApplicationError::invalid(
                "Compaction queue entries cannot be edited as messages",
            ));
        }
        let snapshot = self
            .repository
            .update_queued_message(
                &input.session_id,
                &input.entry_id,
                input.expected_revision,
                &input.submitted_text,
                input.model,
                input.reasoning_mode,
                input.sandbox_mode,
                self.clock.now(),
            )
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(&input.session_id);
        self.wake_session_queue(input.session_id);
        Ok(snapshot)
    }

    pub(crate) fn retry_queue_entry(
        &self,
        session_id: &AgentSessionId,
        entry_id: &str,
    ) -> Result<(), AgentSessionApplicationError> {
        let snapshot = self.load_session_queue(session_id)?;
        let entry = snapshot
            .entries
            .iter()
            .find(|entry| entry.id == entry_id)
            .ok_or_else(|| AgentSessionApplicationError::not_found("Queue entry not found"))?;
        match &entry.item {
            SessionQueueEntryItem::Message { .. } | SessionQueueEntryItem::Compaction { .. } => {
                self.repository
                    .retry_queue_entry(entry_id, self.clock.now())
                    .map_err(AgentSessionApplicationError::repository)?;
            }
            SessionQueueEntryItem::TargetOperation { .. } => {
                self.repository
                    .retry_queue_entry(entry_id, self.clock.now())
                    .map_err(AgentSessionApplicationError::repository)?;
            }
        }
        self.notify_queue(session_id);
        self.wake_session_queue(session_id.clone());
        Ok(())
    }

    pub(crate) fn cancel_queue_entry(
        &self,
        session_id: &AgentSessionId,
        entry_id: &str,
    ) -> Result<(), AgentSessionApplicationError> {
        let before = self
            .load_session(session_id)?
            .invocations
            .into_iter()
            .map(|i| i.invocation.id.as_str().to_string())
            .collect::<Vec<_>>();
        self.repository
            .discard_queue_entry(session_id, entry_id, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)?;
        let after = self
            .load_session(session_id)?
            .invocations
            .into_iter()
            .map(|i| i.invocation.id.as_str().to_string())
            .collect::<HashSet<_>>();
        self.notify_queue_removals(
            session_id,
            before
                .into_iter()
                .filter(|id| !after.contains(id))
                .collect(),
        );
        self.notify_queue(session_id);
        self.wake_session_queue(session_id.clone());
        Ok(())
    }

    pub(super) fn wake_session_queue(&self, session_id: AgentSessionId) {
        let application = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _ = application.dispatch_session_queue(&session_id);
        });
    }

    pub(crate) fn run_target_operation_now(
        &self,
        session_id: &AgentSessionId,
        entry_id: &str,
    ) -> Result<(), AgentSessionApplicationError> {
        let lane = self.queue_dispatch_lanes.lane(session_id);
        let _guard = lane
            .lock()
            .map_err(|_| AgentSessionApplicationError::conflict("Session queue unavailable"))?;
        let queue = self.load_session_queue(session_id)?;
        let head = queue
            .entries
            .iter()
            .find(|e| e.state.is_unfinished())
            .ok_or_else(|| AgentSessionApplicationError::not_found("Queue entry not found"))?;
        if head.id != entry_id || head.state != SessionQueueEntryState::Pending {
            return Err(AgentSessionApplicationError::conflict(
                "Only the next waiting operation can be started",
            ));
        }
        let SessionQueueEntryItem::TargetOperation { operation_id } = &head.item else {
            return Err(AgentSessionApplicationError::invalid(
                "Not a target operation",
            ));
        };
        self.repository
            .set_queue_entry_state(
                entry_id,
                SessionQueueEntryState::Executing,
                None,
                self.clock.now(),
            )
            .map_err(AgentSessionApplicationError::repository)?;
        self.start_head_target_operation(operation_id.clone())?;
        self.notify_queue(session_id);
        Ok(())
    }

    pub(super) fn wake_all_session_queues(&self) {
        let sessions = self.repository.list_session_summaries(
            crate::agent_sessions::ports::ListAgentSessionsQuery::default(),
        );
        if let Ok(sessions) = sessions {
            for session in sessions {
                self.wake_session_queue(session.session.id);
            }
        }
    }

    pub(super) fn dispatch_session_queue(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<(), AgentSessionApplicationError> {
        let lane = self.queue_dispatch_lanes.lane(session_id);
        let _guard = lane
            .lock()
            .map_err(|_| AgentSessionApplicationError::conflict("Session queue unavailable"))?;
        loop {
            let snapshot = self.load_session_queue(session_id)?;
            let has_waiting_message = snapshot.entries.iter().any(|entry| {
                entry.state == SessionQueueEntryState::Pending
                    && matches!(
                        entry.item,
                        SessionQueueEntryItem::Message { .. }
                            | SessionQueueEntryItem::Compaction { .. }
                    )
            });
            let Some(entry) = snapshot
                .entries
                .into_iter()
                .find(|entry| entry.state.is_unfinished())
            else {
                return Ok(());
            };
            if matches!(
                entry.state,
                SessionQueueEntryState::Executing | SessionQueueEntryState::Blocked
            ) {
                return Ok(());
            }
            if self
                .load_session(session_id)?
                .invocations
                .iter()
                .any(|i| i.invocation.status == AgentInvocationStatus::Running)
            {
                return Ok(());
            }
            match entry.item {
                SessionQueueEntryItem::TargetOperation { operation_id } => {
                    if self
                        .load_session(session_id)?
                        .invocations
                        .iter()
                        .any(|item| item.invocation.status == AgentInvocationStatus::Running)
                    {
                        return Ok(());
                    }
                    if entry.state == SessionQueueEntryState::Pending && !has_waiting_message {
                        return Ok(());
                    }
                    let operation = self
                        .repository
                        .target_operation(&operation_id)
                        .map_err(AgentSessionApplicationError::repository)?
                        .ok_or_else(|| {
                            AgentSessionApplicationError::not_found("Target operation not found")
                        })?;
                    match operation.phase {
                        TargetOperationPhase::Pending | TargetOperationPhase::Staged => {
                            self.repository
                                .set_queue_entry_state(
                                    &entry.id,
                                    SessionQueueEntryState::Executing,
                                    None,
                                    self.clock.now(),
                                )
                                .map_err(AgentSessionApplicationError::repository)?;
                            self.start_head_target_operation(operation_id)?;
                            return Ok(());
                        }
                        TargetOperationPhase::Staging => return Ok(()),
                        TargetOperationPhase::Blocked => {
                            self.repository
                                .set_queue_entry_state(
                                    &entry.id,
                                    SessionQueueEntryState::Blocked,
                                    operation.error,
                                    self.clock.now(),
                                )
                                .map_err(AgentSessionApplicationError::repository)?;
                            self.notify_queue(session_id);
                            return Ok(());
                        }
                        TargetOperationPhase::Completed => {
                            self.repository
                                .set_queue_entry_state(
                                    &entry.id,
                                    SessionQueueEntryState::Completed,
                                    None,
                                    self.clock.now(),
                                )
                                .map_err(AgentSessionApplicationError::repository)?;
                            continue;
                        }
                        TargetOperationPhase::Canceled | TargetOperationPhase::Activating => {
                            return Ok(())
                        }
                    }
                }
                SessionQueueEntryItem::Message { invocation_id }
                | SessionQueueEntryItem::Compaction { invocation_id } => {
                    let history = self.load_session(session_id)?;
                    if history
                        .invocations
                        .iter()
                        .any(|item| item.invocation.status == AgentInvocationStatus::Running)
                    {
                        return Ok(());
                    }
                    let cancel = match self.preparation_workers.reserve(&invocation_id) {
                        Ok(cancel) => cancel,
                        Err(_) => return Ok(()),
                    };
                    self.repository
                        .set_queue_entry_state(
                            &entry.id,
                            SessionQueueEntryState::Executing,
                            None,
                            self.clock.now(),
                        )
                        .map_err(AgentSessionApplicationError::repository)?;
                    self.notify_queue(session_id);
                    self.spawn_preparation(invocation_id, cancel);
                    return Ok(());
                }
            }
        }
    }

    pub(super) fn complete_queued_invocation(&self, invocation_id: &AgentInvocationId) {
        let entry = self.repository.queue_entry_for_invocation(invocation_id);
        let Ok(Some(entry)) = entry else { return };
        if self
            .repository
            .set_queue_entry_state(
                &entry.id,
                SessionQueueEntryState::Completed,
                None,
                self.clock.now(),
            )
            .is_ok()
        {
            self.notify_queue(&entry.session_id);
            self.wake_session_queue(entry.session_id);
        }
    }

    pub(super) fn block_queued_invocation(
        &self,
        invocation_id: &AgentInvocationId,
        reason: String,
    ) {
        let entry = self.repository.queue_entry_for_invocation(invocation_id);
        let Ok(Some(entry)) = entry else { return };
        if self
            .repository
            .set_queue_entry_state(
                &entry.id,
                SessionQueueEntryState::Blocked,
                Some(reason),
                self.clock.now(),
            )
            .is_ok()
        {
            self.notify_queue(&entry.session_id);
        }
    }

    pub(super) fn notify_queue(&self, session_id: &AgentSessionId) {
        self.notify_queue_removals(session_id, Vec::new());
    }
    fn notify_queue_removals(
        &self,
        session_id: &AgentSessionId,
        removed_invocation_ids: Vec<String>,
    ) {
        if let Ok(history) = self.load_session(session_id) {
            let _ = self
                .notifier
                .notify(AgentSessionNotification::QueueUpdated {
                    session_id: session_id.clone(),
                    session: history.session,
                    timeline: history.timeline,
                    removed_invocation_ids,
                });
        }
    }
}

pub(super) struct QueueDispatchRuntimeUpdateSink {
    inner: Arc<dyn AgentRuntimeUpdateSink>,
    application: AgentSessionApplication,
}

impl QueueDispatchRuntimeUpdateSink {
    pub(super) fn new(
        inner: Arc<dyn AgentRuntimeUpdateSink>,
        application: AgentSessionApplication,
    ) -> Self {
        Self { inner, application }
    }
}

impl AgentRuntimeUpdateSink for QueueDispatchRuntimeUpdateSink {
    fn emit_update(
        &self,
        invocation_id: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError> {
        let terminal = matches!(update, RuntimeUpdate::Finished(_));
        let result = self.inner.emit_update(invocation_id, update);
        if terminal && result.is_ok() {
            self.application.complete_queued_invocation(invocation_id);
        }
        result
    }

    fn report_delivery_failure(
        &self,
        invocation_id: &AgentInvocationId,
        failure: RuntimeUpdateDeliveryFailure,
    ) {
        self.inner.report_delivery_failure(invocation_id, failure)
    }
}
