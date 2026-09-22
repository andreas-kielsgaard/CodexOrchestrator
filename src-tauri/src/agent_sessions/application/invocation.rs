//! Invocation preparation, idempotent launch and launch failure handling.

use super::update_sink::PersistedRuntimeUpdateSink;
use super::{
    AgentSessionApplication, AgentSessionApplicationError, AgentSessionNotification,
    ApplicationInvocationLaunchEvidence, CreateAgentSessionCommand, SendAgentSessionMessageCommand,
    SendAgentSessionMessageLaunchResult, SendAgentSessionMessageResult,
    SendIdempotentApplicationAgentSessionMessageCommand,
};
use crate::agent_sessions::domain::{
    AgentDiagnosticSource, AgentInvocation, AgentInvocationId, AgentInvocationInputProvenance,
    AgentInvocationStatus, AgentInvocationTerminalStatus, AgentRuntimeFailure, AgentSession,
    AgentSessionAvailability, AgentSessionId, InvocationCompletion,
};
use crate::agent_sessions::ports::{
    AgentRuntimeUpdateSink, RepositoryError, RuntimeInvocationMode, RuntimeInvocationRequest,
    RuntimeLaunchExtension, RuntimePortError, RuntimePortErrorKind,
};
use serde_json::json;
use std::sync::Arc;

use super::creation::{normalize_optional, title_from_message};

impl AgentSessionApplication {
    pub(crate) fn send_message(
        &self,
        command: SendAgentSessionMessageCommand,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        self.send_message_with_launch_extension(command, None)
    }

    /// Explicit opt-in for an application-owned invocation service. Generic callers cannot
    /// acquire an extension accidentally because the normal send path always supplies `None`.
    pub(crate) fn send_message_with_launch_extension(
        &self,
        command: SendAgentSessionMessageCommand,
        launch_extension: Option<RuntimeLaunchExtension>,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        self.send_message_with_provenance(
            command,
            launch_extension,
            AgentInvocationInputProvenance::User,
            None,
            false,
        )
        .map(|result| result.acknowledgement)
    }

    pub(crate) fn send_idempotent_application_message_with_launch_observation(
        &self,
        command: SendIdempotentApplicationAgentSessionMessageCommand,
        launch_extension: Option<RuntimeLaunchExtension>,
    ) -> Result<SendAgentSessionMessageLaunchResult, AgentSessionApplicationError> {
        self.send_message_with_provenance(
            command.message,
            launch_extension,
            AgentInvocationInputProvenance::Application,
            Some(command.invocation_id),
            false,
        )
    }

