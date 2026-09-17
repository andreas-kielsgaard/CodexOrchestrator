//! Session-owned worktree/device moves. Prompt preparation joins a completed move rather than
//! pretending that a file migration is itself an invocation.

use super::*;
use crate::{
    agent_sessions::{
        domain::AgentSessionId,
        ports::ListAgentSessionsQuery,
        target_transition::{
            QueuedTransitionPrompt, SessionTargetTransition, SnapshotArtifact,
            SnapshotTransferEstimate, TargetTransitionPhase, TargetTransitionTask,
            TargetTransitionTaskKind, TargetTransitionTaskStatus, WorktreeSnapshotDescriptor,
        },
    },
    execution_targets::domain::{
        SessionExecutionSelection, SessionExecutionTarget, SessionWorkspaceSelection,
    },
};
use orchid_engine::protocol::{WorktreeHeadRelation, WorktreeInspection, WorktreeSnapshot};
use chrono::Utc;
use std::{
    thread,
    time::{Duration, Instant},
};

#[derive(Clone, Debug)]
pub(crate) struct RequestTargetTransitionInput {
    pub(crate) session_id: AgentSessionId,
    /// The UI echoes the visible source. The application confirms it against the Session so an
    /// old modal cannot move a different worktree after a navigation change.
    pub(crate) source_target: SessionExecutionTarget,
    pub(crate) destination: SessionExecutionSelection,
}

