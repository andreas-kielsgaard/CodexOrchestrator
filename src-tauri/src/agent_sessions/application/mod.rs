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
pub(crate) mod preparation;
mod quick_features;
pub(crate) mod target_transition;
mod targets;
mod update_sink;
mod workspaces;
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
    ProviderLaunchPreparation, SessionHarnessLaunchAuthority, SessionHarnessVersionResolver,
    SystemAgentSessionProviders,
};
pub(crate) use observation::{project_invocation_observation, AgentInvocationObservation};

#[derive(Clone)]
pub(crate) struct AgentSessionApplication {
    repository: Arc<dyn AgentSessionRepository>,
    /// Runtime for unprofiled legacy Sessions when no execution endpoints are composed (tests).
    runtime: Arc<dyn AgentRuntime>,
    notifier: Arc<dyn AgentSessionNotifier>,
    clock: Arc<dyn AgentSessionClock>,
    ids: Arc<dyn AgentSessionIdProvider>,
    runtime_version: Option<String>,
    session_harness_version_resolver: Option<Arc<dyn SessionHarnessVersionResolver>>,
    session_harness_launch_authority: Option<Arc<dyn SessionHarnessLaunchAuthority>>,
    update_lanes: Arc<InvocationUpdateLanes>,
    workspaces: Option<SessionWorkspaces>,
    product_skills: Arc<crate::execution_configuration::ProductSkillRoots>,
    interaction_lanes: Arc<interactions::InteractionLanes>,
    capability_profiles: Option<Arc<crate::execution_configuration::CapabilityProfileService>>,
    endpoints: Option<Arc<crate::execution_targets::endpoints::ExecutionEndpoints>>,
    execution_target_service: Option<Arc<crate::execution_targets::ExecutionTargetService>>,
    preparation_workers: Arc<preparation::PreparationWorkers>,
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
            session_harness_version_resolver: None,
            session_harness_launch_authority: None,
            update_lanes: Arc::new(InvocationUpdateLanes::default()),
            workspaces: None,
            product_skills: Default::default(),
            interaction_lanes: Arc::new(interactions::InteractionLanes::default()),
            capability_profiles: None,
            endpoints: None,
            execution_target_service: None,
            preparation_workers: Arc::new(preparation::PreparationWorkers::default()),
        }
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
