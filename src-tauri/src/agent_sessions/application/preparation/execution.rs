//! Resolve the destination, establish native readiness, then release the accepted turn.
use super::*;
use crate::agent_sessions::application::update_sink::PersistedRuntimeUpdateSink;
use crate::execution_configuration::{
    validate_session_skill_inputs, DirectUserInvocationRequest, NodeProfile,
    SessionCreationRequest, SessionProfileResolver,
};
impl AgentSessionApplication {
    pub(super) fn run_preparation(
        &self,
        id: &AgentInvocationId,
        cancel: &AtomicBool,
    ) -> Result<(), AgentSessionApplicationError> {
        let mut p = self
            .repository
            .preparation(id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Preparation not found"))?;
        if let Some(transition) = self.await_target_transition(&p.session_id)? {
            let target = transition.resolved_target.ok_or_else(|| {
                AgentSessionApplicationError::invalid(
                    "The device switch completed without a destination worktree",
                )
            })?;
            p.selection = Some(SessionExecutionSelection {
                capability_profile_id: target.capability_profile_id.clone(),
                capability_profile_revision: target.capability_profile_revision,
                execution: target.execution.clone(),
                workspace: SessionWorkspaceSelection::Existing { target },
            });
            p.source_target = Some(transition.source_target);
            self.save_progress(&p)?;
        }
        let session = self.load_session(&p.session_id)?.session;
        let endpoints = self.endpoints.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid("Execution endpoints unavailable")
        })?;
        let profiles = self.capability_profiles.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid("Capability Profiles unavailable")
        })?;
        let targets = self.execution_target_service.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid("Execution targets unavailable")
        })?;
        p.phase = PreparationPhase::Preparing;
        p.error = None;
        p.can_retry = false;
        self.save_progress(&p)?;
        Self::check_preparation_cancel(cancel)?;
        let mut selection = match &p.selection {
            Some(selection) => selection.clone(),
            None => match &session.execution_target {
                Some(target) => SessionExecutionSelection {
                    capability_profile_id: target.capability_profile_id.clone(),
                    capability_profile_revision: target.capability_profile_revision,
                    execution: target.execution.clone(),
                    workspace: SessionWorkspaceSelection::Existing {
                        target: target.clone(),
                    },
                },
                None => {
                    let profile = profiles
                        .default_profile()
                        .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?;
                    SessionExecutionSelection {
                        capability_profile_id: profile.capability_profile_id.clone(),
                        capability_profile_revision: profile.revision,
                        execution: profile.execution.clone(),
                        workspace: SessionWorkspaceSelection::Auxiliary,
                    }
                }
            },
        };
        let workspace_label = if matches!(
            selection.workspace,
            SessionWorkspaceSelection::Create { .. }
        ) {
            "Create confirmed worktree"
        } else {
            "Resolve working folder"
        };
        selection.execution = endpoints
            .freeze_binding(selection.execution.clone())
            .map_err(AgentSessionApplicationError::invalid)?;
        let source = if p.source_binding.external_context_id.is_some() {
            let mut source = p
                .source_target
                .as_ref()
                .map(|target| target.execution.clone())
                .unwrap_or_default();
            if p.source_target.is_none() {
                if let Some(authority) = &self.native_profile_launch_authority {
                    if let Some(reference) = authority
                        .bound_configuration_ref(&session.id)
                        .map_err(AgentSessionApplicationError::invalid)?
                    {
                        source.configuration_ref = reference;
                    }
                }
            }
            Some(
                endpoints
                    .freeze_binding(source)
                    .map_err(AgentSessionApplicationError::invalid)?,
            )
        } else {
            None
        };
        let mut required = vec![
            ("workspace", workspace_label),
            ("configuration", "Resolve execution settings"),
        ];
        if source
            .as_ref()
            .is_some_and(|source| source != &selection.execution)
        {
            required.push(("history", "Transfer conversation history"));
        }
        required.extend([
            ("conversation", "Prepare conversation"),
            ("delivery", "Deliver submitted prompt"),
        ]);
        p.steps = required
            .into_iter()
            .map(|(id, label)| {
                p.steps
                    .iter()
                    .find(|step| step.id == id)
                    .cloned()
                    .unwrap_or_else(|| PreparationStep {
                        id: id.into(),
                        label: label.into(),
                        status: PreparationStepStatus::Pending,
                        error: None,
                    })
            })
            .collect();
        self.save_progress(&p)?;
        if p.resolved_target.is_none() {
            self.step(
                &mut p,
                "workspace",
                workspace_label,
                PreparationStepStatus::Running,
            )?;
        }
        if p.resolved_target.is_none() {
            let existing_auxiliary = session.execution_target.as_ref().filter(|target| {
                target.repository_id.is_empty()
                    && target.execution.device_id == selection.execution.device_id
                    && target.execution.connection == selection.execution.connection
            });
            let target = if matches!(selection.workspace, SessionWorkspaceSelection::Auxiliary)
                && existing_auxiliary.is_some()
            {
                let mut target = existing_auxiliary.unwrap().clone();
                target.execution = selection.execution.clone();
                target.capability_profile_id = selection.capability_profile_id.clone();
                target.capability_profile_revision = selection.capability_profile_revision;
                target
            } else if session.execution_target.is_none()
                && p.resolved_working_directory.is_some()
                && !selection.execution.is_remote()
                && matches!(selection.workspace, SessionWorkspaceSelection::Auxiliary)
            {
                SessionExecutionTarget {
                    capability_profile_id: selection.capability_profile_id.clone(),
                    capability_profile_revision: selection.capability_profile_revision,
                    execution: selection.execution.clone(),
                    repository_id: String::new(),
                    branch_ref: String::new(),
                    worktree_id: session.id.as_str().into(),
                    path: self
                        .prepare_working_directory(
                            &session.id,
                            p.resolved_working_directory.clone(),
                        )?
                        .ok_or_else(|| {
                            AgentSessionApplicationError::invalid("Working folder unavailable")
                        })?,
                    head: None,
                }
            } else {
                let key = if matches!(selection.workspace, SessionWorkspaceSelection::Auxiliary) {
                    p.session_id.as_str()
                } else {
                    p.invocation_id.as_str()
                };
                targets
                    .materialize_selection(&selection, key)
                    .map_err(AgentSessionApplicationError::invalid)?
            };
            p.resolved_working_directory = Some(target.path.clone());
            p.resolved_target = Some(target);
        }
        self.step(
            &mut p,
            "workspace",
            workspace_label,
            PreparationStepStatus::Completed,
        )?;
        Self::check_preparation_cancel(cancel)?;
        let destination = p.resolved_target.as_ref().expect("materialized").clone();
        let capability = profiles
            .read(&selection.capability_profile_id)
            .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?;
        if capability.revision != selection.capability_profile_revision
            || endpoints
                .freeze_binding(capability.execution.clone())
                .map_err(AgentSessionApplicationError::invalid)?
                != destination.execution
        {
            return Err(AgentSessionApplicationError::conflict(
                "Capability Profile changed during setup",
            ));
        }
        self.step(
            &mut p,
            "configuration",
            "Resolve execution settings",
            PreparationStepStatus::Running,
        )?;
        let runtime_profile = profiles
            .runtime_for_binding(&destination.execution, Some(&destination.path))
            .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?;
        let session_skill_inputs = self
            .compile_capability_skill_inputs(
                &capability,
                &destination.execution.configuration_ref,
                Some(&destination.path),
            )
            .map_err(AgentSessionApplicationError::invalid)?;
        let creation = SessionProfileResolver::resolve_snapshot(
            runtime_profile.clone(),
            SessionCreationRequest {
                contract_version: 1,
                agent_mcp_configuration: Default::default(),
                node_profile: NodeProfile {
                    contract_version: 1,
                    allowed_capabilities: super::super::configuration::default_node_capabilities(
                        &capability,
                    ),
                    pinned_defaults: Default::default(),
                },
                capability_profile: capability,
                session_skill_inputs,
            },
        )
        .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?;
        let pinned_skill_inputs = creation.session_profile().session_skill_inputs().to_vec();
        let resolution = SessionProfileResolver::resolve_direct_user_snapshot(
            runtime_profile,
            &creation,
            DirectUserInvocationRequest {
                contract_version: 1,
                model: p.model.clone(),
                reasoning_mode: p.reasoning_mode.clone(),
                sandbox_mode: p.sandbox_mode,
            },
        )
        .map_err(|e| AgentSessionApplicationError::invalid(e.to_string()))?;
        p.current_resolution = Some(creation);
        p.resolution = Some(resolution.clone());
        self.step(
            &mut p,
            "configuration",
            "Resolve execution settings",
            PreparationStepStatus::Completed,
        )?;
        if let Some(context) = p.source_binding.external_context_id.clone() {
            let source = source
                .as_ref()
                .expect("existing context has a source binding");
            if source != &destination.execution
                && !p
                    .steps
                    .iter()
                    .any(|s| s.id == "history" && s.status == PreparationStepStatus::Completed)
            {
                self.step(
                    &mut p,
                    "history",
                    "Transfer conversation history",
                    PreparationStepStatus::Running,
                )?;
                endpoints
                    .transfer_continuation(source, &destination.execution, &context)
                    .map_err(AgentSessionApplicationError::invalid)?;
                self.step(
                    &mut p,
                    "history",
                    "Transfer conversation history",
                    PreparationStepStatus::Completed,
                )?;
            }
        }
        Self::check_preparation_cancel(cancel)?;
        let runtime = endpoints
            .runtime(&destination.execution)
            .map_err(AgentSessionApplicationError::invalid)?;
        let requested = runtime_options(&resolution.selections);
        let resume_context = p
            .prepared_binding
            .as_ref()
            .and_then(|b| b.external_context_id.clone())
            .or(p.source_binding.external_context_id.clone());
        let mode = if resume_context.is_some() {
            RuntimeInvocationMode::Resume
        } else {
            RuntimeInvocationMode::Start
        };
        let preflight = runtime
            .preflight_invocation(mode, &requested)
            .map_err(AgentSessionApplicationError::runtime)?;
        let selected_skills = validate_session_skill_inputs(&pinned_skill_inputs)
            .map_err(AgentSessionApplicationError::invalid)?;
        let mut extension = reasoning_launch_extension(&resolution.selections).unwrap_or_default();
        extension.skill_inputs = selected_skills;
        extension = super::super::configuration::pinned_exposure_extension(
            p.current_resolution.as_ref().expect("resolved creation"),
            extension,
        )
        .map_err(AgentSessionApplicationError::invalid)?;
        let mut extension = Some(extension);
        if !destination.execution.is_remote() {
            extension = self.add_workspace_capabilities(extension);
            if let Some(authority) = &self.native_profile_launch_authority {
                extension = Some(
                    authority
                        .prepare_destination_launch(
                            &destination.execution.configuration_ref,
                            &p.session_id,
                            &p.invocation_id,
                            resume_context.is_some(),
                            extension,
                        )
                        .map_err(AgentSessionApplicationError::invalid)?,
                );
            }
        }
        let invocation = self
            .repository
            .get_invocation(id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Invocation not found"))?;
        if !destination.execution.is_remote() {
            if let Some(extension) = extension.as_mut() {
                for skill in self.direct_user_native_skill_inputs(
                    &destination.execution.configuration_ref,
                    Some(&destination.path),
                    &invocation.submitted_text,
                ) {
                    if !extension
                        .skill_inputs
                        .iter()
                        .any(|existing| existing.path == skill.path)
                    {
                        extension.skill_inputs.push(skill);
                    }
                }
            }
        }
        let sink = Arc::new(PreparationUpdateGate {
            inner: Arc::new(PersistedRuntimeUpdateSink::new(
                self.repository.clone(),
                self.notifier.clone(),
                self.clock.clone(),
                self.ids.clone(),
                self.update_lanes.clone(),
            )),
            buffer: Mutex::new(Some(Vec::new())),
        });
        self.step(
            &mut p,
            "conversation",
            "Prepare conversation",
            PreparationStepStatus::Running,
        )?;
        let ready = runtime
            .prepare_invocation(
                RuntimeInvocationRequest {
                    session_id: p.session_id.clone(),
                    invocation_id: id.clone(),
                    submitted_text: invocation.submitted_text,
                    working_directory: Some(destination.path.clone()),
                    options: preflight.effective_options.clone(),
                    launch_extension: extension,
                },
                resume_context,
                sink.clone(),
            )
            .map_err(AgentSessionApplicationError::runtime)?;
        if let Err(e) = Self::check_preparation_cancel(cancel) {
            let _ = runtime.cancel_invocation(id);
            return Err(e);
        }
        p.resolved_working_directory = Some(ready.working_directory.clone());
        let binding = AgentRuntimeBinding {
            external_context_id: Some(ready.external_context_id),
            runtime_version: session.runtime_binding.runtime_version,
        };
        p.prepared_binding = Some(binding.clone());
        self.save_progress(&p)?;
        if !destination.execution.is_remote() {
            if let Some(authority) = &self.native_profile_launch_authority {
                authority
                    .commit_destination(&destination.execution.configuration_ref, &p.session_id)
                    .map_err(AgentSessionApplicationError::invalid)?;
            }
        }
        self.repository
            .commit_prepared_binding(&p, binding, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)?;
        self.step(
            &mut p,
            "conversation",
            "Prepare conversation",
            PreparationStepStatus::Completed,
        )?;
        self.repository
            .mark_invocation_running(
                id,
                self.clock.now(),
                preflight.effective_options,
                self.clock.now(),
            )
            .map_err(AgentSessionApplicationError::repository)?;
        sink.open().map_err(AgentSessionApplicationError::runtime)?;
        // Persist the boundary before transport write. A lost response never authorizes replay.
        let delivery_gate = self.preparation_workers.slots.lock().map_err(|_| {
            AgentSessionApplicationError::conflict("Preparation supervisor unavailable")
        })?;
        if let Err(error) = Self::check_preparation_cancel(cancel) {
            drop(delivery_gate);
            let _ = runtime.cancel_invocation(id);
            return Err(error);
        }
        p.delivery_started = true;
        p.phase = PreparationPhase::Ready;
        self.step(
            &mut p,
            "delivery",
            "Deliver submitted prompt",
            PreparationStepStatus::Running,
        )?;
        drop(delivery_gate);
        runtime
            .deliver_prepared_invocation(id)
            .map_err(AgentSessionApplicationError::runtime)?;
        self.repository
            .record_invocation_launch_accepted(id, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)?;
        self.step(
            &mut p,
            "delivery",
            "Deliver submitted prompt",
            PreparationStepStatus::Completed,
        )?;
        Ok(())
    }
}

