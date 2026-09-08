use super::compiled_plan::WorkflowConnectionTrigger;
use serde::Serialize;
use serde_json::{json, Value};

pub(crate) const CONTINUATION_TOOL: &str = "trigger_workflow_continuation";

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TriggerCapability {
    pub(crate) id: &'static str,
    pub(crate) version: u32,
    pub(crate) name: &'static str,
    pub(crate) server: &'static str,
    pub(crate) tool: &'static str,
    pub(crate) input_schema: Value,
    pub(crate) fields: Vec<TriggerField>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct TriggerField {
    pub(crate) name: &'static str,
    pub(crate) label: &'static str,
    pub(crate) schema: Value,
}

pub(crate) fn continuation() -> TriggerCapability {
    TriggerCapability {
        id: "workflow.continuation",
        version: 1,
        name: "Workflow continuation",
        server: super::mcp::SERVER_NAME,
        tool: CONTINUATION_TOOL,
        input_schema: json!({"type":"object", "properties":{
            "outputFiles":{"type":"array", "items":{"type":"string"}}
        }, "additionalProperties":false}),
        fields: vec![
            TriggerField {
                name: "outputFiles",
                label: "Output files",
                schema: json!({"type":"array","items":{"type":"string"}}),
            },
            TriggerField {
                name: "sourceNode",
                label: "Source node",
                schema: json!({"type":"object","properties":{"id":{"type":"string"},"name":{"type":"string"}},"required":["id","name"]}),
            },
        ],
    }
}

pub(crate) fn for_trigger(trigger: &WorkflowConnectionTrigger) -> Option<TriggerCapability> {
    let WorkflowConnectionTrigger::McpCall { server, tool } = trigger else {
        return None;
    };
    let capability = continuation();
    (server.namespace() == "mcp"
        && server.kind() == "server"
        && server.id() == capability.server
        && tool.namespace() == "mcp"
        && tool.kind() == "tool"
        && tool.id() == capability.tool)
        .then_some(capability)
}
