use super::{application::VoiceSessionApplication, domain::Delegation};
use crate::{
    agent_sessions::domain::{AgentInvocationInputProvenance, AgentSessionId},
    orchid_control::{optional, required},
};
use serde_json::{json, Value};
use std::sync::Arc;
impl VoiceSessionApplication {
    pub(crate) fn delegate(self: &Arc<Self>, caller: &str, arguments: Value) -> Result<Value, String> {
        let conversation = self.require_member(caller)?;
        if !matches!(conversation.role(caller), Some("router" | "worker")) {
            return Err("The router and delegated workers own work handoffs".into());
        }
        let task = required(&arguments, "task")?.to_string();
        let state = self.snapshot()?;
        let recipient = if let Some(id) = optional(&arguments, "sessionId") {
            AgentSessionId::new(id).map_err(|e| e.to_string())?
        } else {
            self.agents
                .create_product_session(
                    optional(&arguments, "title").unwrap_or_else(|| "Voice delegated work".into()),
                    optional(&arguments, "workingDirectory").or(state.settings.working_directory),
                    &optional(&arguments, "profileId").unwrap_or(state.settings.profile_id),
                    crate::orchid_control::tool_map(),
                )?
                .id
        };
        if recipient.to_string() == caller {
            return Err("Delegate to an independent session".into());
        }
        // Record membership before dispatch: a fast recipient may finish during launch.
        if conversation
            .role(recipient.as_str())
            .is_some_and(|role| role != "worker")
        {
            return Err("Delegate work to a worker, not an internal voice stage".into());
        }
        self.update(|state| {
            state
                .conversation
                .as_mut()
                .ok_or("No conversation")?
                .delegations
                .push(Delegation {
                    session_id: recipient.to_string(),
                    from_session_id: caller.into(),
                    return_session_id: caller.into(),
                    task: task.clone(),
                    invocation_id: None,
                    created_at: chrono::Utc::now().to_rfc3339(),
                });
            Ok(())
        })?;
        let content=format!("Delegated work from session {caller}. Complete this task independently and report facts, results, and unresolved issues in your final response. Do not publish speech directly. You may hand off further work with voice_session.delegate_work. If waiting for delegated work, finish the current turn with a progress response; result observations resume this session.\n\n{task}");
        let receipt = self.deliver(
            &recipient,
            content,
            AgentInvocationInputProvenance::Application,
            &conversation,
        );
        if receipt.is_err() {
            self.update(|state| {
                if let Some(conversation) = state.conversation.as_mut() {
                    if let Some(index) = conversation.delegations.iter().rposition(|d| {
                        d.session_id == recipient.to_string()
                            && d.from_session_id == caller
                            && d.invocation_id.is_none()
                    }) {
                        conversation.delegations.remove(index);
                    }
                }
                Ok(())
            })?;
            return receipt;
        }
        self.update(|state| {
            if let Some(delegation) = state.conversation.as_mut().and_then(|c| {
                c.delegations
                    .iter_mut()
                    .rev()
                    .find(|d| d.session_id == recipient.to_string() && d.invocation_id.is_none())
            }) {
                delegation.invocation_id = receipt
                    .as_ref()
                    .ok()
                    .and_then(|v| v["invocationId"].as_str())
                    .map(str::to_owned);
            }
            Ok(())
        })?;
        self.record_activity(caller, "task_handoff", json!({"sessionId":recipient,"task":task,"delivery":receipt.as_ref().ok()}), &self.digest_targets())?;
        self.schedule_observations();
        receipt.map(|delivery| json!({"accepted":true,"delivery":delivery,"task":task}))
    }
    pub(crate) fn observe(&self, caller: &str, arguments: Value) -> Result<Value, String> {
        self.require_member(caller)?;
        let sessions: Vec<String> =
            serde_json::from_value(arguments["sessionIds"].clone()).map_err(|e| e.to_string())?;
        for id in &sessions {
            self.agents
                .load_session(&AgentSessionId::new(id.clone()).map_err(|e| e.to_string())?)
                .map_err(|e| e.to_string())?;
        }
        self.update(|state| {
            state
                .conversation
                .as_mut()
                .ok_or("No conversation")?
                .observed_sessions = sessions.clone();
            Ok(())
        })?;
        Ok(json!({"observing":sessions}))
    }
}
