use super::{domain::*, VoiceSessionApplication};
use crate::otp_api::*;
use crate::workflows::{authoring::WorkflowNodeBinding, capabilities::WorkflowCapabilityRuntime};
use serde_json::{json, Value};
use std::sync::{Arc, Weak};

pub(super) struct VoiceRuntime(pub Weak<VoiceSessionApplication>);
fn speech_payload(configuration: &Value, payload: Value) -> Value {
    if let Some(text) = configuration["acknowledgmentText"].as_str().filter(|s| !s.trim().is_empty()) {
        json!({"text":text,"kind":"acknowledgement"})
    } else {
        payload.get("data").cloned().unwrap_or(payload)
    }
}

#[cfg(test)]
mod speech_tests {
    use super::*;
    #[test]
    fn configured_receipt_does_not_speak_incoming_request_data() {
        assert_eq!(speech_payload(&json!({"acknowledgmentText":"Got it."}), json!({"data":{"text":"user request"}})), json!({"text":"Got it.","kind":"acknowledgement"}));
    }
    #[test]
    fn ordinary_speech_preserves_publication_metadata() {
        let publication = json!({"text":"Result","kind":"announcement","topicId":"topic"});
        assert_eq!(speech_payload(&json!({}), json!({"data":publication})), publication);
    }
}
fn emitted(output: &str, payload: Value) -> ToolResult {
    ToolResult {
        text: json!({"recorded":true}).to_string(),
        emitted_outputs: vec![CapabilityEmission {
            output: output.into(),
            payload,
        }],
        ..Default::default()
    }
}
impl VoiceSessionApplication {
    pub(crate) fn register_workflow_runtime(self: &Arc<Self>) -> Result<(), String> {
        self.workflows
            .capability_runtimes
            .register("voice", Arc::new(VoiceRuntime(Arc::downgrade(self))))?;
        self.ensure_default_design()?;
        if let Some(id) = self
            .snapshot()?
            .conversation
            .and_then(|c| c.workflow_instance_id)
        {
            self.select_instance(&id)?;
        }
        Ok(())
    }

    pub(super) fn workflow_session_labels(
        &self,
        c: &Conversation,
    ) -> Result<Vec<(String, String)>, String> {
        let Some(id) = &c.workflow_instance_id else {
            return Ok(vec![]);
        };
        let instance = self.workflows.instances.load(id)?;
        let mut labels = vec![];
        for node in &instance.recipe.nodes {
            for session in self
                .workflows
                .directory
                .list_at_address(&crate::workflows::execution::node_address(
                    id,
                    &node.node_id,
                )?)
                .map_err(|e| e.to_string())?
            {
                labels.push((node.name.clone(), session.session.id().into()));
            }
        }
        Ok(labels)
    }
    pub(crate) fn select_instance(&self, id: &str) -> Result<(), String> {
        if self.snapshot()?.enabled {
            return Err("Stop the phone connection before switching instances".into());
        }
        let instance = self.workflows.instances.load(id)?;
        if !instance.recipe.nodes.iter().any(|n| matches!(&n.binding, WorkflowNodeBinding::Capability(b) if b.capability.package == "voice" && b.capability.tool == "transcript")) {
            return Err("Choose a Workflow instance with a Voice transcript capability".into());
        }
        let next = self.runtime_instance(id)?;
        let mut selected = self
            .selected
            .lock()
            .map_err(|_| "Voice selection unavailable")?;
        if let Some(previous) = selected.as_ref() {
            previous
                .connected_instance
                .store(false, std::sync::atomic::Ordering::Release);
        }
        next.connected_instance
            .store(true, std::sync::atomic::Ordering::Release);
        next.repository.save_scoped(&next.snapshot()?, true)?;
        *selected = Some(next);
        Ok(())
    }
    pub(super) fn runtime_instance(&self, id: &str) -> Result<Arc<Self>, String> {
        let mut instances = self
            .instances
            .lock()
            .map_err(|_| "Voice instances unavailable")?;
        if let Some(app) = instances.get(id) {
            return Ok(app.clone());
        }
        let mut state = self.snapshot()?;
        let instance = self.workflows.instances.load(id)?;
        if let Some(configuration) =
            instance
                .recipe
                .nodes
                .iter()
                .find_map(|node| match &node.binding {
                    WorkflowNodeBinding::Capability(b)
                        if b.capability.package == "voice" && b.capability.tool == "transcript" =>
                    {
                        Some(&b.configuration)
                    }
                    _ => None,
                })
        {
            state.input_policy =
                serde_json::from_value(configuration.clone()).map_err(|e| e.to_string())?;
            state.input_policy.validate()?;
        }
        state.conversation =
            Some(
                self.repository
                    .load_instance(id)?
                    .unwrap_or_else(|| Conversation {
                        id: id.into(),
                        workflow_instance_id: Some(id.into()),
                        ..Default::default()
                    }),
            );
        state.enabled = false;
        state.phone_connected = false;
        if let Some(c) = &mut state.conversation {
            c.input_buffer.complete = false;
            for input in &mut c.inputs {
                input.filter_pending = false;
            }
        }
        let app = self.scoped(state);
        instances.insert(id.into(), app.clone());
        Ok(app)
    }

