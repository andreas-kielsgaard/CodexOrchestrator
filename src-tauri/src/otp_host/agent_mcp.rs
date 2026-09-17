use crate::{
    agent_sessions::domain::AgentSessionId, execution_configuration::SessionCreationResolution,
    harness_engine::ManagedMcpUpstreamRegistry,
};

pub(crate) trait AgentMcpUpstreamProvisioner: Send + Sync {
    fn provision(
        &self,
        session_id: &AgentSessionId,
        profile: &SessionCreationResolution,
        upstreams: &ManagedMcpUpstreamRegistry,
    ) -> Result<(), String>;
}
