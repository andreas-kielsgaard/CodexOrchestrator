//! Ordinary-session acceptance, destination setup, and explicit prompt release.
use super::*;
use crate::agent_sessions::{domain::*, ports::*, preparation::*};
use crate::execution_targets::domain::*;
use serde::Deserialize;
use std::collections::HashMap;
use std::sync::{
    atomic::{AtomicBool, Ordering},
    Mutex,
};

#[derive(Clone, Debug, Deserialize, serde::Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct PreparedMessageInput {
    pub session_id: Option<AgentSessionId>,
    pub submission_id: AgentInvocationId,
    pub submitted_text: String,
    #[serde(default)]
    pub attachments: Vec<RuntimeImageAttachment>,
    #[serde(default)]
    pub context: Vec<InvocationContextPart>,
    pub title: Option<String>,
    pub working_directory: Option<String>,
    pub execution_selection: Option<SessionExecutionSelection>,
    pub model: Option<String>,
    pub reasoning_mode: Option<String>,
    pub sandbox_mode: Option<crate::execution_configuration::SandboxMode>,
    pub folder_target: Option<crate::agent_sessions::organization::SessionFolderTarget>,
}

fn submission_fingerprint(
    input: &PreparedMessageInput,
) -> Result<String, AgentSessionApplicationError> {
    use sha2::Digest;
    let encoded = serde_json::to_vec(input)
        .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?;
    Ok(format!("{:x}", sha2::Sha256::digest(encoded)))
}

#[derive(Default)]
pub(super) struct PreparationWorkers {
    slots: Mutex<HashMap<AgentInvocationId, Arc<AtomicBool>>>,
    stopping: AtomicBool,
}
impl PreparationWorkers {
    pub(super) fn reserve(
        &self,
        id: &AgentInvocationId,
    ) -> Result<Arc<AtomicBool>, AgentSessionApplicationError> {
        let mut slots = self.slots.lock().map_err(|_| {
            AgentSessionApplicationError::conflict("Preparation supervisor unavailable")
        })?;
        if self.stopping.load(Ordering::SeqCst) || slots.len() >= 8 {
            return Err(AgentSessionApplicationError::conflict(
                "Session setup is busy; try Send again",
            ));
        }
        if slots.contains_key(id) {
            return Err(AgentSessionApplicationError::conflict(
                "This submission is already preparing",
            ));
        }
        let cancel = Arc::new(AtomicBool::new(false));
        slots.insert(id.clone(), cancel.clone());
        Ok(cancel)
    }
    pub(super) fn cancel_all(&self) {
        self.stopping.store(true, Ordering::SeqCst);
        if let Ok(slots) = self.slots.lock() {
            for flag in slots.values() {
                flag.store(true, Ordering::SeqCst);
            }
        }
    }
    pub(super) fn release(&self, id: &AgentInvocationId) {
        if let Ok(mut slots) = self.slots.lock() {
            slots.remove(id);
        }
    }
}

impl AgentSessionApplication {
    pub fn accept_prepared_message(
        &self,
        input: PreparedMessageInput,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        self.delivery().accept_prepared(input)
    }

    pub(super) fn accept_prepared_delivery(
        &self,
        input: PreparedMessageInput,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        self.accept_prepared_delivery_as(input, AgentInvocationInputProvenance::User)
    }

    pub fn accept_prepared_product(
        &self,
        input: PreparedMessageInput,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        self.accept_prepared_delivery_as(input, AgentInvocationInputProvenance::Application)
    }

