//! Provider-neutral Agent Session use cases and persisted runtime update delivery.

mod lifecycle;
pub(crate) mod observation;
mod update_sink;

pub(crate) use lifecycle::{
    AgentSessionApplication, AgentSessionNotification, AgentSessionNotifier, AgentSessionOwnership,
    ApplicationInvocationLaunchEvidence, CancelAgentInvocationCommand, CreateAgentSessionCommand,
    CreateApplicationAgentSessionCommand, NativeProfileLaunchAuthority,
    SendAgentSessionMessageCommand, SendAgentSessionMessageResult,
    SendIdempotentApplicationAgentSessionMessageCommand, SessionHarnessLaunchAuthority,
    SessionHarnessVersionResolver, SystemAgentSessionProviders, UpdateAgentSessionHarnessCommand,
    UpdateAgentSessionIdentityCommand, UpdateAgentSessionModelOverrideCommand,
};
pub(crate) use observation::{project_invocation_observation, AgentInvocationObservation};

#[cfg(test)]
mod tests;
