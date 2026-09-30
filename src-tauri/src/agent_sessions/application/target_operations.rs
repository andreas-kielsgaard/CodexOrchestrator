use super::queue::*;
use super::*;
use crate::agent_sessions::domain::AgentSessionId;
use crate::agent_sessions::{
    queue::{SessionQueueEntryItem, SessionQueueEntryState, SessionQueueSnapshot},
    target_operation::*,
};
use crate::execution_targets::domain::{SessionExecutionSelection, SessionWorkspaceSelection};

mod execution;

impl AgentSessionApplication {
    pub(crate) fn enqueue_session_target_operation(
        &self,
        session_id: &AgentSessionId,
        request: TargetSwitchRequest,
        before_entry_id: Option<&str>,
    ) -> Result<SessionQueueSnapshot, AgentSessionApplicationError> {
        request
            .validate()
            .map_err(AgentSessionApplicationError::invalid)?;
        let source = self.projected_queue_source(session_id, before_entry_id)?;
        let mut operation = SessionTargetOperation::new(
            uuid::Uuid::new_v4().to_string(),
            session_id.clone(),
            source,
            request,
            self.clock.now(),
        );
        self.calculate_projection(&mut operation);
        let snapshot = self
            .repository
            .enqueue_target_operation(operation, before_entry_id, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(session_id);
        self.wake_session_queue(session_id.clone());
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
                input.request,
                self.clock.now(),
            )
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(&input.session_id);
        self.wake_session_queue(input.session_id);
        Ok(snapshot)
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
            if matches!(
                selection.workspace,
                SessionWorkspaceSelection::Create { .. }
            ) {
                projected_matches = queue
                    .entries
                    .iter()
                    .filter(|entry| entry.state == SessionQueueEntryState::Completed)
                    .filter_map(|entry| match &entry.item {
                        SessionQueueEntryItem::TargetOperation { operation_id } => self
                            .repository
                            .target_operation(operation_id)
                            .ok()
                            .flatten(),
                        _ => None,
                    })
                    .any(|operation| {
                        operation.receipt.is_some()
                            && same_selection_intent(selection, &operation.request.destination)
                            && operation
                                .staged_target
                                .as_ref()
                                .zip(session.execution_target.as_ref())
                                .is_some_and(|(resolved, current)| {
                                    same_workspace_identity(resolved, current)
                                })
                    });
            }

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
                        same_selection_intent(selection, &operation.request.destination);
                }
            }
        }
        if projected_matches {
            return Ok(None);
        }
        let now = self.clock.now();
        let mut operation = SessionTargetOperation::new(
            uuid::Uuid::new_v4().to_string(),
            session.id.clone(),
            session.execution_target.clone(),
            TargetSwitchRequest::retarget(selection.clone()),
            now,
        );
        operation.source_target = self
            .projected_queue_source(&session.id, None)
            .unwrap_or(session.execution_target.clone());
        self.calculate_projection(&mut operation);
        Ok(Some(operation))
    }
    fn projected_queue_source(
        &self,
        session_id: &AgentSessionId,
        before_entry_id: Option<&str>,
    ) -> Result<
        Option<crate::execution_targets::domain::SessionExecutionTarget>,
        AgentSessionApplicationError,
    > {
        let mut source = self.load_session(session_id)?.session.execution_target;
        for entry in self
            .load_session_queue(session_id)?
            .entries
            .into_iter()
            .take_while(|e| Some(e.id.as_str()) != before_entry_id)
            .filter(|e| e.state.is_unfinished())
        {
            if let SessionQueueEntryItem::TargetOperation { operation_id } = entry.item {
                if let Some(operation) = self
                    .repository
                    .target_operation(&operation_id)
                    .map_err(AgentSessionApplicationError::repository)?
                {
                    source = operation.staged_target.or_else(|| {
                        match operation.request.destination.workspace {
                            SessionWorkspaceSelection::Existing { target } => Some(target),
                            _ => None,
                        }
                    });
                }
            }
        }
        Ok(source)
    }

    fn calculate_projection(&self, operation: &mut SessionTargetOperation) {
        let mut facts = self.observe_target_switch(&operation.request);
        if operation.needs_resolution {
            facts
                .blockers
                .push("Choose the intent and migration resolution for this older switch".into());
        }
        if let Err(error) = self.validate_migration_resolution(operation) {
            facts.blockers.push(error.to_string());
        }
        if operation.source_target.is_none() {
            facts
                .unknown
                .push("Source workspace will be resolved by preceding queue work".into());
        }
        operation.projected_plan = TargetSwitchProjection {
            calculated_at: self.clock.now(),
            request_revision: operation.generation,
            stale: false,
            source: operation.source_target.clone(),
            steps: plan_target_switch(operation.source_target.as_ref(), &operation.request, &facts),
            facts,
        };
    }

    pub(super) fn observe_target_switch(&self, request: &TargetSwitchRequest) -> TargetSwitchFacts {
        let mut facts = TargetSwitchFacts::default();
        let Some(targets) = self.execution_target_service.as_ref() else {
            facts.unknown.push("Execution targets unavailable".into());
            return facts;
        };
        match &request.destination.workspace {
            SessionWorkspaceSelection::Existing { target } if !target.repository_id.is_empty() => {
                let root = match targets
                    .repository_root(&target.repository_id, &request.destination.execution)
                {
                    Ok(root) => root,
                    Err(error) => {
                        facts.blockers.push(error);
                        return facts;
                    }
                };
                let observed =
                    targets
                        .endpoints
                        .worktrees(&request.destination.execution, &root, None);
                match observed {
                    Ok(instances) => {
                        facts.destination_ready = Some(true);
                        if let Some(instance) = instances.into_iter().find(|i| {
                            i.handle == target.worktree_id
                                && i.path == target.path
                                && i.branch_ref.as_deref().unwrap_or_default() == target.branch_ref
                        }) {
                            facts.destination =
                                Some(crate::execution_targets::domain::SessionExecutionTarget {
                                    head: instance.head,
                                    ..target.clone()
                                });
                        } else {
                            facts.blockers.push(
                                "Selected worktree is no longer available on this device".into(),
                            );
                        }
                    }
                    Err(error) => facts.unknown.push(error),
                }
            }
            SessionWorkspaceSelection::Existing { target } => {
                facts.destination = Some(target.clone());
                if request.destination.execution.is_remote() {
                    facts.destination_ready = None;
                    facts
                        .unknown
                        .push("Destination availability will be checked at execution".into());
                } else {
                    facts.destination_ready = Some(true);
                    if !std::path::Path::new(&target.path).is_dir() {
                        facts
                            .blockers
                            .push("Selected workspace folder is no longer available".into());
                    }
                }
            }
            _ => {
                facts.destination_ready = if request.destination.execution.is_remote() {
                    None
                } else {
                    Some(true)
                };
            }
        }
        facts
    }

    pub(crate) fn recalculate_target_operation_projection(
        &self,
        session_id: &AgentSessionId,
        entry_id: &str,
        expected_revision: u64,
    ) -> Result<SessionQueueSnapshot, AgentSessionApplicationError> {
        let queue = self.load_session_queue(session_id)?;
        if queue.revision != expected_revision {
            return Err(AgentSessionApplicationError::conflict(
                "Queue changed; refresh before recalculating",
            ));
        }
        let entry = queue
            .entries
            .iter()
            .find(|e| e.id == entry_id)
            .ok_or_else(|| AgentSessionApplicationError::not_found("Queue entry not found"))?;
        let SessionQueueEntryItem::TargetOperation { operation_id } = &entry.item else {
            return Err(AgentSessionApplicationError::invalid(
                "Select a target operation",
            ));
        };
        let mut operation = self
            .repository
            .target_operation(operation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Target operation not found"))?;
        let mut source = self.load_session(session_id)?.session.execution_target;
        for preceding in queue
            .entries
            .iter()
            .take_while(|e| e.id != entry_id)
            .filter(|e| e.state.is_unfinished())
        {
            if let SessionQueueEntryItem::TargetOperation { operation_id } = &preceding.item {
                if let Some(op) = self
                    .repository
                    .target_operation(operation_id)
                    .map_err(AgentSessionApplicationError::repository)?
                {
                    source = op
                        .staged_target
                        .or_else(|| match op.request.destination.workspace {
                            SessionWorkspaceSelection::Existing { target } => Some(target),
                            _ => None,
                        });
                }
            }
        }
        operation.source_target = source;
        self.calculate_projection(&mut operation);
        let snapshot = self
            .repository
            .save_target_projection(&operation, expected_revision)
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(session_id);
        Ok(snapshot)
    }
}

fn same_selection_intent(
    left: &SessionExecutionSelection,
    right: &SessionExecutionSelection,
) -> bool {
    if left.execution.device_id != right.execution.device_id {
        return false;
    }
    match (&left.workspace, &right.workspace) {
        (
            SessionWorkspaceSelection::Existing { target: left },
            SessionWorkspaceSelection::Existing { target: right },
        ) => same_workspace_identity(left, right),
        (
            SessionWorkspaceSelection::Create {
                repository_id: left_repository,
                branch_ref: left_branch,
                attachment: left_attachment,
                commit: left_commit,
            },
            SessionWorkspaceSelection::Create {
                repository_id: right_repository,
                branch_ref: right_branch,
                attachment: right_attachment,
                commit: right_commit,
            },
        ) => {
            left_repository == right_repository
                && left_branch == right_branch
                && left_attachment == right_attachment
                && left_commit == right_commit
        }
        (SessionWorkspaceSelection::Auxiliary, SessionWorkspaceSelection::Auxiliary) => true,
        _ => false,
    }
}
