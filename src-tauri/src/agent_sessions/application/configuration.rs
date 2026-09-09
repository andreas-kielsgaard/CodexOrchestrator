//! Converts resolved execution choices to provider-neutral invocation inputs.

use crate::agent_sessions::{
    domain::{AgentRuntimeOptions, RuntimeSandboxMode},
    ports::RuntimeLaunchExtension,
};
use crate::execution_configuration::{RuntimeSelections, SandboxMode};

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
    selections
        .reasoning_mode
        .as_ref()
        .map(|reasoning| RuntimeLaunchExtension {
            managed_mcp_servers: Vec::new(),
            skill_roots: Vec::new(),
            ignore_user_rules: false,
            reasoning_mode: Some(reasoning.clone()),
            ..RuntimeLaunchExtension::default()
        })
}

use super::{
    AgentSessionApplication, AgentSessionApplicationError, AgentSessionOwnership,
    CreateAgentSessionCommand, SendAgentSessionMessageCommand, SendAgentSessionMessageResult,
};
use crate::{
    agent_sessions::domain::AgentSessionId,
    execution_configuration::{
        DirectUserInvocationRequest, DirectUserInvocationResolution, NodeProfile, ResolutionError,
        SelectedRuntimeProfileSource, SessionCreationRequest, SessionCreationResolution,
        SessionProfileResolver,
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
    pub(crate) sandbox_mode: Option<SandboxMode>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SendDirectUserAgentSessionMessageResult {
    pub(crate) acknowledgement: SendAgentSessionMessageResult,
    /// The resolved per-message choices, including inherited defaults and runtime locks.
    pub(crate) invocation_resolution: DirectUserInvocationResolution,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SessionConfigurationErrorKind {
    AgentSession,
    MissingPinnedProfile,
    InvalidInvocationSelection,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SessionConfigurationError {
    pub(crate) kind: SessionConfigurationErrorKind,
    pub(crate) message: String,
}

impl fmt::Display for SessionConfigurationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SessionConfigurationError {}

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

    pub(crate) fn create_default_session(
        &self,
        mut command: CreateAgentSessionCommand,
        mut ownership: AgentSessionOwnership,
    ) -> Result<crate::agent_sessions::domain::AgentSession, SessionConfigurationError> {
        if command.requested_options != AgentRuntimeOptions::default() {
            return Err(SessionConfigurationError::new(SessionConfigurationErrorKind::InvalidInvocationSelection, "Session creation uses the default Capability Profile. Submit runtime choices with the first message."));
        }
        let working_directory = command.working_directory.clone();
        let (_, resolution) = self.resolve_default_creation(working_directory.as_deref())?;
        command.requested_options = runtime_options(resolution.session_profile().pinned_defaults());
        ownership.session_profile = Some(resolution);
        self.create_session_with_ownership(command, ownership)
            .map_err(SessionConfigurationError::agent_session)
    }

    fn resolve_default_creation(
        &self,
        working_directory: Option<&str>,
    ) -> Result<
        (
            crate::execution_configuration::RuntimeProfileSnapshot,
            SessionCreationResolution,
        ),
        SessionConfigurationError,
    > {
        let capability = self.capability_profiles.as_ref().ok_or_else(|| SessionConfigurationError::new(SessionConfigurationErrorKind::MissingPinnedProfile, "Choose a default Capability Profile in Capabilities before starting a new session"))?
            .default_profile().map_err(|error| SessionConfigurationError::new(SessionConfigurationErrorKind::MissingPinnedProfile, error.to_string()))?;
        let runtime = self
            .profile_source()?
            .selected_runtime_profile_at(working_directory)
            .map_err(|error| SessionConfigurationError::resolution(error.into()))?;
        let resolution = SessionProfileResolver::resolve_snapshot(
            runtime.clone(),
            SessionCreationRequest {
                contract_version: 1,
                node_profile: NodeProfile {
                    contract_version: 1,
                    allowed_capabilities: capability.allowed_capabilities.clone(),
                    pinned_defaults: Default::default(),
                },
                capability_profile: capability,
            },
        )
        .map_err(SessionConfigurationError::resolution)?;
        Ok((runtime, resolution))
    }

    pub(crate) fn with_capability_profiles(
        mut self,
        profiles: Arc<crate::execution_configuration::CapabilityProfileService>,
    ) -> Self {
        self.capability_profiles = Some(profiles);
        self
    }

    pub(crate) fn with_profile_source(
        mut self,
        source: Arc<dyn SelectedRuntimeProfileSource>,
    ) -> Self {
        self.profile_source = Some(source);
        self
    }

    pub(super) fn profile_source(
        &self,
    ) -> Result<&dyn SelectedRuntimeProfileSource, SessionConfigurationError> {
        self.profile_source.as_deref().ok_or_else(|| {
            SessionConfigurationError::new(
                SessionConfigurationErrorKind::MissingPinnedProfile,
                "No runtime profile source is configured",
            )
        })
    }

    pub(crate) fn load_pinned_session_profile(
        &self,
        query: LoadPinnedSessionProfileQuery,
    ) -> Result<PinnedAgentSessionProfile, SessionConfigurationError> {
        let history = self
            .load_session(&query.session_id)
            .map_err(SessionConfigurationError::agent_session)?;
        let creation_resolution = history.session.session_profile.ok_or_else(|| {
            SessionConfigurationError::new(
                SessionConfigurationErrorKind::MissingPinnedProfile,
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

impl SessionConfigurationError {
    fn new(kind: SessionConfigurationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    fn agent_session(error: AgentSessionApplicationError) -> Self {
        Self::new(
            SessionConfigurationErrorKind::AgentSession,
            error.to_string(),
        )
    }

    fn resolution(error: ResolutionError) -> Self {
        Self::new(
            SessionConfigurationErrorKind::InvalidInvocationSelection,
            error.to_string(),
        )
    }
}
