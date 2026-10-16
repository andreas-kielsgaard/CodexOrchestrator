//! Provider-neutral session application facade. Use cases are separated by responsibility.

use self::update_sink::InvocationUpdateLanes;
use crate::agent_sessions::ports::{AgentRuntime, AgentSessionRepository};
use std::sync::Arc;

mod addressed;
mod commands;
pub mod configuration;
mod creation;
mod delivery;
mod dependencies;
mod diagnostics;
mod direct_user;
mod product_delivery;
mod product_sessions;
pub use product_delivery::ProductDeliveryReceipt;
pub use product_sessions::ProductSessionOptions;
mod interactions;
mod invocation;
pub mod jev;
mod lifecycle;
pub mod observation;
pub mod preparation;
pub mod queue;
mod quick_features;
pub mod steering_queue;
mod target_operations;
mod targets;
mod update_sink;
mod workspaces;
pub use crate::agent_sessions::interactions::{project_interactions, SessionInteraction};
pub use crate::agent_sessions::workspace::SessionWorkspaces;
pub use interactions::{RespondToRuntimeRequestCommand, SteerAgentSessionCommand};

pub use commands::{
    AgentSessionApplicationError, AgentSessionOwnership, ApplicationInvocationLaunchEvidence,
    CancelAgentInvocationCommand, CreateAgentSessionCommand, CreateApplicationAgentSessionCommand,
    ListAgentSessionsResult, RenameAgentSessionCommand, SendAgentSessionMessageCommand,
    SendAgentSessionMessageLaunchResult, SendAgentSessionMessageResult,
    SendIdempotentApplicationAgentSessionMessageCommand, UpdateAgentSessionHarnessCommand,
    UpdateAgentSessionIdentityCommand, UpdateAgentSessionModelOverrideCommand,
};
pub use configuration::{reasoning_launch_extension, runtime_options};
pub use configuration::{LoadPinnedSessionProfileQuery, SendDirectUserAgentSessionMessageCommand};
pub use delivery::InvocationDeliveryIntent;
pub use dependencies::{
    AgentSessionClock, AgentSessionIdProvider, AgentSessionNotification, AgentSessionNotifier,
    ProviderLaunchPreparation, ProviderUpdateReadiness, SessionHarnessLaunchAuthority,
    SessionHarnessVersionResolver, SystemAgentSessionProviders,
};
pub use observation::{project_invocation_observation, AgentInvocationObservation};

#[derive(Clone)]
pub struct AgentSessionApplication {
    replicas: Option<crate::synchronization::repository::ReplicaStore>,
    workers: Arc<crate::background::WorkerExecutor>,
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
    steering_queue: Arc<steering_queue::PendingSteering>,
    capability_profiles: Option<Arc<crate::execution_configuration::CapabilityProfileService>>,
    endpoints: Option<Arc<crate::execution_targets::endpoints::ExecutionEndpoints>>,
    execution_target_service: Option<Arc<crate::execution_targets::ExecutionTargetService>>,
    provider_update_readiness: Option<Arc<dyn ProviderUpdateReadiness>>,
    preparation_workers: Arc<preparation::PreparationWorkers>,
    queue_dispatch_lanes: Arc<queue::QueueDispatchLanes>,
    target_operation_workers: Arc<queue::TargetOperationWorkers>,
}

impl AgentSessionApplication {
    pub fn route_owned_command(
        &self,
        id: &crate::agent_sessions::domain::AgentSessionId,
        command: crate::agent_sessions::client::HostCommand,
    ) -> Result<Option<serde_json::Value>, String> {
        if self
            .repository
            .owns_execution(id)
            .map_err(|e| e.to_string())?
        {
            return Ok(None);
        }
        let owner = self
            .repository
            .execution_device(id)
            .map_err(|e| e.to_string())?
            .ok_or("Session execution owner is unknown")?;
        self.execution_targets()?
            .endpoints
            .host_command(&owner, command)
            .map(Some)
    }
    pub fn wake_transferred_session(&self, id: crate::agent_sessions::domain::AgentSessionId) {
        self.wake_session_queue(id);
    }
    pub fn execution_targets(
        &self,
    ) -> Result<&crate::execution_targets::ExecutionTargetService, String> {
        self.execution_target_service
            .as_deref()
            .ok_or_else(|| "Device operations are not configured".into())
    }
    pub fn queries(&self) -> crate::agent_sessions::queries::AgentSessionQueries {
        let queries =
            crate::agent_sessions::queries::AgentSessionQueries::new(self.repository.clone());
        match &self.replicas {
            Some(replicas) => queries.with_replicas(replicas.clone()),
            None => queries,
        }
    }
    pub fn with_replicas(
        mut self,
        replicas: crate::synchronization::repository::ReplicaStore,
    ) -> Self {
        self.replicas = Some(replicas);
        self
    }
    pub fn new(
        repository: Arc<dyn AgentSessionRepository>,
        runtime: Arc<dyn AgentRuntime>,
        notifier: Arc<dyn AgentSessionNotifier>,
        clock: Arc<dyn AgentSessionClock>,
        ids: Arc<dyn AgentSessionIdProvider>,
        runtime_version: Option<String>,
    ) -> Self {
        Self {
            replicas: None,
            workers: Default::default(),
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
            steering_queue: Arc::new(steering_queue::PendingSteering::default()),
            capability_profiles: None,
            endpoints: None,
            execution_target_service: None,
            provider_update_readiness: None,
            preparation_workers: Arc::new(preparation::PreparationWorkers::default()),
            queue_dispatch_lanes: Arc::new(queue::QueueDispatchLanes::default()),
            target_operation_workers: Arc::new(queue::TargetOperationWorkers::default()),
        }
    }

    pub fn with_session_harness_launch_authority(
        mut self,
        authority: Arc<dyn SessionHarnessLaunchAuthority>,
    ) -> Self {
        self.session_harness_launch_authority = Some(authority);
        self
    }

    pub fn with_provider_update_readiness(
        mut self,
        readiness: Arc<dyn ProviderUpdateReadiness>,
    ) -> Self {
        self.provider_update_readiness = Some(readiness);
        self
    }

    pub fn with_session_harness_version_resolver(
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

pub mod import;
