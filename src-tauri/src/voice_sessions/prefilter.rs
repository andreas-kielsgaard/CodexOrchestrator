use super::{
    domain::*,
    input_buffer::{prepare_window, recent_inputs},
    VoiceSessionApplication,
};
use serde_json::{json, Value};
use std::sync::Arc;
impl VoiceSessionApplication {
    pub(crate) fn input(self: &Arc<Self>, id: String, text: String) -> Result<Value, String> {
        if text.trim().is_empty() {
            return Ok(json!({"ignored":"empty transcript"}));
        }
        self.update(|state| {
            if !state.enabled {
                return Err("Voice connection is stopped".into());
            }
            let c = state
                .conversation
                .as_mut()
                .ok_or("No active voice conversation")?;
            if !c.inputs.iter().any(|i| i.id == id) {
                c.inputs.push(SpokenInput {
                    id: id.clone(),
                    text: text.clone(),
                    created_at: chrono::Utc::now().to_rfc3339(),
                    revision: 1,
                    ..Default::default()
                });
            }
            Ok(())
        })?;
        let _ = self
            .events
            .send(json!({"type":"transcript","id":id,"text":text}));
        self.request_filter()?;
        Ok(json!({"queued":true,"inputId":id}))
    }
    pub(super) fn request_filter(self: &Arc<Self>) -> Result<(), String> {
        self.update(|state| {
            if !state.enabled {
                return Ok(());
            }
            let c = state.conversation.as_mut().ok_or("No conversation")?;
            prepare_window(c, state.input_policy.context_seconds);
            Ok(())
        })?;
        let id = self
            .snapshot()?
            .conversation
            .and_then(|c| c.workflow_instance_id)
            .ok_or("No Voice instance")?;
        let app = if self.instance_only {
            self.clone()
        } else {
            self.runtime_instance(&id)?
        };
        tauri::async_runtime::spawn_blocking(move || {
            let Ok(_lane) = app.input_dispatch.lock() else {
                return;
            };
            let mut retries = 0;
            loop {
                let Ok(state) = app.snapshot() else { break };
                let Some(c) = state.conversation else { break };
                if !state.enabled || !c.inputs.iter().any(|i| i.filter_pending) {
                    break;
                }
                if let Err(error) = app.emit_voice_event(
                    "transcript",
                    "transcript",
                    "window",
                    json!({"conversationId":c.id}),
                ) {
                    let _ = app.update(|s| {
                        if let Some(current) = &mut s.conversation {
                            if current.id == c.id {
                                current.input_buffer.complete = false;
                                for input in &mut current.inputs {
                                    if recent_inputs(&c, state.input_policy.context_seconds)
                                        .iter()
                                        .any(|i| i.id == input.id && i.revision == input.revision)
                                    {
                                        input.evaluation_failed = true;
                                        input.filter_pending =
                                            retries == 0 && !error.contains("timed out");
                                    }
                                }
                                s.filter_error = Some(error.clone());
                            }
                        }
                        Ok(())
                    });
                    if retries == 0 && !error.contains("timed out") {
                        retries += 1;
                        continue;
                    }
                    break;
                }
                let _ = app.update(|s| {
                    if let Some(current) = &mut s.conversation {
                        for input in &mut current.inputs {
                            if c.inputs
                                .iter()
                                .any(|old| old.id == input.id && old.revision == input.revision)
                            {
                                input.filter_pending = false;
                            }
                        }
                    }
                    Ok(())
                });
                break;
            }
        });
        Ok(())
    }
}