impl AgentSessionApplication {
    pub(crate) fn load_target_transition(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<SessionTargetTransition>, AgentSessionApplicationError> {
        self.repository
            .target_transition(session_id)
            .map_err(AgentSessionApplicationError::repository)
    }

    pub(crate) fn request_target_transition(
        &self,
        input: RequestTargetTransitionInput,
    ) -> Result<SessionTargetTransition, AgentSessionApplicationError> {
        let session = self.load_session(&input.session_id)?.session;
        let source = session.execution_target.clone().ok_or_else(|| {
            AgentSessionApplicationError::invalid(
                "Choose a concrete worktree before moving this conversation to another device",
            )
        })?;
        if source != input.source_target {
            return Err(AgentSessionApplicationError::conflict(
                "The conversation worktree changed. Reopen Destination device.",
            ));
        }
        if input.destination.execution.device_id == source.execution.device_id {
            return Err(AgentSessionApplicationError::invalid(
                "Choose a device other than the current conversation device",
            ));
        }
        let endpoints = self.endpoints.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid("Execution endpoints unavailable")
        })?;
        let targets = self.execution_target_service.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid("Execution targets unavailable")
        })?;
        let destination_head =
            existing_target(&input.destination).and_then(|target| target.head.as_deref());
        let source_inspection = endpoints
            .inspect_worktree(&source.execution, &source.path, destination_head)
            .map_err(AgentSessionApplicationError::invalid)?;
        let destination_inspection = existing_target(&input.destination)
            // The source can be local-only. Its host has not received the source commit objects
            // until after the snapshot arrives, so establish ahead/behind on the source instead.
            .map(|target| endpoints.inspect_worktree(&target.execution, &target.path, None))
            .transpose()
            .map_err(AgentSessionApplicationError::invalid)?;
        let source_turns = self.active_turn_count(&source)?;
        let destination_turns = existing_target(&input.destination)
            .map(|target| self.active_turn_count(target))
            .transpose()?
            .unwrap_or(0);
        let migration_error = migration_blocker(&source_inspection, destination_inspection.as_ref());
        let current_lock = targets
            .sisters
            .lock_for(&source.repository_id, &source.branch_ref)
            .map_err(AgentSessionApplicationError::invalid)?;
        let now = self.clock.now();
        let sister_group_id = match current_lock.as_ref() {
            Some(lock) if lock.owner_session_id != input.session_id.as_str() => {
                Some(lock.sister_group_id.clone())
            }
            Some(lock) => Some(lock.sister_group_id.clone()),
            None if migration_error.is_none() => Some(
                targets
                    .sisters
                    .claim(&source, input.session_id.as_str(), now)
                    .map_err(AgentSessionApplicationError::conflict)?,
            ),
            None => None,
        };
        let snapshot = planned_snapshot(&source_inspection, destination_inspection.as_ref());
        let transfer_estimate = initial_transfer_estimate(&snapshot, now);
        let mut transition = SessionTargetTransition {
            session_id: input.session_id,
            source_target: source.clone(),
            destination_selection: input.destination,
            sister_group_id,
            phase: TargetTransitionPhase::Pending,
            tasks: planned_tasks(
                &source_inspection,
                destination_inspection.as_ref(),
                source_turns,
                destination_turns,
            ),
            snapshot: Some(snapshot),
            transfer_estimate: Some(transfer_estimate),
            queued_prompt: None,
            resolved_target: None,
            error: None,
            created_at: now,
            updated_at: now,
        };
        // A current group owned by another Session is rendered as locked. Requesting the plan is
        // useful for visibility, but executing it must remain unavailable.
        if let Some(lock) = current_lock {
            if lock.owner_session_id != transition.session_id.as_str() {
                transition.error = Some(format!(
                    "This sister worktree is locked to Session {} on device {}",
                    lock.owner_session_id, lock.active_device_id
                ));
            }
        }
        if let Some(blocker) = migration_error {
            transition.error = Some(blocker);
        }
        self.save_target_transition(&transition)
    }

    pub(crate) fn start_target_transition(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<SessionTargetTransition, AgentSessionApplicationError> {
        let mut transition = self.load_target_transition(session_id)?.ok_or_else(|| {
            AgentSessionApplicationError::not_found("Target transition not found")
        })?;
        if transition.phase == TargetTransitionPhase::Ready
            || transition.phase == TargetTransitionPhase::Running
        {
            return Ok(transition);
        }
        if transition.phase != TargetTransitionPhase::Pending {
            return Err(AgentSessionApplicationError::conflict(
                "Create a new device switch plan before starting it",
            ));
        }
        if transition.error.is_some() {
            return Err(AgentSessionApplicationError::conflict(
                transition.error.clone().expect("checked"),
            ));
        }
        let source_turns = self.active_turn_count(&transition.source_target)?;
        let destination_turns = existing_target(&transition.destination_selection)
            .map(|target| self.active_turn_count(target))
            .transpose()?
            .unwrap_or(0);
        if source_turns + destination_turns > 0 {
            return Err(AgentSessionApplicationError::conflict(
                "Wait for active Orchid turns on these worktrees before switching devices",
            ));
        }
        transition.phase = TargetTransitionPhase::Running;
        transition.updated_at = self.clock.now();
        transition = self.save_target_transition(&transition)?;
        let application = self.clone();
        let id = session_id.clone();
        thread::spawn(move || {
            let _ = application.run_target_transition(&id);
        });
        Ok(transition)
    }

    pub(crate) fn queue_target_transition_prompt(
        &self,
        session_id: &AgentSessionId,
        text: String,
        client_message_id: String,
    ) -> Result<Option<SessionTargetTransition>, AgentSessionApplicationError> {
        let Some(mut transition) = self.load_target_transition(session_id)? else {
            return Ok(None);
        };
        if self.load_session(session_id)?.session.execution_target.as_ref()
            != Some(&transition.source_target)
        {
            // The first prepared prompt already committed the resolved destination. The retained
            // transition remains useful history, but must not move later prompts back to its
            // original source.
            return Ok(None);
        }
        if transition.phase.is_unfinished() && transition.error.is_some() {
            return Err(AgentSessionApplicationError::conflict(
                transition.error.clone().expect("checked"),
            ));
        }
        if transition.phase.is_unfinished() {
            if let Some(existing) = &transition.queued_prompt {
                if existing.client_message_id != client_message_id {
                    return Err(AgentSessionApplicationError::conflict(
                        "This device switch already has a queued prompt",
                    ));
                }
            } else {
                transition.queued_prompt = Some(QueuedTransitionPrompt {
                    text,
                    client_message_id,
                });
                transition.updated_at = self.clock.now();
                transition = self.save_target_transition(&transition)?;
            }
        }
        Ok(Some(transition))
    }

    /// Called by prompt preparation. It returns the move's resolved worktree only after a pending
    /// move has started and reached the same durable ready boundary used by an explicit click.
    pub(crate) fn await_target_transition(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<Option<SessionTargetTransition>, AgentSessionApplicationError> {
        let Some(mut transition) = self.load_target_transition(session_id)? else {
            return Ok(None);
        };
        if self.load_session(session_id)?.session.execution_target.as_ref()
            != Some(&transition.source_target)
        {
            return Ok(None);
        }
        if transition.phase == TargetTransitionPhase::Pending {
            transition = self.start_target_transition(session_id)?;
        }
        if transition.phase == TargetTransitionPhase::Running {
            let deadline = Instant::now() + Duration::from_secs(120);
            loop {
                thread::sleep(Duration::from_millis(100));
                transition = self.load_target_transition(session_id)?.ok_or_else(|| {
                    AgentSessionApplicationError::not_found("Target transition disappeared")
                })?;
                if transition.phase != TargetTransitionPhase::Running || Instant::now() >= deadline
                {
                    break;
                }
            }
        }
        match transition.phase {
            TargetTransitionPhase::Ready => Ok(Some(transition)),
            TargetTransitionPhase::Failed => Err(AgentSessionApplicationError::invalid(
                transition
                    .error
                    .unwrap_or_else(|| "The device switch failed".into()),
            )),
            TargetTransitionPhase::Canceled => Err(AgentSessionApplicationError::conflict(
                "The device switch was canceled",
            )),
            TargetTransitionPhase::Pending | TargetTransitionPhase::Running => {
                Err(AgentSessionApplicationError::runtime(
                    crate::agent_sessions::ports::RuntimePortError::new(
                        crate::agent_sessions::ports::RuntimePortErrorKind::Unavailable,
                        "The device switch did not finish before prompt preparation timed out",
                    ),
                ))
            }
        }
    }

    fn run_target_transition(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<(), AgentSessionApplicationError> {
        let result = self.run_target_transition_inner(session_id);
        if let Err(error) = result {
            if let Ok(Some(mut transition)) = self.load_target_transition(session_id) {
                transition.phase = TargetTransitionPhase::Failed;
                transition.error = Some(error.message.clone());
                transition.updated_at = self.clock.now();
                let _ = self.save_target_transition(&transition);
                let _ = self.notify_target_transition(&transition);
            }
            return Err(error);
        }
        Ok(())
    }

    fn run_target_transition_inner(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<(), AgentSessionApplicationError> {
        let mut transition = self.load_target_transition(session_id)?.ok_or_else(|| {
            AgentSessionApplicationError::not_found("Target transition not found")
        })?;
        let endpoints = self.endpoints.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid("Execution endpoints unavailable")
        })?;
        let targets = self.execution_target_service.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid("Execution targets unavailable")
        })?;

        let source = transition.source_target.clone();
        let initial_destination = existing_target(&transition.destination_selection).cloned();
        transition_task(
            &mut transition,
            TargetTransitionTaskKind::InspectSource,
            TargetTransitionTaskStatus::Running,
            None,
            None,
        );
        self.save_and_notify_target_transition(&transition)?;
        let source_inspection = endpoints
            .inspect_worktree(
                &source.execution,
                &source.path,
                initial_destination
                    .as_ref()
                    .and_then(|target| target.head.as_deref()),
            )
            .map_err(AgentSessionApplicationError::invalid)?;
        transition_task(
            &mut transition,
            TargetTransitionTaskKind::InspectSource,
            TargetTransitionTaskStatus::Completed,
            Some(summary(&source_inspection)),
            None,
        );

        let destination_inspection = if let Some(destination) = initial_destination.as_ref() {
            transition_task(
                &mut transition,
                TargetTransitionTaskKind::InspectDestination,
                TargetTransitionTaskStatus::Running,
                None,
                None,
            );
            self.save_and_notify_target_transition(&transition)?;
            let inspection = endpoints
                .inspect_worktree(&destination.execution, &destination.path, None)
                .map_err(AgentSessionApplicationError::invalid)?;
            transition_task(
                &mut transition,
                TargetTransitionTaskKind::InspectDestination,
                TargetTransitionTaskStatus::Completed,
                Some(summary(&inspection)),
                None,
            );
            Some(inspection)
        } else {
            transition_task(&mut transition, TargetTransitionTaskKind::InspectDestination, TargetTransitionTaskStatus::Completed, Some("A new destination worktree will be materialized from the latest published branch commit".into()), None);
            None
        };
        validate_migration_facts(&source_inspection, destination_inspection.as_ref())?;
        self.save_and_notify_target_transition(&transition)?;

        let destination = match initial_destination {
            Some(destination) => destination,
            None => {
                transition_task(
                    &mut transition,
                    TargetTransitionTaskKind::MaterializeDestination,
                    TargetTransitionTaskStatus::Running,
                    None,
                    None,
                );
                self.save_and_notify_target_transition(&transition)?;
                let (repository_id, branch_ref) = match &transition.destination_selection.workspace
                {
                    SessionWorkspaceSelection::Create {
                        repository_id,
                        branch_ref,
                        ..
                    } => (repository_id, branch_ref),
                    _ => unreachable!("existing destination handled above"),
                };
                let root = targets
                    .repository_root(repository_id, &transition.destination_selection.execution)
                    .map_err(AgentSessionApplicationError::invalid)?;
                let published = endpoints
                    .published_commit(
                        &transition.destination_selection.execution,
                        &root,
                        branch_ref,
                    )
                    .map_err(AgentSessionApplicationError::invalid)?;
                let instance = endpoints
                    .materialize_worktree(
                        &transition.destination_selection.execution,
                        &root,
                        branch_ref,
                        &published,
                        &format!("sister-{}", session_id.as_str()),
                    )
                    .map_err(AgentSessionApplicationError::invalid)?;
                let result = SessionExecutionTarget {
                    capability_profile_id: transition
                        .destination_selection
                        .capability_profile_id
                        .clone(),
                    capability_profile_revision: transition
                        .destination_selection
                        .capability_profile_revision,
                    execution: transition.destination_selection.execution.clone(),
                    repository_id: repository_id.clone(),
                    branch_ref: instance.branch_ref.unwrap_or_else(|| branch_ref.clone()),
                    worktree_id: instance.handle,
                    path: instance.path,
                    head: instance.head,
                };
                transition_task(
                    &mut transition,
                    TargetTransitionTaskKind::MaterializeDestination,
                    TargetTransitionTaskStatus::Completed,
                    Some(result.path.clone()),
                    None,
                );
                result
            }
        };

        transition_task(
            &mut transition,
            TargetTransitionTaskKind::CaptureSnapshot,
            TargetTransitionTaskStatus::Running,
            None,
            None,
        );
        self.save_and_notify_target_transition(&transition)?;
        let started = Instant::now();
        let snapshot = endpoints
            .capture_worktree_snapshot(
                &source.execution,
                &source.path,
                destination.head.as_deref(),
                &format!("{}-{}", session_id.as_str(), uuid::Uuid::new_v4().simple()),
            )
            .map_err(AgentSessionApplicationError::invalid)?;
        transition.snapshot = Some(captured_snapshot(&snapshot));
        transition_task(
            &mut transition,
            TargetTransitionTaskKind::CaptureSnapshot,
            TargetTransitionTaskStatus::Completed,
            Some(format_bytes(snapshot.descriptor.total_bytes)),
            None,
        );
        transition_task(
            &mut transition,
            TargetTransitionTaskKind::TransferSnapshot,
            TargetTransitionTaskStatus::Running,
            Some("Transferring Orchid snapshot to the destination".into()),
            None,
        );
        self.save_and_notify_target_transition(&transition)?;
        let applied = endpoints
            .apply_worktree_snapshot(&destination.execution, &destination.path, &snapshot)
            .map_err(AgentSessionApplicationError::invalid)?;
        let elapsed = started.elapsed().as_secs_f64().max(0.001);
        let bytes_per_second = (snapshot.descriptor.total_bytes as f64 / elapsed).ceil() as u64;
        transition.transfer_estimate = Some(SnapshotTransferEstimate {
            snapshot_bytes: snapshot.descriptor.total_bytes,
            bytes_per_second: bytes_per_second.max(1),
            estimated_seconds: elapsed.ceil() as u64,
            measured_at: self.clock.now(),
        });
        transition_task(
            &mut transition,
            TargetTransitionTaskKind::TransferSnapshot,
            TargetTransitionTaskStatus::Completed,
            Some(format!(
                "{} transferred",
                format_bytes(snapshot.descriptor.total_bytes)
            )),
            None,
        );
        transition_task(
            &mut transition,
            TargetTransitionTaskKind::ApplySnapshot,
            TargetTransitionTaskStatus::Completed,
            Some(summary(&applied)),
            None,
        );
        self.save_and_notify_target_transition(&transition)?;

        transition_task(
            &mut transition,
            TargetTransitionTaskKind::VerifyDestination,
            TargetTransitionTaskStatus::Running,
            None,
            None,
        );
        self.save_and_notify_target_transition(&transition)?;
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
            return Err(AgentSessionApplicationError::invalid(
                "The destination HEAD did not reach the source HEAD",
            ));
        }
        transition_task(
            &mut transition,
            TargetTransitionTaskKind::VerifyDestination,
            TargetTransitionTaskStatus::Completed,
            Some(summary(&verified)),
            None,
        );

        transition_task(
            &mut transition,
            TargetTransitionTaskKind::ActivateSister,
            TargetTransitionTaskStatus::Running,
            None,
            None,
        );
        self.save_and_notify_target_transition(&transition)?;
        let group = targets
            .sisters
            .activate(&source, &destination, session_id.as_str(), self.clock.now())
            .map_err(AgentSessionApplicationError::conflict)?;
        transition.sister_group_id = Some(group);
        transition_task(
            &mut transition,
            TargetTransitionTaskKind::ActivateSister,
            TargetTransitionTaskStatus::Completed,
            Some(format!(
                "{} is now the active sister device",
                destination.execution.device_name
            )),
            None,
        );
        transition.resolved_target = Some(SessionExecutionTarget {
            head: Some(verified.head),
            ..destination
        });
        transition.phase = TargetTransitionPhase::Ready;
        transition.error = None;
        transition.updated_at = self.clock.now();
        self.save_and_notify_target_transition(&transition)?;
        Ok(())
    }

    fn active_turn_count(
        &self,
        target: &SessionExecutionTarget,
    ) -> Result<usize, AgentSessionApplicationError> {
        let sessions = self
            .repository
            .list_sessions(ListAgentSessionsQuery::default())
            .map_err(AgentSessionApplicationError::repository)?;
        let mut count = 0;
        for session in sessions {
            if session.execution_target.as_ref() != Some(target) {
                continue;
            }
            let history = self
                .repository
                .load_session_history(&session.id)
                .map_err(AgentSessionApplicationError::repository)?;
            count += history
                .into_iter()
                .flat_map(|history| history.invocations)
                .filter(|invocation| {
                    matches!(
                        invocation.invocation.status,
                        crate::agent_sessions::domain::AgentInvocationStatus::Running
                    )
                })
                .count();
        }
        Ok(count)
    }

    fn save_target_transition(
        &self,
        transition: &SessionTargetTransition,
    ) -> Result<SessionTargetTransition, AgentSessionApplicationError> {
        self.repository
            .save_target_transition(transition)
            .map_err(AgentSessionApplicationError::repository)
    }

    fn save_and_notify_target_transition(
        &self,
        transition: &SessionTargetTransition,
    ) -> Result<(), AgentSessionApplicationError> {
        let saved = self.save_target_transition(transition)?;
        self.notify_target_transition(&saved)?;
        Ok(())
    }

    fn notify_target_transition(
        &self,
        transition: &SessionTargetTransition,
    ) -> Result<(), AgentSessionApplicationError> {
        self.notifier
            .notify(AgentSessionNotification::TargetTransitionUpdated {
                session_id: transition.session_id.clone(),
            })
            .map_err(AgentSessionApplicationError::invalid)
    }
}

