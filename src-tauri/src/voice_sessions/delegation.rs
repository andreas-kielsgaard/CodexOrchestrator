use super::{
    application::VoiceSessionApplication,
    domain::Observation,
    tasks::{assign, AssignmentInput},
};
use crate::agent_sessions::domain::AgentSessionId;
use serde_json::{json, Value};
use std::sync::Arc;

impl VoiceSessionApplication {
    pub(crate) fn delegate(
        self: &Arc<Self>,
        caller: &str,
        invocation: &str,
        arguments: Value,
    ) -> Result<Value, String> {
        let conversation = self.require_member(caller)?;
        if !matches!(conversation.role(caller), Some("router" | "worker")) {
            return Err("The router and delegated workers own work handoffs".into());
        }
        let input: AssignmentInput =
            serde_json::from_value(arguments).map_err(|e| e.to_string())?;
        if input.task.trim().is_empty() {
            return Err("task is required".into());
        }
        if let Some(id) = &input.task_id {
            if !conversation.tasks.iter().any(|t| &t.id == id) {
                return Err("Unknown taskId".into());
            }
        }
        let state = self.snapshot()?;
        let recipient = if let Some(id) = &input.session_id {
            AgentSessionId::new(id.clone()).map_err(|e| e.to_string())?
        } else {
            self.agents
                .create_product_session(
                    input
                        .title
                        .clone()
                        .unwrap_or_else(|| "Voice delegated work".into()),
                    input
                        .working_directory
                        .clone()
                        .or(state.settings.working_directory),
                    &input
                        .profile_id
                        .clone()
                        .unwrap_or(state.settings.profile_id),
                    crate::orchid_control::tool_map(),
                )?
                .id
        };
        if recipient.as_str() == caller {
            return Err("Delegate to an independent session".into());
        }
        if conversation
            .role(recipient.as_str())
            .is_some_and(|r| r != "worker")
        {
            return Err("Delegate work to a worker, not an internal voice stage".into());
        }
        self.agents
            .load_session(&recipient)
            .map_err(|e| e.to_string())?;
        let assignment = self.update(|state| {
            let c = state.conversation.as_mut().ok_or("No conversation")?;
            let assignment = assign(c, caller, Some(invocation), recipient.as_str(), &input)?;
            c.observations.push(Observation {
                session_id: caller.into(), invocation_id: assignment.id.clone(),
                return_session_id: recipient.to_string(), created_at: assignment.created_at.clone(), delivery: None,
                detail: json!({"kind":"assignment_input","assignmentId":assignment.id,"taskId":assignment.task_id,
                    "revision":assignment.revision,"instructions":assignment.task,
                    "guidance":"Complete this delegated assignment independently. Report with voice_session.report_outcome using its assignmentId and revision; retain detailed material in this session or referenced files. Do not publish speech. Further work may be delegated; finish your turn when waiting for its results."}),
            });
            Ok(assignment)
        })?;
        self.record_activity(
            caller,
            "task_handoff",
            json!({"assignmentId":assignment.id,"taskId":assignment.task_id,
            "revision":assignment.revision,"sessionId":recipient,"task":assignment.task}),
            &self.digest_targets(),
        )?;
        self.schedule_observations();
        Ok(
            json!({"accepted":true,"assignmentId":assignment.id,"taskId":assignment.task_id,
            "revision":assignment.revision,"sessionId":recipient,"deliveryState":"pending"}),
        )
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
