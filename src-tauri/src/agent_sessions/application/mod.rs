//! Provider-neutral Agent Session use cases and persisted runtime update delivery.

mod lifecycle;
pub(crate) mod observation;
mod update_sink;

pub(crate) use lifecycle::{
    title_from_message, AgentSessionApplication, AgentSessionNotification, AgentSessionNotifier,
    ApplicationInvocationLaunchEvidence, CancelAgentInvocationCommand, CreateAgentSessionCommand,
    CreateApplicationAgentSessionCommand, NativeProfileLaunchAuthority,
    SessionHarnessLaunchAuthority,
    SendAgentSessionMessageCommand, SendAgentSessionMessageResult,
    SendIdempotentApplicationAgentSessionMessageCommand, SystemAgentSessionProviders,
};
pub(crate) use observation::{project_invocation_observation, AgentInvocationObservation};

#[cfg(test)]
mod tests;
