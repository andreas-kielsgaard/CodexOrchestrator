use crate::agent_sessions::{
    domain::{
        AgentInvocationId, AgentInvocationInputProvenance, AgentRuntimeOptions, AgentSessionId,
    },
    ports::{InvocationContent, RuntimeLaunchExtension},
};

/// Orchid-owned semantic delivery request. Provider selection is deliberately absent: the model,
/// selected device and Capability Profile resolve one route during planning.
pub struct InvocationDeliveryIntent {
    pub invocation_id: Option<AgentInvocationId>,
    pub provenance: AgentInvocationInputProvenance,
    pub session_id: Option<AgentSessionId>,
    pub title: Option<String>,
    pub working_directory: Option<String>,
    pub requested_options: Option<AgentRuntimeOptions>,
    pub content: InvocationContent,
    pub attachments: Vec<crate::agent_sessions::domain::RuntimeImageAttachment>,
    pub launch_extension: Option<RuntimeLaunchExtension>,
    pub prepared_only: bool,
}
