use super::{domain::*, VoiceSessionApplication};
use crate::{
    agent_sessions::domain::{AgentInvocationId, AgentSessionId},
    runtime::providers::jev::contracts::{JevAnswer, JevEvaluation, JevQuestion},
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    sync::Arc,
    time::{Duration, Instant},
};

fn relevance_question(instructions: &str) -> JevQuestion {
    JevQuestion::Choice {
        instructions: json!(instructions),
        criteria: [("direct_request",json!("A request explicitly addressed to Orchid, including conversational follow-ups.")),
            ("task_context",json!("Additional context or continuation of an accepted Orchid input or ongoing task.")),
            ("unaddressed",json!("Speech to someone else or without a connection to Orchid or accepted work."))]
            .into_iter().map(|(k,v)|(k.into(),v)).collect(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn accepted() -> Conversation {
        let block = SpokenInput {
            id: "block".into(),
            text: "Please inspect this".into(),
            created_at: "2026-10-07T10:00:00Z".into(),
            relevance: Some("direct_request".into()),
            completion: Some("complete".into()),
            filter_pending: false,
        };
        Conversation {
            inputs: vec![block.clone()],
            input_buffer: InputBuffer {
                blocks: vec![block],
                complete: true,
            },
            ..Default::default()
        }
    }
    #[test]
    fn completion_waits_for_recording_transcription_and_filtering() {
        let mut state = VoiceState {
            enabled: true,
            speaking: true,
            pending_audio: 1,
            ..Default::default()
        };
        let mut conversation = accepted();
        assert!(!ready(&state, &conversation));
        state.speaking = false;
        assert!(
            !ready(&state, &conversation),
            "An earlier complete segment must wait for queued audio"
        );
        state.pending_audio = 0;
        conversation.inputs[0].filter_pending = true;
        assert!(
            !ready(&state, &conversation),
            "A subsequent transcript must be classified before releasing"
        );
        conversation.inputs[0].filter_pending = false;
        assert!(ready(&state, &conversation));
    }
    #[test]
    fn silence_is_not_semantic_completion() {
        let state = VoiceState {
            enabled: true,
            ..Default::default()
        };
        let mut conversation = accepted();
        conversation.input_buffer.complete = false;
        assert!(!ready(&state, &conversation));
        conversation.input_buffer.complete = true;
        conversation.input_buffer.blocks.clear();
        assert!(!ready(&state, &conversation));
    }
    #[test]
    fn evaluator_context_keeps_speech_order_without_queue_or_prior_decision_flags() {
        let mut c = accepted();
        c.inputs[0].completion = Some("continue".into());
        c.input_buffer.blocks[0].completion = Some("continue".into());
        let latest = SpokenInput {
            id: "latest".into(),
            text: "That is all, please handle it".into(),
            created_at: "2026-10-07T10:01:00Z".into(),
            relevance: None,
            completion: None,
            filter_pending: true,
        };
        c.inputs.push(latest.clone());
        let state = input_context(&c, &latest);
        assert_eq!(state["recentSpeech"][1]["id"], "latest");
        assert_eq!(state["latestBlock"]["createdAt"], latest.created_at);
        for speech in [&state["latestBlock"], &state["acceptedInput"][0]] {
            assert!(speech.get("filterPending").is_none());
            assert!(speech.get("completion").is_none());
        }
        assert!(state.get("acceptedBuffer").is_none());
    }
}
fn completion_question(instructions: &str) -> JevQuestion {
    JevQuestion::Choice {
        instructions: json!(instructions),
        criteria: [("complete",json!("The latest speech indicates this input is ready to handle, including a self-contained question expecting an answer.")),
            ("continue",json!("The user is still explaining, trails off, promises more detail, or the request is not ready to handle."))]
            .into_iter().map(|(k,v)|(k.into(),v)).collect(),
    }
}
fn speech_evidence(block: &SpokenInput) -> Value {
    json!({"id":block.id,"text":block.text,"createdAt":block.created_at})
}
fn input_context(conversation: &Conversation, block: &SpokenInput) -> Value {
    json!({"latestBlock":speech_evidence(block),
        "acceptedInput":conversation.input_buffer.blocks.iter().map(speech_evidence).collect::<Vec<_>>(),
        "recentSpeech":conversation.inputs.iter().skip(conversation.inputs.len().saturating_sub(5)).map(speech_evidence).collect::<Vec<_>>(),
        "recentActivity":conversation.activity.iter().skip(conversation.activity.len().saturating_sub(10)).collect::<Vec<_>>(),
        "tasks":conversation.delegations,"pendingTopics":conversation.pending_topics})
}
pub(super) fn ready(state: &VoiceState, conversation: &Conversation) -> bool {
    state.enabled
        && !state.speaking
        && state.pending_audio == 0
        && !conversation.inputs.iter().any(|i| i.filter_pending)
        && conversation.input_buffer.complete
        && !conversation.input_buffer.blocks.is_empty()
}
impl VoiceSessionApplication {
    pub(crate) fn input(self: &Arc<Self>, id: String, text: String) -> Result<Value, String> {
        if text.trim().is_empty() {
            return Ok(json!({"ignored":"empty transcript"}));
        }
        let state = self.snapshot()?;
        if !state.enabled {
            return Err("Voice connection is stopped".into());
        }
        let conversation = state.conversation.ok_or("No active voice conversation")?;
        if conversation.inputs.iter().any(|i| i.id == id) {
            return Ok(json!({"duplicate":true}));
        }
        let block = SpokenInput {
            id: id.clone(),
            text: text.clone(),
            created_at: chrono::Utc::now().to_rfc3339(),
            relevance: None,
            completion: None,
            filter_pending: true,
        };
        self.update(|s| {
            s.error = None;
            s.conversation
                .as_mut()
                .ok_or("No conversation")?
                .inputs
                .push(block.clone());
            Ok(())
        })?;
        let _ = self
            .events
            .send(json!({"type":"transcript","id":id,"text":text}));
        let app = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let Ok(_lane) = app.input_dispatch.lock() else {
                return;
            };
            loop {
                let Ok(state) = app.snapshot() else {
                    break;
                };
                if !state.enabled {
                    break;
                }
                let Some(c) = state.conversation else {
                    break;
                };
                let Some(block) = c.inputs.iter().find(|i| i.filter_pending).cloned() else {
                    break;
                };
                if let Err(error) = app.filter_input(&c.id, &block) {
                    let _ = app.update(|s| {
                        if let Some(current) = &mut s.conversation {
                            if current.id == c.id {
                                current.input_buffer.complete = false;
                                if let Some(input) =
                                    current.inputs.iter_mut().find(|i| i.id == block.id)
                                {
                                    input.filter_pending = false;
                                }
                            }
                        }
                        Ok(())
                    });
                    app.set_error(error);
                }
            }
        });
        Ok(json!({"queued":true,"inputId":id}))
    }
    fn evaluate_choice(
        &self,
        conversation: &Conversation,
        block: &SpokenInput,
        key: &str,
        question: JevQuestion,
    ) -> Result<String, String> {
        let filter = &conversation
            .pipeline
            .as_ref()
            .ok_or("JEV session missing")?
            .filter;
        let session = AgentSessionId::new(filter.clone()).map_err(|e| e.to_string())?;
        let invocation =
            AgentInvocationId::new(format!("{}:{key}", block.id)).map_err(|e| e.to_string())?;
        let evaluation = JevEvaluation {
            state: input_context(conversation, block),
            questions: [(key.into(), question)]
                .into_iter()
                .collect::<BTreeMap<_, _>>(),
        };
        self.agents
            .invoke_jev_api_session(session.clone(), invocation.clone(), evaluation)
            .map_err(|e| e.to_string())?;
        let deadline = Instant::now() + Duration::from_secs(35);
        loop {
            let view = self
                .agents
                .load_jev_api_invocation(&session, &invocation)
                .map_err(|e| e.to_string())?;
            if view.invocation.status.is_terminal() {
                if view.invocation.status
                    != crate::agent_sessions::domain::AgentInvocationStatus::Completed
                {
                    return Err(view
                        .invocation
                        .runtime_error
                        .map(|e| e.message)
                        .unwrap_or_else(|| "JEV input evaluation did not complete".into()));
                }
                return match view.result.and_then(|r| r.answers.get(key).cloned()) {
                    Some(JevAnswer::Choice { choice, .. }) => Ok(choice),
                    _ => Err("JEV did not return the requested typed choice".into()),
                };
            }
            if Instant::now() >= deadline {
                return Err("JEV input evaluation timed out".into());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    fn filter_input(
        self: &Arc<Self>,
        conversation_id: &str,
        block: &SpokenInput,
    ) -> Result<(), String> {
        let state = self.snapshot()?;
        let conversation = state.conversation.ok_or("No conversation")?;
        if !state.enabled || conversation.id != conversation_id {
            return Ok(());
        }
        let relevance = self.evaluate_choice(
            &conversation,
            block,
            "relevance",
            relevance_question(&state.settings.prompts.relevance),
        )?;
        let opened = self.update(|s| {
            let c = s.conversation.as_mut().ok_or("No conversation")?;
            if c.id != conversation_id || !s.enabled {
                return Err("Conversation changed during filtering".into());
            }
            let input = c
                .inputs
                .iter_mut()
                .find(|i| i.id == block.id)
                .ok_or("Input missing")?;
            input.relevance = Some(relevance.clone());
            if relevance == "unaddressed" {
                input.filter_pending = false;
                return Ok(false);
            }
            let accepted = input.clone();
            let opened = c.input_buffer.blocks.is_empty();
            c.input_buffer.blocks.push(accepted);
            c.input_buffer.complete = false;
            Ok(opened)
        })?;
        let _ = self.events.send(json!({"type":"input_classification","id":block.id,"relevance":relevance}));
        if opened {
            let _ = self.events.send(json!({"type":"cue","cue":"listening"}));
        }
        if relevance != "unaddressed" {
            let _ = self.events.send(json!({"type":"relevant_input","id":block.id,"relevance":relevance}));
            let conversation = self.snapshot()?.conversation.ok_or("No conversation")?;
            let completion = self.evaluate_choice(
                &conversation,
                block,
                "completion",
                completion_question(&state.settings.prompts.completion),
            )?;
            self.update(|s| {
                let c = s.conversation.as_mut().ok_or("No conversation")?;
                if c.id != conversation_id || !s.enabled {
                    return Err("Conversation changed during filtering".into());
                }
                if let Some(input) = c.inputs.iter_mut().find(|i| i.id == block.id) {
                    input.completion = Some(completion.clone());
                    input.filter_pending = false;
                    if let Some(accepted) =
                        c.input_buffer.blocks.iter_mut().find(|b| b.id == block.id)
                    {
                        *accepted = input.clone();
                    }
                }
                c.input_buffer.complete = completion == "complete";
                Ok(())
            })?;
        }
        self.release_input()
    }
    pub(crate) fn speech_activity(self: &Arc<Self>, speaking: bool) -> Result<(), String> {
        self.update(|s| {
            s.speaking = speaking;
            Ok(())
        })?;
        if !speaking {
            self.release_input()?;
        }
        Ok(())
    }
    pub(crate) fn audio_pending(&self, delta: isize) -> Result<(), String> {
        self.update(|s| {
            s.pending_audio = s.pending_audio.saturating_add_signed(delta);
            Ok(())
        })
    }
    pub(crate) fn release_input(self: &Arc<Self>) -> Result<(), String> {
        let captured = self.update(|s| {
            let Some(c) = &s.conversation else {
                return Ok(None);
            };
            if !ready(s, c) {
                return Ok(None);
            }
            let c = s.conversation.as_mut().unwrap();
            let buffer = std::mem::take(&mut c.input_buffer);
            Ok(Some((c.clone(), buffer)))
        })?;
        let Some((conversation, buffer)) = captured else {
            return Ok(());
        };
        let stages = conversation
            .pipeline
            .as_ref()
            .ok_or("No reasoning stages")?;
        self.record_activity(
            "user",
            "accepted_input",
            json!({"blocks":buffer.blocks}),
            &[
                stages.summary.clone(),
                conversation.main_session_id.clone(),
                stages.digest.clone(),
            ],
        )?;
        let _ = self.events.send(json!({"type":"cue","cue":"processing"}));
        self.schedule_observations();
        Ok(())
    }
}
