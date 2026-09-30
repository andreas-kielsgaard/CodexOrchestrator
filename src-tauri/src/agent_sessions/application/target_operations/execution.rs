use super::*;
use crate::agent_sessions::domain::AgentInvocationStatus;
use orchid_engine::protocol::{WorktreeHeadRelation, WorktreeInspection};

impl AgentSessionApplication {
    pub(in crate::agent_sessions::application) fn start_head_target_operation(
        &self,
        operation_id: String,
    ) -> Result<(), AgentSessionApplicationError> {
        if !self.target_operation_workers.reserve(&operation_id) {
            return Ok(());
        }
        let application = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let result = application.execute_head_target_operation(&operation_id);
            if let Err(error) = result {
                let _ = application.block_target_operation(&operation_id, error.to_string());
            }
            application.target_operation_workers.release(&operation_id);
            application.wake_all_session_queues();
        });
        Ok(())
    }

    fn execute_head_target_operation(
        &self,
        operation_id: &str,
    ) -> Result<(), AgentSessionApplicationError> {
        let mut operation = self
            .repository
            .target_operation(operation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Target operation not found"))?;
        let history = self.load_session(&operation.session_id)?;
        if history
            .invocations
            .iter()
            .any(|i| i.invocation.status == AgentInvocationStatus::Running)
        {
            return Ok(());
        }
        let queue = self.load_session_queue(&operation.session_id)?;
        if !queue.entries.iter().find(|entry| entry.state.is_unfinished()).is_some_and(|entry| matches!(&entry.item, SessionQueueEntryItem::TargetOperation { operation_id: id } if id == operation_id)) { return Ok(()) }
        operation.source_target = history.session.execution_target;
        if operation.needs_resolution {
            return Err(AgentSessionApplicationError::conflict(
                "Choose the intent and migration resolution for this older switch",
            ));
        }
        operation
            .request
            .validate()
            .map_err(AgentSessionApplicationError::invalid)?;
        self.validate_migration_resolution(&operation)?;
        let mut observation_request = operation.request.clone();
        if let Some(resolved) = operation.staged_target.as_ref() {
            observation_request.destination.workspace = SessionWorkspaceSelection::Existing {
                target: resolved.clone(),
            };
        }
        let mut facts = self.observe_target_switch(&observation_request);
        if !facts.blockers.is_empty() {
            return Err(AgentSessionApplicationError::conflict(
                facts.blockers.join("; "),
            ));
        }
        operation.actions = plan_target_switch(
            operation.source_target.as_ref(),
            &observation_request,
            &facts,
        );
        operation.phase = TargetOperationPhase::Staging;
        operation.error = None;
        operation.updated_at = self.clock.now();
        self.repository
            .save_target_operation(&operation)
            .map_err(AgentSessionApplicationError::repository)?;
        if operation.actions.is_empty() {
            self.repository
                .discard_target_operation(operation_id, self.clock.now())
                .map_err(AgentSessionApplicationError::repository)?;
            self.notify_queue(&operation.session_id);
            return Ok(());
        }
        if operation
            .actions
            .iter()
            .any(|a| a.kind == TargetOperationActionKind::PrepareDevice)
        {
            set_action(
                &mut operation,
                TargetOperationActionKind::PrepareDevice,
                TargetOperationActionStatus::Running,
                None,
                None,
            );
            self.save_staging_operation(&operation, operation.generation)?;
            let targets = self.execution_target_service.as_ref().ok_or_else(|| {
                AgentSessionApplicationError::invalid("Execution targets unavailable")
            })?;
            let started = targets
                .ensure_ready_with_effect(&observation_request.destination.execution)
                .map_err(AgentSessionApplicationError::invalid)?;
            facts = self.observe_target_switch(&observation_request);
            if !facts.blockers.is_empty() {
                return Err(AgentSessionApplicationError::conflict(
                    facts.blockers.join("; "),
                ));
            }
            facts.destination_ready = Some(true);
            operation.actions = plan_target_switch(
                operation.source_target.as_ref(),
                &observation_request,
                &facts,
            );
            if started {
                operation.actions.insert(
                    0,
                    TargetOperationAction {
                        kind: TargetOperationActionKind::PrepareDevice,
                        status: TargetOperationActionStatus::Completed,
                        detail: Some("Destination device started".into()),
                        error: None,
                    },
                );
            }
            self.save_staging_operation(&operation, operation.generation)?;
            if operation.actions.is_empty() {
                self.repository
                    .discard_target_operation(operation_id, self.clock.now())
                    .map_err(AgentSessionApplicationError::repository)?;
                self.notify_queue(&operation.session_id);
                return Ok(());
            }
        }
        self.run_target_operation_stage(operation_id)?;
        self.activate_staged_target_operation(operation_id)
    }

    pub(super) fn validate_migration_resolution(
        &self,
        operation: &SessionTargetOperation,
    ) -> Result<(), AgentSessionApplicationError> {
        if operation.request.intent != TargetSwitchIntent::ContinueWork {
            return Ok(());
        }
        let resolution = operation
            .request
            .migration_resolution
            .as_ref()
            .expect("validated resolution");
        let source = operation.source_target.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::conflict("There is no source workspace to continue")
        })?;
        let lineage = match &operation.request.destination.workspace {
            SessionWorkspaceSelection::Existing { target } => {
                (&target.repository_id, &target.branch_ref)
            }
            SessionWorkspaceSelection::Create {
                repository_id,
                branch_ref,
                ..
            } => (repository_id, branch_ref),
            SessionWorkspaceSelection::Auxiliary => {
                return Err(AgentSessionApplicationError::conflict(
                    "An auxiliary workspace cannot receive a migration",
                ))
            }
        };
        if source.repository_id != resolution.repository_id
            || source.branch_ref != resolution.branch_ref
            || lineage != (&resolution.repository_id, &resolution.branch_ref)
        {
            return Err(AgentSessionApplicationError::conflict(
                "The source or destination lineage changed. Edit the migration resolution.",
            ));
        }
        let targets = self.execution_target_service.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid("Execution targets unavailable")
        })?;
        if let Some(lock) = targets
            .sisters
            .lock_for(&source.repository_id, &source.branch_ref)
            .map_err(AgentSessionApplicationError::conflict)?
        {
            if lock.owner_session_id != operation.session_id.as_str() {
                return Err(AgentSessionApplicationError::conflict(format!(
                    "This sister worktree group is locked by Session {}",
                    lock.owner_session_id
                )));
            }
        }
        match (
            &operation.request.destination.workspace,
            resolution.strategy,
        ) {
            (SessionWorkspaceSelection::Create { .. }, MigrationStrategy::CreateLinked) => (),
            (SessionWorkspaceSelection::Existing { target }, MigrationStrategy::ExistingSister) => {
                let lock = targets
                    .sisters
                    .lock_for(&source.repository_id, &source.branch_ref)
                    .map_err(AgentSessionApplicationError::conflict)?
                    .ok_or_else(|| {
                        AgentSessionApplicationError::conflict(
                            "The selected sister relationship no longer exists",
                        )
                    })?;
                if !targets
                    .sisters
                    .contains_instance(
                        &lock.sister_group_id,
                        &target.execution.device_id,
                        &target.worktree_id,
                    )
                    .map_err(AgentSessionApplicationError::conflict)?
                    || !targets
                        .sisters
                        .contains_instance(
                            &lock.sister_group_id,
                            &source.execution.device_id,
                            &source.worktree_id,
                        )
                        .map_err(AgentSessionApplicationError::conflict)?
                {
                    return Err(AgentSessionApplicationError::conflict(
                        "Select an explicit link resolution for these worktrees",
                    ));
                }
            }
            (SessionWorkspaceSelection::Existing { .. }, MigrationStrategy::LinkExisting) => (),
            _ => {
                return Err(AgentSessionApplicationError::conflict(
                    "The selected destination does not support this migration resolution",
                ))
            }
        }
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
        set_action(
            &mut operation,
            TargetOperationActionKind::ResolveRevision,
            TargetOperationActionStatus::Running,
            None,
            None,
        );
        self.save_staging_operation(&operation, generation)?;

        let mut selection = operation.request.destination.clone();
        if let Some(resolved) = operation.staged_target.as_ref() {
            selection.workspace = SessionWorkspaceSelection::Existing {
                target: resolved.clone(),
            };
        }
        if let SessionWorkspaceSelection::Create {
            repository_id,
            branch_ref,
            commit,
            ..
        } = &mut selection.workspace
        {
            if commit.len() != 40 || !commit.bytes().all(|byte| byte.is_ascii_hexdigit()) {
                let root = targets
                    .repository_root(repository_id, &operation.request.destination.execution)
                    .map_err(AgentSessionApplicationError::invalid)?;
                *commit = targets
                    .endpoints
                    .branch_commit(&operation.request.destination.execution, &root, branch_ref)
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
            .materialize_ready_selection(&selection, operation_id)
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
            let head = target.head.as_deref().unwrap_or("unknown");
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

    pub(in crate::agent_sessions::application) fn activate_staged_target_operation(
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
        let mut destination = operation
            .staged_target
            .clone()
            .ok_or_else(|| AgentSessionApplicationError::not_found("Staged target unavailable"))?;
        let source = self
            .load_session(&operation.session_id)?
            .session
            .execution_target;
        operation.source_target = source.clone();
        operation.phase = TargetOperationPhase::Activating;
        operation.updated_at = self.clock.now();
        self.repository
            .save_target_operation(&operation)
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(&operation.session_id);

        let migrate = operation.request.intent == TargetSwitchIntent::ContinueWork
            && source
                .as_ref()
                .is_some_and(|s| !same_workspace_identity(s, &destination));
        if let Some(source) = source.filter(|_| migrate) {
            let targets = self.execution_target_service.as_ref().ok_or_else(|| {
                AgentSessionApplicationError::invalid("Execution targets unavailable")
            })?;
            let endpoints = self.endpoints.as_ref().ok_or_else(|| {
                AgentSessionApplicationError::invalid("Execution endpoints unavailable")
            })?;
            targets
                .sisters
                .link(
                    &source,
                    &destination,
                    operation.session_id.as_str(),
                    self.clock.now(),
                )
                .map_err(AgentSessionApplicationError::conflict)?;
            let fresh_destination = endpoints
                .inspect_worktree(&destination.execution, &destination.path, None)
                .map_err(AgentSessionApplicationError::invalid)?;
            destination.head = Some(fresh_destination.head.clone());
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
            if source_inspection.branch_ref.as_deref() != Some(source.branch_ref.as_str())
                || destination_inspection.branch_ref.as_deref()
                    != Some(destination.branch_ref.as_str())
            {
                return Err(AgentSessionApplicationError::conflict(
                    "Worktree branch changed; edit the migration resolution",
                ));
            }
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

            destination.head = Some(verified.head.clone());
            operation.staged_target = Some(destination.clone());
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
            TargetOperationActionKind::ActivateTarget,
            TargetOperationActionStatus::Running,
            None,
            None,
        );
        self.save_target_activation_progress(&operation)?;
        self.repository
            .activate_target_operation(operation_id, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_queue(&operation.session_id);
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

    pub(in crate::agent_sessions::application) fn block_target_operation(
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
