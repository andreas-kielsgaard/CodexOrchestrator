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
    CreateAgentSessionCommand, SendAgentSessionMessageResult,
};
use crate::{
    agent_sessions::domain::AgentSessionId,
    execution_configuration::{
        DirectUserInvocationResolution, NodeProfile, ResolutionError, SelectedRuntimeProfileSource,
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
    pub(crate) fn create_default_session(
        &self,
        command: CreateAgentSessionCommand,
        ownership: AgentSessionOwnership,
    ) -> Result<crate::agent_sessions::domain::AgentSession, SessionConfigurationError> {
        if command.requested_options != AgentRuntimeOptions::default() {
            return Err(SessionConfigurationError::new(SessionConfigurationErrorKind::InvalidInvocationSelection, "Session creation uses the default Capability Profile. Submit runtime choices with the first message."));
        }
        let session = self.prepare_default_session(self.ids.session_id(), command, ownership)?;
        self.repository.create_session(session).map_err(|e| {
            SessionConfigurationError::agent_session(AgentSessionApplicationError::repository(e))
        })
    }

    pub(crate) fn prepare_default_session(
        &self,
        id: AgentSessionId,
        mut command: CreateAgentSessionCommand,
        mut ownership: AgentSessionOwnership,
    ) -> Result<crate::agent_sessions::domain::AgentSession, SessionConfigurationError> {
        let (_, resolution) =
            self.resolve_default_creation(command.working_directory.as_deref())?;
        command.requested_options = runtime_options(resolution.session_profile().pinned_defaults());
        ownership.session_profile = Some(resolution);
        self.prepare_session_with_id(command, id, ownership)
            .map_err(SessionConfigurationError::agent_session)
    }

    pub(crate) fn resolve_default_creation(
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
}

impl SessionConfigurationError {
    pub(super) fn new(kind: SessionConfigurationErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }

    pub(super) fn agent_session(error: AgentSessionApplicationError) -> Self {
        Self::new(
            SessionConfigurationErrorKind::AgentSession,
            error.to_string(),
        )
    }

    pub(super) fn resolution(error: ResolutionError) -> Self {
        Self::new(
            SessionConfigurationErrorKind::InvalidInvocationSelection,
            error.to_string(),
        )
    }
}
