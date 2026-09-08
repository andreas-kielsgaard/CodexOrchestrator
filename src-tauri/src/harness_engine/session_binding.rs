use super::service::HarnessEngineService;
use crate::agent_sessions::{
    application::SessionHarnessLaunchAuthority,
    domain::{AgentInvocationId, AgentSessionId},
    ports::{AgentSessionRepository, RuntimeLaunchExtension},
};
use std::sync::Arc;

/// Technical launch mediation for pinned Sessions. The engine need not consume a Workflow
/// recipe, Role, or old Harness catalogue entry to bind their MCP exposure.
pub(crate) struct SessionProfileHarnessAuthority {
    pub(crate) engine: Arc<HarnessEngineService>,
    pub(crate) sessions: Arc<dyn AgentSessionRepository>,
}

impl SessionHarnessLaunchAuthority for SessionProfileHarnessAuthority {
    fn prepare_launch(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<Option<RuntimeLaunchExtension>, String> {
        let session = self
            .sessions
            .get_session(session_id)
            .map_err(|error| error.to_string())?
            .ok_or("Session is missing")?;
        if let Some(profile) = &session.session_profile {
            self.engine.bind_session_profile(session_id, profile)?;
        }
        self.engine
            .prepare_launch(session_id, invocation_id, extension)
    }
}
