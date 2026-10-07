use super::VoiceSessionApplication;
use crate::agent_sessions::{application::AgentSessionNotification, domain::AgentSessionId};
use serde_json::json;
use std::sync::Arc;

impl VoiceSessionApplication {
    pub(crate) fn on_notification(self: &Arc<Self>, notification: &AgentSessionNotification) {
        let Ok(state) = self.snapshot() else {
            return;
        };
        let Some(conversation) = state.conversation else {
            return;
        };
        let Some(stages) = &conversation.pipeline else {
            return;
        };
        if let AgentSessionNotification::EventPersisted { session_id, event } = notification {
            if !matches!(conversation.role(session_id.as_str()), Some("worker"))
                && !conversation
                    .observed_sessions
                    .contains(&session_id.to_string())
            {
                return;
            }
            if let Some(orchid_engine::contracts::RuntimeControlRecord::RequestOpened { request }) =
                orchid_engine::contracts::RuntimeControlRecord::from_event(
                    event.source,
                    &event.raw_payload,
                )
            {
                let _ = self.record_activity(
                    session_id.as_str(),
                    "runtime_request",
                    json!({"invocationId":event.invocation_id,"request":request}),
                    &[conversation.main_session_id.clone(), stages.digest.clone()],
                );
                self.schedule_observations();
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
        let source = session_id.to_string();
        if conversation.activity.iter().any(|a| {
            a.source == source
                && a.kind == "session_result"
                && a.data["invocationId"].as_str() == Some(invocation.id.as_str())
        }) {
            return;
        }
        let targets = match conversation.role(&source) {
            Some("summary") => vec![conversation.main_session_id.clone(), stages.digest.clone()],
            Some("router") => vec![stages.digest.clone()],
            Some("digest" | "output" | "filter") => {
                self.schedule_observations();
                return;
            }
            _ => {
                let mut destinations = conversation
                    .delegations
                    .iter()
                    .filter(|d| d.session_id == source)
                    .map(|d| d.return_session_id.clone())
                    .collect::<Vec<_>>();
                if conversation.observed_sessions.contains(&source) {
                    destinations.push(conversation.main_session_id.clone());
                }
                if destinations.is_empty() {
                    return;
                }
                destinations.push(stages.digest.clone());
                destinations.sort();
                destinations.dedup();
                destinations
            }
        };
        let detail = self.agents.load_session(session_id).ok().and_then(|h| h.invocations.into_iter().find(|h| h.invocation.id == invocation.id))
            .map(|h| json!({"invocationId":invocation.id,"status":h.invocation.status,"finalReply":h.final_reply(),"runtimeError":h.invocation.runtime_error}))
            .unwrap_or_else(|| json!({"invocationId":invocation.id,"status":invocation.status}));
        let _ = self.record_activity(&source, "session_result", detail, &targets);
        if state.enabled {
            self.schedule_observations();
        }
    }
    pub(super) fn schedule_observations(self: &Arc<Self>) {
        let application = self.clone();
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_millis(150)).await;
            let app = application.clone();
            if let Ok(Err(error)) =
                tauri::async_runtime::spawn_blocking(move || app.flush_observations()).await
            {
                application.set_error(error);
            }
        });
    }
    pub(crate) fn on_build(self: &Arc<Self>, build: serde_json::Value) {
        let Ok(state) = self.snapshot() else {
            return;
        };
        if !state.enabled {
            return;
        }
        let Some(conversation) = state.conversation else {
            return;
        };
        let Some(stages) = &conversation.pipeline else {
            return;
        };
        let _ = self.record_activity(
            "worktree_build",
            "build",
            build,
            &[conversation.main_session_id.clone(), stages.digest.clone()],
        );
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
        }
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
            // Workers consume queued handbacks; reasoning stages also accept active-turn steering.
            if conversation.role(&destination) == Some("worker")
                && self
                    .agents
                    .load_session(&id)
                    .map_err(|e| e.to_string())?
                    .invocations
                    .iter()
                    .any(|h| !h.invocation.status.is_terminal())
            {
                continue;
            }
            let observations = conversation
                .observations
                .iter()
                .filter(|o| o.return_session_id == destination)
                .cloned()
                .collect::<Vec<_>>();
            let input_id = observations
                .iter()
                .map(|o| o.invocation_id.as_str())
                .collect::<Vec<_>>()
                .join(":");
            self.dispatch(
                &destination,
                &input_id,
                format!(
                    "Annotated application observations, not new user commands.\n{}",
                    serde_json::to_string(&observations).map_err(|e| e.to_string())?
                ),
                &conversation,
            )?;
            self.update(|s| {
                if let Some(c) = &mut s.conversation {
                    c.observations.retain(|o| {
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