    /// Launches only an already-persisted pending application invocation. It never allocates a
    /// replacement invocation or claims that an existing process can be reattached.
    pub(crate) fn launch_prepared_application_invocation_with_launch_observation(
        &self,
        command: SendIdempotentApplicationAgentSessionMessageCommand,
        launch_extension: Option<RuntimeLaunchExtension>,
    ) -> Result<SendAgentSessionMessageLaunchResult, AgentSessionApplicationError> {
        let session_id = command.message.session_id.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid(
                "prepared application invocation requires a Session",
            )
        })?;
        let session = self
            .repository
            .get_session(session_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent Session not found"))?;
        if session.working_directory
            != normalize_optional(command.message.working_directory.clone())
        {
            return Err(AgentSessionApplicationError::conflict(
                "prepared application invocation working directory does not match its Session",
            ));
        }
        self.send_message_with_provenance(
            command.message,
            launch_extension,
            AgentInvocationInputProvenance::Application,
            Some(command.invocation_id),
            true,
        )
    }

    pub(crate) fn send_idempotent_user_message_with_launch_observation(
        &self,
        command: SendIdempotentApplicationAgentSessionMessageCommand,
        launch_extension: Option<RuntimeLaunchExtension>,
    ) -> Result<SendAgentSessionMessageLaunchResult, AgentSessionApplicationError> {
        self.send_message_with_provenance(
            command.message,
            launch_extension,
            AgentInvocationInputProvenance::User,
            Some(command.invocation_id),
            false,
        )
    }

    pub(crate) fn allocate_application_invocation_id(&self) -> AgentInvocationId {
        self.ids.invocation_id()
    }

    /// Persist an application-owned invocation without runtime preflight or a launch attempt.
    pub(crate) fn prepare_idempotent_application_invocation(
        &self,
        command: SendIdempotentApplicationAgentSessionMessageCommand,
    ) -> Result<SendAgentSessionMessageResult, AgentSessionApplicationError> {
        if command.message.submitted_text.trim().is_empty() {
            return Err(AgentSessionApplicationError::invalid(
                "submitted text cannot be empty",
            ));
        }
        let session_id = command.message.session_id.as_ref().ok_or_else(|| {
            AgentSessionApplicationError::invalid(
                "prepared application invocation requires a Session",
            )
        })?;
        let session = self
            .repository
            .get_session(session_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent Session not found"))?;
        if session.availability != AgentSessionAvailability::Available {
            return Err(AgentSessionApplicationError::conflict(
                "archived Agent Sessions cannot accept messages",
            ));
        }
        let requested_options = command
            .message
            .requested_options
            .clone()
            .unwrap_or_else(|| session.requested_options.clone());
        if let Some(existing) = self
            .repository
            .get_invocation(&command.invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
        {
            if existing.session_id != session.id
                || existing.submitted_text != command.message.submitted_text
                || existing.input_provenance != AgentInvocationInputProvenance::Application
                || existing.requested_options != requested_options
            {
                return Err(AgentSessionApplicationError::conflict(
                    "application Agent Invocation identity was already used for different semantics",
                ));
            }
            return Ok(SendAgentSessionMessageResult {
                session_id: session.id,
                invocation_id: existing.id,
            });
        }
        let created_at = self.clock.now();
        let invocation = self
            .repository
            .create_pending_invocation(AgentInvocation {
                id: command.invocation_id,
                session_id: session.id.clone(),
                submitted_text: command.message.submitted_text,
                input_provenance: AgentInvocationInputProvenance::Application,
                status: AgentInvocationStatus::Pending,
                requested_options,
                effective_options: None,
                started_at: None,
                completed_at: None,
                exit_code: None,
                signal: None,
                runtime_error: None,
                diagnostics: Vec::new(),
                created_at,
                updated_at: created_at,
            })
            .map_err(AgentSessionApplicationError::repository)?;
        Ok(SendAgentSessionMessageResult {
            session_id: session.id,
            invocation_id: invocation.id,
        })
    }

    pub(crate) fn application_invocation_launch_evidence(
        &self,
        invocation_id: &AgentInvocationId,
        expected_session_id: &AgentSessionId,
    ) -> Result<ApplicationInvocationLaunchEvidence, AgentSessionApplicationError> {
        self.invocation_launch_evidence(
            invocation_id,
            expected_session_id,
            AgentInvocationInputProvenance::Application,
        )
    }

    /// Reopens only the classified restart gap before durable launch acceptance. The caller must
    /// still launch through the prepared-invocation seam with the same identity and semantics.
    pub(crate) fn recover_pre_acceptance_application_invocation(
        &self,
        invocation_id: &AgentInvocationId,
        expected_session_id: &AgentSessionId,
    ) -> Result<(), AgentSessionApplicationError> {
        let invocation = self
            .repository
            .get_invocation(invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent invocation not found"))?;
        if invocation.session_id != *expected_session_id
            || invocation.input_provenance != AgentInvocationInputProvenance::Application
        {
            return Err(AgentSessionApplicationError::conflict(
                "pre-acceptance recovery does not match the expected application Session",
            ));
        }
        if self
            .repository
            .invocation_launch_accepted_at(invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .is_some()
        {
            return Err(AgentSessionApplicationError::conflict(
                "launch-accepted invocation cannot return to pre-acceptance state",
            ));
        }
        if invocation.status == AgentInvocationStatus::Pending {
            return Ok(());
        }
        self.repository
            .recover_pre_acceptance_interruption(invocation_id, self.clock.now())
            .map_err(AgentSessionApplicationError::repository)?;
        Ok(())
    }

    pub(crate) fn user_invocation_launch_evidence(
        &self,
        invocation_id: &AgentInvocationId,
        expected_session_id: &AgentSessionId,
    ) -> Result<ApplicationInvocationLaunchEvidence, AgentSessionApplicationError> {
        self.invocation_launch_evidence(
            invocation_id,
            expected_session_id,
            AgentInvocationInputProvenance::User,
        )
    }

    fn invocation_launch_evidence(
        &self,
        invocation_id: &AgentInvocationId,
        expected_session_id: &AgentSessionId,
        expected_provenance: AgentInvocationInputProvenance,
    ) -> Result<ApplicationInvocationLaunchEvidence, AgentSessionApplicationError> {
        let Some(invocation) = self
            .repository
            .get_invocation(invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
        else {
            return Ok(ApplicationInvocationLaunchEvidence::NeverPersisted);
        };
        if invocation.session_id != *expected_session_id
            || invocation.input_provenance != expected_provenance
        {
            return Err(AgentSessionApplicationError::conflict(
                "allocated Agent Invocation launch evidence does not match the expected session and provenance",
            ));
        }
        Ok(
            if self
                .repository
                .invocation_launch_accepted_at(invocation_id)
                .map_err(AgentSessionApplicationError::repository)?
                .is_some()
            {
                ApplicationInvocationLaunchEvidence::LaunchAccepted
            } else {
                ApplicationInvocationLaunchEvidence::PersistedNotAccepted
            },
        )
    }

    fn send_message_with_provenance(
        &self,
        command: SendAgentSessionMessageCommand,
        launch_extension: Option<RuntimeLaunchExtension>,
        input_provenance: AgentInvocationInputProvenance,
        requested_invocation_id: Option<AgentInvocationId>,
        launch_existing_prepared: bool,
    ) -> Result<SendAgentSessionMessageLaunchResult, AgentSessionApplicationError> {
        if command.submitted_text.trim().is_empty() {
            return Err(AgentSessionApplicationError::invalid(
                "submitted text cannot be empty",
            ));
        }

        let session = match command.session_id.as_ref() {
            Some(session_id) => self
                .repository
                .get_session(session_id)
                .map_err(AgentSessionApplicationError::repository)?
                .ok_or_else(|| {
                    AgentSessionApplicationError::not_found("Agent Session not found")
                })?,
            None => self.create_session(CreateAgentSessionCommand {
                title: command
                    .title
                    .clone()
                    .or_else(|| Some(title_from_message(&command.submitted_text))),
                working_directory: command.working_directory.clone(),
                requested_options: command.requested_options.clone().unwrap_or_default(),
            })?,
        };
        if session.availability != AgentSessionAvailability::Available {
            return Err(AgentSessionApplicationError::conflict(
                "archived Agent Sessions cannot accept messages",
            ));
        }
        if session.execution_target.is_some()
            && command
                .working_directory
                .as_ref()
                .is_some_and(|path| Some(path) != session.working_directory.as_ref())
        {
            return Err(AgentSessionApplicationError::conflict(
                "The session worktree binding cannot change",
            ));
        }
        if session
            .execution_target
            .as_ref()
            .is_some_and(|target| target.execution.is_remote())
            && input_provenance == AgentInvocationInputProvenance::Application
        {
            return Err(AgentSessionApplicationError::invalid(
                "Remote execution is available for ordinary Agent Sessions only",
            ));
        }
        let session = self.repair_missing_runtime_binding(session)?;

        let requested_options = command
            .requested_options
            .clone()
            .unwrap_or_else(|| session.requested_options.clone());
        let existing_invocation = if let Some(invocation_id) = requested_invocation_id.as_ref() {
            if let Some(existing) = self
                .repository
                .get_invocation(invocation_id)
                .map_err(AgentSessionApplicationError::repository)?
            {
                if existing.session_id != session.id
                    || existing.submitted_text != command.submitted_text
                    || existing.input_provenance != input_provenance
                    || existing.requested_options != requested_options
                {
                    return Err(AgentSessionApplicationError::conflict(
                        "application Agent Invocation identity was already used for different semantics",
                    ));
                }
                let launch_accepted = self
                    .repository
                    .invocation_launch_accepted_at(invocation_id)
                    .map_err(AgentSessionApplicationError::repository)?
                    .is_some();
                if launch_accepted
                    || !launch_existing_prepared
                    || existing.status != AgentInvocationStatus::Pending
                {
                    return Ok(SendAgentSessionMessageLaunchResult {
                        acknowledgement: SendAgentSessionMessageResult {
                            session_id: session.id,
                            invocation_id: existing.id,
                        },
                        launch_accepted,
                    });
                }
                Some(existing)
            } else {
                None
            }
        } else {
            None
        };
        if launch_existing_prepared && existing_invocation.is_none() {
            return Err(AgentSessionApplicationError::not_found(
                "prepared application invocation not found",
            ));
        }
        let invocation = match existing_invocation {
            Some(existing) => existing,
            None => {
                let created_at = self.clock.now();
                self.repository
                    .create_pending_invocation(AgentInvocation {
                        id: requested_invocation_id.unwrap_or_else(|| self.ids.invocation_id()),
                        session_id: session.id.clone(),
                        submitted_text: command.submitted_text.clone(),
                        input_provenance,
                        status: AgentInvocationStatus::Pending,
                        requested_options: requested_options.clone(),
                        effective_options: None,
                        started_at: None,
                        completed_at: None,
                        exit_code: None,
                        signal: None,
                        runtime_error: None,
                        diagnostics: Vec::new(),
                        created_at,
                        updated_at: created_at,
                    })
                    .map_err(AgentSessionApplicationError::repository)?
            }
        };
        let acknowledgement = SendAgentSessionMessageResult {
            session_id: session.id.clone(),
            invocation_id: invocation.id.clone(),
        };

        let session = match self.resolve_owned_harness_version(session) {
            Ok(session) => session,
            Err(message) => {
                self.finish_preflight_failure(
                    &invocation,
                    RuntimePortError::new(RuntimePortErrorKind::Unavailable, message),
                )?;
                return Ok(SendAgentSessionMessageLaunchResult {
                    acknowledgement,
                    launch_accepted: false,
                });
            }
        };

        let remote = session
            .execution_target
            .as_ref()
            .is_some_and(|target| target.execution.is_remote());
        let launch_extension = if remote {
            launch_extension
        } else {
            let launch_extension = if let Some(profile) = session.session_profile.as_ref() {
                match super::configuration::pinned_exposure_extension(profile, launch_extension.unwrap_or_default()) {
                    Ok(extension) => Some(extension),
                    Err(message) => {
                        self.finish_preflight_failure(&invocation, RuntimePortError::new(RuntimePortErrorKind::Unavailable, message))?;
                        return Ok(SendAgentSessionMessageLaunchResult { acknowledgement, launch_accepted: false });
                    }
                }
            } else { launch_extension };
            let launch_extension = self.add_workspace_capabilities(launch_extension);
            let launch_extension = match self.native_profile_launch_authority.as_ref() {
                Some(authority) => match authority.prepare_configured_launch(
                    session.execution_target.as_ref()
                        .map(|target| target.execution.configuration_ref.as_str())
                        .or_else(|| session.session_profile.as_ref().and_then(|profile| profile.session_profile().runtime_profile_ref().strip_prefix("native-codex:")))
                        .unwrap_or("selected"),
                    &session.id,
                    &invocation.id,
                    session.runtime_binding.external_context_id.is_some(),
                    launch_extension,
                ) {
                    Ok(extension) => Some(extension),
                    Err(message) => {
                        self.finish_preflight_failure(
                            &invocation,
                            RuntimePortError::new(RuntimePortErrorKind::Unavailable, message),
                        )?;
                        return Ok(SendAgentSessionMessageLaunchResult {
                            acknowledgement,
                            launch_accepted: false,
                        });
                    }
                },
                None => launch_extension,
            };
            let launch_extension = match self.session_harness_launch_authority.as_ref() {
                Some(authority) => {
                    match authority.prepare_launch(&session.id, &invocation.id, launch_extension) {
                        Ok(extension) => extension,
                        Err(message) => {
                            self.finish_preflight_failure(
                                &invocation,
                                RuntimePortError::new(RuntimePortErrorKind::Unavailable, message),
                            )?;
                            return Ok(SendAgentSessionMessageLaunchResult {
                                acknowledgement,
                                launch_accepted: false,
                            });
                        }
                    }
                }
                None => launch_extension,
            };

            launch_extension
        };

        let mode = if session.runtime_binding.external_context_id.is_some() {
            RuntimeInvocationMode::Resume
        } else {
            RuntimeInvocationMode::Start
        };
        let runtime = match self.session_runtime(&session) {
            Ok(runtime) => runtime,
            Err(error) => {
                self.finish_preflight_failure(
                    &invocation,
                    RuntimePortError::new(RuntimePortErrorKind::Unavailable, error.to_string()),
                )?;
                return Ok(SendAgentSessionMessageLaunchResult {
                    acknowledgement,
                    launch_accepted: false,
                });
            }
        };
        let preflight = match runtime.preflight_invocation(mode, &requested_options) {
            Ok(preflight) => preflight,
            Err(error) => {
                self.finish_preflight_failure(&invocation, error)?;
                return Ok(SendAgentSessionMessageLaunchResult {
                    acknowledgement,
                    launch_accepted: false,
                });
            }
        };

        let started_at = self.clock.now();
        self.repository
            .mark_invocation_running(
                &invocation.id,
                started_at,
                preflight.effective_options.clone(),
                started_at,
            )
            .map_err(AgentSessionApplicationError::repository)?;

        let request = RuntimeInvocationRequest {
            session_id: session.id.clone(),
            invocation_id: invocation.id.clone(),
            submitted_text: launch_extension
                .as_ref()
                .and_then(|extension| extension.initial_prompt_prefix.as_ref())
                .map(|context| context.render_before_user_query(&invocation.submitted_text))
                .unwrap_or_else(|| invocation.submitted_text.clone()),
            // Product-specific callers may select a neutral per-invocation discovery root without
            // mutating the provider-neutral Agent Session identity. Ordinary sends continue to
            // inherit the durable session directory because they do not supply an override.
            working_directory: command
                .working_directory
                .clone()
                .or_else(|| session.working_directory.clone()),
            options: preflight.effective_options,
            launch_extension,
        };
        let sink: Arc<dyn AgentRuntimeUpdateSink> = Arc::new(PersistedRuntimeUpdateSink::new(
            self.repository.clone(),
            self.notifier.clone(),
            self.clock.clone(),
            self.ids.clone(),
            self.update_lanes.clone(),
        ));
        let launch = match session.runtime_binding.external_context_id {
            Some(external_context_id) => {
                runtime.resume_invocation(request, external_context_id, sink)
            }
            None => runtime.start_invocation(request, sink),
        };
        let launch_accepted = match launch {
            Ok(()) => match self
                .repository
                .record_invocation_launch_accepted(&invocation.id, self.clock.now())
            {
                Ok(()) => true,
                Err(error) => {
                    self.handle_launch_acceptance_persistence_failure(&invocation.id, error)?;
                    false
                }
            },
            Err(error) => {
                self.handle_launch_error(&invocation.id, error)?;
                false
            }
        };
        Ok(SendAgentSessionMessageLaunchResult {
            acknowledgement,
            launch_accepted,
        })
    }

    fn resolve_owned_harness_version(&self, session: AgentSession) -> Result<AgentSession, String> {
        let Some(requested) = session.harness_version.as_ref() else {
            return Ok(session);
        };
        let resolver = self
            .session_harness_version_resolver
            .as_ref()
            .ok_or_else(|| {
                "Session has a Harness reference, but Harness version resolution is unavailable."
                    .to_string()
            })?;
        let resolved = resolver.resolve_session_harness_version(&session.id, requested)?;
        if &resolved == requested {
            return Ok(session);
        }
        self.repository
            .update_harness_version(&session.id, Some(resolved), self.clock.now())
            .map_err(|error| format!("Unable to persist resolved Session Harness: {error}"))
    }

    pub(super) fn finish_preflight_failure(
        &self,
        invocation: &AgentInvocation,
        error: RuntimePortError,
    ) -> Result<(), AgentSessionApplicationError> {
        let completed_at = self.clock.now();
        let updated = self
            .repository
            .finish_invocation(
                &invocation.id,
                InvocationCompletion {
                    status: AgentInvocationTerminalStatus::Failed,
                    completed_at,
                    exit_code: None,
                    signal: None,
                    runtime_error: Some(AgentRuntimeFailure {
                        code: "runtime_preflight_failed".to_string(),
                        message: error.message.clone(),
                        details: serde_json::to_value(&error).ok(),
                    }),
                },
                completed_at,
            )
            .map_err(AgentSessionApplicationError::repository)?;
        self.notify_or_record(AgentSessionNotification::InvocationTerminal {
            session_id: invocation.session_id.clone(),
            invocation: updated,
        });
        Ok(())
    }

    fn handle_launch_error(
        &self,
        invocation_id: &AgentInvocationId,
        error: RuntimePortError,
    ) -> Result<(), AgentSessionApplicationError> {
        let invocation = self
            .repository
            .get_invocation(invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent invocation not found"))?;
        if invocation.status.is_active() {
            let completed_at = self.clock.now();
            let updated = self
                .repository
                .finish_invocation(
                    invocation_id,
                    InvocationCompletion {
                        status: AgentInvocationTerminalStatus::Failed,
                        completed_at,
                        exit_code: None,
                        signal: None,
                        runtime_error: Some(AgentRuntimeFailure {
                            code: "runtime_launch_failed".to_string(),
                            message: error.message.clone(),
                            details: serde_json::to_value(&error).ok(),
                        }),
                    },
                    completed_at,
                )
                .map_err(AgentSessionApplicationError::repository)?;
            self.update_lanes.remove_invocation(invocation_id);
            self.notify_or_record(AgentSessionNotification::InvocationTerminal {
                session_id: updated.session_id.clone(),
                invocation: updated,
            });
        }
        self.record_diagnostic(
            invocation_id,
            AgentDiagnosticSource::Runtime,
            "runtime_launch_returned_error",
            error.message.clone(),
            serde_json::to_value(&error).ok(),
        );
        Ok(())
    }

    fn handle_launch_acceptance_persistence_failure(
        &self,
        invocation_id: &AgentInvocationId,
        error: RepositoryError,
    ) -> Result<(), AgentSessionApplicationError> {
        let invocation = self
            .repository
            .get_invocation(invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Agent invocation not found"))?;
        let terminal = if invocation.status.is_active() {
            let completed_at = self.clock.now();
            let updated = self
                .repository
                .finish_invocation(
                    invocation_id,
                    InvocationCompletion {
                        status: AgentInvocationTerminalStatus::Failed,
                        completed_at,
                        exit_code: None,
                        signal: None,
                        runtime_error: Some(AgentRuntimeFailure {
                            code: "runtime_launch_acceptance_persistence_failed".to_string(),
                            message: error.message.clone(),
                            details: Some(json!({"repositoryError": error.message.clone()})),
                        }),
                    },
                    completed_at,
                )
                .map_err(AgentSessionApplicationError::repository)?;
            self.update_lanes.remove_invocation(invocation_id);
            Some(updated)
        } else {
            None
        };
        if let Err(cancel_error) = self
            .runtime_for_invocation(invocation_id)?
            .cancel_invocation(invocation_id)
        {
            self.record_diagnostic(
                invocation_id,
                AgentDiagnosticSource::Runtime,
                "runtime_cancellation_after_launch_acceptance_persistence_failure",
                cancel_error.message.clone(),
                serde_json::to_value(&cancel_error).ok(),
            );
        }
        if let Some(updated) = terminal {
            self.notify_or_record(AgentSessionNotification::InvocationTerminal {
                session_id: updated.session_id.clone(),
                invocation: updated,
            });
        }
        self.record_diagnostic(
            invocation_id,
            AgentDiagnosticSource::Repository,
            "runtime_launch_acceptance_persistence_failed",
            error.message.clone(),
            Some(json!({"repositoryError": error.message.clone()})),
        );
        Ok(())
    }
}
