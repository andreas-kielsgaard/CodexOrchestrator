use super::{domain::*, VoiceSessionApplication};
use crate::agent_sessions::{
    application::{jev::CreateJevApiSessionInput, ProductDeliveryReceipt},
    domain::AgentSessionId,
};
use serde_json::{json, Value};
use std::sync::Arc;

#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct ContextInput {
    activity_ids: Option<Vec<String>>,
}
#[derive(serde::Deserialize)]
#[serde(deny_unknown_fields)]
struct ProgressInput {
    text: String,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct OutputInput {
    reason: String,
    activity_ids: Option<Vec<String>>,
    topic_id: Option<String>,
}
#[derive(serde::Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct RouteInput {
    request_id: String,
    reason: String,
}

pub(super) fn may_publish(role: Option<&str>, kind: &str) -> bool {
    role == Some("output") || (role == Some("summary") && kind == "acknowledgement")
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn publication_authority_separates_receipt_from_output() {
        assert!(may_publish(Some("summary"), "acknowledgement"));
        assert!(!may_publish(Some("summary"), "explanation"));
        assert!(may_publish(Some("output"), "announcement"));
        for role in [
            None,
            Some("router"),
            Some("digest"),
            Some("worker"),
            Some("filter"),
        ] {
            assert!(!may_publish(role, "acknowledgement"));
            assert!(!may_publish(role, "explanation"));
        }
    }
}

impl VoiceSessionApplication {
    pub(super) fn ensure_pipeline(&self) -> Result<(), String> {
        let state = self.snapshot()?;
        if state
            .conversation
            .as_ref()
            .is_some_and(|c| c.pipeline.is_some())
        {
            return Ok(());
        }
        let create = |title: &str| {
            self.agents
                .create_product_session_configured(
                    title.into(),
                    state.settings.working_directory.clone(),
                    &state.settings.profile_id,
                    crate::orchid_control::tool_map(),
                    state.settings.models.response.options(),
                )
                .map(|s| s.id.to_string())
        };
        let stages = PipelineSessions {
            filter: self
                .agents
                .create_jev_api_session(CreateJevApiSessionInput {
                    title: Some("Voice relevance and completion".into()),
                    model: None,
                })
                .map_err(|e| e.to_string())?
                .id
                .to_string(),
            summary: create("Voice input summarization")?,
            digest: create("Voice digest")?,
            output: create("Voice output")?,
        };
        self.update(|s| {
            s.conversation.as_mut().ok_or("No conversation")?.pipeline = Some(stages);
            Ok(())
        })
    }
    pub(super) fn role_instructions<'a>(
        &self,
        conversation: &Conversation,
        target: &str,
        prompts: &'a VoicePrompts,
    ) -> Option<&'a str> {
        match conversation.role(target) {
            Some("router") => Some(&prompts.router),
            Some("summary") => Some(&prompts.summary),
            Some("digest") => Some(&prompts.digest),
            Some("output") => Some(&prompts.output),
            _ => None,
        }
    }
    /// Shared by fresh input and observations; active reasoning gets a turn steer.
    pub(super) fn dispatch(
        &self,
        target: &str,
        id: &str,
        text: String,
        conversation: &Conversation,
    ) -> Result<ProductDeliveryReceipt, String> {
        let session = AgentSessionId::new(target.to_string()).map_err(|e| e.to_string())?;
        let content = self.delivery_content(&session, text, conversation)?;
        self.agents
            .deliver_or_steer_product(session, id.into(), content)
            .map_err(|e| e.to_string())
    }
    pub(super) fn record_activity(
        &self,
        source: &str,
        kind: &str,
        data: Value,
        targets: &[String],
    ) -> Result<Activity, String> {
        let conversation = self.snapshot()?.conversation.ok_or("No conversation")?;
        let activity = Activity {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.into(),
            kind: kind.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            data,
            route: Some(super::workflow::evidence(&conversation, source, kind, targets)),
            deliveries: vec![],
        };
        self.update(|state| {
            let conversation = state.conversation.as_mut().ok_or("No conversation")?;
            conversation.activity.push(activity.clone());
            for target in targets {
                conversation.observations.push(Observation {
                    session_id: source.into(),
                    invocation_id: activity.id.clone(),
                    return_session_id: target.clone(),
                    detail: serde_json::to_value(&activity).map_err(|e| e.to_string())?,
                    delivery: None,
                    created_at: activity.created_at.clone(),
                });
            }
            Ok(())
        })?;
        Ok(activity)
    }
    pub(super) fn digest_targets(&self) -> Vec<String> {
        self.snapshot()
            .ok()
            .and_then(|s| s.conversation)
            .map(|c| super::workflow::targets(&c, super::workflow::FlowEvent::Progress))
            .unwrap_or_default()
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
    pub(crate) fn report_progress(
        self: &Arc<Self>,
        caller: &str,
        args: Value,
    ) -> Result<Value, String> {
        self.require_member(caller)?;
        let input: ProgressInput = serde_json::from_value(args).map_err(|e| e.to_string())?;
        let text = input.text.trim();
        if text.is_empty() {
            return Err("text is required".into());
        }
        let activity = self.record_activity(
            caller,
            "progress",
            json!({"text":text}),
            &self.digest_targets(),
        )?;
        self.schedule_observations();
        Ok(json!({"activityId":activity.id}))
    }
    pub(crate) fn request_output(
        self: &Arc<Self>,
        caller: &str,
        args: Value,
    ) -> Result<Value, String> {
        let conversation = self.require_member(caller)?;
        if conversation.role(caller) != Some("digest") {
            return Err("Only the digest requests output".into());
        }
        let input: OutputInput = serde_json::from_value(args).map_err(|e| e.to_string())?;
        let reason = input.reason.trim();
        if reason.is_empty() {
            return Err("reason is required".into());
        }
        conversation.pipeline.as_ref().ok_or("No output session")?;
        let activity = self.record_activity(
            caller,
            "output_request",
            json!({"reason":reason,"activityIds":input.activity_ids,"topicId":input.topic_id}),
            &super::workflow::targets(&conversation, super::workflow::FlowEvent::OutputRequested),
        )?;
        self.schedule_observations();
        Ok(json!({"accepted":true,"activityId":activity.id}))
    }
    pub(crate) fn route_input(self: &Arc<Self>, caller: &str, args: Value) -> Result<Value, String> {
        let c = self.require_member(caller)?;
        if c.role(caller) != Some("output") { return Err("Only output forwards a classified reaction to the work router".into()); }
        let input: RouteInput = serde_json::from_value(args).map_err(|e| e.to_string())?;
        if input.reason.trim().is_empty() { return Err("reason is required".into()); }
        let accepted = c.activity.iter().find(|a| a.kind=="accepted_input"
            && a.data["requestId"].as_str()==Some(input.request_id.as_str())
            && a.data["responseRoute"]=="direct_reaction").ok_or("Direct reaction request not found")?;
        if c.activity.iter().any(|a| a.kind=="reaction_rerouted" && a.data["requestId"].as_str()==Some(input.request_id.as_str())) {
            return Ok(json!({"accepted":true,"alreadyRouted":true}));
        }
        let activity = self.record_activity(caller,"reaction_rerouted",json!({"requestId":input.request_id,
            "reason":input.reason,"acceptedInput":accepted}), &super::workflow::targets(&c, super::workflow::FlowEvent::ReactionRerouted))?;
        self.schedule_observations();
        Ok(json!({"accepted":true,"activityId":activity.id}))
    }
}
