//! Profiled direct-user creation and message execution.
use super::configuration::{
    reasoning_launch_extension, runtime_options, LoadPinnedSessionProfileQuery,
    SendDirectUserAgentSessionMessageCommand, SendDirectUserAgentSessionMessageResult,
    SessionConfigurationError, SessionConfigurationErrorKind,
};
use super::{
    AgentSessionApplication, AgentSessionApplicationError, AgentSessionOwnership,
    CreateAgentSessionCommand, SendAgentSessionMessageCommand,
};
use crate::execution_configuration::{
    validate_session_skill_inputs, DirectUserInvocationRequest, DirectUserInvocationResolution,
    NodeProfile, SandboxMode, SessionCreationRequest, SessionProfileResolver,
};
impl AgentSessionApplication {
    /// Standalone defaults are application-owned, not a hidden Workflow node. Resolve before
    /// persistence, and keep the first message's choices out of the pinned defaults.
    pub(crate) fn start_direct_user_session(
        &self,
        submitted_text: String,
        title: Option<String>,
        working_directory: Option<String>,
        model: Option<String>,
        reasoning_mode: Option<String>,
        sandbox_mode: Option<SandboxMode>,
        placement: Option<crate::agent_sessions::organization::SessionPlacement>,
    ) -> Result<SendDirectUserAgentSessionMessageResult, SessionConfigurationError> {
        self.start_direct_user_session_with_target(
            submitted_text,
            title,
            working_directory,
            model,
            reasoning_mode,
            sandbox_mode,
            None,
            placement,
        )
    }

    pub(crate) fn start_direct_user_session_with_target(
        &self,
        submitted_text: String,
        title: Option<String>,
        working_directory: Option<String>,
        model: Option<String>,
        reasoning_mode: Option<String>,
        sandbox_mode: Option<SandboxMode>,
        mut target: Option<crate::execution_targets::domain::SessionExecutionTarget>,
        placement: Option<crate::agent_sessions::organization::SessionPlacement>,
    ) -> Result<SendDirectUserAgentSessionMessageResult, SessionConfigurationError> {
        if submitted_text.trim().is_empty() {
            return Err(SessionConfigurationError::new(
                SessionConfigurationErrorKind::InvalidInvocationSelection,
                "A message must contain text",
            ));
        }
        let (runtime, resolution) = if let Some(target) = target.as_mut() {
            let service = self.capability_profiles.as_ref().ok_or_else(|| {
                SessionConfigurationError::new(
                    SessionConfigurationErrorKind::MissingPinnedProfile,
                    "Capability Profiles are unavailable",
                )
            })?;
            let capability = service
                .resolve_draft_selection(&target.capability_profile_id, &target.execution)
                .map_err(|e| {
                    SessionConfigurationError::new(
                        SessionConfigurationErrorKind::InvalidInvocationSelection,
                        e.to_string(),
                    )
                })?;
            target.capability_profile_revision = capability.revision;
            let endpoints = self.endpoints.as_ref().ok_or_else(|| {
                SessionConfigurationError::new(
                    SessionConfigurationErrorKind::InvalidInvocationSelection,
                    "Execution endpoints are unavailable",
                )
            })?;
            target.execution = endpoints
                .freeze_binding(target.execution.clone())
                .map_err(|e| {
                    SessionConfigurationError::new(
                        SessionConfigurationErrorKind::InvalidInvocationSelection,
                        e,
                    )
                })?;
            let runtime = service
                .runtime_for_binding(&target.execution, Some(&target.path))
                .map_err(|e| {
                    SessionConfigurationError::new(
                        SessionConfigurationErrorKind::InvalidInvocationSelection,
                        e.to_string(),
                    )
                })?;
            let session_skill_inputs = self
                .compile_capability_skill_inputs(
                    &capability,
                    &target.execution.configuration_ref,
                    Some(&target.path),
                )
                .map_err(|error| {
                    SessionConfigurationError::new(
                        SessionConfigurationErrorKind::InvalidInvocationSelection,
                        error,
                    )
                })?;
            let resolution = SessionProfileResolver::resolve_snapshot(
                runtime.clone(),
                SessionCreationRequest {
                    contract_version: 1,
                    agent_mcp_configuration: Default::default(),
                    node_profile: NodeProfile {
                        contract_version: 1,
                        allowed_capabilities: super::configuration::default_node_capabilities(
                            &capability,
                        ),
                        pinned_defaults: Default::default(),
                    },
                    capability_profile: capability,
                    session_skill_inputs,
                },
            )
            .map_err(SessionConfigurationError::resolution)?;
            (runtime, resolution)
        } else {
            self.resolve_default_creation(working_directory.as_deref())?
        };
        // Reject an invalid first-message selection before creating any Session.
        let invocation_resolution = SessionProfileResolver::resolve_direct_user_snapshot(
            runtime,
            &resolution,
            DirectUserInvocationRequest {
                contract_version: 1,
                model: model.clone(),
                reasoning_mode: reasoning_mode.clone(),
                sandbox_mode,
            },
        )
        .map_err(SessionConfigurationError::resolution)?;
        let pinned_skill_inputs = resolution.session_profile().session_skill_inputs().to_vec();
        let session = self
            .prepare_session_with_id(
                CreateAgentSessionCommand {
                    title,
                    working_directory,
                    requested_options: runtime_options(
                        resolution.session_profile().pinned_defaults(),
                    ),
                },
                self.ids.session_id(),
                AgentSessionOwnership {
                    execution_target: target,
                    session_profile: Some(resolution),
                    ..AgentSessionOwnership::default()
                },
            )
            .map_err(SessionConfigurationError::agent_session)?;
        let session = match placement {
            Some(placement) => self
                .repository
                .create_session_with_placement(session, &placement),
            None => self.repository.create_session(session),
        }
        .map_err(AgentSessionApplicationError::repository)
        .map_err(SessionConfigurationError::agent_session)?;
        self.send_resolved_direct_user_message(
            SendDirectUserAgentSessionMessageCommand {
                session_id: session.id,
                submitted_text,
                model,
                reasoning_mode,
                sandbox_mode,
            },
            invocation_resolution,
            &pinned_skill_inputs,
        )
    }

