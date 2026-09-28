use crate::agent_sessions::{
    domain::{
        AgentInvocationId, AgentInvocationInputProvenance, AgentRuntimeOptions, AgentSessionId,
    },
    ports::{InvocationContent, RuntimeLaunchExtension},
};

/// Orchid-owned semantic delivery request. Provider selection is deliberately absent: the model,
/// selected device and Capability Profile resolve one route during planning.
pub(crate) struct InvocationDeliveryIntent {
    pub(crate) invocation_id: Option<AgentInvocationId>,
    pub(crate) provenance: AgentInvocationInputProvenance,
    pub(crate) session_id: Option<AgentSessionId>,
    pub(crate) title: Option<String>,
    pub(crate) working_directory: Option<String>,
    pub(crate) requested_options: Option<AgentRuntimeOptions>,
    pub(crate) content: InvocationContent,
    pub(crate) launch_extension: Option<RuntimeLaunchExtension>,
    pub(crate) prepared_only: bool,
}
