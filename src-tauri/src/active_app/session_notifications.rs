//! Routes persisted session notifications to product observers and the UI.

use std::sync::{Arc, Mutex, Weak};

pub(super) struct CompletionSoundPolicy {
    pub(super) voice: Arc<Mutex<Option<Weak<crate::voice_sessions::VoiceSessionApplication>>>>,
    pub(super) workflow: Arc<Mutex<Option<Weak<crate::workflows::execution::WorkflowExecutionService>>>>,
}
impl CompletionSoundPolicy {
    pub(super) fn allows(&self, session: &str) -> bool {
        let voice = self.voice.lock().ok().and_then(|slot| slot.clone()).and_then(|owner| owner.upgrade());
        if let Some(enabled) = voice.and_then(|voice| voice.completion_sound_for_session(session)) { return enabled; }
        self.workflow.lock().ok().and_then(|slot| slot.clone()).and_then(|owner| owner.upgrade())
            .and_then(|workflow| workflow.completion_sound_for_session(session)).unwrap_or(true)
    }
}

pub(super) struct SessionNotificationFanout {
    pub(super) inner: Arc<dyn crate::agent_sessions::application::AgentSessionNotifier>,
    pub(super) registry: Arc<crate::orchestration::application::ManagedPlanBuilderRegistry>,
    pub(super) transition: Arc<
        Mutex<
            Option<
                Weak<crate::orchestration::bootstrap_transition::PostConfirmationTransitionService>,
            >,
        >,
    >,
    pub(super) sprint_transition: Arc<
        Mutex<
            Option<
                Weak<crate::orchestration::sprint_runner_transition::SprintRunnerTransitionService>,
            >,
        >,
    >,
    pub(super) workflow_execution:
        Arc<Mutex<Option<Weak<crate::workflows::execution::WorkflowExecutionService>>>>,
    pub(super) voice: Arc<Mutex<Option<Weak<crate::voice_sessions::VoiceSessionApplication>>>>,
}
impl crate::agent_sessions::application::AgentSessionNotifier for SessionNotificationFanout {
    fn notify(
        &self,
        notification: crate::agent_sessions::application::AgentSessionNotification,
    ) -> Result<(), String> {
        let voice=self.voice.lock().ok().and_then(|slot|slot.clone()).and_then(|service|service.upgrade());
        if let Some(voice)=voice { voice.on_notification(&notification); }
        if let crate::agent_sessions::application::AgentSessionNotification::InvocationTerminal {
            invocation,
            ..
        } = &notification
        {
            self.registry.on_terminal(invocation);
        }
        let execution = self
            .workflow_execution
            .lock()
            .ok()
            .and_then(|slot| slot.clone())
            .and_then(|service| service.upgrade());
        if let Some(execution) = execution {
            // Handoff failures have their own stored attempt; never relabel the sender's result.
            let _ = execution.on_agent_notification(&notification);
        }
        // Runtime launch provenance is persisted synchronously before the process start returns.
        // A Bootstrap-terminal transition can therefore launch the Runner and re-enter this
        // notifier before the outer notification completes. Never retain a registry lock while
        // dispatching that callback.
        let transition = {
            self.transition
                .lock()
                .map_err(|_| "post-confirmation notification registry is unavailable".to_string())?
                .clone()
        };
        let transition_error = transition
            .and_then(|service| service.upgrade())
            .map(|service| service.on_agent_notification(&notification))
            .transpose()
            .err()
            .map(|error| error.to_string());
        let sprint_transition = {
            self.sprint_transition
                .lock()
                .map_err(|_| "Sprint Runner notification registry is unavailable".to_string())?
                .clone()
        };
        let sprint_transition_error = sprint_transition
            .and_then(|service| service.upgrade())
            .map(|service| service.on_agent_notification(&notification))
            .transpose()
            .err()
            .map(|error| error.to_string());
        let inner_error = self.inner.notify(notification).err();
        match (transition_error, sprint_transition_error, inner_error) {
            (None, None, None) => Ok(()),
            (Some(error), None, None) | (None, Some(error), None) | (None, None, Some(error)) => {
                Err(error)
            }
            (transition, sprint, inner) => Err([transition, sprint, inner]
                .into_iter()
                .flatten()
                .collect::<Vec<_>>()
                .join("; ")),
        }
    }
}
