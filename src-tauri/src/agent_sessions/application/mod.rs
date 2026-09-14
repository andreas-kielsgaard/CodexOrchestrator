//! Provider-neutral session application facade. Use cases are separated by responsibility.

use self::update_sink::InvocationUpdateLanes;
use crate::agent_sessions::ports::{AgentRuntime, AgentSessionRepository};
use std::sync::Arc;

mod addressed;
mod commands;
pub(crate) mod configuration;
mod creation;
mod dependencies;
mod diagnostics;
mod direct_user;
mod interactions;
mod invocation;
mod lifecycle;
pub(crate) mod observation;
mod quick_features;
mod update_sink;
mod workspaces;
mod targets;
pub(crate) use crate::agent_sessions::interactions::{project_interactions, SessionInteraction};
pub(crate) use crate::agent_sessions::workspace::SessionWorkspaces;
pub(crate) use interactions::{RespondToRuntimeRequestCommand, SteerAgentSessionCommand};

pub(crate) use commands::{
    AgentSessionApplicationError, AgentSessionOwnership, ApplicationInvocationLaunchEvidence,
    CancelAgentInvocationCommand, CreateAgentSessionCommand, CreateApplicationAgentSessionCommand,
    ListAgentSessionsResult, SendAgentSessionMessageCommand, SendAgentSessionMessageLaunchResult,
    SendAgentSessionMessageResult, SendIdempotentApplicationAgentSessionMessageCommand,
    UpdateAgentSessionHarnessCommand, UpdateAgentSessionIdentityCommand,
    UpdateAgentSessionModelOverrideCommand,
};
pub(crate) use configuration::{reasoning_launch_extension, runtime_options};
pub(crate) use configuration::{
    LoadPinnedSessionProfileQuery, SendDirectUserAgentSessionMessageCommand,
};
pub(crate) use dependencies::{
    AgentSessionClock, AgentSessionIdProvider, AgentSessionNotification, AgentSessionNotifier,
    NativeProfileLaunchAuthority, SessionHarnessLaunchAuthority, SessionHarnessVersionResolver,
    SystemAgentSessionProviders,
};
pub(crate) use observation::{project_invocation_observation, AgentInvocationObservation};

#[derive(Clone)]
pub(crate) struct AgentSessionApplication {
    repository: Arc<dyn AgentSessionRepository>,
    runtime: Arc<dyn AgentRuntime>,
    notifier: Arc<dyn AgentSessionNotifier>,
    clock: Arc<dyn AgentSessionClock>,
    ids: Arc<dyn AgentSessionIdProvider>,
    runtime_version: Option<String>,
    native_profile_launch_authority: Option<Arc<dyn NativeProfileLaunchAuthority>>,
    session_harness_version_resolver: Option<Arc<dyn SessionHarnessVersionResolver>>,
    session_harness_launch_authority: Option<Arc<dyn SessionHarnessLaunchAuthority>>,
    update_lanes: Arc<InvocationUpdateLanes>,
    workspaces: Option<SessionWorkspaces>,
    profile_source: Option<Arc<dyn crate::execution_configuration::SelectedRuntimeProfileSource>>,
    interaction_lanes: Arc<interactions::InteractionLanes>,
    capability_profiles: Option<Arc<crate::execution_configuration::CapabilityProfileService>>,
    endpoints: Option<Arc<crate::execution_targets::endpoints::ExecutionEndpoints>>,
}

impl AgentSessionApplication {
    pub(crate) fn new(
        repository: Arc<dyn AgentSessionRepository>,
        runtime: Arc<dyn AgentRuntime>,
        notifier: Arc<dyn AgentSessionNotifier>,
        clock: Arc<dyn AgentSessionClock>,
        ids: Arc<dyn AgentSessionIdProvider>,
        runtime_version: Option<String>,
    ) -> Self {
        Self {
            repository,
            runtime,
            notifier,
            clock,
            ids,
            runtime_version,
            native_profile_launch_authority: None,
            session_harness_version_resolver: None,
            session_harness_launch_authority: None,
            update_lanes: Arc::new(InvocationUpdateLanes::default()),
            workspaces: None,
            profile_source: None,
            interaction_lanes: Arc::new(interactions::InteractionLanes::default()),
            capability_profiles: None,
            endpoints: None,
        }
    }

    pub(crate) fn with_native_profile_launch_authority(
        mut self,
        authority: Arc<dyn NativeProfileLaunchAuthority>,
    ) -> Self {
        self.native_profile_launch_authority = Some(authority);
        self
    }

    pub(crate) fn with_session_harness_launch_authority(
        mut self,
        authority: Arc<dyn SessionHarnessLaunchAuthority>,
    ) -> Self {
        self.session_harness_launch_authority = Some(authority);
        self
    }

    pub(crate) fn with_session_harness_version_resolver(
        mut self,
        resolver: Arc<dyn SessionHarnessVersionResolver>,
    ) -> Self {
        self.session_harness_version_resolver = Some(resolver);
        self
    }

    #[cfg(test)]
    pub(super) fn update_lane_count(&self) -> usize {
        self.update_lanes.len()
    }
}

#[cfg(test)]
mod tests;

pub(crate) mod import;
