use super::lifecycle::{
    AgentSessionApplication, AgentSessionApplicationError, AgentSessionOwnership,
    CreateAgentSessionCommand, SendAgentSessionMessageCommand, SendAgentSessionMessageResult,
};
use crate::{
    agent_sessions::{
        domain::{AgentRuntimeOptions, AgentSessionId, RuntimeSandboxMode},
        ports::RuntimeLaunchExtension,
    },
    execution_configuration::{
        CapabilityProfile, DirectUserInvocationRequest, DirectUserInvocationResolution,
        NodeProfile, ResolutionError, RuntimeSelections, SandboxMode, SelectedRuntimeProfileSource,
        SessionCreationRequest, SessionCreationResolution, SessionProfileResolver,
    },
};
use std::{error::Error, fmt, sync::Arc};

/// Read request for the immutable execution configuration stored with one Agent Session.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct LoadPinnedSessionProfileQuery {
    pub(crate) session_id: AgentSessionId,
}

/// Exact creation resolution persisted with the Session. The creation wrapper retains the
/// integrity digest as well as the resolved Session Profile it protects.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct PinnedAgentSessionProfile {
    pub(crate) session_id: AgentSessionId,
    pub(crate) creation_resolution: SessionCreationResolution,
}

/// User-owned choices for one message. They are not Session configuration updates.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SendDirectUserAgentSessionMessageCommand {
    pub(crate) session_id: AgentSessionId,
    pub(crate) submitted_text: String,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_mode: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SendDirectUserAgentSessionMessageResult {
    pub(crate) acknowledgement: SendAgentSessionMessageResult,
    /// The resolved per-message choices, including inherited defaults and runtime locks.
    pub(crate) invocation_resolution: DirectUserInvocationResolution,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum AgentSessionProfileApplicationErrorKind {
    AgentSession,
    MissingPinnedProfile,
    InvalidInvocationSelection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct AgentSessionProfileApplicationError {
    pub(crate) kind: AgentSessionProfileApplicationErrorKind,
    pub(crate) message: String,
}

impl fmt::Display for AgentSessionProfileApplicationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for AgentSessionProfileApplicationError {}

/// Agent Session application surface for pinned configuration reads and direct-user invocation
/// choices. Workflow-owned messages continue to enter through the Session Event adapter.
pub(crate) struct AgentSessionProfileApplication {
    sessions: Arc<AgentSessionApplication>,
    profile_source: Arc<dyn SelectedRuntimeProfileSource>,
}

impl AgentSessionProfileApplication {
    /// Standalone defaults are application-owned, not a hidden Workflow node. Resolve before
    /// persistence, and keep the first message's choices out of the pinned defaults.
    pub(crate) fn start_direct_user_session(
        &self,
        submitted_text: String,
        title: Option<String>,
        working_directory: Option<String>,
        model: Option<String>,
        reasoning_mode: Option<String>,
    ) -> Result<SendDirectUserAgentSessionMessageResult, AgentSessionProfileApplicationError> {
        if submitted_text.trim().is_empty() {
            return Err(AgentSessionProfileApplicationError::new(
                AgentSessionProfileApplicationErrorKind::InvalidInvocationSelection,
                "A message must contain text",
            ));
        }
        let runtime = self
            .profile_source
            .selected_runtime_profile()
            .map_err(|error| AgentSessionProfileApplicationError::resolution(error.into()))?;
        let resolution = SessionProfileResolver::resolve_snapshot(
            runtime.clone(),
            SessionCreationRequest {
                contract_version: 1,
                capability_profile: CapabilityProfile {
                    contract_version: 1,
                    capability_profile_id: "application:standalone".into(),
                    name: "Selected runtime defaults".into(),
                    revision: 1,
                    allowed_capabilities: runtime.exposure.clone(),
                },
                node_profile: NodeProfile {
                    contract_version: 1,
                    allowed_capabilities: runtime.exposure,
                    pinned_defaults: runtime.locked,
                },
                agent_mcp_configuration: Default::default(),
            },
        )
        .map_err(AgentSessionProfileApplicationError::resolution)?;
        // Reject an invalid first-message selection before creating any Session.
        SessionProfileResolver::validate_direct_user_invocation(
            self.profile_source.as_ref(),
            &resolution,
            DirectUserInvocationRequest {
                contract_version: 1,
                model: model.clone(),
                reasoning_mode: reasoning_mode.clone(),
            },
        )
        .map_err(AgentSessionProfileApplicationError::resolution)?;
        let session = self
            .sessions
            .create_session_with_ownership(
                CreateAgentSessionCommand {
                    title,
                    working_directory,
                    requested_options: runtime_options(
                        resolution.session_profile().pinned_defaults(),
                    ),
                },
                AgentSessionOwnership {
                    session_profile: Some(resolution),
                    ..AgentSessionOwnership::default()
                },
            )
            .map_err(AgentSessionProfileApplicationError::agent_session)?;
        self.send_direct_user_message(SendDirectUserAgentSessionMessageCommand {
            session_id: session.id,
            submitted_text,
            model,
            reasoning_mode,
        })
    }

    pub(crate) fn new(
        sessions: Arc<AgentSessionApplication>,
        profile_source: Arc<dyn SelectedRuntimeProfileSource>,
    ) -> Self {
        Self {
            sessions,
            profile_source,
        }
    }

    pub(crate) fn load_pinned_session_profile(
        &self,
        query: LoadPinnedSessionProfileQuery,
    ) -> Result<PinnedAgentSessionProfile, AgentSessionProfileApplicationError> {
        let history = self
            .sessions
            .load_session(&query.session_id)
            .map_err(AgentSessionProfileApplicationError::agent_session)?;
        let creation_resolution = history.session.session_profile.ok_or_else(|| {
            AgentSessionProfileApplicationError::new(
                AgentSessionProfileApplicationErrorKind::MissingPinnedProfile,
                format!(
                    "Agent Session `{}` has no pinned Session Profile",
                    query.session_id
                ),
            )
        })?;
        Ok(PinnedAgentSessionProfile {
            session_id: query.session_id,
            creation_resolution,
        })
    }

    pub(crate) fn send_direct_user_message(
        &self,
        command: SendDirectUserAgentSessionMessageCommand,
    ) -> Result<SendDirectUserAgentSessionMessageResult, AgentSessionProfileApplicationError> {
        let pinned = self.load_pinned_session_profile(LoadPinnedSessionProfileQuery {
            session_id: command.session_id.clone(),
        })?;
        let invocation_resolution = SessionProfileResolver::validate_direct_user_invocation(
            self.profile_source.as_ref(),
            &pinned.creation_resolution,
            DirectUserInvocationRequest {
                contract_version: 1,
                model: command.model,
                reasoning_mode: command.reasoning_mode,
            },
        )
        .map_err(AgentSessionProfileApplicationError::resolution)?;
        let requested_options = runtime_options(&invocation_resolution.selections);
        let launch_extension = reasoning_launch_extension(&invocation_resolution.selections);
        let acknowledgement = self
            .sessions
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
            .map_err(AgentSessionProfileApplicationError::agent_session)?;
        Ok(SendDirectUserAgentSessionMessageResult {
            acknowledgement,
            invocation_resolution,
        })
    }
}

impl AgentSessionProfileApplicationError {
    fn new(kind: AgentSessionProfileApplicationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn agent_session(error: AgentSessionApplicationError) -> Self {
        Self::new(
            AgentSessionProfileApplicationErrorKind::AgentSession,
            error.to_string(),
        )
    }

    fn resolution(error: ResolutionError) -> Self {
        Self::new(
            AgentSessionProfileApplicationErrorKind::InvalidInvocationSelection,
            error.to_string(),
        )
    }
}

pub(crate) fn runtime_options(selections: &RuntimeSelections) -> AgentRuntimeOptions {
    AgentRuntimeOptions {
        model: selections.model.clone(),
        sandbox: selections.sandbox_mode.map(|sandbox| match sandbox {
            SandboxMode::ReadOnly => RuntimeSandboxMode::ReadOnly,
            SandboxMode::WorkspaceWrite => RuntimeSandboxMode::WorkspaceWrite,
            SandboxMode::DangerFullAccess => RuntimeSandboxMode::DangerFullAccess,
        }),
    }
}

pub(crate) fn reasoning_launch_extension(
    selections: &RuntimeSelections,
) -> Option<RuntimeLaunchExtension> {
    selections.reasoning_mode.as_ref().map(|reasoning| {
        let mut extension = RuntimeLaunchExtension::default();
        extension.additional_args = vec![
            "-c".to_string(),
            format!("model_reasoning_effort=\"{reasoning}\""),
        ];
        extension
    })
}
