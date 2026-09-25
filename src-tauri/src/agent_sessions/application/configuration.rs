//! Converts resolved execution choices to provider-neutral invocation inputs.

use crate::agent_sessions::{
    domain::{AgentRuntimeOptions, RuntimeSandboxMode},
    ports::{InitialPromptPrefix, RuntimeLaunchExtension},
};
use crate::execution_configuration::{
    compile_session_skill_inputs, skill_mentions, CapabilityProfile, RuntimeSelections,
    SandboxMode,
};
use orchid_engine::contracts::ProviderConfigurationRef;

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
            skill_inputs: Vec::new(),
            ignore_user_rules: false,
            reasoning_mode: Some(reasoning.clone()),
            ..RuntimeLaunchExtension::default()
        })
}

pub(super) fn pinned_exposure_extension(
    profile: &crate::execution_configuration::SessionCreationResolution,
    mut extension: RuntimeLaunchExtension,
) -> Result<RuntimeLaunchExtension, String> {
    let pinned = profile.session_profile();
    extension.native_mcp_enabled = pinned.native_mcp_enabled();
    extension.provider_options = pinned.provider_options().cloned();
    let skills = crate::execution_configuration::validate_session_skill_inputs(
        pinned.session_skill_inputs(),
    )?;
    if !skills.is_empty() {
        let context = skill_manifest_context(&skills);
        match extension.initial_prompt_prefix.as_mut() {
            Some(prefix) => prefix.content = format!("{}

{}", prefix.content, context),
            None => {
                extension.initial_prompt_prefix = Some(InitialPromptPrefix {
                    source: "session_skill_manifest".into(),
                    version: 1,
                    content: context,
                })
            }
        }
    }
    extension.skill_inputs = skills;
    Ok(extension)
}

/// Provider-independent manifest guidance. `read_skill` serves every pinned skill whatever its
/// source; a provider may additionally attach an invoked skill in its native form.
fn skill_manifest_context(skills: &[crate::agent_sessions::ports::RuntimeSkillInput]) -> String {
    let manifest = skills
        .iter()
        .map(|skill| format!("- {}: {}", skill.name, skill.description))
        .collect::<Vec<_>>()
        .join("
");
    format!(
        "Skills available to this session (not invoked automatically). A $name mention refers to the skill with that name; read its instructions with read_skill unless they are already attached:
{manifest}"
    )
}

/// The provider configuration a Session executes with: its execution target, otherwise the
/// configuration its profile pinned. Unbound legacy Sessions predate provider identity and ran
/// only on the default execution binding.
pub(super) fn session_configuration(
    session: &crate::agent_sessions::domain::AgentSession,
) -> ProviderConfigurationRef {
    session
        .execution_target
        .as_ref()
        .map(|target| target.execution.configuration())
        .or_else(|| {
            session
                .session_profile
                .as_ref()
                .map(|profile| profile.session_profile().configuration().clone())
        })
        .unwrap_or_else(|| crate::execution_targets::domain::ExecutionBinding::default().configuration())
}

pub(super) fn default_node_capabilities(
    capability: &CapabilityProfile,
) -> crate::execution_configuration::CapabilitySet {
    if capability.route_policies.is_empty() {
        capability.allowed_capabilities.clone()
    } else {
        Default::default()
    }
}

use super::{
    AgentSessionApplication, AgentSessionApplicationError, AgentSessionOwnership,
    CreateAgentSessionCommand, SendAgentSessionMessageResult,
};
use crate::{
    agent_sessions::domain::AgentSessionId,
    execution_configuration::{
        DirectUserInvocationResolution, NodeProfile, ResolutionError, ProviderConfigurationSource,
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
    /// Resolves Orchid `$name` mentions to skills in `extension.skill_inputs` and records them as
    /// explicitly invoked; the provider delivers each in its native form. A Session with a pinned
    /// profile invokes only its pinned manifest. An unprofiled Session may also invoke a skill
    /// from the provider's native catalogue, which is pinned for this invocation.
    pub(super) fn apply_skill_mentions(
        &self,
        configuration: &ProviderConfigurationRef,
        cwd: Option<&str>,
        submitted_text: &str,
        pinned_profile: bool,
        extension: &mut RuntimeLaunchExtension,
    ) {
        if !submitted_text.contains('$') {
            return;
        }
        let mut invoked = extension
            .skill_inputs
            .iter()
            .filter(|input| skill_mentions::mentions(submitted_text, &input.name))
            .map(|input| input.id.clone())
            .collect::<Vec<_>>();
        let catalogue = (!pinned_profile).then(|| self.configuration_source(&configuration.provider).ok()).flatten().and_then(|source| {
            source
                .native_skills_for_configuration(&configuration.configuration_id, cwd)
                .ok()
        });
        for skill in catalogue.iter().flat_map(|catalogue| {
            catalogue.skills.iter().filter(|skill| {
                skill.enabled
                    && catalogue
                        .skills
                        .iter()
                        .filter(|other| other.enabled && other.name == skill.name)
                        .count()
                        == 1
                    && skill_mentions::mentions(submitted_text, &skill.name)
            })
        }) {
            if extension
                .skill_inputs
                .iter()
                .any(|input| input.name == skill.name)
            {
                continue;
            }
            if let Ok(input) = crate::execution_configuration::pin_discovered_skill(skill) {
                invoked.push(input.id.clone());
                extension.skill_inputs.push(input);
            }
        }
        extension.invoked_skill_ids = invoked;
    }

    /// A device has at most one route per provider, so the execution's device and provider
    /// identify its route even when the route names a configuration alias.
    pub(super) fn compile_capability_skill_inputs(
        &self,
        capability: &CapabilityProfile,
        execution: &crate::execution_targets::domain::ExecutionBinding,
        cwd: Option<&str>,
    ) -> Result<Vec<crate::agent_sessions::ports::RuntimeSkillInput>, String> {
        let configuration_ref = execution.configuration_ref.as_str();
        let Some(route) = capability.route_policies.iter().find(|route| {
            route.execution.device_id == execution.device_id
                && route.execution.provider == execution.provider
        }) else {
            return Ok(Vec::new());
        };
        if route.skill_groups.is_empty() {
            return Ok(Vec::new());
        }
        let source = self
            .configuration_source(&route.execution.provider)
            .map_err(|error| error.to_string())?;
        let features = source
            .quick_features_for_configuration(configuration_ref, cwd)
            .map_err(|error| error.to_string())?;
        let mut features = features;
        self.product_skills.append_quick_skills(&mut features);
        compile_session_skill_inputs(route, &features, self.product_skills.roots())
    }

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
        if capability.execution.is_remote() {
            return Err(SessionConfigurationError::new(SessionConfigurationErrorKind::InvalidInvocationSelection, "Select an existing remote worktree before starting a session with this Capability Profile"));
        }
        let runtime =
            self.capability_profiles
                .as_ref()
                .expect("default Capability Profile service is configured")
                .runtime_for_binding(&capability.execution, working_directory)
                .map_err(|error| {
                    SessionConfigurationError::new(
                SessionConfigurationErrorKind::InvalidInvocationSelection,
                format!("Cannot start session on the selected Capability Profile route: {error}"),
            )
                })?;
        let session_skill_inputs = self
            .compile_capability_skill_inputs(
                &capability,
                &capability.execution,
                working_directory,
            )
            .map_err(|error| {
                SessionConfigurationError::new(
                    SessionConfigurationErrorKind::MissingPinnedProfile,
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
                    allowed_capabilities: default_node_capabilities(&capability),
                    pinned_defaults: Default::default(),
                },
                capability_profile: capability,
                session_skill_inputs,
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

    /// Orchid-owned skill roots offered with every provider configuration.
    pub(crate) fn with_product_skills(
        mut self,
        roots: Arc<crate::execution_configuration::ProductSkillRoots>,
    ) -> Self {
        self.product_skills = roots;
        self
    }

    /// The configuration source of the provider a route or Session runs on.
    pub(super) fn configuration_source(
        &self,
        provider: &str,
    ) -> Result<Arc<dyn ProviderConfigurationSource>, SessionConfigurationError> {
        self.endpoints
            .as_ref()
            .ok_or("No execution endpoints are configured".to_string())
            .and_then(|endpoints| endpoints.configuration_source(provider))
            .map_err(|message| {
                SessionConfigurationError::new(
                    SessionConfigurationErrorKind::MissingPinnedProfile,
                    message,
                )
            })
    }

    /// The native launch preparation of a provider, if it has one.
    pub(super) fn launch_preparation(
        &self,
        provider: &str,
    ) -> Option<Arc<dyn super::ProviderLaunchPreparation>> {
        self.endpoints
            .as_ref()
            .and_then(|endpoints| endpoints.launch_preparation(provider))
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

impl AgentSessionApplication {
    pub(crate) fn load_current_session_profile(
        &self,
        session_id: &AgentSessionId,
    ) -> Result<PinnedAgentSessionProfile, SessionConfigurationError> {
        let current = self
            .repository
            .current_execution_resolution(session_id)
            .map_err(AgentSessionApplicationError::repository)
            .map_err(SessionConfigurationError::agent_session)?;
        match current {
            Some(creation_resolution) => Ok(PinnedAgentSessionProfile {
                session_id: session_id.clone(),
                creation_resolution,
            }),
            None => self.load_pinned_session_profile(LoadPinnedSessionProfileQuery {
                session_id: session_id.clone(),
            }),
        }
    }
}
