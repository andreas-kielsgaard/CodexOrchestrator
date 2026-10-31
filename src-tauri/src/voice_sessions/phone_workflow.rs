use super::VoiceSessionApplication;
use crate::{agent_sessions::domain::AgentSessionId, workflows::authoring::WorkflowNodeBinding};
use serde_json::{json, Value};

fn preview(value: &Value) -> String {
    let text = serde_json::to_string_pretty(value).unwrap_or_default();
    let mut result: String = text.chars().take(2400).collect();
    if text.chars().count() > 2400 {
        result.push_str("\n… (preview truncated)");
    }
    result
}

impl VoiceSessionApplication {
    pub(crate) fn phone_workflow(&self) -> Result<Value, String> {
        let state = self.snapshot()?;
        let Some(id) = state
            .conversation
            .as_ref()
            .and_then(|c| c.workflow_instance_id.as_deref())
        else {
            return Ok(json!({"instance":null,"nodes":[],"connections":[],"attempts":[]}));
        };
        let instance = self.workflows.instances.load(id)?;
        let mut nodes = vec![];
        for node in &instance.recipe.nodes {
            let mut sessions = vec![];
            for entry in self
                .workflows
                .directory
                .list_at_address(&crate::workflows::execution::node_address(
                    id,
                    &node.node_id,
                )?)
                .map_err(|e| e.to_string())?
            {
                let history = self
                    .agents
                    .load_session(
                        &AgentSessionId::new(entry.session.id())
                            .map_err(|e| e.to_string())?,
                    )
                    .map_err(|e| e.to_string())?;
                let last = history.invocations.last();
                sessions.push(json!({"id":entry.session.id(),"title":history.session.title,"status":last.map(|h|h.invocation.status),"invocationId":last.map(|h|&h.invocation.id),"error":last.and_then(|h|h.invocation.runtime_error.as_ref()).map(|e|&e.message)}));
            }
            let (role, model, reasoning) = match &node.binding {
                WorkflowNodeBinding::Agent(agent) => (
                    agent.initial_prompt.as_deref().unwrap_or("").split("\n\n").next().unwrap_or("").to_owned(),
                    agent.node_profile.pinned_defaults.model.clone(),
                    agent.node_profile.pinned_defaults.reasoning_mode.clone(),
                ),
                WorkflowNodeBinding::Capability(capability) => (
                    match capability.capability.tool.as_str() {
                        "transcript" => "Collects speech into transcript windows for evaluation.",
                        "evaluation" => "JEV checks who you are addressing, whether your request is complete, and whether it needs work or a conversational response.",
                        "buffer" => "Keeps accepted speech together, then releases work requests or direct reactions through the configured connections.",
                        "speech" if capability.configuration["acknowledgmentText"].as_str().is_some_and(|s| !s.trim().is_empty()) => "Speaks a short, configured receipt immediately when a request is accepted. No agent or model is needed.",
                        "speech" => "Turns published replies into phone audio and records playback feedback.",
                        "external_work" => "Brings observations from watched sessions into this conversation.",
                        _ => "Runs the capability configured in this workflow design.",
                    }.to_owned(), None, None,
                ),
            };
            nodes.push(json!({"id":node.node_id,"name":node.name,"role":role,"model":model,"reasoning":reasoning,"x":node.position_x,"y":node.position_y,"kind":match &node.binding {WorkflowNodeBinding::Agent(_)=>"agent",WorkflowNodeBinding::Capability(_)=>"capability"},"sessions":sessions}));
        }
        let records = self.workflows.instances.attempts(id)?;
        let mut attempts = vec![];
        for attempt in records.iter().take(60) {
            let mut deliveries = vec![];
            for group in &attempt.event_groups {
                let current = self
                    .workflows
                    .session_events
                    .recorded_result(&group.group.event_group_id)?
                    .unwrap_or_else(|| group.clone());
                for delivery in current.deliveries {
                    deliveries.push(json!({"sessionId":delivery.target_session.id(),"outcome":delivery.outcome}));
                }
            }
            attempts.push(json!({"id":attempt.id,"connectionId":attempt.context.connection_id,"destinationNodeId":attempt.context.output_node_id,"createdAt":attempt.created_at,"completedAt":attempt.completed_at,"action":attempt.context.capability.tool,"error":attempt.error,"message":attempt.message,"input":preview(&attempt.payload),"outputs":preview(&json!(attempt.emitted_outputs)),"deliveries":deliveries}));
        }
        let connections=instance.recipe.connections.iter().map(|edge|json!({"id":edge.connection_id,"name":edge.name,"source":edge.source_node_id,"destination":edge.destination_node_id,"trigger":format!("{}.{}",edge.trigger.capability.tool,edge.trigger.output),"action":edge.action.tool})).collect::<Vec<_>>();
        Ok(
            json!({"updatedAt":chrono::Utc::now().to_rfc3339(),"instance":{"id":instance.id,"name":instance.name,"design":instance.recipe.name,"revision":instance.recipe.revision,"worktree":instance.target.worktree.path},"nodes":nodes,"connections":connections,"attempts":attempts,"totalAttempts":records.len(),"diagnostics":self.diagnostics()?}),
        )
    }
}