    pub(crate) fn send_direct_user_message(
        &self,
        command: SendDirectUserAgentSessionMessageCommand,
    ) -> Result<SendDirectUserAgentSessionMessageResult, SessionConfigurationError> {
        let pinned = self.load_pinned_session_profile(LoadPinnedSessionProfileQuery {
            session_id: command.session_id.clone(),
        })?;
        let history = self
            .load_session(&command.session_id)
            .map_err(SessionConfigurationError::agent_session)?;
        let runtime = if let Some(target) = &history.session.execution_target {
            self.capability_profiles
                .as_ref()
                .ok_or_else(|| {
                    SessionConfigurationError::new(
                        SessionConfigurationErrorKind::MissingPinnedProfile,
                        "Capability Profiles are unavailable",
                    )
                })?
                .runtime_for_binding(&target.execution, Some(&target.path))
                .map_err(|e| {
                    SessionConfigurationError::new(
                        SessionConfigurationErrorKind::InvalidInvocationSelection,
                        e.to_string(),
                    )
                })?
        } else {
            self.profile_source()?
                .selected_runtime_profile_at(history.session.working_directory.as_deref())
                .map_err(|e| SessionConfigurationError::resolution(e.into()))?
        };
        let invocation_resolution = SessionProfileResolver::resolve_direct_user_snapshot(
            runtime,
            &pinned.creation_resolution,
            DirectUserInvocationRequest {
                contract_version: 1,
                model: command.model.clone(),
                reasoning_mode: command.reasoning_mode.clone(),
                sandbox_mode: command.sandbox_mode,
            },
        )
        .map_err(SessionConfigurationError::resolution)?;
        self.send_resolved_direct_user_message(
            command,
            invocation_resolution,
            pinned
                .creation_resolution
                .session_profile()
                .session_skill_inputs(),
        )
    }

    fn send_resolved_direct_user_message(
        &self,
        command: SendDirectUserAgentSessionMessageCommand,
        invocation_resolution: DirectUserInvocationResolution,
        session_skill_inputs: &[crate::agent_sessions::ports::RuntimeSkillInput],
    ) -> Result<SendDirectUserAgentSessionMessageResult, SessionConfigurationError> {
        let requested_options = runtime_options(&invocation_resolution.selections);
        let selected_skills =
            validate_session_skill_inputs(session_skill_inputs).map_err(|error| {
                SessionConfigurationError::new(
                    SessionConfigurationErrorKind::InvalidInvocationSelection,
                    error,
                )
            })?;
        let mut launch_extension =
            reasoning_launch_extension(&invocation_resolution.selections).unwrap_or_default();
        launch_extension.skill_inputs = selected_skills;
        if let Ok(history) = self.load_session(&command.session_id) {
            self.apply_skill_mentions(
                &super::configuration::session_configuration(&history.session),
                history.session.working_directory.as_deref(),
                &command.submitted_text,
                history.session.session_profile.is_some(),
                &mut launch_extension,
            );
        }
        let acknowledgement = self
            .send_message_with_launch_extension(
                SendAgentSessionMessageCommand {
                    session_id: Some(command.session_id),
                    submitted_text: command.submitted_text,
                    title: None,
                    working_directory: None,
                    requested_options: Some(requested_options),
                },
                Some(launch_extension),
            )
            .map_err(SessionConfigurationError::agent_session)?;
        Ok(SendDirectUserAgentSessionMessageResult {
            acknowledgement,
            invocation_resolution,
        })
    }
}
