use crate::otp_api::*;
use serde_json::{json, Value};

pub struct VoicePackage;

pub fn reference(tool: &str) -> CapabilityRef {
    CapabilityRef {
        package: "voice".into(),
        tool: tool.into(),
    }
}
pub fn output(tool: &str, name: &str) -> OutputRef {
    OutputRef {
        capability: reference(tool),
        output: name.into(),
    }
}

fn data(name: &str) -> OutputDescriptor {
    OutputDescriptor {
        id: name.into(),
        name: name.replace('_', " "),
        kind: OutputKind::Data,
        schema: json!({"type":"object","properties":{"data":{"type":"object"}}}),
    }
}

fn tool(id: &str, entrypoint: Entrypoint, outputs: &[&str]) -> ToolDescriptor {
    ToolDescriptor {
        id: id.into(),
        name: id.replace('_', " "),
        description: format!("Voice {id}"),
        expected_behavior: "Operates within the bound Voice workflow instance.".into(),
        recommended_usage:
            "Connect the declared output to the desired destination in the workflow design.".into(),
        annotations: tool_annotations(id, id == "read_context", false, false, false),
        entrypoint,
        outputs: outputs.iter().map(|name| data(name)).collect(),
        configuration: vec![],
    }
}

fn node(
    id: &str,
    configuration_schema: Value,
    actions: &[&str],
    outputs: &[(&str, &str)],
) -> NodeCapabilityDescriptor {
    NodeCapabilityDescriptor {
        id: id.into(),
        name: id.replace('_', " "),
        configuration_schema,
        actions: actions.iter().map(|id| reference(id)).collect(),
        outputs: outputs
            .iter()
            .map(|(tool, name)| output(tool, name))
            .collect(),
    }
}

