//! Which native conversation a destination instance continues, and what its provider has not seen.
use super::*;
use crate::agent_sessions::{
    application::history_handoff::Handoff, domain::ParkedNativeConversation,
    ports::AgentSessionHistory,
};
use crate::execution_targets::{domain::ExecutionBinding, endpoints::ExecutionEndpoints};

#[derive(Debug, PartialEq)]
pub(super) struct ConversationPlan {
    /// The native conversation to continue, and the route it lives on.
    pub(super) continued: Option<(ExecutionBinding, ExternalRuntimeContextId)>,
    /// The current provider's conversation, parked when the provider changes.
    pub(super) parked_source: Option<ParkedNativeConversation>,
    pub(super) handoff: Handoff,
}

impl ConversationPlan {
    pub(super) fn needs_transfer(&self, destination: &ExecutionBinding) -> bool {
        self.continued
            .as_ref()
            .is_some_and(|(location, _)| location != destination)
    }
}

/// The Session's current conversation, before the destination instance is prepared.
pub(super) struct CurrentConversation {
    pub(super) provider: String,
    pub(super) conversation: Option<(ExecutionBinding, ExternalRuntimeContextId)>,
    pub(super) runtime_version: Option<String>,
    /// The Session's latest invocation before the one being prepared.
    pub(super) last_invocation_id: Option<AgentInvocationId>,
}

/// The same provider continues its conversation. A provider change parks the current conversation
/// and continues the destination provider's parked one, if any. A conversation that cannot be
/// transferred to the destination route restarts from the Session log.
pub(super) fn plan(
    current: CurrentConversation,
    destination: &ExecutionBinding,
    parked_destination: Option<ParkedNativeConversation>,
    can_transfer: bool,
) -> ConversationPlan {
    let mut plan = if current.provider == destination.provider {
        ConversationPlan {
            continued: current.conversation,
            parked_source: None,
            handoff: Handoff::None,
        }
    } else {
        let parked_source =
            current
                .conversation
                .map(|(location, external_context_id)| ParkedNativeConversation {
                    provider: current.provider,
                    external_context_id,
                    runtime_version: current.runtime_version,
                    location,
                    last_invocation_id: current.last_invocation_id,
                });
        match parked_destination {
            Some(parked) => ConversationPlan {
                continued: Some((parked.location, parked.external_context_id)),
                parked_source,
                handoff: Handoff::Since(parked.last_invocation_id),
            },
            None => ConversationPlan {
                continued: None,
                parked_source,
                handoff: Handoff::Full,
            },
        }
    };
    if plan.needs_transfer(destination) && !can_transfer {
        plan.continued = None;
        plan.handoff = Handoff::Full;
    }
    plan
}

impl AgentSessionApplication {
    /// `source` is where the Session's current conversation lives, when it has one.
    pub(super) fn plan_conversation(
        &self,
        history: &AgentSessionHistory,
        p: &SessionPreparation,
        source: Option<ExecutionBinding>,
        destination: &ExecutionBinding,
        endpoints: &ExecutionEndpoints,
    ) -> Result<ConversationPlan, AgentSessionApplicationError> {
        let provider = p
            .source_target
            .as_ref()
            .map(|target| target.execution.provider.clone())
            .unwrap_or_else(|| {
                super::super::configuration::session_configuration(&history.session).provider
            });
        let parked_destination = if provider == destination.provider {
            None
        } else {
            self.repository
                .parked_native_conversation(&history.session.id, &destination.provider)
                .map_err(AgentSessionApplicationError::repository)?
        };
        let current = CurrentConversation {
            provider,
            conversation: source.zip(p.source_binding.external_context_id.clone()),
            runtime_version: p.source_binding.runtime_version.clone(),
            last_invocation_id: history
                .invocations
                .iter()
                .take_while(|entry| entry.invocation.id != p.invocation_id)
                .last()
                .map(|entry| entry.invocation.id.clone()),
        };
        Ok(plan(
            current,
            destination,
            parked_destination,
            endpoints.supports_continuation(&destination.provider),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_targets::domain::ExecutionConnection;

    fn route(provider: &str, device: &str) -> ExecutionBinding {
        ExecutionBinding {
            device_id: device.into(),
            device_name: device.into(),
            provider: provider.into(),
            configuration_ref: "configuration".into(),
            connection: ExecutionConnection::Local,
        }
    }

    fn id(value: &str) -> ExternalRuntimeContextId {
        ExternalRuntimeContextId::new(value).unwrap()
    }

    fn current(provider: &str, device: &str) -> CurrentConversation {
        CurrentConversation {
            provider: provider.into(),
            conversation: Some((route(provider, device), id("current-thread"))),
            runtime_version: None,
            last_invocation_id: Some(AgentInvocationId::new("previous").unwrap()),
        }
    }

    #[test]
    fn the_same_provider_continues_and_transfers_between_devices() {
        let plan = plan(current("codex", "laptop"), &route("codex", "server"), None, true);
        assert_eq!(plan.continued.unwrap().1, id("current-thread"));
        assert!(plan.parked_source.is_none());
        assert_eq!(plan.handoff, Handoff::None);
    }

    #[test]
    fn a_new_provider_parks_the_current_conversation_and_starts_from_the_log() {
        let plan = plan(current("codex", "laptop"), &route("claude", "laptop"), None, false);
        assert!(plan.continued.is_none());
        let parked = plan.parked_source.unwrap();
        assert_eq!((parked.provider.as_str(), parked.external_context_id), ("codex", id("current-thread")));
        assert_eq!(parked.last_invocation_id.unwrap().as_str(), "previous");
        assert_eq!(plan.handoff, Handoff::Full);
    }

    #[test]
    fn a_returning_provider_continues_its_parked_conversation_with_what_it_missed() {
        let parked = ParkedNativeConversation {
            provider: "codex".into(),
            external_context_id: id("codex-thread"),
            runtime_version: None,
            location: route("codex", "laptop"),
            last_invocation_id: Some(AgentInvocationId::new("codex-last").unwrap()),
        };
        let plan = plan(current("claude", "laptop"), &route("codex", "laptop"), Some(parked), true);
        assert_eq!(plan.continued.unwrap().1, id("codex-thread"));
        assert_eq!(plan.parked_source.unwrap().provider, "claude");
        assert_eq!(
            plan.handoff,
            Handoff::Since(Some(AgentInvocationId::new("codex-last").unwrap()))
        );
    }

    #[test]
    fn a_conversation_without_transfer_restarts_from_the_log_on_another_route() {
        let plan = plan(current("claude", "laptop"), &route("claude", "server"), None, false);
        assert!(plan.continued.is_none());
        assert_eq!(plan.handoff, Handoff::Full);
    }
}
