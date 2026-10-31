use super::{domain::*, VoiceSessionApplication};
use serde_json::{json, Value};
use std::sync::Arc;
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextInput {
    activity_ids: Option<Vec<String>>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RouteInput {
    request_id: String,
    reason: String,
}
impl VoiceSessionApplication {
    pub(crate) fn context(&self, conversation: &Conversation) -> Value {
        json!({"conversationId":conversation.id,"mainSessionId":conversation.main_session_id,"pipeline":conversation.pipeline,
            "tasks":conversation.tasks,"assignments":conversation.delegations.iter().rev().take(20).map(|d|json!({"id":d.id,"taskId":d.task_id,"revision":d.revision,"sessionId":d.session_id,"invocationId":d.invocation_id,"delivery":d.delivery,"status":d.runtime_status,"outcome":d.outcome})).collect::<Vec<_>>(),
            "pendingTopics":conversation.pending_topics.iter().map(|t|json!({"id":t.id,"title":t.title})).collect::<Vec<_>>(),"recentOutput":conversation.utterances.iter().rev().take(5).collect::<Vec<_>>(),"playback":conversation.playback,
            "utterance":conversation.playback.as_ref().and_then(|p| conversation.utterances.iter().find(|u|u.id==p.utterance_id))})
    }

    pub(super) fn record_activity(
        &self,
        source: &str,
        kind: &str,
        data: Value,
        _targets: &[String],
    ) -> Result<Activity, String> {
        let activity = Activity {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.into(),
            kind: kind.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            data,
            route: None,
            deliveries: vec![],
        };
        self.update(|state| {
            let conversation = state.conversation.as_mut().ok_or("No conversation")?;
            conversation.activity.push(activity.clone());
            Ok(())
        })?;
        Ok(activity)
    }
    pub(crate) fn read_context(&self, caller: &str, args: Value) -> Result<Value, String> {
        let conversation = self.require_member(caller)?;
        let input: ContextInput = serde_json::from_value(args).map_err(|e| e.to_string())?;
        let ids = input.activity_ids.as_ref();
        let activity: Vec<_> = conversation
            .activity
            .iter()
            .filter(|a| ids.is_none_or(|ids| ids.contains(&a.id)))
            .rev()
            .take(if ids.is_some() { usize::MAX } else { 40 })
            .cloned()
            .collect();
        Ok(
            json!({"context":self.context(&conversation), "activity":activity,
            "acceptedInputs":conversation.inputs.iter().filter(|i| i.relevance.as_deref().is_some_and(|r| r != "unaddressed")).rev().take(20).collect::<Vec<_>>(),
            "inputBuffer":conversation.input_buffer,"pendingTopics":conversation.pending_topics,
            "assignments":conversation.delegations}),
        )
    }
    pub(crate) fn route_input(
        self: &Arc<Self>,
        caller: &str,
        args: Value,
    ) -> Result<Value, String> {
        let c = self.require_member(caller)?;

        let input: RouteInput = serde_json::from_value(args).map_err(|e| e.to_string())?;
        if input.reason.trim().is_empty() {
            return Err("reason is required".into());
        }
        let accepted = c
            .activity
            .iter()
            .find(|a| {
                a.kind == "accepted_input"
                    && a.data["requestId"].as_str() == Some(input.request_id.as_str())
                    && a.data["responseRoute"] == "direct_reaction"
            })
            .ok_or("Direct reaction request not found")?;
        if c.activity.iter().any(|a| {
            a.kind == "reaction_rerouted"
                && a.data["requestId"].as_str() == Some(input.request_id.as_str())
        }) {
            return Ok(json!({"accepted":true,"alreadyRouted":true}));
        }
        let activity = self.record_activity(
            caller,
            "reaction_rerouted",
            json!({"requestId":input.request_id,
            "reason":input.reason,"acceptedInput":accepted}),
            &[],
        )?;
        Ok(activity.data)
    }
}
