use super::{domain::*, VoiceSessionApplication};
use crate::{
    agent_sessions::domain::{AgentInvocationId, AgentSessionId},
    runtime::providers::jev::contracts::{JevAnswer, JevEvaluation, JevQuestion},
};
use serde_json::{json, Value};
use std::{
    collections::BTreeMap,
    time::{Duration, Instant},
};

use super::input_buffer::{recent_inputs, recent_release};
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
impl VoiceSessionApplication {
    fn evaluate_window(
        &self,
        c: &Conversation,
        filter: &str,
        recent: &[SpokenInput],
        prompts: &VoicePrompts,
        policy: &VoiceInputPolicy,
    ) -> Result<(String, BTreeMap<String, String>), String> {
        let session = AgentSessionId::new(filter.to_owned()).map_err(|e| e.to_string())?;
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
            questions.insert(format!("reaction:{}",block.id), question(&prompts.reaction,&block.id,&[
                ("reaction_request","An addressed request to respond to, explain, or clarify available output or task status; no new work or changed requirements."),
                ("work_or_context","New work, changed requirements, mixed reaction and work, or context requiring the work router."),
                ("none","Unaddressed speech or no expected conversational reaction.")]));
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
            "recentActivity":c.activity.iter().rev().take(10).collect::<Vec<_>>(),"tasks":c.tasks,"pendingTopics":c.pending_topics,
            "recentOutput":c.utterances.iter().rev().take(5).collect::<Vec<_>>()}),
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
                let answers = view
                    .result
                    .ok_or("JEV result missing".to_string())?
                    .answers
                    .into_iter()
                    .map(|(key, answer)| match answer {
                        JevAnswer::Choice { choice, .. } => Ok((key, choice)),
                        _ => Err("JEV returned an unexpected answer type".into()),
                    })
                    .collect::<Result<BTreeMap<_, _>, String>>()?;
                return Ok((invocation.as_str().to_owned(), answers));
            }
            if Instant::now() >= deadline {
                return Err("JEV input evaluation timed out".into());
            }
            std::thread::sleep(Duration::from_millis(25));
        }
    }
    pub(super) fn evaluate_input(
        &self,
        configuration: &Value,
        filter: &str,
    ) -> Result<Value, String> {
        let state = self.snapshot()?;
        let c = state.conversation.ok_or("No conversation")?;
        let recent = recent_inputs(&c, state.input_policy.context_seconds);
        let prompts: VoicePrompts =
            serde_json::from_value(configuration.get("prompts").cloned().unwrap_or(json!({})))
                .map_err(|e| e.to_string())?;
        let (evaluation_id, answers) =
            self.evaluate_window(&c, filter, &recent, &prompts, &state.input_policy)?;
        Ok(json!({"conversation":c,"recent":recent,"evaluationId":evaluation_id,"answers":answers}))
    }
}