fn existing_target(selection: &SessionExecutionSelection) -> Option<&SessionExecutionTarget> {
    match &selection.workspace {
        SessionWorkspaceSelection::Existing { target } => Some(target),
        SessionWorkspaceSelection::Create { .. } | SessionWorkspaceSelection::Auxiliary => None,
    }
}

fn transition_task(
    transition: &mut SessionTargetTransition,
    kind: TargetTransitionTaskKind,
    status: TargetTransitionTaskStatus,
    detail: Option<String>,
    error: Option<String>,
) {
    if let Some(task) = transition.tasks.iter_mut().find(|task| task.kind == kind) {
        task.status = status;
        task.detail = detail;
        task.error = error;
    }
    transition.updated_at = Utc::now();
}

fn planned_tasks(
    source: &WorktreeInspection,
    destination: Option<&WorktreeInspection>,
    source_turns: usize,
    destination_turns: usize,
) -> Vec<TargetTransitionTask> {
    let destination_detail = destination.map(summary).unwrap_or_else(|| {
        "No sister exists on this device; Orchid will materialize one from the latest published commit".into()
    });
    [
        (
            TargetTransitionTaskKind::InspectSource,
            "Source inspected",
            Some(summary_with_turns(source, source_turns)),
        ),
        (
            TargetTransitionTaskKind::InspectDestination,
            "Destination inspected",
            Some(match destination {
                Some(target) => summary_with_turns(target, destination_turns),
                None => destination_detail,
            }),
        ),
        (
            TargetTransitionTaskKind::CaptureSnapshot,
            "Capture source snapshot",
            None,
        ),
        (
            TargetTransitionTaskKind::MaterializeDestination,
            "Materialize destination worktree",
            None,
        ),
        (
            TargetTransitionTaskKind::TransferSnapshot,
            "Transfer source snapshot",
            None,
        ),
        (
            TargetTransitionTaskKind::ApplySnapshot,
            "Apply source state",
            None,
        ),
        (
            TargetTransitionTaskKind::VerifyDestination,
            "Verify destination",
            None,
        ),
        (
            TargetTransitionTaskKind::ActivateSister,
            "Lock active sister worktree",
            None,
        ),
    ]
    .into_iter()
    .map(|(kind, _label, detail)| TargetTransitionTask {
        kind,
        status: if matches!(
            kind,
            TargetTransitionTaskKind::InspectSource | TargetTransitionTaskKind::InspectDestination
        ) {
            TargetTransitionTaskStatus::Completed
        } else {
            TargetTransitionTaskStatus::Pending
        },
        detail,
        error: None,
    })
    .collect()
}

