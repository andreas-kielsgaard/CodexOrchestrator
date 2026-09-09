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
        PackageDescriptor { id:"workflow".into(), contract_version:1,
            requested_handles: vec![Handle::Definitions, Handle::NodeSessions, Handle::EmitOutput],
            tools: vec![
                ToolDescriptor { id:"trigger_workflow_continuation".into(), name:"Workflow continuation".into(),
                    description:"Trigger connections configured for this call from this node. Follow your instructions for when to call it. Optionally supply output file paths.".into(),
                    entrypoint: Entrypoint::Mcp {input_schema:json!({"type":"object","properties":{"outputFiles":paths},"additionalProperties":false})},
                    outputs: vec![data_output("continuation","Continuation",json!({"outputFiles":paths,"sourceNode":source}))], configuration:vec![] },
                ToolDescriptor { id:"handoff_to_agent".into(), name:"Handoff to agent".into(),
                    description:"Publish file paths and prompt text to this node's configured handoff connections.".into(),
                    entrypoint: Entrypoint::Mcp {input_schema:json!({"type":"object","properties":{"filePaths":paths,"promptText":{"type":"string"}},"required":["filePaths","promptText"],"additionalProperties":false})},
                    outputs: vec![data_output("handoff","Handoff",json!({"filePaths":paths,"promptText":{"type":"string"},"sourceNode":source}))], configuration:vec![] },
                ToolDescriptor {id:"on_invocation_completed".into(), name:"Invocation completed".into(),
                    description:"Emit when an invocation on the bound source node completes.".into(),
                    entrypoint:Entrypoint::SessionEvent{event:SessionEventKind::InvocationTerminal},
                    outputs:vec![data_output("completed","Completed",json!({"output":{"type":"string"},"sourceNode":source}))],configuration:vec![]},
                ToolDescriptor {id:"prompt_agent".into(),name:"Prompt agent".into(),
                    description:"Choose destination-node sessions and request delivery of the configured prompt.".into(),
                    entrypoint:Entrypoint::Action { uses_prompt: true },
                    outputs:vec![OutputDescriptor{id:"session".into(),name:"Agent session".into(),kind:OutputKind::SessionRequest,schema:json!({"type":"object"})}],
                    configuration:prompt_agent::fields()},
                ToolDescriptor {id:"stop_session".into(), name:"Stop session".into(),
                    description:"Request cancellation of one running session of the destination node. The session remains available for later prompts.".into(),
                    entrypoint:Entrypoint::Action { uses_prompt: false },
                    outputs:vec![OutputDescriptor{id:"stop".into(),name:"Session cancellation".into(),kind:OutputKind::SessionStopRequest,schema:json!({"type":"object"})}],
                    configuration:stop_session::fields()},
            ] }
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
