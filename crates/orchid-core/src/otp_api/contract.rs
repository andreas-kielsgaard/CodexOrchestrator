use rmcp::model::ToolAnnotations;
use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CapabilityRef {
    pub package: String,
    pub tool: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OutputRef {
    pub capability: CapabilityRef,
    pub output: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PackageDescriptor {
    pub id: String,
    pub name: String,
    pub summary: String,
    pub description: String,
    pub contract_version: u32,
    pub requested_handles: Vec<Handle>,
    pub tools: Vec<ToolDescriptor>,
    pub agent_mcp_servers: Vec<AgentMcpServerDescriptor>,
    /// Registered skill roots; an OTP with none cannot expose a skill bucket.
    pub skill_roots: Vec<String>,
    pub node_capabilities: Vec<NodeCapabilityDescriptor>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct NodeCapabilityDescriptor {
    pub id: String,
    pub name: String,
    pub configuration_schema: Value,
    pub actions: Vec<CapabilityRef>,
    pub outputs: Vec<OutputRef>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Handle {
    Definitions,
    NodeSessions,
    EmitOutput,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ToolDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub expected_behavior: String,
    pub recommended_usage: String,
    pub annotations: ToolAnnotations,
    pub entrypoint: Entrypoint,
    pub outputs: Vec<OutputDescriptor>,
    pub configuration: Vec<ConfigurationField>,
}

/// A package-declared MCP service that is delivered to an agent through the Harness.
/// It is intentionally distinct from Workflow trigger and action tools.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMcpServerDescriptor {
    pub server_name: String,
    pub name: String,
    pub description: String,
    pub capability_groups: Vec<AgentMcpCapabilityGroupDescriptor>,
    pub tools: Vec<AgentMcpToolDescriptor>,
    pub grants: Vec<AgentMcpGrantDescriptor>,
    pub configuration: Vec<ConfigurationField>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMcpCapabilityGroupDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMcpGrantDescriptor {
    pub id: String,
    pub label: String,
    pub description: String,
    pub capability: String,
    pub transition: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AgentMcpToolDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub capability: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub expected_behavior: String,
    pub recommended_usage: String,
    pub required_grants: Vec<String>,
    pub annotations: ToolAnnotations,
}

pub fn tool_annotations(
    title: &str,
    read_only: bool,
    destructive: bool,
    idempotent: bool,
    open_world: bool,
) -> ToolAnnotations {
    ToolAnnotations::from_raw(
        Some(title.into()),
        Some(read_only),
        Some(destructive),
        Some(idempotent),
        Some(open_world),
    )
}

#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum Entrypoint {
    ServiceEvent,
    Mcp { input_schema: Value },
    SessionEvent { event: SessionEventKind },
    Action { uses_prompt: bool },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SessionEventKind {
    InvocationTerminal,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct OutputDescriptor {
    pub id: String,
    pub name: String,
    pub kind: OutputKind,
    pub schema: Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OutputKind {
    Data,
    SessionRequest,
    SessionStopRequest,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ConfigurationField {
    pub key: String,
    pub label: String,
    pub choices: Vec<String>,
    pub default_value: String,
    pub when: Option<(String, String)>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SourceContext {
    pub node_id: String,
    pub node_name: String,
    pub session_id: Option<String>,
    pub invocation_id: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InvocationContext {
    pub instance_id: String,
    pub occurrence_id: String,
    pub capability: CapabilityRef,
    pub source: Option<SourceContext>,
    pub connection_id: Option<String>,
    pub output_node_id: Option<String>,
}

#[derive(Clone, Debug)]
pub enum ToolInput {
    Mcp(Value),
    SessionEvent {
        status: String,
        output: String,
    },
    Action {
        configuration: Value,
        payload: Value,
        inputs: Vec<ResolvedInput>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ResolvedInput {
    pub reference: String,
    pub value: Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct PromptPart {
    pub reference: String,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum SessionRequestTarget {
    New,
    Exact { session_id: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionRequest {
    #[serde(default)]
    pub delivery_mode: crate::session_events::SessionDeliveryMode,
    pub node_id: String,
    pub target: SessionRequestTarget,
    pub prompt: Vec<PromptPart>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SessionStopRequest {
    pub node_id: String,
    pub session_id: String,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CapabilityEmission {
    pub output: String,
    pub payload: Value,
}

#[derive(Clone, Debug, Default)]
pub struct ToolResult {
    pub emitted_outputs: Vec<CapabilityEmission>,
    pub stop_requests: Vec<SessionStopRequest>,
    pub text: String,
    pub session_requests: Vec<SessionRequest>,
}

#[derive(Clone, Debug, Default)]
pub struct RoutingReceipt {
    pub stops: usize,
    pub deliveries: usize,
}

pub trait OtpPackage: Send + Sync {
    fn descriptor(&self) -> PackageDescriptor;
    fn validate_configuration(&self, tool: &str, configuration: &Value) -> Result<(), String>;
    fn invoke(
        &self,
        context: &InvocationContext,
        input: ToolInput,
        host: &dyn super::OtpHost,
    ) -> Result<ToolResult, String>;
}

pub fn validate_json(schema: &Value, value: &Value) -> Result<(), String> {
    jsonschema::validator_for(schema)
        .map_err(|error| format!("Invalid OTP schema: {error}"))?
        .validate(value)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod schema_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn capability_settings_enforce_types_ranges_and_choices() {
        let schema = json!({"type":"object", "required":["enabled","seconds","mode"],
        "additionalProperties":false, "properties": {
            "enabled":{"type":"boolean"}, "seconds":{"type":"integer","minimum":1},
            "mode":{"type":"string","enum":["work","reaction"]}
        }});
        assert!(validate_json(&schema, &json!({"enabled":true,"seconds":2,"mode":"work"})).is_ok());
        for invalid in [
            json!({"enabled":"true","seconds":2,"mode":"work"}),
            json!({"enabled":true,"seconds":0,"mode":"work"}),
            json!({"enabled":true,"seconds":2,"mode":"unknown"}),
            json!({"enabled":true,"seconds":2}),
        ] {
            assert!(validate_json(&schema, &invalid).is_err());
        }
    }
}