    pub(super) fn emit_voice_event(
        &self,
        node_kind: &str,
        tool: &str,
        output: &str,
        payload: Value,
    ) -> Result<(), String> {
        let c = self.snapshot()?.conversation.ok_or("No Voice instance")?;
        let id = c
            .workflow_instance_id
            .ok_or("Legacy Voice history is read-only")?;
        let instance = self.workflows.instances.load(&id)?;
        for node in &instance.recipe.nodes {
            if matches!(&node.binding, WorkflowNodeBinding::Capability(b) if b.capability.package == "voice" && b.capability.tool == node_kind)
            {
                self.workflows.emit_capability_output(
                    &id,
                    &node.node_id,
                    &uuid::Uuid::new_v4().to_string(),
                    &orchid_core::otp_packages::voice::output(tool, output),
                    payload.clone(),
                )?;
            }
        }
        Ok(())
    }
}
impl WorkflowCapabilityRuntime for VoiceRuntime {
    fn invoke(
        &self,
        context: &InvocationContext,
        configuration: &Value,
        payload: Value,
    ) -> Result<ToolResult, String> {
        let root = self.0.upgrade().ok_or("Voice runtime stopped")?;
        let app = root.runtime_instance(&context.instance_id)?;
        let c = app
            .snapshot()?
            .conversation
            .ok_or("Select a Voice instance first")?;
        if c.workflow_instance_id.as_deref() != Some(&context.instance_id) {
            return Err("This Voice instance is not connected".into());
        }
        let caller = context
            .source
            .as_ref()
            .and_then(|s| s.session_id.as_deref())
            .unwrap_or("");
        let invocation = context
            .source
            .as_ref()
            .and_then(|s| s.invocation_id.as_deref())
            .unwrap_or("");
        match context.capability.tool.as_str() {
            "assign_work" | "return_to_requester" => {
                let node = context
                    .output_node_id
                    .as_deref()
                    .ok_or("Missing destination node")?;
                let candidates = app
                    .workflows
                    .directory
                    .list_at_address(&crate::workflows::execution::node_address(
                        &context.instance_id,
                        node,
                    )?)
                    .map_err(|e| e.to_string())?;
                let data = payload.get("data").cloned().unwrap_or(payload.clone());
                let target = if context.capability.tool == "return_to_requester" {
                    let requester = data["requesterSessionId"]
                        .as_str()
                        .ok_or("Missing requester")?;
                    if !candidates.iter().any(|s| s.session.id() == requester) {
                        return Ok(ToolResult::default());
                    }
                    SessionRequestTarget::Exact {
                        session_id: requester.into(),
                    }
                } else if let Some(id) = data["sessionId"].as_str() {
                    if !candidates.iter().any(|s| s.session.id() == id) {
                        return Err(
                            "Worker session must belong to the configured destination node".into(),
                        );
                    }
                    SessionRequestTarget::Exact {
                        session_id: id.into(),
                    }
                } else {
                    SessionRequestTarget::New
                };
                let content = if context.capability.tool == "assign_work" {
                    let input: super::tasks::AssignmentInput =
                        serde_json::from_value(data).map_err(|e| e.to_string())?;
                    let requester = payload["requesterSessionId"]
                        .as_str()
                        .ok_or("Missing requester")?;
                    app.require_member(requester)?;
                    let recipient = match &target {
                        SessionRequestTarget::Exact { session_id } => session_id.as_str(),
                        _ => "",
                    };
                    let assignment = app.update(|s| {
                        let c = s.conversation.as_mut().ok_or("No conversation")?;
                        let assignment = super::tasks::assign(
                            c,
                            requester,
                            payload["requesterInvocationId"].as_str(),
                            recipient,
                            &input,
                        )?;
                        let saved = c.delegations.last_mut().unwrap();
                        saved.destination_node_id = node.into();
                        saved.request_key = format!(
                            "{}:{}",
                            context.occurrence_id,
                            context.connection_id.as_deref().unwrap_or("")
                        );
                        Ok(assignment)
                    })?;
                    json!({"assignmentId":assignment.id,"taskId":assignment.task_id,"revision":assignment.revision,"task":assignment.task})
                } else {
                    data
                };
                Ok(ToolResult {
                    session_requests: vec![SessionRequest {
                        node_id: node.into(),
                        target,
                        delivery_mode:
                            crate::session_events::SessionDeliveryMode::SteerActiveOtherwiseQueue,
                        prompt: vec![PromptPart {
                            reference: "voice-assignment".into(),
                            text: content.to_string(),
                        }],
                    }],
                    ..Default::default()
                })
            }
            "evaluate" => {
                let node = context
                    .output_node_id
                    .as_deref()
                    .ok_or("Missing evaluation node")?;
                let filter = if let Some(id) = c.evaluation_sessions.get(node) {
                    id.clone()
                } else {
                    let session = app
                        .agents
                        .create_jev_api_session(
                            crate::agent_sessions::application::jev::CreateJevApiSessionInput {
                                title: Some("Voice input evaluation".into()),
                                model: configuration["model"]
                                    .as_str()
                                    .filter(|s| !s.is_empty())
                                    .map(str::to_owned),
                            },
                        )
                        .map_err(|e| e.to_string())?;
                    let id = session.id.to_string();
                    app.update(|s| {
                        s.conversation
                            .as_mut()
                            .ok_or("No conversation")?
                            .evaluation_sessions
                            .insert(node.into(), id.clone());
                        Ok(())
                    })?;
                    id
                };
                Ok(emitted(
                    "evaluated",
                    app.evaluate_input(configuration, &filter)?,
                ))
            }
            "accept_evaluation" => {
                let value = app.accept_evaluation(
                    payload,
                    configuration["directReactions"].as_bool().unwrap_or(true),
                )?;
                if value.is_null() {
                    return Ok(ToolResult::default());
                }
                Ok(emitted(
                    if value["responseRoute"] == "direct_reaction" {
                        "reaction_ready"
                    } else {
                        "work_ready"
                    },
                    json!({"data":value}),
                ))
            }
            "speak" => Ok(emitted(
                "published",
                  json!({"data":app.publish(caller,speech_payload(configuration, payload))?}),
            )),
            "read_context" => Ok(ToolResult {
                text: app.read_context(caller, payload)?.to_string(),
                ..Default::default()
            }),
            "observe_sessions" => Ok(ToolResult {
                text: app.observe(caller, payload)?.to_string(),
                ..Default::default()
            }),
            "report_outcome" => {
                let result = app.report_outcome(caller, invocation, payload)?;
                Ok(emitted("outcome", json!({"data":result})))
            }
            "route_input" => Ok(emitted(
                "rerouted",
                json!({"data":app.route_input(caller,payload)?}),
            )),
            "report_progress" | "request_output" | "publish" | "acknowledge" | "delegate_work" => {
                app.require_member(caller)?;
                let name = match context.capability.tool.as_str() {
                    "report_progress" => "progress",
                    _ => "requested",
                };
                app.record_activity(caller, &context.capability.tool, payload.clone(), &[])?;
                Ok(emitted(
                    name,
                    json!({"data":payload,"requesterSessionId":caller,"requesterInvocationId":invocation}),
                ))
            }
            _ => Err(format!(
                "Unsupported Voice action {}",
                context.capability.tool
            )),
        }
    }
    fn allow_delivery_retry(
        &self,
        context: &InvocationContext,
        _request: &SessionRequest,
    ) -> Result<bool, String> {
        if context.capability.tool != "assign_work" {
            return Ok(true);
        }
        let root = self.0.upgrade().ok_or("Voice stopped")?;
        let app = root.runtime_instance(&context.instance_id)?;
        let c = app.snapshot()?.conversation.ok_or("No conversation")?;
        let key = format!(
            "{}:{}",
            context.occurrence_id,
            context.connection_id.as_deref().unwrap_or("")
        );
        let assignment = c
            .delegations
            .iter()
            .find(|d| d.request_key == key)
            .ok_or("Assignment missing")?;
        Ok(!c.delegations.iter().any(|newer| {
            newer.task_id == assignment.task_id
                && newer.session_id == assignment.session_id
                && newer.revision > assignment.revision
                && newer
                    .delivery
                    .as_ref()
                    .is_some_and(|d| d.input_id.is_none() || d.state == "accepted")
        }))
    }
    fn delivered(
        &self,
        context: &InvocationContext,
        _attempt_id: &str,
        _ordinal: usize,
        _request: &SessionRequest,
        receipt: &crate::session_events::SessionEventResult,
    ) -> Result<(), String> {
        if context.capability.tool != "assign_work" {
            return Ok(());
        }
        let root = self.0.upgrade().ok_or("Voice stopped")?;
        let app = root.runtime_instance(&context.instance_id)?;
        let key = format!(
            "{}:{}",
            context.occurrence_id,
            context.connection_id.as_deref().unwrap_or("")
        );
        app.update(|s| {
            let c = s.conversation.as_mut().ok_or("No conversation")?;
            if c.workflow_instance_id.as_deref() != Some(&context.instance_id) {
                return Err("Voice instance changed during assignment".into());
            }
            let assignment = c
                .delegations
                .iter_mut()
                .find(|d| d.request_key == key)
                .ok_or("Assignment missing")?;
            if let Some(delivery) = receipt.deliveries.first() {
                assignment.session_id = delivery.target_session.id().into();
                if let crate::session_events::DeliveryOutcome::Dispatched {
                    invocation,
                    input_id,
                    state,
                } = &delivery.outcome
                {
                    assignment.invocation_id = Some(invocation.id().into());
                    assignment.delivery =
                        Some(crate::agent_sessions::application::ProductDeliveryReceipt {
                            session_id: assignment.session_id.clone(),
                            invocation_id: invocation.id().into(),
                            input_id: input_id.clone(),
                            state: state.clone().unwrap_or("queued".into()),
                        });
                }
            }
            Ok(())
        })
    }
}
