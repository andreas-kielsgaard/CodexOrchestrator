use super::references::parse_session_reference;
use super::{
    application::AgentSessionApplication,
    domain::{AgentInvocationId, AgentSessionId},
    repository::SqliteAgentSessionRepository,
};
use crate::{
    execution_configuration::{
        CapabilityProfileService, ProviderConfigurationSource, SessionCreationIntent,
        SessionCreationRequest,
    },
    identities::{service::IdentityService, IdentityId},
    session_events::{
        ReferenceIdentity, SessionCreationSpec, SessionDirectory, SessionDirectoryEntry,
        SessionDirectoryError, SessionEventSource, SessionInvocationDispatcher,
        SessionInvocationError, SessionInvocationReceipt, SessionInvocationRequest,
        SessionLogicalAddress,
    },
};
use sha2::{Digest, Sha256};
use std::sync::Arc;

const CREATION_REQUEST_NAMESPACE: &str = "orchestrator.execution_configuration";
const CREATION_REQUEST_KIND: &str = "session_creation_request";
const CREATION_REQUEST_VERSION: &str = "v1";
const IDENTITY_REF_NAMESPACE: &str = "orchestrator.identities";
const IDENTITY_REF_KIND: &str = "identity";

/// The concrete application boundary between generic Session addressing and managed Agent
/// Sessions. Workflow identities remain opaque references on this side of the port.
pub(crate) struct AgentSessionEventAdapter {
    application: AgentSessionApplication,
    repository: Arc<SqliteAgentSessionRepository>,
    capability_profiles: Option<Arc<CapabilityProfileService>>,
    identities: IdentityService,
}

impl AgentSessionEventAdapter {
    pub(crate) fn new(
        application: Arc<AgentSessionApplication>,
        repository: Arc<SqliteAgentSessionRepository>,
        profile_source: Arc<dyn ProviderConfigurationSource>,
        identities: IdentityService,
    ) -> Self {
        Self {
            application: application
                .as_ref()
                .clone()
                .with_profile_source(profile_source),
            repository,
            capability_profiles: None,
            identities,
        }
    }

    pub(crate) fn with_capability_profiles(
        mut self,
        profiles: Arc<CapabilityProfileService>,
    ) -> Self {
        self.application = self.application.with_capability_profiles(profiles.clone());
        self.capability_profiles = Some(profiles);
        self
    }
}

impl SessionDirectory for AgentSessionEventAdapter {
    fn find_exact(
        &self,
        session: &ReferenceIdentity,
    ) -> Result<Option<SessionDirectoryEntry>, SessionDirectoryError> {
        let session_id = parse_session_reference(session)?;
        let history = match self.application.load_session(&session_id) {
            Ok(history) => history,
            Err(error) if error.to_string() == "Agent Session not found" => return Ok(None),
            Err(error) => return Err(SessionDirectoryError::new(error.to_string())),
        };
        let running = history
            .invocations
            .iter()
            .any(|history| history.invocation.status.is_active());
        let stored = self.repository.load_address(&session_id)?;
        Ok(Some(match stored {
            Some(stored) => stored.entry(session.clone(), running),
            None => SessionDirectoryEntry {
                session: session.clone(),
                logical_address: None,
                running,
                created_sequence: 0,
                last_addressed_sequence: None,
                created_by_event: None,
                created_by_session: None,
            },
        }))
    }

    fn list_at_address(
        &self,
        address: &SessionLogicalAddress,
    ) -> Result<Vec<SessionDirectoryEntry>, SessionDirectoryError> {
        self.repository.list_session_directory(address)
    }

    fn create_session(
        &self,
        request: SessionCreationSpec,
    ) -> Result<SessionDirectoryEntry, SessionDirectoryError> {
        let (creation_request, working_directory, title) =
            if request.configuration.contract.namespace() == CREATION_REQUEST_NAMESPACE
                && request.configuration.contract.kind() == "session_creation_intent"
                && request.configuration.contract.id() == "v1"
            {
                let intent: SessionCreationIntent =
                    serde_json::from_value(request.configuration.payload.clone())
                        .map_err(|error| SessionDirectoryError::new(error.to_string()))?;
                let profile = self
                    .capability_profiles
                    .as_ref()
                    .ok_or_else(|| {
                        SessionDirectoryError::new("Capability Profile source is unavailable")
                    })?
                    .read(&intent.capability_profile_id)
                    .map_err(|error| SessionDirectoryError::new(error.to_string()))?;
                (
                    SessionCreationRequest {
                        contract_version: 1,
                        capability_profile: profile,
                        node_profile: intent.node_profile,
                        agent_mcp_configuration: intent.agent_mcp_configuration,
                        session_skill_inputs: Vec::new(),
                    },
                    Some(intent.working_directory),
                    Some(intent.title),
                )
            } else {
                validate_creation_contract(&request.configuration.contract)?;
                let creation_request: SessionCreationRequest = serde_json::from_value(
                    request.configuration.payload.clone(),
                )
                .map_err(|error| {
                    SessionDirectoryError::new(format!(
                        "Pinned Session creation request is invalid: {error}"
                    ))
                })?;
                (
                    creation_request,
                    None,
                    Some(request.logical_address.subject.id().to_string()),
                )
            };
        let assigned_identity = request
            .configuration
            .assigned_identity
            .as_ref()
            .map(|reference| {
                if reference.namespace() != IDENTITY_REF_NAMESPACE
                    || reference.kind() != IDENTITY_REF_KIND
                {
                    return Err(SessionDirectoryError::new(
                        "Session creation identity reference has an unsupported contract",
                    ));
                }
                let identity_id = IdentityId::new(reference.id().to_string())
                    .map_err(|error| SessionDirectoryError::new(error.to_string()))?;
                self.identities
                    .assignment(&identity_id)
                    .map_err(SessionDirectoryError::new)
            })
            .transpose()?;
        let session_id = session_id_for_creation(&request.event_group_id)?;
        self.application.create_addressed_profiled_session(
            self.repository.as_ref(),
            request,
            session_id,
            creation_request,
            working_directory,
            title,
            assigned_identity,
        )
    }

