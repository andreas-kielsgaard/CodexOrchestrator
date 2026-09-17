mod prompt_agent;
mod stop_session;
#[cfg(test)]
mod tests;
mod tools;
use crate::otp_api::*;
use serde_json::{json, Value};

pub(crate) struct WorkflowPackage;

fn data_output(id: &str, name: &str, fields: Value) -> OutputDescriptor {
    OutputDescriptor {
        id: id.into(),
        name: name.into(),
        kind: OutputKind::Data,
        schema: json!({"type":"object","properties":fields,"additionalProperties":false}),
    }
}

impl OtpPackage for WorkflowPackage {
    fn descriptor(&self) -> PackageDescriptor {
        let source = json!({"type":"object","properties":{"id":{"type":"string"},"name":{"type":"string"}},"required":["id","name"]});
        let paths = json!({"type":"array","items":{"type":"string"}});
        let data = json!({"type":"object"});
        PackageDescriptor {
            id: "workflow".into(),
            name: "Workflow".into(),
            summary: "Workflow triggers, observed session events, and destinations.".into(),
            description: "Provides the routing-facing capabilities used to continue a Workflow, respond to agent activity, and request work from destination-node sessions.".into(),
            contract_version: 1,
            requested_handles: vec![Handle::Definitions, Handle::NodeSessions, Handle::EmitOutput],
            agent_mcp_servers: vec![],
            tools: vec![
                ToolDescriptor {
                    id:"trigger_workflow_continuation".into(), name:"Workflow continuation".into(),
                    description:"Trigger connections configured for this call from this node. Follow your instructions for when to call it. Optionally supply output file paths or structured data.".into(),
                    expected_behavior:"Emits the continuation output in the trusted context of the calling node, then lets the Workflow engine route only connections configured for that source node and trigger.".into(),
                    recommended_usage:"Use after the agent has completed the work or discussion its node instructions require. Supply output file paths or structured data only when downstream connections use them.".into(),
                    entrypoint: Entrypoint::Mcp {input_schema:json!({"type":"object","properties":{"outputFiles":paths,"data":data},"additionalProperties":false})},
                    outputs: vec![data_output("continuation","Continuation",json!({"outputFiles":paths,"sourceNode":source,"data":data}))], configuration:vec![]
                },
                ToolDescriptor {
                    id:"handoff_to_agent".into(), name:"Handoff to agent".into(),
                    description:"Publish file paths and prompt text to this node's configured handoff connections.".into(),
                    expected_behavior:"Emits one handoff output containing the declared file paths, prompt text, and trusted source-node identity for configured connections to consume.".into(),
                    recommended_usage:"Use when the node's instructions call for an explicit handoff and its configured connections expect the supplied files or prompt text.".into(),
                    entrypoint: Entrypoint::Mcp {input_schema:json!({"type":"object","properties":{"filePaths":paths,"promptText":{"type":"string"}},"required":["filePaths","promptText"],"additionalProperties":false})},
                    outputs: vec![data_output("handoff","Handoff",json!({"filePaths":paths,"promptText":{"type":"string"},"sourceNode":source}))], configuration:vec![]
                },
                ToolDescriptor {
                    id:"on_invocation_completed".into(),name:"Invocation completed".into(),description:"Emit when an invocation on the bound source node completes.".into(),
                    expected_behavior:"Observes a terminal invocation recorded by the product and emits its text output with the source-node identity.".into(),
                    recommended_usage:"Use when a connection should continue from an invocation ending, without requiring an agent MCP call.".into(),
                    entrypoint:Entrypoint::SessionEvent{event:SessionEventKind::InvocationTerminal},
                    outputs:vec![data_output("completed","Completed",json!({"output":{"type":"string"},"sourceNode":source}))],configuration:vec![]
                },
                ToolDescriptor {
                    id:"prompt_agent".into(),name:"Prompt agent".into(),description:"Choose destination-node sessions and request delivery of the configured prompt.".into(),
                    expected_behavior:"Resolves destination-node sessions from its configuration and requests prompt delivery through the Workflow engine.".into(),
                    recommended_usage:"Use as a connection or entry destination when the next step requires an agent session to receive constructed prompt material.".into(),
                    entrypoint:Entrypoint::Action { uses_prompt: true },
                    outputs:vec![OutputDescriptor{id:"session".into(),name:"Agent session".into(),kind:OutputKind::SessionRequest,schema:json!({"type":"object"})}],
                    configuration:prompt_agent::fields()
                },
                ToolDescriptor {
                    id:"stop_session".into(), name:"Stop session".into(),description:"Request cancellation of one running session of the destination node. The session remains available for later prompts.".into(),
                    expected_behavior:"Selects one running destination-node session according to its configuration and requests cancellation of its current invocation.".into(),
                    recommended_usage:"Use when a Workflow needs to stop active work in a destination-node session while retaining that session for later prompts.".into(),
                    entrypoint:Entrypoint::Action { uses_prompt: false },
                    outputs:vec![OutputDescriptor{id:"stop".into(),name:"Session cancellation".into(),kind:OutputKind::SessionStopRequest,schema:json!({"type":"object"})}],
                    configuration:stop_session::fields()
                },
            ],
        }
    }

    fn validate_configuration(&self, tool: &str, configuration: &Value) -> Result<(), String> {
        if tool == "stop_session" {
            stop_session::validate(configuration)
        } else if tool == "prompt_agent" {
            prompt_agent::validate(configuration)
        } else if self.descriptor().tools.iter().any(|item| item.id == tool)
            && configuration == &json!({})
        {
            Ok(())
        } else {
            Err(format!(
                "Unsupported configuration for Workflow tool {tool}"
            ))
        }
    }

    fn invoke(
        &self,
        context: &InvocationContext,
        input: ToolInput,
        host: &dyn OtpHost,
    ) -> Result<ToolResult, String> {
        match context.capability.tool.as_str() {
            "stop_session" => stop_session::invoke(context, input, host),
            "prompt_agent" => prompt_agent::invoke(context, input, host),
            _ => tools::invoke(context, input, host),
        }
    }
}
