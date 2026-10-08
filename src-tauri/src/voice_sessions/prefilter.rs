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

fn question(instructions: &str, target: &str, criteria: &[(&str, &str)]) -> JevQuestion {
    JevQuestion::Choice {
        instructions: json!({"instructions":instructions,"sentenceId":target}),
        criteria: criteria
            .iter()
            .map(|(key, detail)| (key.to_string(), json!(detail)))
            .collect(),
    }
}
fn evidence(block: &SpokenInput) -> Value {
    json!({"id":block.id,"text":block.text,"createdAt":block.created_at,"revision":block.revision})
}
fn recent_inputs(c: &Conversation, seconds: u64) -> Vec<SpokenInput> {
    let cutoff = chrono::Utc::now() - chrono::Duration::seconds(seconds as i64);
    c.inputs
        .iter()
        .filter(|i| {
            !i.text.trim().is_empty()
                && chrono::DateTime::parse_from_rfc3339(&i.created_at).is_ok_and(|at| at >= cutoff)
        })
        .cloned()
        .collect()
}
fn recent_release(c: &Conversation, seconds: u64) -> Option<&InputRelease> {
    c.last_input_release.as_ref().filter(|release| {
        chrono::DateTime::parse_from_rfc3339(&release.created_at)
            .is_ok_and(|at| at >= chrono::Utc::now() - chrono::Duration::seconds(seconds as i64))
    })
}
fn sticky_relevance(previous: Option<&str>, next: &str) -> String {
    match (previous, next) {
        (Some("direct_request"), _) | (_, "direct_request") => "direct_request",
        (Some("task_context"), _) | (_, "task_context") => "task_context",
        _ => "unaddressed",
    }
    .into()
}
fn may_accumulate(input: &SpokenInput, continuation: bool) -> bool {
    input.delivered_revision == 0 || (continuation && input.revision > input.delivered_revision)
}
pub(super) fn ready(state: &VoiceState, c: &Conversation) -> bool {
    state.enabled
        && !c.inputs.iter().any(|i| i.filter_pending)
        && c.input_buffer.complete
        && !c.input_buffer.blocks.is_empty()
}
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
            let recent = recent_inputs(c, state.input_policy.context_seconds);
            for input in &mut c.inputs {
                input.filter_pending = recent.iter().any(|r| r.id == input.id);
            }
            c.input_buffer
                .blocks
                .retain(|b| recent.iter().any(|r| r.id == b.id));
            if !recent.is_empty() {
                c.input_buffer.complete = false;
            }
            Ok(())
        })?;
        let app = self.clone();
        tauri::async_runtime::spawn_blocking(move || {
            let Ok(_lane) = app.input_dispatch.lock() else {
                return;
            };
            loop {
                let Ok(state) = app.snapshot() else { break };
                let Some(c) = state.conversation else { break };
                if !state.enabled || !c.inputs.iter().any(|i| i.filter_pending) {
                    break;
                }
                if let Err(error) =
                    app.filter_window(&state.settings.prompts, &state.input_policy, &c)
                {
                    let _ = app.update(|s| {
                        if let Some(current) = &mut s.conversation {
                            if current.id == c.id {
                                current.input_buffer.complete = false;
                                for input in &mut current.inputs {
                                    input.filter_pending = false;
                                }
                            }
                        }
                        Ok(())
                    });
                    app.set_error(error);
                    break;
                }
            }
        });
        Ok(())
    }
    fn evaluate_window(
        &self,
        c: &Conversation,
        recent: &[SpokenInput],
        prompts: &VoicePrompts,
        policy: &VoiceInputPolicy,
    ) -> Result<BTreeMap<String, String>, String> {
        let filter = &c.pipeline.as_ref().ok_or("JEV session missing")?.filter;
        let session = AgentSessionId::new(filter.clone()).map_err(|e| e.to_string())?;
        let invocation =
            AgentInvocationId::new(uuid::Uuid::new_v4().to_string()).map_err(|e| e.to_string())?;
        let mut questions = BTreeMap::new();
        let released = recent_release(c, policy.followup_seconds);
        for block in recent {
            questions.insert(format!("relevance:{}", block.id), question(&prompts.relevance, &block.id, &[
                ("direct_request","A request explicitly addressed to Orchid, including conversational follow-ups."),
                ("task_context","Context or continuation of an Orchid request or task."),
                ("unaddressed","Speech to someone else or without a connection to Orchid.")]));
            questions.insert(format!("completion:{}",block.id), question(&prompts.completion,&block.id,&[
                ("complete","The request including this sentence is ready to handle in the current transcript context."),
                ("continue","The request is still being explained or is not ready to handle.")]));
            if released.is_some() {
                questions.insert(format!("continuation:{}",block.id), question(&prompts.continuation,&block.id,&[
                ("continued","This sentence continues or corrects the recently delivered request."),
                ("new_request","This sentence belongs to a different request."),
                ("unrelated","This sentence is unrelated to the recently delivered request.")]));
            }
        }
        let evaluation = JevEvaluation {
            state: json!({"recentSpeech":recent.iter().map(evidence).collect::<Vec<_>>(),
            "acceptedInput":c.input_buffer.blocks.iter().map(evidence).collect::<Vec<_>>(),"lastDeliveredRequest":released,
            "recentActivity":c.activity.iter().rev().take(10).collect::<Vec<_>>(),"tasks":c.delegations,"pendingTopics":c.pending_topics}),
            questions,
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
                        .unwrap_or_else(|| "JEV evaluation did not complete".into()));
                }
                return view
                    .result
                    .ok_or("JEV result missing".to_string())?
                    .answers
                    .into_iter()
                    .map(|(key, answer)| match answer {
                        JevAnswer::Choice { choice, .. } => Ok((key, choice)),
                        _ => Err("JEV returned an unexpected answer type".into()),
                    })
                    .collect();
            }
            if Instant::now() >= deadline {
                return Err("JEV input evaluation timed out".into());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    fn filter_window(
        self: &Arc<Self>,
        prompts: &VoicePrompts,
        policy: &VoiceInputPolicy,
        evaluated: &Conversation,
    ) -> Result<(), String> {
        let recent = recent_inputs(evaluated, policy.context_seconds);
        if recent.is_empty() {
            self.update(|s| {
                if let Some(c) = &mut s.conversation {
                    for input in &mut c.inputs {
                        input.filter_pending = false;
                    }
                }
                Ok(())
            })?;
            return Ok(());
        }
        let answers = self.evaluate_window(evaluated, &recent, prompts, policy)?;
        let (opened, relevant, classifications, continued) = self.update(|state| {
            if !state.enabled {
                return Err("Voice connection stopped during filtering".into());
            }
            let c = state.conversation.as_mut().ok_or("No conversation")?;
            if c.id != evaluated.id {
                return Err("Conversation changed during filtering".into());
            }
            let was_empty = c.input_buffer.blocks.is_empty();
            let mut relevant = vec![];
            let mut classifications = vec![];
            let mut continued = false;
            for block in &recent {
                let input = c
                    .inputs
                    .iter_mut()
                    .find(|i| i.id == block.id)
                    .ok_or("Sentence missing")?;
                let relevance = answers
                    .get(&format!("relevance:{}", block.id))
                    .ok_or("Relevance answer missing")?;
                let effective = sticky_relevance(input.relevance.as_deref(), relevance);
                let accepted = effective != "unaddressed";
                let revised = input.revision > input.evaluated_revision;
                let continuation = answers
                    .get(&format!("continuation:{}", block.id))
                    .is_some_and(|v| v == "continued");
                let eligible = accepted && may_accumulate(input, continuation);
                if eligible
                    && (revised
                        || input.relevance.as_deref() == Some("unaddressed")
                        || input.relevance.is_none())
                {
                    relevant.push(input.id.clone());
                }
                input.relevance = Some(effective.clone());
                input.evaluated_revision = block.revision;
                input.filter_pending = input.revision != block.revision;
                input.completion = if input.filter_pending {
                    None
                } else {
                    Some(
                        answers
                            .get(&format!("completion:{}", block.id))
                            .ok_or("Completion answer missing")?
                            .clone(),
                    )
                };
                classifications
                    .push(json!({"id":input.id,"relevance":effective,"revision":input.revision}));
                if eligible {
                    continued |= continuation && revised;
                    if let Some(existing) =
                        c.input_buffer.blocks.iter_mut().find(|b| b.id == input.id)
                    {
                        *existing = input.clone();
                    } else {
                        c.input_buffer.blocks.push(input.clone());
                    }
                }
            }
            if continued {
                c.input_buffer.continuation_of =
                    recent_release(evaluated, policy.followup_seconds).map(|r| r.id.clone());
            }
            c.input_buffer
                .blocks
                .sort_by(|a, b| a.created_at.cmp(&b.created_at));
            c.input_buffer.complete = c
                .input_buffer
                .blocks
                .last()
                .is_some_and(|b| b.completion.as_deref() == Some("complete"));
            Ok((
                was_empty && !c.input_buffer.blocks.is_empty(),
                relevant,
                classifications,
                continued,
            ))
        })?;
        for value in classifications {
            let _=self.events.send(json!({"type":"input_classification","id":value["id"],"relevance":value["relevance"],"revision":value["revision"]}));
        }
        if opened {
            let _ = self.events.send(json!({"type":"cue","cue":"listening"}));
        }
        for id in relevant {
            let _ = self.events.send(json!({"type":"relevant_input","id":id}));
        }
        if continued {
            let _ = self.events.send(json!({"type":"input_continuation"}));
        }
        self.release_input()
    }
    pub(crate) fn speech_activity(self: &Arc<Self>, speaking: bool) -> Result<(), String> {
        self.state
            .lock()
            .map_err(|_| "Voice state unavailable")?
            .speaking = speaking;
        Ok(())
    }
    pub(crate) fn audio_pending(&self, delta: isize) -> Result<(), String> {
        let mut state = self.state.lock().map_err(|_| "Voice state unavailable")?;
        state.pending_audio = state.pending_audio.saturating_add_signed(delta);
        Ok(())
    }
    pub(crate) fn release_input(self: &Arc<Self>) -> Result<(), String> {
        let captured = self.update(|state| {
            let Some(c) = &state.conversation else {
                return Ok(None);
            };
            if !ready(state, c) {
                return Ok(None);
            }
            let policy = state.input_policy.clone();
            let c = state.conversation.as_mut().unwrap();
            let buffer = std::mem::take(&mut c.input_buffer);
            let previous = recent_release(c, policy.followup_seconds).cloned();
            for block in &buffer.blocks {
                if let Some(input) = c.inputs.iter_mut().find(|i| i.id == block.id) {
                    input.delivered_revision = input.revision;
                }
            }
            let release = InputRelease {
                id: uuid::Uuid::new_v4().to_string(),
                created_at: chrono::Utc::now().to_rfc3339(),
                blocks: buffer.blocks.clone(),
            };
            c.last_input_release = Some(release.clone());
            Ok(Some((
                c.clone(),
                release,
                previous,
                policy,
                buffer.continuation_of,
            )))
        })?;
        let Some((c, release, previous, policy, continuation_of)) = captured else {
            return Ok(());
        };
        let stages = c.pipeline.as_ref().ok_or("No reasoning stages")?;
        self.record_activity("user","accepted_input",json!({"requestId":release.id,"blocks":release.blocks,"previousRequest":previous,"continuationOf":continuation_of}),
            &[stages.summary.clone(),c.main_session_id.clone(),stages.digest.clone()])?;
        let _ = self.events.send(json!({"type":"cue","cue":"processing"}));
        let _=self.events.send(json!({"type":"input_released","requestId":release.id,"followupSeconds":policy.followup_seconds}));
        self.schedule_observations();
        Ok(())
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn delivered_revision_requires_a_confirmed_continuation() {
        let mut input = SpokenInput {
            revision: 2,
            delivered_revision: 1,
            ..Default::default()
        };
        assert!(!may_accumulate(&input, false));
        assert!(may_accumulate(&input, true));
        input.delivered_revision = 2;
        assert!(!may_accumulate(&input, true));
        input.delivered_revision = 0;
        assert!(may_accumulate(&input, false));
    }
    #[test]
    fn relevance_is_sticky_and_can_upgrade() {
        assert_eq!(
            sticky_relevance(Some("direct_request"), "unaddressed"),
            "direct_request"
        );
        assert_eq!(
            sticky_relevance(Some("task_context"), "unaddressed"),
            "task_context"
        );
        assert_eq!(
            sticky_relevance(Some("task_context"), "direct_request"),
            "direct_request"
        );
    }
    #[test]
    fn completion_releases_during_capture_but_not_against_unevaluated_revisions() {
        let state = VoiceState {
            enabled: true,
            speaking: true,
            pending_audio: 1,
            ..Default::default()
        };
        let block = SpokenInput {
            id: "sentence".into(),
            text: "Orchid, what time is it?".into(),
            ..Default::default()
        };
        let mut c = Conversation {
            input_buffer: InputBuffer {
                blocks: vec![block.clone()],
                complete: true,
                ..Default::default()
            },
            inputs: vec![block],
            ..Default::default()
        };
        assert!(ready(&state, &c));
        c.inputs[0].filter_pending = true;
        assert!(!ready(&state, &c));
    }
    #[test]
    fn query_window_uses_time_not_a_fixed_number_of_blocks() {
        let mut c = Conversation::default();
        for index in 0..12 {
            c.inputs.push(SpokenInput {
                id: index.to_string(),
                text: "Sentence.".into(),
                created_at: (chrono::Utc::now() - chrono::Duration::seconds(index * 5))
                    .to_rfc3339(),
                ..Default::default()
            });
        }
        assert_eq!(recent_inputs(&c, 60).len(), 12);
        assert_eq!(recent_inputs(&c, 20).len(), 4);
    }
}