    fn mark_addressed(
        &self,
        session: &ReferenceIdentity,
        _event_group_id: &ReferenceIdentity,
    ) -> Result<u64, SessionDirectoryError> {
        self.repository.mark_session_addressed(session)
    }
}

impl SessionInvocationDispatcher for AgentSessionEventAdapter {
    fn dispatch(
        &self,
        request: SessionInvocationRequest,
    ) -> Result<SessionInvocationReceipt, SessionInvocationError> {
        let SessionInvocationRequest {
            event_group_id,
            delivery_id,
            target_session,
            source,
            prompt,
            initial_prompt,
            direct_user_options,
        } = request;
        let session_id = parse_session_reference(&target_session)
            .map_err(|error| SessionInvocationError::new(error.to_string()))?;
        let invocation_id = invocation_id_for_delivery(&delivery_id)
            .map_err(|e| SessionInvocationError::new(e.to_string()))?;
        let launched = self.application.deliver_profiled_message(
            session_id,
            invocation_id,
            prompt,
            initial_prompt,
            event_group_id.to_string(),
            direct_user_options,
            matches!(source, SessionEventSource::UserRequest { .. }),
        )?;
        if !launched.launch_accepted {
            return Err(SessionInvocationError::new(
                "Agent runtime did not accept the Session Event delivery",
            ));
        }
        let invocation =
            session_invocation_reference(launched.acknowledgement.invocation_id.as_str())
                .map_err(|error| SessionInvocationError::new(error.to_string()))?;
        Ok(SessionInvocationReceipt { invocation })
    }
}

fn validate_creation_contract(contract: &ReferenceIdentity) -> Result<(), SessionDirectoryError> {
    if contract.namespace() == CREATION_REQUEST_NAMESPACE
        && contract.kind() == CREATION_REQUEST_KIND
        && contract.id() == CREATION_REQUEST_VERSION
    {
        Ok(())
    } else {
        Err(SessionDirectoryError::new(format!(
            "Unsupported Session creation configuration contract `{contract}`"
        )))
    }
}

fn session_id_for_creation(
    event_group: &ReferenceIdentity,
) -> Result<AgentSessionId, SessionDirectoryError> {
    let encoded = serde_json::to_vec(event_group)
        .map_err(|error| SessionDirectoryError::new(error.to_string()))?;
    let mut digest = Sha256::new();
    digest.update(b"agent-sessions/session-event-creation/v1\0");
    digest.update(encoded);
    AgentSessionId::new(format!("session-event-{:x}", digest.finalize()))
        .map_err(|error| SessionDirectoryError::new(error.to_string()))
}

fn session_invocation_reference(
    id: &str,
) -> Result<ReferenceIdentity, crate::session_events::SessionEventDomainError> {
    ReferenceIdentity::new("orchestrator.agent_sessions", "invocation", id)
}

fn invocation_id_for_delivery(
    delivery: &ReferenceIdentity,
) -> Result<AgentInvocationId, crate::agent_sessions::domain::ContractViolation> {
    AgentInvocationId::new(format!(
        "session-event:{}:{}:{}:{}:{}:{}",
        delivery.namespace().len(),
        delivery.namespace(),
        delivery.kind().len(),
        delivery.kind(),
        delivery.id().len(),
        delivery.id(),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invocation_identity_preserves_the_complete_delivery_reference() {
        let first = ReferenceIdentity::new("a-b", "delivery", "shared").unwrap();
        let second = ReferenceIdentity::new("a", "b-delivery", "shared").unwrap();

        assert_ne!(
            invocation_id_for_delivery(&first).unwrap(),
            invocation_id_for_delivery(&second).unwrap()
        );
    }
}