fn planned_snapshot(
    source: &WorktreeInspection,
    destination: Option<&WorktreeInspection>,
) -> WorktreeSnapshotDescriptor {
    WorktreeSnapshotDescriptor {
        source_head: Some(source.head.clone()),
        destination_head: destination.map(|target| target.head.clone()),
        commit_bundle: SnapshotArtifact {
            entry_count: 0,
            bytes: 0,
            digest: None,
        },
        staged_patch: SnapshotArtifact {
            entry_count: u64::from(source.staged.files),
            bytes: source.staged.bytes,
            digest: None,
        },
        unstaged_patch: SnapshotArtifact {
            entry_count: u64::from(source.unstaged.files),
            bytes: source.unstaged.bytes,
            digest: None,
        },
        untracked_files: SnapshotArtifact {
            entry_count: u64::from(source.untracked.files),
            bytes: source.untracked.bytes,
            digest: None,
        },
        total_bytes: source
            .staged
            .bytes
            .saturating_add(source.unstaged.bytes)
            .saturating_add(source.untracked.bytes),
    }
}

fn captured_snapshot(snapshot: &WorktreeSnapshot) -> WorktreeSnapshotDescriptor {
    WorktreeSnapshotDescriptor {
        source_head: Some(snapshot.descriptor.source_head.clone()),
        destination_head: snapshot.descriptor.destination_head.clone(),
        commit_bundle: SnapshotArtifact {
            entry_count: u64::from(!snapshot.commit_bundle.is_empty()),
            bytes: snapshot.descriptor.commit_bundle_bytes,
            digest: None,
        },
        staged_patch: SnapshotArtifact {
            entry_count: u64::from(snapshot.descriptor.staged_files),
            bytes: snapshot.descriptor.staged_patch_bytes,
            digest: None,
        },
        unstaged_patch: SnapshotArtifact {
            entry_count: u64::from(snapshot.descriptor.unstaged_files),
            bytes: snapshot.descriptor.unstaged_patch_bytes,
            digest: None,
        },
        untracked_files: SnapshotArtifact {
            entry_count: u64::from(snapshot.descriptor.untracked_files),
            bytes: snapshot.descriptor.untracked_bytes,
            digest: None,
        },
        total_bytes: snapshot.descriptor.total_bytes,
    }
}

