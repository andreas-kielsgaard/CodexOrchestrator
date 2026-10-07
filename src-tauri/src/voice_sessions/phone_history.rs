use super::{domain::Conversation, VoiceSessionApplication};
use crate::agent_sessions::{domain::AgentSessionId, ports::AgentInvocationHistory};
use serde_json::{json, Value};

fn is_member(conversation: &Conversation, id: &str) -> bool {
    conversation.main_session_id == id
        || conversation.delegations.iter().any(|d| d.session_id == id)
}

fn turn(history: &AgentInvocationHistory) -> Value {
    let events = history
        .events
        .iter()
        .filter_map(|event| {
            let normalized = event.normalized.as_ref()?;
            let text = normalized.text.as_deref()?;
            if text.is_empty() {
                return None;
            }
            Some(json!({"id":event.id,"kind":normalized.kind,"text":text,
            "time":event.recorded_at,"tool":normalized.tool_activity}))
        })
        .collect::<Vec<_>>();
    let input = serde_json::from_str::<Value>(&history.invocation.submitted_text).ok();
    let request = input
        .as_ref()
        .filter(|v| v["type"] == "speech_fragment")
        .and_then(|v| v["text"].as_str())
        .unwrap_or(&history.invocation.submitted_text);
    json!({"id":history.invocation.id,"request":request,
        "inputKind":if input.as_ref().is_some_and(|v| v["type"] == "speech_fragment") {"speech"} else {"context"},
        "provenance":history.invocation.input_provenance,"status":history.invocation.status,
        "createdAt":history.invocation.created_at,"reply":history.final_reply(),
        "error":history.invocation.runtime_error,"events":events})
}

impl VoiceSessionApplication {
    pub(crate) fn phone_sessions(&self) -> Result<Value, String> {
        let conversation = self
            .snapshot()?
            .conversation
            .ok_or("No voice conversation")?;
        let mut ids = vec![conversation.main_session_id.clone()];
        for delegation in &conversation.delegations {
            if !ids.contains(&delegation.session_id) {
                ids.push(delegation.session_id.clone());
            }
        }
        let sessions = ids
            .iter()
            .map(|id| {
                let history = self
                    .agents
                    .load_session_display(
                        &AgentSessionId::new(id.clone()).map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                let delegation = conversation
                    .delegations
                    .iter()
                    .rev()
                    .find(|d| d.session_id == *id);
                Ok(json!({"id":id,"title":history.session.title,
                "role":if id == &conversation.main_session_id {"conversation"} else {"worker"},
                "parentSessionId":delegation.map(|d| &d.from_session_id),
                "task":delegation.map(|d| &d.task),
                "status":history.invocations.last().map(|h| h.invocation.status),
                "turnCount":history.invocations.len()}))
            })
            .collect::<Result<Vec<_>, String>>()?;
        Ok(json!({"conversationId":conversation.id,"sessions":sessions}))
    }

    pub(crate) fn phone_history(&self, id: &str, before: Option<&str>) -> Result<Value, String> {
        let conversation = self
            .snapshot()?
            .conversation
            .ok_or("No voice conversation")?;
        if !is_member(&conversation, id) {
            return Err("Session is outside this voice conversation".into());
        }
        let history = self
            .agents
            .load_session_display(&AgentSessionId::new(id.to_string()).map_err(|e| e.to_string())?)
            .map_err(|e| e.to_string())?;
        let end = match before {
            Some(cursor) => history
                .invocations
                .iter()
                .position(|h| h.invocation.id.as_str() == cursor)
                .ok_or("Unknown history cursor")?,
            None => history.invocations.len(),
        };
        let start = end.saturating_sub(10);
        let turns = history.invocations[start..end]
            .iter()
            .map(turn)
            .collect::<Vec<_>>();
        let mut exchanges = vec![];
        if id == conversation.main_session_id {
            for input in &conversation.inputs {
                exchanges.push(json!({"id":input.id,"speaker":"You","text":input.text,"createdAt":input.created_at}));
            }
            for utterance in &conversation.utterances {
                exchanges.push(json!({"id":utterance.id,"speaker":"Orchid · published","text":utterance.text,"createdAt":utterance.created_at}));
            }
            exchanges.sort_by(|a, b| a["createdAt"].as_str().cmp(&b["createdAt"].as_str()));
        }
        Ok(
            json!({"sessionId":id,"title":history.session.title,"turns":turns,
            "exchanges":exchanges,
            "nextBefore":if start > 0 {Some(history.invocations[start].invocation.id.as_str())} else {None}}),
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn phone_history_is_scoped_to_the_main_session_and_recursive_delegates() {
        let conversation: Conversation = serde_json::from_value(json!({"id":"voice","mainSessionId":"main",
            "delegations":[{"sessionId":"worker","fromSessionId":"main","returnSessionId":"main","task":"Inspect","invocationId":null},
                {"sessionId":"nested","fromSessionId":"worker","returnSessionId":"worker","task":"Read","invocationId":null}],
            "observedSessions":["unrelated"],"observations":[],"pendingTopics":[],"utterances":[],"inputs":[],"playback":null})).unwrap();
        for id in ["main", "worker", "nested"] {
            assert!(is_member(&conversation, id));
        }
        assert!(!is_member(&conversation, "unrelated"));
    }
}