    fn accept_prepared_delivery_as(
        &self,
        input: PreparedMessageInput,
        provenance: AgentInvocationInputProvenance,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        let command = crate::agent_sessions::client::HostCommand {
            method: if provenance == AgentInvocationInputProvenance::User {
                "send"
            } else {
                "sendProduct"
            }
            .into(),
            input: serde_json::to_value(&input)
                .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?,
        };
        let routed = if let Some(id) = &input.session_id {
            self.route_owned_command(id, command)
        } else if let Some(selection) = input
            .execution_selection
            .as_ref()
            .filter(|s| matches!(s.execution.connection, ExecutionConnection::Device))
        {
            self.execution_targets()
                .and_then(|t| {
                    t.endpoints
                        .host_command(&selection.execution.device_id, command)
                })
                .map(Some)
        } else {
            Ok(None)
        }
        .map_err(AgentSessionApplicationError::invalid)?;
        if let Some(result) = routed {
            return serde_json::from_value(result)
                .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()));
        }
        if input.submitted_text.trim().is_empty() && input.attachments.is_empty() {
            return Err(AgentSessionApplicationError::invalid(
                "A message must contain text",
            ));
        }
        if let Some(existing) = self
            .repository
            .get_invocation(&input.submission_id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            let previous = self
                .repository
                .preparation(&existing.id)
                .map_err(AgentSessionApplicationError::repository)?;
            let fingerprint = submission_fingerprint(&input)?;
            let matches = existing.input_provenance == provenance
                && existing.submitted_text == input.submitted_text
                && existing.attachments == input.attachments
                && input
                    .session_id
                    .as_ref()
                    .is_none_or(|id| id == &existing.session_id)
                && previous.as_ref().is_some_and(|p| {
                    p.submission_fingerprint
                        .as_ref()
                        .is_none_or(|accepted| accepted == &fingerprint)
                        && p.model == input.model
                        && p.reasoning_mode == input.reasoning_mode
                        && p.sandbox_mode == input.sandbox_mode
                        && p.accepted_working_directory == input.working_directory
                        && p.context == input.context
                });
            if !matches {
                return Err(AgentSessionApplicationError::conflict(
                    "Submission identity was already used for different choices",
                ));
            }
            return Ok(SendAgentSessionMessageResult {
                session_id: existing.session_id,
                invocation_id: existing.id,
            });
        }
        if let Some(id) = &input.session_id {
            let history = self.load_session(id)?;
            if history
                .invocations
                .last()
                .is_some_and(|turn| turn.invocation.status == AgentInvocationStatus::Interrupted)
            {
                return Err(AgentSessionApplicationError::conflict("The last turn was interrupted. Fork this conversation and choose a target to continue."));
            }
        }
        self.accept_prepared_inner(input, provenance)
    }
    pub(super) fn accept_prepared_inner(
        &self,
        input: PreparedMessageInput,
        provenance: AgentInvocationInputProvenance,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        self.accept_queued_content(input, provenance, false)
    }
    pub(super) fn accept_queued_content(
        &self,
        mut input: PreparedMessageInput,
        provenance: AgentInvocationInputProvenance,
        pinned_delivery: bool,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        let fingerprint = submission_fingerprint(&input)?;
        if let Some(id) = &input.session_id {
            if self
                .load_session(id)?
                .invocations
                .last()
                .is_some_and(|turn| turn.invocation.status == AgentInvocationStatus::Interrupted)
            {
                return Err(AgentSessionApplicationError::conflict("The last turn was interrupted. Fork this conversation and choose a target to continue."));
            }
        }
        let now = self.clock.now();
        let session = match &input.session_id {
            Some(id) => self.load_session(id)?.session,
            None => AgentSession {
                id: self.ids.session_id(),
                title: input
                    .title
                    .clone()
                    .unwrap_or_else(|| super::creation::title_from_message(&input.submitted_text)),
                execution_target: None,
                working_directory: None,
                workspace_origin: None,
                availability: AgentSessionAvailability::Available,
                runtime_binding: AgentRuntimeBinding {
                    external_context_id: None,
                    runtime_version: self.runtime_version.clone(),
                },
                requested_options: AgentRuntimeOptions::default(),
                session_profile: None,
                harness_version: None,
                assigned_identity: None,
                created_at: now,
                updated_at: now,
            },
        };
        if session.harness_version.is_some() && !pinned_delivery {
            return Err(AgentSessionApplicationError::invalid(
                "Mutable execution targets are available for ordinary sessions only",
            ));
        }
        if let Some(selection) = input.execution_selection.as_mut() {
            let capability = self
                .capability_profiles
                .as_ref()
                .ok_or_else(|| {
                    AgentSessionApplicationError::invalid("Capability Profiles unavailable")
                })?
                .resolve_draft_selection(&selection.capability_profile_id, &selection.execution)
                .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?;
            selection.capability_profile_revision = capability.revision;
        }
        let operation = if pinned_delivery {
            None
        } else {
            self.target_operation_for_submission(&session, input.execution_selection.as_ref())?
        };
        let id = input.submission_id.clone();
        let pending = AgentInvocation {
            attachments: input.attachments,
            id: id.clone(),
            session_id: session.id.clone(),
            submitted_text: input.submitted_text,
            input_provenance: provenance,
            status: AgentInvocationStatus::Pending,
            requested_options: if pinned_delivery {
                session.requested_options.clone()
            } else {
                AgentRuntimeOptions::default()
            },
            effective_options: None,
            started_at: None,
            completed_at: None,
            exit_code: None,
            signal: None,
            runtime_error: None,
            diagnostics: vec![],
            created_at: now,
            updated_at: now,
        };
        let preparation = SessionPreparation {
            submission_fingerprint: Some(fingerprint),
            pinned_delivery,
            invocation_id: id.clone(),
            session_id: session.id.clone(),
            phase: PreparationPhase::Accepted,
            steps: vec![],
            error: None,
            can_retry: false,
            selection: None,
            requested_capability_profile: input.execution_selection.as_ref().map(|s| {
                crate::agent_sessions::preparation::QueuedCapabilityProfile {
                    id: s.capability_profile_id.clone(),
                    revision: s.capability_profile_revision,
                }
            }),
            source_target: session.execution_target.clone(),
            source_binding: session.runtime_binding.clone(),
            prepared_binding: None,
            source_provider_binding: None,
            resolved_target: None,
            accepted_working_directory: input.working_directory.clone(),
            context: input.context,
            resolved_working_directory: input
                .working_directory
                .or(session.working_directory.clone()),
            current_resolution: None,
            resolution: None,
            execution_snapshot: None,
            model: input.model,
            reasoning_mode: input.reasoning_mode,
            sandbox_mode: input.sandbox_mode,
            delivery_started: false,
        };
        let new_session = input
            .session_id
            .is_none()
            .then(|| (session.clone(), input.folder_target.map(Into::into)));
        self.repository
            .accept_queued_preparation(new_session, pending, preparation, operation, now)
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_or_record(AgentSessionNotification::PreparationUpdated {
            session_id: session.id.clone(),
            invocation_id: id.clone(),
        });
        self.wake_session_queue(session.id.clone());
        Ok(SendAgentSessionMessageResult {
            session_id: session.id,
            invocation_id: id,
        })
    }
    pub(super) fn spawn_preparation(&self, id: AgentInvocationId, cancel: Arc<AtomicBool>) {
        let app = self.clone();
        self.workers.spawn_blocking(move || {
            let result = app.run_preparation(&id, &cancel);
            if let Err(error) = result {
                let message = error.to_string();
                let _ = app.fail_preparation(&id, message.clone(), cancel.load(Ordering::SeqCst));
                app.block_queued_invocation(&id, message);
            } else if app
                .repository
                .get_invocation(&id)
                .ok()
                .flatten()
                .is_some_and(|invocation| invocation.status.is_terminal())
            {
                app.complete_queued_invocation(&id);
            }
            app.preparation_workers.release(&id);
            app.wake_all_session_queues();
        });
    }
    pub(super) fn reconcile_preparation_on_startup(
        &self,
        id: &AgentInvocationId,
    ) -> Result<(), AgentSessionApplicationError> {
        let Some(mut preparation) = self
            .repository
            .preparation(id)
            .map_err(AgentSessionApplicationError::repository)?
        else {
            return Ok(());
        };
        let unfinished = matches!(
            preparation.phase,
            PreparationPhase::Accepted | PreparationPhase::Preparing
        ) || (preparation.phase == PreparationPhase::Ready
            && preparation.steps.iter().any(|step| {
                step.id == "delivery" && step.status != PreparationStepStatus::Completed
            }));
        if !unfinished {
            return Ok(());
        }
        preparation.phase = PreparationPhase::Failed;
        preparation.can_retry = !preparation.delivery_started;
        preparation.error=Some(if preparation.delivery_started {
            "Application stopped after prompt delivery began. Delivery is uncertain and cannot be retried automatically."
        }else{
            "Application stopped during setup. The prompt was not automatically replayed."
        }.into());
        for step in &mut preparation.steps {
            if step.status == PreparationStepStatus::Running {
                step.status = PreparationStepStatus::Failed;
                step.error = preparation.error.clone();
            }
        }
        self.save_progress(&preparation)
    }
    pub fn load_preparation(
        &self,
        id: &AgentSessionId,
    ) -> Result<Option<SessionPreparation>, AgentSessionApplicationError> {
        self.repository
            .latest_preparation(id)
            .map_err(AgentSessionApplicationError::repository)
    }
    pub fn cancel_preparation(
        &self,
        id: &AgentInvocationId,
    ) -> Result<(), AgentSessionApplicationError> {
        let slots = self.preparation_workers.slots.lock().map_err(|_| {
            AgentSessionApplicationError::conflict("Preparation supervisor unavailable")
        })?;
        let p = self
            .repository
            .preparation(id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Preparation not found"))?;
        if p.delivery_started {
            drop(slots);
            self.runtime_for_invocation(id)?
                .cancel_invocation(id)
                .map_err(AgentSessionApplicationError::runtime)?;
            return Ok(());
        }
        if let Some(flag) = slots.get(id) {
            flag.store(true, Ordering::SeqCst);
        }
        drop(slots);
        if p.steps
            .iter()
            .any(|s| s.id == "conversation" && s.status == PreparationStepStatus::Running)
        {
            let _ = self.runtime_for_invocation(id)?.cancel_invocation(id);
        }
        Ok(())
    }
    pub fn retry_preparation(
        &self,
        id: &AgentInvocationId,
    ) -> Result<(), AgentSessionApplicationError> {
        let flag = self.preparation_workers.reserve(id)?;
        if let Err(e) = self.repository.retry_preparation(id, self.clock.now()) {
            self.preparation_workers.release(id);
            return Err(AgentSessionApplicationError::repository(e));
        }
        self.spawn_preparation(id.clone(), flag);
        Ok(())
    }
    pub(super) fn save_progress(
        &self,
        p: &SessionPreparation,
    ) -> Result<(), AgentSessionApplicationError> {
        self.repository
            .save_preparation(p)
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_or_record(AgentSessionNotification::PreparationUpdated {
            session_id: p.session_id.clone(),
            invocation_id: p.invocation_id.clone(),
        });
        Ok(())
    }
    fn step(
        &self,
        p: &mut SessionPreparation,
        id: &str,
        label: &str,
        status: PreparationStepStatus,
    ) -> Result<(), AgentSessionApplicationError> {
        if let Some(step) = p.steps.iter_mut().find(|s| s.id == id) {
            step.status = status;
            step.error = None;
        } else {
            p.steps.push(PreparationStep {
                id: id.into(),
                label: label.into(),
                status,
                error: None,
            });
        }
        self.save_progress(p)
    }
    pub(super) fn check_preparation_cancel(
        cancel: &AtomicBool,
    ) -> Result<(), AgentSessionApplicationError> {
        if cancel.load(Ordering::SeqCst) {
            Err(AgentSessionApplicationError::conflict(
                "Setup canceled before prompt delivery",
            ))
        } else {
            Ok(())
        }
    }
    fn fail_preparation(
        &self,
        id: &AgentInvocationId,
        error: String,
        canceled: bool,
    ) -> Result<(), AgentSessionApplicationError> {
        let mut p = self
            .repository
            .preparation(id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Preparation not found"))?;
        if !p.delivery_started {
            if let Ok(runtime) = self.runtime_for_invocation(id) {
                let _ = runtime.cancel_invocation(id);
            }
        }
        p.phase = if canceled {
            PreparationPhase::Canceled
        } else {
            PreparationPhase::Failed
        };
        p.can_retry = !p.delivery_started;
        p.error = Some(error.clone());
        if let Some(step) = p
            .steps
            .iter_mut()
            .find(|s| s.status == PreparationStepStatus::Running)
        {
            step.status = PreparationStepStatus::Failed;
            step.error = Some(error.clone());
        }
        self.save_progress(&p)?;
        if let Some(invocation) = self
            .repository
            .get_invocation(id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            if invocation.status.is_active() {
                let invocation = self
                    .repository
                    .finish_invocation(
                        id,
                        InvocationCompletion {
                            status: if canceled {
                                AgentInvocationTerminalStatus::Canceled
                            } else {
                                AgentInvocationTerminalStatus::Failed
                            },
                            completed_at: self.clock.now(),
                            exit_code: None,
                            signal: None,
                            runtime_error: Some(AgentRuntimeFailure {
                                code: if p.delivery_started {
                                    "delivery_uncertain"
                                } else {
                                    "preparation_failed"
                                }
                                .into(),
                                message: error,
                                details: None,
                            }),
                        },
                        self.clock.now(),
                    )
                    .map_err(AgentSessionApplicationError::repository)?;
                self.notify_or_record(AgentSessionNotification::InvocationTerminal {
                    session_id: p.session_id,
                    invocation,
                });
            }
        }
        Ok(())
    }
}

mod execution;
