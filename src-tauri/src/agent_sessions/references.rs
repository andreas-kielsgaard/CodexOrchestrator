//! Stable references at the Session Events boundary; these are not provider thread IDs.

use super::domain::AgentSessionId;
use crate::session_events::{ReferenceIdentity, SessionDirectoryError};

const SESSION_NAMESPACE: &str = "orchestrator.agent_sessions";

pub(crate) fn session_reference(id: &str) -> Result<ReferenceIdentity, SessionDirectoryError> {
    ReferenceIdentity::new(SESSION_NAMESPACE, "session", id)
        .map_err(|error| SessionDirectoryError::new(error.to_string()))
}

pub(crate) fn parse_session_reference(
    reference: &ReferenceIdentity,
) -> Result<AgentSessionId, SessionDirectoryError> {
    if reference.namespace() != SESSION_NAMESPACE || reference.kind() != "session" {
        return Err(SessionDirectoryError::new(format!(
            "Reference `{reference}` is not an Agent Session"
        )));
    }
    AgentSessionId::new(reference.id().to_string())
        .map_err(|error| SessionDirectoryError::new(error.to_string()))
}
