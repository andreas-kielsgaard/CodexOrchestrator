use super::*;
use crate::agent_sessions::{
    domain::{AgentInvocationId, AgentInvocationStatus, AgentSessionId},
    ports::{
        AgentRuntimeUpdateSink, RuntimePortError, RuntimeUpdate, RuntimeUpdateDeliveryFailure,
    },
    queue::{SessionQueueEntryItem, SessionQueueEntryState, SessionQueueSnapshot},
    target_operation::{
        planned_actions, replan_actions, selection_target_relation, SelectionTargetRelation,
        SessionTargetOperation, TargetOperationActionKind, TargetOperationActionStatus,
        TargetOperationPhase,
    },
};
use crate::execution_targets::domain::{SessionExecutionSelection, SessionWorkspaceSelection};
use orchid_engine::protocol::{WorktreeHeadRelation, WorktreeInspection};
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
    fn reserve(&self, id: &str) -> bool {
        self.0
            .lock()
            .map(|mut workers| workers.insert(id.to_string()))
            .unwrap_or(false)
    }

    fn release(&self, id: &str) {
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
    pub(crate) destination: SessionExecutionSelection,
}

impl AgentSessionApplication {
    pub(crate) fn enqueue_session_compaction(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<SessionQueueSnapshot, AgentSessionApplicationError> {
        let session = self.load_session(session_id)?.session;
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
        let session = self.load_session(session_id)?.session;
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

    pub(crate) fn enqueue_session_target_operation(
        &self,
        session_id: &AgentSessionId,
        destination: SessionExecutionSelection,
    ) -> Result<SessionQueueSnapshot, AgentSessionApplicationError> {
        let session = self.load_session(session_id)?.session;
        let Some(operation) = self.target_operation_for_submission(&session, Some(&destination))?
        else {
            return self.load_session_queue(session_id);
        };
        let operation_id = operation.id.clone();
        let snapshot = self
            .repository
            .enqueue_target_operation(operation, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(session_id);
        self.start_target_operation_early(operation_id)?;
        self.wake_session_queue(session_id.clone());
        Ok(snapshot)
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

    pub(crate) fn update_target_operation(
        &self,
        input: UpdateTargetOperationInput,
    ) -> Result<SessionQueueSnapshot, AgentSessionApplicationError> {
        let snapshot = self
            .repository
            .replace_target_operation(
                &input.session_id,
                &input.entry_id,
                input.expected_revision,
                input.destination,
                self.clock.now(),
            )
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(&input.session_id);
        if let Some(operation_id) = snapshot.entries.iter().find_map(|entry| {
            (entry.id == input.entry_id).then(|| match &entry.item {
                SessionQueueEntryItem::TargetOperation { operation_id } => {
                    Some(operation_id.clone())
                }
                SessionQueueEntryItem::Message { .. }
                | SessionQueueEntryItem::Compaction { .. } => None,
            })?
        }) {
            self.start_target_operation_early(operation_id)?;
        }
        self.wake_session_queue(input.session_id);
        Ok(snapshot)
    }

    pub(crate) fn start_queue_entry(
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
        let SessionQueueEntryItem::TargetOperation { operation_id } = &entry.item else {
            return Err(AgentSessionApplicationError::invalid(
                "Only target operations can start early",
            ));
        };
        self.start_target_operation_early(operation_id.clone())
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
            SessionQueueEntryItem::Message { .. }
            | SessionQueueEntryItem::Compaction { .. } => {
                self.repository
                    .retry_queue_entry(entry_id, self.clock.now())
                    .map_err(AgentSessionApplicationError::repository)?;
            }
            SessionQueueEntryItem::TargetOperation { operation_id } => {
                self.repository
                    .retry_queue_entry(entry_id, self.clock.now())
                    .map_err(AgentSessionApplicationError::repository)?;
                self.start_target_operation_early(operation_id.clone())?;
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
        let snapshot = self.load_session_queue(session_id)?;
        let entry = snapshot
            .entries
            .iter()
            .find(|entry| entry.id == entry_id)
            .ok_or_else(|| AgentSessionApplicationError::not_found("Queue entry not found"))?;
        if entry.state == SessionQueueEntryState::Executing {
            return Err(AgentSessionApplicationError::conflict(
                "Cancel the active turn from the composer",
            ));
        }
        match &entry.item {
            SessionQueueEntryItem::Message { invocation_id }
            | SessionQueueEntryItem::Compaction { invocation_id } => {
                if let Some(invocation) = self
                    .repository
                    .get_invocation(invocation_id)
                    .map_err(AgentSessionApplicationError::repository)?
                {
                    if invocation.status == AgentInvocationStatus::Pending {
                        self.repository
                            .finish_invocation(
                                invocation_id,
                                crate::agent_sessions::domain::InvocationCompletion {
                                    status: crate::agent_sessions::domain::AgentInvocationTerminalStatus::Canceled,
                                    completed_at: self.clock.now(),
                                    exit_code: None,
                                    signal: None,
                                    runtime_error: None,
                                },
                                self.clock.now(),
                            )
                            .map_err(AgentSessionApplicationError::repository)?;
                    }
                }
            }
            SessionQueueEntryItem::TargetOperation { operation_id } => {
                let mut operation = self
                    .repository
                    .target_operation(operation_id)
                    .map_err(AgentSessionApplicationError::repository)?
                    .ok_or_else(|| {
                        AgentSessionApplicationError::not_found("Target operation not found")
                    })?;
                operation.phase = TargetOperationPhase::Canceled;
                operation.updated_at = self.clock.now();
                self.repository
                    .save_target_operation(&operation)
                    .map_err(AgentSessionApplicationError::repository)?;
            }
        }
        self.repository
            .set_queue_entry_state(
                entry_id,
                SessionQueueEntryState::Canceled,
                None,
                self.clock.now(),
            )
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(session_id);
        self.wake_session_queue(session_id.clone());
        Ok(())
    }

    pub(super) fn target_operation_for_submission(
        &self,
        session: &crate::agent_sessions::domain::AgentSession,
        selection: Option<&SessionExecutionSelection>,
    ) -> Result<Option<SessionTargetOperation>, AgentSessionApplicationError> {
        let Some(selection) = selection else {
            return Ok(None);
        };
        let mut projected_matches =
            selection_target_relation(selection, session.execution_target.as_ref())
                != SelectionTargetRelation::Different;
        if let Ok(queue) = self.repository.session_queue(&session.id) {
            for operation_id in queue.entries.iter().filter_map(|entry| match &entry.item {
                SessionQueueEntryItem::TargetOperation { operation_id }
                    if entry.state.is_unfinished() =>
                {
                    Some(operation_id)
                }
                _ => None,
            }) {
                if let Some(operation) = self
                    .repository
                    .target_operation(operation_id)
                    .map_err(AgentSessionApplicationError::repository)?
                {
                    projected_matches =
                        same_selection_intent(selection, &operation.destination_selection);
                }
            }
        }
        if projected_matches {
            return Ok(None);
        }
        let now = self.clock.now();
        Ok(Some(SessionTargetOperation::new(
            uuid::Uuid::new_v4().to_string(),
            session.id.clone(),
            session.execution_target.clone(),
            selection.clone(),
            now,
        )))
    }

    pub(super) fn wake_session_queue(&self, session_id: AgentSessionId) {
        let application = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let _ = application.dispatch_session_queue(&session_id);
        });
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

    fn dispatch_session_queue(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<(), AgentSessionApplicationError> {
        let lane = self.queue_dispatch_lanes.lane(session_id);
        let _guard = lane
            .lock()
            .map_err(|_| AgentSessionApplicationError::conflict("Session queue unavailable"))?;
        loop {
            let snapshot = self.load_session_queue(session_id)?;
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
            match entry.item {
                SessionQueueEntryItem::TargetOperation { operation_id } => {
                    let operation = self
                        .repository
                        .target_operation(&operation_id)
                        .map_err(AgentSessionApplicationError::repository)?
                        .ok_or_else(|| {
                            AgentSessionApplicationError::not_found("Target operation not found")
                        })?;
                    match operation.phase {
                        TargetOperationPhase::Staged => {
                            self.repository
                                .set_queue_entry_state(
                                    &entry.id,
                                    SessionQueueEntryState::Executing,
                                    None,
                                    self.clock.now(),
                                )
                                .map_err(AgentSessionApplicationError::repository)?;
                            if let Err(error) = self.activate_staged_target_operation(&operation_id)
                            {
                                self.block_target_operation(&operation_id, error.to_string())?;
                                return Ok(());
                            }
                            self.notify_queue(session_id);
                            continue;
                        }
                        TargetOperationPhase::Pending => {
                            self.start_target_operation_early(operation_id)?;
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

    pub(super) fn start_target_operation_early(
        &self,
        operation_id: String,
    ) -> Result<(), AgentSessionApplicationError> {
        if !self.target_operation_workers.reserve(&operation_id) {
            return Ok(());
        }
        let mut operation = self
            .repository
            .target_operation(&operation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Target operation not found"))?;
        if !matches!(
            operation.phase,
            TargetOperationPhase::Pending | TargetOperationPhase::Blocked
        ) {
            self.target_operation_workers.release(&operation_id);
            return Ok(());
        }
        let active_target = self
            .load_session(&operation.session_id)?
            .session
            .execution_target;
        if selection_target_relation(&operation.destination_selection, active_target.as_ref())
            != SelectionTargetRelation::Different
        {
            let target = active_target.expect("a matching target exists");
            operation.destination_selection = SessionExecutionSelection {
                capability_profile_id: target.capability_profile_id.clone(),
                capability_profile_revision: target.capability_profile_revision,
                execution: target.execution.clone(),
                workspace: SessionWorkspaceSelection::Existing {
                    target: target.clone(),
                },
            };
            operation.actions = planned_actions(
                operation.source_target.as_ref(),
                &operation.destination_selection,
            );
            for kind in [
                TargetOperationActionKind::PrepareDevice,
                TargetOperationActionKind::VerifyWorktree,
            ] {
                set_action(
                    &mut operation,
                    kind,
                    TargetOperationActionStatus::Completed,
                    Some("The Session already uses this target".into()),
                    None,
                );
            }
            operation.staged_target = Some(target);
            operation.phase = TargetOperationPhase::Staged;
            operation.error = None;
            operation.updated_at = self.clock.now();
            self.repository
                .save_target_operation(&operation)
                .map_err(AgentSessionApplicationError::repository)?;
            self.target_operation_workers.release(&operation_id);
            self.notify_queue(&operation.session_id);
            self.wake_session_queue(operation.session_id);
            return Ok(());
        }
        operation.phase = TargetOperationPhase::Staging;
        operation.error = None;
        operation.updated_at = self.clock.now();
        set_action(
            &mut operation,
            TargetOperationActionKind::PrepareDevice,
            TargetOperationActionStatus::Running,
            None,
            None,
        );
        self.repository
            .save_target_operation(&operation)
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(&operation.session_id);
        let application = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let result = application.run_target_operation_stage(&operation_id);
            let changed_while_staging = result.as_ref().err().is_some_and(|error| {
                error
                    .to_string()
                    .contains("Target operation changed while staging")
            });
            if let Err(error) = result {
                if !changed_while_staging {
                    let _ = application.block_target_operation(&operation_id, error.to_string());
                }
            }
            application.target_operation_workers.release(&operation_id);
            if changed_while_staging {
                let _ = application.start_target_operation_early(operation_id.clone());
            }
            application.wake_all_session_queues();
        });
        Ok(())
    }

    fn run_target_operation_stage(
        &self,
        operation_id: &str,
    ) -> Result<(), AgentSessionApplicationError> {
        let mut operation = self
            .repository
            .target_operation(operation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Target operation not found"))?;
        let generation = operation.generation;
        let targets = self.execution_target_service.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid("Execution targets unavailable")
        })?;
        targets
            .ensure_ready(&operation.destination_selection.execution)
            .map_err(AgentSessionApplicationError::invalid)?;
        set_action(
            &mut operation,
            TargetOperationActionKind::PrepareDevice,
            TargetOperationActionStatus::Completed,
            Some("Destination device is ready".into()),
            None,
        );
        set_action(
            &mut operation,
            TargetOperationActionKind::ResolveRevision,
            TargetOperationActionStatus::Running,
            None,
            None,
        );
        self.save_staging_operation(&operation, generation)?;

        if let SessionWorkspaceSelection::Create {
            repository_id,
            branch_ref,
            commit,
            ..
        } = &mut operation.destination_selection.workspace
        {
            if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                let root = targets
                    .repository_root(repository_id, &operation.destination_selection.execution)
                    .map_err(AgentSessionApplicationError::invalid)?;
                *commit = targets
                    .endpoints
                    .branch_commit(
                        &operation.destination_selection.execution,
                        &root,
                        branch_ref,
                    )
                    .map_err(AgentSessionApplicationError::invalid)?;
            }
        }
        set_action(
            &mut operation,
            TargetOperationActionKind::ResolveRevision,
            TargetOperationActionStatus::Completed,
            None,
            None,
        );
        set_action(
            &mut operation,
            TargetOperationActionKind::MaterializeWorktree,
            TargetOperationActionStatus::Running,
            None,
            None,
        );
        self.save_staging_operation(&operation, generation)?;
        let target = targets
            .materialize_selection(&operation.destination_selection, operation_id)
            .map_err(AgentSessionApplicationError::invalid)?;
        set_action(
            &mut operation,
            TargetOperationActionKind::MaterializeWorktree,
            TargetOperationActionStatus::Completed,
            Some(target.path.clone()),
            None,
        );
        set_action(
            &mut operation,
            TargetOperationActionKind::VerifyWorktree,
            TargetOperationActionStatus::Running,
            None,
            None,
        );
        self.save_staging_operation(&operation, generation)?;
        if !target.repository_id.is_empty() {
            let head = target.head.as_deref().ok_or_else(|| {
                AgentSessionApplicationError::invalid(
                    "The selected worktree did not report its current HEAD",
                )
            })?;
            set_action(
                &mut operation,
                TargetOperationActionKind::VerifyWorktree,
                TargetOperationActionStatus::Completed,
                Some(format!(
                    "HEAD {}",
                    head.chars().take(10).collect::<String>()
                )),
                None,
            );
        } else {
            set_action(
                &mut operation,
                TargetOperationActionKind::VerifyWorktree,
                TargetOperationActionStatus::Completed,
                Some("Workspace is ready".into()),
                None,
            );
        }
        operation.staged_target = Some(target);
        operation.phase = TargetOperationPhase::Staged;
        operation.updated_at = self.clock.now();
        self.save_staging_operation(&operation, generation)?;
        self.notify_queue(&operation.session_id);
        Ok(())
    }

    fn activate_staged_target_operation(
        &self,
        operation_id: &str,
    ) -> Result<(), AgentSessionApplicationError> {
        let mut operation = self
            .repository
            .target_operation(operation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Target operation not found"))?;
        if operation.phase != TargetOperationPhase::Staged {
            return Err(AgentSessionApplicationError::conflict(
                "Target operation is not ready to activate",
            ));
        }
        let destination = operation
            .staged_target
            .clone()
            .ok_or_else(|| AgentSessionApplicationError::not_found("Staged target unavailable"))?;
        let source = self
            .load_session(&operation.session_id)?
            .session
            .execution_target;
        operation.source_target = source.clone();
        operation.actions = replan_actions(
            &operation.actions,
            source.as_ref(),
            &operation.destination_selection,
        );
        operation.phase = TargetOperationPhase::Activating;
        operation.updated_at = self.clock.now();
        self.repository
            .save_target_operation(&operation)
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(&operation.session_id);

        let migrate = source.as_ref().is_some_and(|source| {
            !source.repository_id.is_empty()
                && source.repository_id == destination.repository_id
                && source.branch_ref == destination.branch_ref
                && source.execution.device_id != destination.execution.device_id
        });
        if let Some(source) = source.filter(|_| migrate) {
            let targets = self.execution_target_service.as_ref().ok_or_else(|| {
                AgentSessionApplicationError::invalid("Execution targets unavailable")
            })?;
            let endpoints = self.endpoints.as_ref().ok_or_else(|| {
                AgentSessionApplicationError::invalid("Execution endpoints unavailable")
            })?;
            set_action(
                &mut operation,
                TargetOperationActionKind::InspectSource,
                TargetOperationActionStatus::Running,
                None,
                None,
            );
            self.save_target_activation_progress(&operation)?;
            let source_inspection = endpoints
                .inspect_worktree(&source.execution, &source.path, destination.head.as_deref())
                .map_err(AgentSessionApplicationError::invalid)?;
            set_action(
                &mut operation,
                TargetOperationActionKind::InspectSource,
                TargetOperationActionStatus::Completed,
                Some(format!(
                    "HEAD {}",
                    source_inspection.head.chars().take(10).collect::<String>()
                )),
                None,
            );

            set_action(
                &mut operation,
                TargetOperationActionKind::InspectDestination,
                TargetOperationActionStatus::Running,
                None,
                None,
            );
            self.save_target_activation_progress(&operation)?;
            let destination_inspection = endpoints
                .inspect_worktree(&destination.execution, &destination.path, None)
                .map_err(AgentSessionApplicationError::invalid)?;
            validate_migration_facts(&source_inspection, Some(&destination_inspection))?;
            set_action(
                &mut operation,
                TargetOperationActionKind::InspectDestination,
                TargetOperationActionStatus::Completed,
                Some(format!(
                    "HEAD {}",
                    destination_inspection
                        .head
                        .chars()
                        .take(10)
                        .collect::<String>()
                )),
                None,
            );

            set_action(
                &mut operation,
                TargetOperationActionKind::CaptureSnapshot,
                TargetOperationActionStatus::Running,
                None,
                None,
            );
            self.save_target_activation_progress(&operation)?;
            let snapshot = endpoints
                .capture_worktree_snapshot(
                    &source.execution,
                    &source.path,
                    destination.head.as_deref(),
                    &format!(
                        "{}-{}",
                        operation.session_id.as_str(),
                        uuid::Uuid::new_v4().simple()
                    ),
                )
                .map_err(AgentSessionApplicationError::invalid)?;
            set_action(
                &mut operation,
                TargetOperationActionKind::CaptureSnapshot,
                TargetOperationActionStatus::Completed,
                Some(format!("{} bytes", snapshot.descriptor.total_bytes)),
                None,
            );

            set_action(
                &mut operation,
                TargetOperationActionKind::TransferSnapshot,
                TargetOperationActionStatus::Running,
                None,
                None,
            );
            set_action(
                &mut operation,
                TargetOperationActionKind::ApplySnapshot,
                TargetOperationActionStatus::Running,
                None,
                None,
            );
            self.save_target_activation_progress(&operation)?;
            let applied = endpoints
                .apply_worktree_snapshot(&destination.execution, &destination.path, &snapshot)
                .map_err(AgentSessionApplicationError::invalid)?;
            set_action(
                &mut operation,
                TargetOperationActionKind::TransferSnapshot,
                TargetOperationActionStatus::Completed,
                Some("Snapshot transferred".into()),
                None,
            );
            set_action(
                &mut operation,
                TargetOperationActionKind::ApplySnapshot,
                TargetOperationActionStatus::Completed,
                Some(format!(
                    "HEAD {}",
                    applied.head.chars().take(10).collect::<String>()
                )),
                None,
            );

            set_action(
                &mut operation,
                TargetOperationActionKind::VerifyMigratedState,
                TargetOperationActionStatus::Running,
                None,
                None,
            );
            self.save_target_activation_progress(&operation)?;
            let verified = endpoints
                .inspect_worktree(
                    &destination.execution,
                    &destination.path,
                    Some(&source_inspection.head),
                )
                .map_err(AgentSessionApplicationError::invalid)?;
            if verified
                .comparison
                .as_ref()
                .map(|comparison| comparison.relation)
                != Some(WorktreeHeadRelation::Equal)
            {
                return Err(AgentSessionApplicationError::conflict(
                    "The destination HEAD did not reach the source HEAD",
                ));
            }
            set_action(
                &mut operation,
                TargetOperationActionKind::VerifyMigratedState,
                TargetOperationActionStatus::Completed,
                Some("Destination matches source".into()),
                None,
            );

            set_action(
                &mut operation,
                TargetOperationActionKind::ActivateSister,
                TargetOperationActionStatus::Running,
                None,
                None,
            );
            self.save_target_activation_progress(&operation)?;
            targets
                .sisters
                .activate(
                    &source,
                    &destination,
                    operation.session_id.as_str(),
                    self.clock.now(),
                )
                .map_err(AgentSessionApplicationError::conflict)?;
            set_action(
                &mut operation,
                TargetOperationActionKind::ActivateSister,
                TargetOperationActionStatus::Completed,
                Some("Destination sister is active".into()),
                None,
            );
        } else {
            for kind in [
                TargetOperationActionKind::InspectSource,
                TargetOperationActionKind::InspectDestination,
                TargetOperationActionKind::CaptureSnapshot,
                TargetOperationActionKind::TransferSnapshot,
                TargetOperationActionKind::ApplySnapshot,
                TargetOperationActionKind::VerifyMigratedState,
                TargetOperationActionKind::ActivateSister,
            ] {
                set_action(
                    &mut operation,
                    kind,
                    TargetOperationActionStatus::Completed,
                    Some("Not required".into()),
                    None,
                );
            }
        }
        set_action(
            &mut operation,
            TargetOperationActionKind::SelectProviderContext,
            TargetOperationActionStatus::Completed,
            Some("Provider context will match this workspace".into()),
            None,
        );
        set_action(
            &mut operation,
            TargetOperationActionKind::ActivateTarget,
            TargetOperationActionStatus::Running,
            None,
            None,
        );
        self.save_target_activation_progress(&operation)?;
        self.repository
            .activate_target_operation(operation_id, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)?;
        Ok(())
    }

    fn save_target_activation_progress(
        &self,
        operation: &SessionTargetOperation,
    ) -> Result<(), AgentSessionApplicationError> {
        self.repository
            .save_target_operation(operation)
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(&operation.session_id);
        Ok(())
    }

    fn save_staging_operation(
        &self,
        operation: &SessionTargetOperation,
        generation: u64,
    ) -> Result<(), AgentSessionApplicationError> {
        let current = self
            .repository
            .target_operation(&operation.id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Target operation not found"))?;
        if current.generation != generation || current.phase != TargetOperationPhase::Staging {
            return Err(AgentSessionApplicationError::conflict(
                "Target operation changed while staging",
            ));
        }
        self.repository
            .save_target_operation(operation)
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(&operation.session_id);
        Ok(())
    }

    pub(super) fn block_target_operation(
        &self,
        operation_id: &str,
        message: String,
    ) -> Result<(), AgentSessionApplicationError> {
        let mut operation = self
            .repository
            .target_operation(operation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Target operation not found"))?;
        if operation.phase == TargetOperationPhase::Canceled {
            return Ok(());
        }
        operation.phase = TargetOperationPhase::Blocked;
        operation.error = Some(message.clone());
        operation.updated_at = self.clock.now();
        if let Some(action) = operation
            .actions
            .iter_mut()
            .find(|action| action.status == TargetOperationActionStatus::Running)
        {
            action.status = TargetOperationActionStatus::Failed;
            action.error = Some(message.clone());
        }
        self.repository
            .save_target_operation(&operation)
            .map_err(AgentSessionApplicationError::repository)?;
        if let Some(entry) = self
            .repository
            .queue_entry_for_operation(operation_id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            self.repository
                .set_queue_entry_state(
                    &entry.id,
                    SessionQueueEntryState::Blocked,
                    Some(message),
                    self.clock.now(),
                )
                .map_err(AgentSessionApplicationError::repository)?;
        }
        self.notify_queue(&operation.session_id);
        Ok(())
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

    fn notify_queue(&self, session_id: &AgentSessionId) {
        let _ = self
            .notifier
            .notify(AgentSessionNotification::QueueUpdated {
                session_id: session_id.clone(),
            });
    }
}

fn validate_migration_facts(
    source: &WorktreeInspection,
    destination: Option<&WorktreeInspection>,
) -> Result<(), AgentSessionApplicationError> {
    let blocker = if source.detached {
        Some("A detached source worktree cannot be moved".to_string())
    } else if destination.is_some_and(|target| target.detached) {
        Some("A detached destination worktree cannot be used".to_string())
    } else if destination.is_some_and(|target| {
        target.staged.files > 0 || target.unstaged.files > 0 || target.untracked.files > 0
    }) {
        Some("The destination worktree has local changes; merging is not supported".to_string())
    } else if destination.is_some()
        && !matches!(
            source
                .comparison
                .as_ref()
                .map(|comparison| comparison.relation),
            Some(WorktreeHeadRelation::Equal | WorktreeHeadRelation::Ahead)
        )
    {
        Some(
            "The destination commit diverges from the source; merging is not supported".to_string(),
        )
    } else {
        None
    };
    blocker.map_or(Ok(()), |message| {
        Err(AgentSessionApplicationError::conflict(message))
    })
}

fn same_selection_intent(
    left: &SessionExecutionSelection,
    right: &SessionExecutionSelection,
) -> bool {
    if left.capability_profile_id != right.capability_profile_id
        || left.capability_profile_revision != right.capability_profile_revision
        || left.execution != right.execution
    {
        return false;
    }
    match (&left.workspace, &right.workspace) {
        (
            SessionWorkspaceSelection::Existing { target: left },
            SessionWorkspaceSelection::Existing { target: right },
        ) => left == right,
        (
            SessionWorkspaceSelection::Create {
                repository_id: left_repository,
                branch_ref: left_branch,
                attachment: left_attachment,
                ..
            },
            SessionWorkspaceSelection::Create {
                repository_id: right_repository,
                branch_ref: right_branch,
                attachment: right_attachment,
                ..
            },
        ) => {
            left_repository == right_repository
                && left_branch == right_branch
                && left_attachment == right_attachment
        }
        (SessionWorkspaceSelection::Auxiliary, SessionWorkspaceSelection::Auxiliary) => true,
        _ => false,
    }
}

fn set_action(
    operation: &mut SessionTargetOperation,
    kind: TargetOperationActionKind,
    status: TargetOperationActionStatus,
    detail: Option<String>,
    error: Option<String>,
) {
    if let Some(action) = operation
        .actions
        .iter_mut()
        .find(|action| action.kind == kind)
    {
        action.status = status;
        action.detail = detail;
        action.error = error;
    }
    operation.updated_at = chrono::Utc::now();
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
