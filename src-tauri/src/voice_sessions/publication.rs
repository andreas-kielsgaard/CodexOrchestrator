use super::{domain::*, VoiceSessionApplication};
use serde_json::{json, Value};
use std::sync::Arc;
impl VoiceSessionApplication {
    pub(crate) fn publish(
        self: &Arc<Self>,
        _caller: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        let text = crate::orchid_control::required(&arguments, "text")?.to_string();
        if text.len() > 8000 {
            return Err("Split long explanations into concise spoken passages".into());
        }
        let kind = crate::orchid_control::required(&arguments, "kind")?.to_string();
        if !["acknowledgement", "announcement", "explanation"].contains(&kind.as_str()) {
            return Err("Invalid publication kind".into());
        }
        let utterance = Utterance {
            id: uuid::Uuid::new_v4().to_string(),
            kind,
            text,
            topic_id: arguments["topicId"].as_str().map(str::to_owned),
            created_at: chrono::Utc::now().to_rfc3339(),
        };
        self.update(|state| {
            let conversation = state.conversation.as_mut().ok_or("Conversation ended")?;
            if let Some(topic) = arguments.get("pendingTopic") {
                let topic: Topic =
                    serde_json::from_value(topic.clone()).map_err(|e| e.to_string())?;
                conversation.pending_topics.retain(|t| t.id != topic.id);
                conversation.pending_topics.push(topic);
            }
            if let Some(id) = arguments["completedTopicId"].as_str() {
                conversation.pending_topics.retain(|t| t.id != id);
            }
            conversation.utterances.push(utterance.clone());
            Ok(())
        })?;
        let sent = self
            .connected_instance
            .load(std::sync::atomic::Ordering::Acquire)
            && self
                .events
                .send(json!({"type":"speak","utterance":utterance}))
                .is_ok();
        Ok(
            json!({"utteranceId":utterance.id,"published":true,"phoneConnected":self.snapshot()?.phone_connected,"sent":sent,"played":false}),
        )
    }
    pub(crate) fn playback(&self, playback: Playback) -> Result<(), String> {
        let event = serde_json::to_value(&playback).map_err(|e| e.to_string())?;
        self.update(|state| {
            let conversation = state.conversation.as_mut().ok_or("No conversation")?;
            if !conversation
                .utterances
                .iter()
                .any(|u| u.id == playback.utterance_id)
            {
                return Err("Unknown utterance".into());
            }
            conversation.playback = Some(playback);
            Ok(())
        })?;
        self.emit_voice_event("speech", "playback", "changed", json!({"data":event}))
    }
}