struct PreparationUpdateGate {
    inner: Arc<dyn AgentRuntimeUpdateSink>,
    buffer: Mutex<Option<Vec<(AgentInvocationId, RuntimeUpdate)>>>,
}
impl PreparationUpdateGate {
    fn open(&self) -> Result<(), RuntimePortError> {
        let mut state = self.buffer.lock().map_err(|_| {
            RuntimePortError::new(
                RuntimePortErrorKind::Unavailable,
                "Preparation updates unavailable",
            )
        })?;
        if let Some(events) = state.take() {
            for (id, update) in events {
                self.inner.emit_update(&id, update)?;
            }
        }
        Ok(())
    }
}
impl AgentRuntimeUpdateSink for PreparationUpdateGate {
    fn emit_update(
        &self,
        id: &AgentInvocationId,
        update: RuntimeUpdate,
    ) -> Result<(), RuntimePortError> {
        let mut state = self.buffer.lock().map_err(|_| {
            RuntimePortError::new(
                RuntimePortErrorKind::Unavailable,
                "Preparation updates unavailable",
            )
        })?;
        if let Some(events) = state.as_mut() {
            events.push((id.clone(), update));
            Ok(())
        } else {
            self.inner.emit_update(id, update)
        }
    }
    fn report_delivery_failure(
        &self,
        id: &AgentInvocationId,
        failure: RuntimeUpdateDeliveryFailure,
    ) {
        self.inner.report_delivery_failure(id, failure)
    }
}
