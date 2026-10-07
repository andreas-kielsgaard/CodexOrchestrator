use super::{domain::*, VoiceSessionApplication};
use crate::agent_sessions::{
    application::{jev::CreateJevApiSessionInput, SteerAgentSessionCommand},
    domain::{AgentInvocationInputProvenance, AgentSessionId},
};
use serde_json::{json, Value};
use std::sync::Arc;

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
                .create_product_session(
                    title.into(),
                    state.settings.working_directory.clone(),
                    &state.settings.profile_id,
                    crate::orchid_control::tool_map(),
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
    ) -> Result<Value, String> {
        let session = AgentSessionId::new(target.to_string()).map_err(|e| e.to_string())?;
        let history = self
            .agents
            .load_session(&session)
            .map_err(|e| e.to_string())?;
        if let Some(active) = history
            .invocations
            .iter()
            .rev()
            .find(|h| !h.invocation.status.is_terminal())
        {
            let settings = self.snapshot()?.settings;
            if let Ok(receipt) = self.agents.steer_session(SteerAgentSessionCommand {
                session_id: session.clone(),
                invocation_id: active.invocation.id.clone(),
                input_id: id.into(),
                text: format!(
                    "{}\n\n{}\n\n{text}\n\nApplication context: {}",
                    self.role_instructions(conversation, target, &settings.prompts)
                        .unwrap_or("Continue your delegated work."),
                    settings.prompts.tools,
                    self.context(conversation)
                ),
                attachments: vec![],
            }) {
                return Ok(json!({"steering":receipt}));
            }
        }
        self.deliver(
            &session,
            text,
            AgentInvocationInputProvenance::Application,
            conversation,
        )
    }
    pub(super) fn record_activity(
        &self,
        source: &str,
        kind: &str,
        data: Value,
        targets: &[String],
    ) -> Result<Activity, String> {
        let activity = Activity {
            id: uuid::Uuid::new_v4().to_string(),
            source: source.into(),
            kind: kind.into(),
            created_at: chrono::Utc::now().to_rfc3339(),
            data,
        };
        self.update(|state| {
            let conversation = state.conversation.as_mut().ok_or("No conversation")?;
            conversation.activity.push(activity.clone());
            for target in targets {
                conversation.observations.push(Observation {
                    session_id: source.into(),
                    invocation_id: activity.id.clone(),
                    return_session_id: target.clone(),
                    detail: serde_json::to_string(&activity).map_err(|e| e.to_string())?,
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
            .and_then(|c| c.pipeline)
            .map(|p| vec![p.digest])
            .unwrap_or_default()
    }
    pub(crate) fn read_context(&self, caller: &str, args: Value) -> Result<Value, String> {
        let conversation = self.require_member(caller)?;
        let ids = args["activityIds"].as_array();
        let activity: Vec<_> = conversation
            .activity
            .iter()
            .filter(|a| ids.is_none_or(|ids| ids.iter().any(|id| id.as_str() == Some(&a.id))))
            .rev()
            .take(if ids.is_some() { usize::MAX } else { 40 })
            .cloned()
            .collect();
        Ok(
            json!({"context":self.context(&conversation), "activity":activity,
            "acceptedInputs":conversation.inputs.iter().filter(|i| i.relevance.as_deref().is_some_and(|r| r != "unaddressed")).rev().take(20).collect::<Vec<_>>(),
            "inputBuffer":conversation.input_buffer}),
        )
    }
    pub(crate) fn report_progress(
        self: &Arc<Self>,
        caller: &str,
        args: Value,
    ) -> Result<Value, String> {
        self.require_member(caller)?;
        let text = crate::orchid_control::required(&args, "text")?;
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
        let reason = crate::orchid_control::required(&args, "reason")?;
        let output = conversation
            .pipeline
            .as_ref()
            .ok_or("No output session")?
            .output
            .clone();
        let activity = self.record_activity(
            caller,
            "output_request",
            json!({"reason":reason,"activityIds":args["activityIds"],"topicId":args["topicId"]}),
            &[output],
        )?;
        self.schedule_observations();
        Ok(json!({"accepted":true,"activityId":activity.id}))
    }
}
