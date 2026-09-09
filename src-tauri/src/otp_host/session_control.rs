use crate::agent_sessions::{
    application::{AgentSessionApplication, CancelAgentInvocationCommand},
    domain::AgentInvocationId,
    ports::AgentSessionRepository,
};
use std::sync::Arc;

/// Product-side cancellation accepts a pinned invocation, never a mutable session target.
pub(crate) trait SessionControl: Send + Sync {
    fn cancel(&self, invocation_id: &str) -> Result<bool, String>;
}
pub(crate) struct AgentSessionControl {
    pub application: Arc<AgentSessionApplication>,
    pub repository: Arc<dyn AgentSessionRepository>,
}
impl SessionControl for AgentSessionControl {
    fn cancel(&self, invocation_id: &str) -> Result<bool, String> {
        let id = AgentInvocationId::new(invocation_id).map_err(|e| e.to_string())?;
        let invocation = self
            .repository
            .get_invocation(&id)
            .map_err(|e| e.to_string())?
            .ok_or("Invocation is missing")?;
        if !invocation.status.is_active() {
            return Ok(false);
        }
        self.application
            .cancel_invocation(CancelAgentInvocationCommand { invocation_id: id })
            .map_err(|e| e.to_string())?;
        Ok(true)
    }
}