fn initial_transfer_estimate(
    snapshot: &WorktreeSnapshotDescriptor,
    measured_at: chrono::DateTime<Utc>,
) -> SnapshotTransferEstimate {
    // The first plan has no prior device-pair sample. Use an explicit conservative baseline,
    // then replace it with the actual effective rate once this transition completes.
    const INITIAL_BYTES_PER_SECOND: u64 = 10 * 1024 * 1024;
    SnapshotTransferEstimate {
        snapshot_bytes: snapshot.total_bytes,
        bytes_per_second: INITIAL_BYTES_PER_SECOND,
        estimated_seconds: snapshot
            .total_bytes
            .div_ceil(INITIAL_BYTES_PER_SECOND)
            .max(1),
        measured_at,
    }
}

fn validate_migration_facts(
    source: &WorktreeInspection,
    destination: Option<&WorktreeInspection>,
) -> Result<(), AgentSessionApplicationError> {
    migration_blocker(source, destination).map_or(Ok(()), |message| {
        Err(AgentSessionApplicationError::conflict(message))
    })
}

fn migration_blocker(
    source: &WorktreeInspection,
    destination: Option<&WorktreeInspection>,
) -> Option<String> {
    if source.detached {
        return Some(
            "A detached source worktree cannot be moved in this first device-switch slice".into(),
        );
    }
    if let Some(destination) = destination {
        if destination.detached {
            return Some(
                "A detached destination worktree cannot be used for this device switch".into(),
            );
        }
        if destination.staged.files > 0
            || destination.unstaged.files > 0
            || destination.untracked.files > 0
        {
            return Some(
                "The destination worktree has local changes; merging is not part of this device switch".into(),
            );
        }
        if !matches!(
            source
                .comparison
                .as_ref()
                .map(|comparison| comparison.relation),
            Some(WorktreeHeadRelation::Equal | WorktreeHeadRelation::Ahead)
        ) {
            return Some(
                "The destination commit diverges from the source; merging is not part of this device switch".into(),
            );
        }
    }
    None
}

fn summary(inspection: &WorktreeInspection) -> String {
    format!(
        "HEAD {} · {} staged, {} unstaged, {} untracked",
        inspection.head.chars().take(10).collect::<String>(),
        inspection.staged.files,
        inspection.unstaged.files,
        inspection.untracked.files,
    )
}

fn summary_with_turns(inspection: &WorktreeInspection, active_turns: usize) -> String {
    let mut value = summary(inspection);
    if active_turns > 0 {
        value.push_str(&format!(
            " · {active_turns} active Orchid turn{}",
            if active_turns == 1 { "" } else { "s" }
        ));
    }
    value
}

fn format_bytes(bytes: u64) -> String {
    if bytes < 1024 {
        format!("{bytes} bytes")
    } else if bytes < 1024 * 1024 {
        format!("{:.1} KB", bytes as f64 / 1024.0)
    } else {
        format!("{:.1} MB", bytes as f64 / (1024.0 * 1024.0))
    }
}
