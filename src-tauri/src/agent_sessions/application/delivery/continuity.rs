//! Which native conversation a destination instance continues, and what its provider has not seen.
use crate::agent_sessions::{
    application::delivery::context::Handoff, domain::ProviderSessionBinding,
    ports::AgentSessionHistory,
};
use crate::agent_sessions::{
    application::{AgentSessionApplication, AgentSessionApplicationError},
    domain::{AgentInvocationId, ExternalRuntimeContextId},
    preparation::SessionPreparation,
};
use crate::execution_targets::domain::ExecutionBinding;

#[derive(Debug, PartialEq)]
pub(crate) struct ConversationPlan {
    /// The native conversation to continue on its owning route.
    pub(crate) continued: Option<(ExecutionBinding, ExternalRuntimeContextId)>,
    /// The current provider session retained when the destination differs.
    pub(crate) source_provider_binding: Option<ProviderSessionBinding>,
    pub(crate) handoff: Handoff,
}

/// The Session's current conversation, before the destination instance is prepared.
pub(super) struct CurrentConversation {
    pub(super) conversation: Option<(ExecutionBinding, ExternalRuntimeContextId)>,
    pub(super) runtime_version: Option<String>,
    /// The Session's latest invocation before the one being prepared.
    pub(super) last_invocation_id: Option<AgentInvocationId>,
}

/// Continue only the provider session owned by the exact destination route. Every other provider,
/// configuration, or device is a separate instance and receives the canonical missed history.
pub(super) fn plan(
    current: CurrentConversation,
    destination: &ExecutionBinding,
    cached_destination: Option<ProviderSessionBinding>,
) -> ConversationPlan {
    if current
        .conversation
        .as_ref()
        .is_some_and(|(location, _)| location == destination)
    {
        return ConversationPlan {
            continued: current.conversation,
            source_provider_binding: None,
            handoff: Handoff::None,
        };
    }
    let source_provider_binding =
        current
            .conversation
            .map(|(location, external_context_id)| ProviderSessionBinding {
                external_context_id,
                runtime_version: current.runtime_version,
                location,
                last_invocation_id: current.last_invocation_id,
            });
    match cached_destination {
        Some(cached) => ConversationPlan {
            continued: Some((cached.location, cached.external_context_id)),
            source_provider_binding,
            handoff: Handoff::Since(cached.last_invocation_id),
        },
        None => ConversationPlan {
            continued: None,
            source_provider_binding,
            handoff: Handoff::Full,
        },
    }
}

impl AgentSessionApplication {
    /// `source` is where the Session's current conversation lives, when it has one.
    pub(crate) fn plan_conversation(
        &self,
        history: &AgentSessionHistory,
        p: &SessionPreparation,
        source: Option<ExecutionBinding>,
        destination: &ExecutionBinding,
    ) -> Result<ConversationPlan, AgentSessionApplicationError> {
        let cached_destination = if source.as_ref() == Some(destination) {
            None
        } else {
            self.repository
                .provider_session_binding(&history.session.id, destination)
                .map_err(AgentSessionApplicationError::repository)?
        };
        let current = CurrentConversation {
            conversation: source.zip(p.source_binding.external_context_id.clone()),
            runtime_version: p.source_binding.runtime_version.clone(),
            last_invocation_id: history
                .invocations
                .iter()
                .take_while(|entry| entry.invocation.id != p.invocation_id)
                .filter(|entry| entry.launch_accepted_at.is_some())
                .last()
                .map(|entry| entry.invocation.id.clone()),
        };
        Ok(plan(current, destination, cached_destination))
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
            conversation: Some((route(provider, device), id("current-thread"))),
            runtime_version: None,
            last_invocation_id: Some(AgentInvocationId::new("previous").unwrap()),
        }
    }

    #[test]
    fn the_same_provider_on_another_device_starts_a_distinct_session() {
        let plan = plan(current("codex", "laptop"), &route("codex", "server"), None);
        assert!(plan.continued.is_none());
        assert_eq!(
            plan.source_provider_binding.unwrap().location,
            route("codex", "laptop")
        );
        assert_eq!(plan.handoff, Handoff::Full);
    }

    #[test]
    fn a_new_provider_parks_the_current_conversation_and_starts_from_the_log() {
        let plan = plan(current("codex", "laptop"), &route("claude", "laptop"), None);
        assert!(plan.continued.is_none());
        let parked = plan.source_provider_binding.unwrap();
        assert_eq!(parked.external_context_id, id("current-thread"));
        assert_eq!(parked.last_invocation_id.unwrap().as_str(), "previous");
        assert_eq!(plan.handoff, Handoff::Full);
    }

    #[test]
    fn a_returning_provider_continues_its_parked_conversation_with_what_it_missed() {
        let parked = ProviderSessionBinding {
            external_context_id: id("codex-thread"),
            runtime_version: None,
            location: route("codex", "laptop"),
            last_invocation_id: Some(AgentInvocationId::new("codex-last").unwrap()),
        };
        let plan = plan(
            current("claude", "laptop"),
            &route("codex", "laptop"),
            Some(parked),
        );
        assert_eq!(plan.continued.unwrap().1, id("codex-thread"));
        assert_eq!(
            plan.source_provider_binding.unwrap().location.provider,
            "claude"
        );
        assert_eq!(
            plan.handoff,
            Handoff::Since(Some(AgentInvocationId::new("codex-last").unwrap()))
        );
    }

    #[test]
    fn the_exact_current_route_continues_without_history_handoff() {
        let plan = plan(
            current("claude", "laptop"),
            &route("claude", "laptop"),
            None,
        );
        assert_eq!(plan.continued.unwrap().1, id("current-thread"));
        assert!(plan.source_provider_binding.is_none());
        assert_eq!(plan.handoff, Handoff::None);
    }
}
