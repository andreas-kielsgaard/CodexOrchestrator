//! Which native conversation a destination instance continues, and what its provider has not seen.
use crate::agent_sessions::{
    application::delivery::context::Handoff,
    domain::{ProviderSessionBinding, ProviderWorkspaceIdentity},
    ports::AgentSessionHistory,
};
use crate::agent_sessions::{
    application::{AgentSessionApplication, AgentSessionApplicationError},
    domain::{AgentInvocationId, ExternalRuntimeContextId},
    preparation::SessionPreparation,
};
use crate::execution_targets::domain::{ExecutionBinding, SessionExecutionTarget};

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
    pub(super) conversation: Option<(
        ExecutionBinding,
        Option<ProviderWorkspaceIdentity>,
        ExternalRuntimeContextId,
    )>,
    pub(super) runtime_version: Option<String>,
    /// The Session's latest invocation before the one being prepared.
    pub(super) last_invocation_id: Option<AgentInvocationId>,
}

/// Continue only the provider session owned by the exact destination route. Every other provider,
/// configuration, or device is a separate instance and receives the canonical missed history.
pub(super) fn plan(
    current: CurrentConversation,
    destination: &SessionExecutionTarget,
    cached_destination: Option<ProviderSessionBinding>,
) -> ConversationPlan {
    let destination_workspace = ProviderWorkspaceIdentity::from(destination);
    if current
        .conversation
        .as_ref()
        .is_some_and(|(location, workspace, _)| {
            location == &destination.execution && workspace.as_ref() == Some(&destination_workspace)
        })
    {
        return ConversationPlan {
            continued: current
                .conversation
                .map(|(location, _, external_context_id)| (location, external_context_id)),
            source_provider_binding: None,
            handoff: Handoff::None,
        };
    }
    let source_provider_binding =
        current
            .conversation
            .map(
                |(location, workspace, external_context_id)| ProviderSessionBinding {
                    external_context_id,
                    runtime_version: current.runtime_version,
                    location,
                    workspace,
                    last_invocation_id: current.last_invocation_id,
                },
            );
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
        source: Option<(ExecutionBinding, Option<ProviderWorkspaceIdentity>)>,
        destination: &SessionExecutionTarget,
    ) -> Result<ConversationPlan, AgentSessionApplicationError> {
        let destination_workspace = ProviderWorkspaceIdentity::from(destination);
        let cached_destination = if source.as_ref().is_some_and(|(execution, workspace)| {
            execution == &destination.execution
                && workspace.as_ref() == Some(&destination_workspace)
        }) {
            None
        } else {
            self.repository
                .provider_session_binding(&history.session.id, destination)
                .map_err(AgentSessionApplicationError::repository)?
        };
        let current = CurrentConversation {
            conversation: source
                .zip(p.source_binding.external_context_id.clone())
                .map(|((execution, workspace), id)| (execution, workspace, id)),
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

    fn target(provider: &str, device: &str, worktree: &str) -> SessionExecutionTarget {
        SessionExecutionTarget {
            capability_profile_id: "profile".into(),
            capability_profile_revision: 1,
            execution: route(provider, device),
            repository_id: "repository".into(),
            branch_ref: "refs/heads/main".into(),
            worktree_id: worktree.into(),
            path: format!("/{device}/{worktree}"),
            head: None,
        }
    }

    fn current(provider: &str, device: &str) -> CurrentConversation {
        let target = target(provider, device, "current-worktree");
        CurrentConversation {
            conversation: Some((
                target.execution.clone(),
                Some(ProviderWorkspaceIdentity::from(&target)),
                id("current-thread"),
            )),
            runtime_version: None,
            last_invocation_id: Some(AgentInvocationId::new("previous").unwrap()),
        }
    }

    #[test]
    fn the_same_provider_on_another_device_starts_a_distinct_session() {
        let plan = plan(
            current("codex", "laptop"),
            &target("codex", "server", "server-worktree"),
            None,
        );
        assert!(plan.continued.is_none());
        assert_eq!(
            plan.source_provider_binding.unwrap().location,
            route("codex", "laptop")
        );
        assert_eq!(plan.handoff, Handoff::Full);
    }

    #[test]
    fn a_new_provider_parks_the_current_conversation_and_starts_from_the_log() {
        let plan = plan(
            current("codex", "laptop"),
            &target("claude", "laptop", "current-worktree"),
            None,
        );
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
            workspace: Some(ProviderWorkspaceIdentity::from(&target(
                "codex",
                "laptop",
                "return-worktree",
            ))),
            last_invocation_id: Some(AgentInvocationId::new("codex-last").unwrap()),
        };
        let plan = plan(
            current("claude", "laptop"),
            &target("codex", "laptop", "return-worktree"),
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
            &target("claude", "laptop", "current-worktree"),
            None,
        );
        assert_eq!(plan.continued.unwrap().1, id("current-thread"));
        assert!(plan.source_provider_binding.is_none());
        assert_eq!(plan.handoff, Handoff::None);
    }

    #[test]
    fn another_worktree_on_the_same_route_starts_a_distinct_provider_session() {
        let plan = plan(
            current("codex", "laptop"),
            &target("codex", "laptop", "other-worktree"),
            None,
        );
        assert!(plan.continued.is_none());
        assert_eq!(plan.handoff, Handoff::Full);
    }
}