impl OtpPackage for VoicePackage {
    fn descriptor(&self) -> PackageDescriptor {
        let object = json!({"type":"object"});
        let mut tools = vec![
            tool("transcript", Entrypoint::ServiceEvent, &["window"]),
            tool(
                "evaluate",
                Entrypoint::Action { uses_prompt: false },
                &["evaluated"],
            ),
            tool(
                "accept_evaluation",
                Entrypoint::Action { uses_prompt: false },
                &["work_ready", "reaction_ready"],
            ),
            tool(
                "speak",
                Entrypoint::Action { uses_prompt: false },
                &["published"],
            ),
            tool("playback", Entrypoint::ServiceEvent, &["changed"]),
            tool("observe", Entrypoint::ServiceEvent, &["observed"]),
        ];
        for (id, fields, required, outputs) in [
            (
                "read_context",
                json!({"activityIds":{"type":"array","items":{"type":"string"}}}),
                vec![],
                vec![],
            ),
            (
                "report_progress",
                json!({"text":{"type":"string"}}),
                vec!["text"],
                vec!["progress"],
            ),
            (
                "request_output",
                json!({"reason":{"type":"string"},"activityIds":{"type":"array","items":{"type":"string"}},"topicId":{"type":"string"}}),
                vec!["reason"],
                vec!["requested"],
            ),
            (
                "route_input",
                json!({"requestId":{"type":"string"},"reason":{"type":"string"}}),
                vec!["requestId", "reason"],
                vec!["rerouted"],
            ),
            (
                "delegate_work",
                json!({"task":{"type":"string"},"title":{"type":"string"},"taskId":{"type":"string"},"sessionId":{"type":"string"},"parentAssignmentId":{"type":"string"}}),
                vec!["task"],
                vec!["requested"],
            ),
            (
                "report_outcome",
                json!({"assignmentId":{"type":"string"},"revision":{"type":"integer","minimum":1},"summary":{"type":"string"},"materialRefs":{"type":"array","items":{"type":"string"}},"unresolved":{"type":"array","items":{"type":"string"}}}),
                vec!["assignmentId", "revision", "summary"],
                vec!["outcome"],
            ),
            (
                "observe_sessions",
                json!({"sessionIds":{"type":"array","items":{"type":"string"}}}),
                vec!["sessionIds"],
                vec![],
            ),
        ] {
            tools.push(tool(id, Entrypoint::Mcp { input_schema: json!({"type":"object","properties":fields,"required":required,"additionalProperties":false}) }, &outputs));
        }
        for (id, kinds) in [
            ("acknowledge", json!(["acknowledgement"])),
            (
                "publish",
                json!(["acknowledgement", "announcement", "explanation"]),
            ),
        ] {
            tools.push(tool(id, Entrypoint::Mcp { input_schema: json!({"type":"object","required":["kind","text"],"properties":{
                "kind":{"type":"string","enum":kinds},"text":{"type":"string"},"topicId":{"type":"string"},"completedTopicId":{"type":"string"},"pendingTopic":{"type":"object"}
            },"additionalProperties":false}) }, &["requested"]));
        }
        let mut assign = tool("assign_work", Entrypoint::Action { uses_prompt: true }, &[]);
        assign.outputs.push(OutputDescriptor {
            id: "session".into(),
            name: "Worker session".into(),
            kind: OutputKind::SessionRequest,
            schema: object.clone(),
        });
        tools.push(assign.clone());
        assign.id = "return_to_requester".into();
        assign.name = "Return to requester".into();
        tools.push(assign);
        PackageDescriptor {
            id: "voice".into(),
            name: "Voice".into(),
            summary: "Voice workflow capabilities".into(),
            description:
                "Transcript processing, typed evaluation, task bookkeeping and deliberate speech."
                    .into(),
            contract_version: 1,
            requested_handles: vec![
                Handle::Definitions,
                Handle::NodeSessions,
                Handle::EmitOutput,
            ],
            agent_mcp_servers: vec![],
            skill_roots: vec![],
            tools,
            node_capabilities: vec![
                node(
                    "transcript",
                    json!({"type":"object","properties":{"contextSeconds":{"type":"integer","minimum":10,"maximum":120},"followupSeconds":{"type":"integer","minimum":1,"maximum":600},"pauseSeconds":{"type":"integer","minimum":1,"maximum":60},"sentencesPerQuery":{"type":"integer","minimum":1,"maximum":20}}}),
                    &[],
                    &[("transcript", "window")],
                ),
                node(
                    "evaluation",
                    json!({"type":"object","properties":{"model":{"type":"string"},"prompts":{"type":"object","properties":{"relevance":{"type":"string"},"completion":{"type":"string"},"continuation":{"type":"string"},"reaction":{"type":"string"}}}}}),
                    &["evaluate"],
                    &[("evaluate", "evaluated")],
                ),
                node(
                    "buffer",
                    json!({"type":"object","properties":{"directReactions":{"type":"boolean"}}}),
                    &["accept_evaluation"],
                    &[
                        ("accept_evaluation", "work_ready"),
                        ("accept_evaluation", "reaction_ready"),
                    ],
                ),
                node(
                    "speech",
                    json!({"type":"object","properties":{"acknowledgmentText":{"type":"string","description":"Optional fixed receipt spoken when this node is triggered, without waiting for an agent."}}}),
                    &["speak"],
                    &[("speak", "published"), ("playback", "changed")],
                ),
                node("external_work", object, &[], &[("observe", "observed")]),
            ],
        }
    }

    fn validate_configuration(&self, tool: &str, configuration: &Value) -> Result<(), String> {
        if !self.descriptor().tools.iter().any(|item| item.id == tool) {
            return Err("Unknown Voice capability".into());
        }
        if !configuration.is_object() {
            return Err("Voice action configuration must be an object".into());
        }
        Ok(())
    }

    fn invoke(
        &self,
        context: &InvocationContext,
        input: ToolInput,
        host: &dyn OtpHost,
    ) -> Result<ToolResult, String> {
        match input {
            ToolInput::Action { payload, .. } => host.invoke_service(context, payload),
            ToolInput::Mcp(arguments) => {
                let result = host.invoke_service(context, arguments)?;
                for emission in &result.emitted_outputs {
                    host.emit(context, &emission.output, emission.payload.clone())?;
                }
                Ok(result)
            }
            _ => Err("Voice service events enter through the bound runtime".into()),
        }
    }
}
