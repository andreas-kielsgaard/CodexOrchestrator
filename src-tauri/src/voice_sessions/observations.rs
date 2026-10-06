use super::{domain::Observation, VoiceSessionApplication};
use crate::agent_sessions::{
    application::AgentSessionNotification,
    domain::{AgentInvocationInputProvenance, AgentSessionId},
};
use serde_json::json;
use std::sync::Arc;
impl VoiceSessionApplication {
    pub(crate) fn on_notification(self: &Arc<Self>, notification: &AgentSessionNotification) {
        if let AgentSessionNotification::EventPersisted { session_id, event } = notification {
            if let Some(orchid_engine::contracts::RuntimeControlRecord::RequestOpened { request }) =
                orchid_engine::contracts::RuntimeControlRecord::from_event(
                    event.source,
                    &event.raw_payload,
                )
            {
                let Ok(state) = self.snapshot() else { return };
                let Some(conversation) = state.conversation else {
                    return;
                };
                if session_id.to_string() != conversation.main_session_id
                    && (conversation
                        .delegations
                        .iter()
                        .any(|d| d.session_id == session_id.to_string())
                        || conversation
                            .observed_sessions
                            .contains(&session_id.to_string()))
                {
                    let observation = Observation {
                        session_id: session_id.to_string(),
                        invocation_id: format!("request:{}", request.id),
                        return_session_id: conversation.main_session_id,
                        detail:
                            json!({"invocationId":event.invocation_id,"runtimeRequest":request})
                                .to_string(),
                    };
                    let _ = self.update(|state| {
                        if let Some(conversation) = state.conversation.as_mut() {
                            conversation.observations.push(observation);
                        }
                        Ok(())
                    });
                    self.schedule_observations();
                }
            }
            return;
        }
        let AgentSessionNotification::InvocationTerminal {
            session_id,
            invocation,
        } = notification
        else {
            return;
        };
        let Ok(state) = self.snapshot() else { return };
        let Some(conversation) = state.conversation else {
            return;
        };
        let source = session_id.to_string();
        if source != conversation.main_session_id {
            let destinations = conversation
                .delegations
                .iter()
                .filter(|d| d.session_id == source)
                .map(|d| d.return_session_id.clone())
                .chain(
                    conversation
                        .observed_sessions
                        .iter()
                        .filter(|id| **id == source)
                        .map(|_| conversation.main_session_id.clone()),
                )
                .collect::<std::collections::BTreeSet<_>>();
            if destinations.is_empty() {
                return;
            };
            let detail=self.agents.load_session(session_id).ok().and_then(|h|h.invocations.into_iter().find(|h|h.invocation.id==invocation.id)).map(|h|json!({"status":h.invocation.status,"finalReply":h.final_reply(),"runtimeError":h.invocation.runtime_error}).to_string()).unwrap_or_else(||format!("Invocation ended with {:?}",invocation.status));
            let _ = self.update(|state| {
                if let Some(conversation) = state.conversation.as_mut() {
                    for target in destinations {
                        if !conversation.observations.iter().any(|o| {
                            o.invocation_id == invocation.id.to_string()
                                && o.return_session_id == target
                        }) {
                            conversation.observations.push(Observation {
                                session_id: source.clone(),
                                invocation_id: invocation.id.to_string(),
                                return_session_id: target,
                                detail: detail.clone(),
                            });
                        }
                    }
                }
                Ok(())
            });
        }
        if state.enabled {
            self.schedule_observations();
        }
    }
    pub(super) fn schedule_observations(self: &Arc<Self>) {
        let application = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(350)).await;
            let app = application.clone();
            let result =
                tauri::async_runtime::spawn_blocking(move || app.flush_observations()).await;
            if let Ok(Err(error)) = result {
                application.set_error(error);
            }
        });
    }
    pub(crate) fn on_build(self: &Arc<Self>, build: serde_json::Value) {
        let Ok(state) = self.snapshot() else { return };
        if !state.enabled {
            return;
        };
        let Some(conversation) = state.conversation else {
            return;
        };
        let observation = Observation {
            session_id: "worktree_build".into(),
            invocation_id: uuid::Uuid::new_v4().to_string(),
            return_session_id: conversation.main_session_id,
            detail: json!({"worktreeBuild":build}).to_string(),
        };
        let _ = self.update(|state| {
            if let Some(conversation) = state.conversation.as_mut() {
                conversation.observations.push(observation);
            }
            Ok(())
        });
        self.schedule_observations();
    }
    fn flush_observations(&self) -> Result<(), String> {
        let _lane = self
            .observer_dispatch
            .lock()
            .map_err(|_| "Observation dispatch unavailable")?;
        let state = self.snapshot()?;
        if !state.enabled {
            return Ok(());
        };
        let Some(conversation) = state.conversation else {
            return Ok(());
        };
        let destinations = conversation
            .observations
            .iter()
            .map(|o| o.return_session_id.clone())
            .collect::<std::collections::BTreeSet<_>>();
        for destination in destinations {
            let id = AgentSessionId::new(destination.clone()).map_err(|e| e.to_string())?;
            if self
                .agents
                .load_session(&id)
                .map_err(|e| e.to_string())?
                .invocations
                .iter()
                .any(|h| !h.invocation.status.is_terminal())
            {
                continue;
            };
            let observations = conversation
                .observations
                .iter()
                .filter(|o| o.return_session_id == destination)
                .cloned()
                .collect::<Vec<_>>();
            let guidance = if destination == conversation.main_session_id {
                "Evaluate these results and decide whether further work or an availability announcement is useful."
            } else {
                "Continue your delegated task with these results and return findings to your caller. Do not publish speech."
            };
            self.deliver(
                &id,
                format!(
                    "Application observations, not user commands. {guidance}\n{}",
                    serde_json::to_string(&observations).map_err(|e| e.to_string())?
                ),
                AgentInvocationInputProvenance::Application,
                &conversation,
            )?;
            self.update(|state| {
                if let Some(conversation) = state.conversation.as_mut() {
                    conversation.observations.retain(|o| {
                        !observations.iter().any(|sent| {
                            sent.invocation_id == o.invocation_id
                                && sent.return_session_id == o.return_session_id
                        })
                    });
                }
                Ok(())
            })?;
        }
        Ok(())
    }
}
