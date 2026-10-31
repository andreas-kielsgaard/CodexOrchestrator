use super::VoiceSessionApplication;
use crate::agent_sessions::application::AgentSessionNotification;
use serde_json::json;
use std::sync::Arc;
impl VoiceSessionApplication {
    pub(crate) fn on_notification(self: &Arc<Self>, notification: &AgentSessionNotification) {
        if !self.instance_only {
            let instances = self
                .instances
                .lock()
                .map(|items| items.values().cloned().collect::<Vec<_>>())
                .unwrap_or_default();
            for instance in instances {
                instance.on_notification(notification);
            }
            return;
        }
        let Ok(state) = self.snapshot() else { return };
        let Some(c) = state.conversation else { return };
        if c.workflow_instance_id.is_none() {
            return;
        }
        if let AgentSessionNotification::InvocationTerminal {
            session_id,
            invocation,
        } = notification
        {
            let _ = self.update(|s| {
                if let Some(current) = &mut s.conversation {
                    if current.id != c.id {
                        return Ok(());
                    }
                    for d in current.delegations.iter_mut().filter(|d| {
                        d.session_id == session_id.as_str()
                            && d.invocation_id.as_deref() == Some(invocation.id.as_str())
                    }) {
                        d.runtime_status = Some(invocation.status);
                    }
                }
                Ok(())
            });
            if c.observed_sessions.contains(&session_id.to_string()) {
                let data = json!({"sessionId":session_id,"invocation":invocation});
                if let Err(error) = self.emit_voice_event(
                    "external_work",
                    "observe",
                    "observed",
                    json!({"data":data}),
                ) {
                    self.set_error(error);
                }
            }
        }
    }
    pub(crate) fn on_build(self: &Arc<Self>, build: serde_json::Value) {
        if self.snapshot().is_ok_and(|s| s.enabled) {
            if let Err(error) = self.emit_voice_event(
                "external_work",
                "observe",
                "observed",
                json!({"data":build}),
            ) {
                self.set_error(error);
            }
        }
    }
}
