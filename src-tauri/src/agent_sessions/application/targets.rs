use super::*;
use crate::{
    agent_sessions::domain::{AgentInvocationId, AgentSession, AgentSessionId},
    execution_targets::endpoints::ExecutionEndpoints,
};

impl AgentSessionApplication {
    pub(crate) fn with_execution_endpoints(mut self, endpoints: Arc<ExecutionEndpoints>) -> Self {
        self.endpoints = Some(endpoints);
        self
    }
    pub(super) fn session_runtime(
        &self,
        session: &AgentSession,
    ) -> Result<Arc<dyn AgentRuntime>, AgentSessionApplicationError> {
        match &session.execution_target {
            Some(target) => self
                .endpoints
                .as_ref()
                .ok_or_else(|| {
                    AgentSessionApplicationError::invalid("Execution endpoints are unavailable")
                })?
                .runtime(&target.execution)
                .map_err(AgentSessionApplicationError::invalid),
            None => Ok(self.runtime.clone()),
        }
    }
    pub(super) fn runtime_for_session_id(
        &self,
        id: &AgentSessionId,
    ) -> Result<Arc<dyn AgentRuntime>, AgentSessionApplicationError> {
        self.session_runtime(&self.load_session(id)?.session)
    }
    pub(super) fn runtime_for_invocation(
        &self,
        id: &AgentInvocationId,
    ) -> Result<Arc<dyn AgentRuntime>, AgentSessionApplicationError> {
        let invocation = self
            .repository
            .get_invocation(id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| AgentSessionApplicationError::not_found("Invocation not found"))?;
        self.runtime_for_session_id(&invocation.session_id)
    }
}
