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
    DirectUserInvocationRequest, DirectUserInvocationResolution, SandboxMode,
    SessionProfileResolver,
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
        if submitted_text.trim().is_empty() {
            return Err(SessionConfigurationError::new(
                SessionConfigurationErrorKind::InvalidInvocationSelection,
                "A message must contain text",
            ));
        }
        let (runtime, resolution) = self.resolve_default_creation(working_directory.as_deref())?;
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
        let source = crate::execution_configuration::WorkingContextProfileSource {
            source: self.profile_source()?,
            cwd: history.session.working_directory.as_deref(),
        };
        let invocation_resolution = SessionProfileResolver::validate_direct_user_invocation(
            &source,
            &pinned.creation_resolution,
            DirectUserInvocationRequest {
                contract_version: 1,
                model: command.model.clone(),
                reasoning_mode: command.reasoning_mode.clone(),
                sandbox_mode: command.sandbox_mode,
            },
        )
        .map_err(SessionConfigurationError::resolution)?;
        self.send_resolved_direct_user_message(command, invocation_resolution)
    }

    fn send_resolved_direct_user_message(
        &self,
        command: SendDirectUserAgentSessionMessageCommand,
        invocation_resolution: DirectUserInvocationResolution,
    ) -> Result<SendDirectUserAgentSessionMessageResult, SessionConfigurationError> {
        let requested_options = runtime_options(&invocation_resolution.selections);
        let launch_extension = reasoning_launch_extension(&invocation_resolution.selections);
        let acknowledgement = self
            .send_message_with_launch_extension(
                SendAgentSessionMessageCommand {
                    session_id: Some(command.session_id),
                    submitted_text: command.submitted_text,
                    title: None,
                    working_directory: None,
                    requested_options: Some(requested_options),
                },
                launch_extension,
            )
            .map_err(SessionConfigurationError::agent_session)?;
        Ok(SendDirectUserAgentSessionMessageResult {
            acknowledgement,
            invocation_resolution,
        })
    }
}
