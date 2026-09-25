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

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct PreparedMessageInput {
    pub(crate) session_id: Option<AgentSessionId>,
    pub(crate) submission_id: AgentInvocationId,
    pub(crate) submitted_text: String,
    pub(crate) title: Option<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) execution_selection: Option<SessionExecutionSelection>,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_mode: Option<String>,
    pub(crate) sandbox_mode: Option<crate::execution_configuration::SandboxMode>,
    pub(crate) folder_target: Option<crate::agent_sessions::organization::SessionFolderTarget>,
}

#[derive(Default)]
pub(super) struct PreparationWorkers {
    slots: Mutex<HashMap<AgentInvocationId, Arc<AtomicBool>>>,
    stopping: AtomicBool,
}
impl PreparationWorkers {
    fn reserve(
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
    fn release(&self, id: &AgentInvocationId) {
        if let Ok(mut slots) = self.slots.lock() {
            slots.remove(id);
        }
    }
}

impl AgentSessionApplication {
    pub(crate) fn accept_prepared_message(
        &self,
        mut input: PreparedMessageInput,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        if input.submitted_text.trim().is_empty() {
            return Err(AgentSessionApplicationError::invalid(
                "A message must contain text",
            ));
        }
        // A device move has no synthetic invocation. When a prompt arrives, retain the normal
        // durable invocation but bind its preparation to the move's requested/resolved target.
        if let Some(session_id) = input.session_id.clone() {
            if let Some(transition) = self.queue_target_transition_prompt(
                &session_id,
                input.submitted_text.clone(),
                input.submission_id.as_str().into(),
            )? {
                if transition.phase.is_unfinished()
                    || transition.phase
                        == crate::agent_sessions::target_transition::TargetTransitionPhase::Ready
                {
                    input.execution_selection = Some(match transition.resolved_target {
                        Some(target) => SessionExecutionSelection {
                            capability_profile_id: target.capability_profile_id.clone(),
                            capability_profile_revision: target.capability_profile_revision,
                            execution: target.execution.clone(),
                            workspace: SessionWorkspaceSelection::Existing { target },
                        },
                        None => transition.destination_selection,
                    });
                }
            }
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
            let matches = existing.submitted_text == input.submitted_text
                && input
                    .session_id
                    .as_ref()
                    .is_none_or(|id| id == &existing.session_id)
                && previous.as_ref().is_some_and(|p| {
                    p.model == input.model
                        && p.reasoning_mode == input.reasoning_mode
                        && p.sandbox_mode == input.sandbox_mode
                        && p.selection == input.execution_selection
                        && p.accepted_working_directory == input.working_directory
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
        let cancel = self.preparation_workers.reserve(&input.submission_id)?;
        let result = self.accept_prepared_inner(input.clone());
        match result {
            Ok(ack) => {
                self.spawn_preparation(ack.invocation_id.clone(), cancel);
                Ok(ack)
            }
            Err(error) => {
                self.preparation_workers.release(&input.submission_id);
                Err(error)
            }
        }
    }
    fn accept_prepared_inner(
        &self,
        mut input: PreparedMessageInput,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
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
        if session.harness_version.is_some() {
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
            if let SessionWorkspaceSelection::Create { commit, .. } = &selection.workspace {
                if commit.len() != 40 || !commit.bytes().all(|c| c.is_ascii_hexdigit()) {
                    return Err(AgentSessionApplicationError::invalid(
                        "Confirm an exact published commit before Send",
                    ));
                }
            }
        }
        let id = input.submission_id.clone();
        let pending = AgentInvocation {
            id: id.clone(),
            session_id: session.id.clone(),
            submitted_text: input.submitted_text,
            input_provenance: AgentInvocationInputProvenance::User,
            status: AgentInvocationStatus::Pending,
            requested_options: AgentRuntimeOptions::default(),
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
            invocation_id: id.clone(),
            session_id: session.id.clone(),
            phase: PreparationPhase::Accepted,
            steps: vec![],
            error: None,
            can_retry: false,
            selection: input.execution_selection,
            source_target: session.execution_target.clone(),
            source_binding: session.runtime_binding.clone(),
            prepared_binding: None,
            parked_source: None,
            resolved_target: None,
            accepted_working_directory: input.working_directory.clone(),
            resolved_working_directory: input
                .working_directory
                .or(session.working_directory.clone()),
            current_resolution: None,
            resolution: None,
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
            .accept_preparation(new_session, pending, preparation)
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_or_record(AgentSessionNotification::PreparationUpdated {
            session_id: session.id.clone(),
            invocation_id: id.clone(),
        });
        Ok(SendAgentSessionMessageResult {
            session_id: session.id,
            invocation_id: id,
        })
    }
    fn spawn_preparation(&self, id: AgentInvocationId, cancel: Arc<AtomicBool>) {
        let app = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let result = app.run_preparation(&id, &cancel);
            if let Err(error) = result {
                let _ = app.fail_preparation(&id, error.to_string(), cancel.load(Ordering::SeqCst));
            }
            app.preparation_workers.release(&id);
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
    pub(crate) fn load_preparation(
        &self,
        id: &AgentSessionId,
    ) -> Result<Option<SessionPreparation>, AgentSessionApplicationError> {
        self.repository
            .latest_preparation(id)
            .map_err(AgentSessionApplicationError::repository)
    }
    pub(crate) fn cancel_preparation(
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
    pub(crate) fn retry_preparation(
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
    fn save_progress(&self, p: &SessionPreparation) -> Result<(), AgentSessionApplicationError> {
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
    fn check_preparation_cancel(cancel: &AtomicBool) -> Result<(), AgentSessionApplicationError> {
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

mod conversation;
mod execution;
