use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CapabilityRef {
    pub package: String,
    pub tool: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct OutputRef {
    pub capability: CapabilityRef,
    pub output: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PackageDescriptor {
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
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum Handle {
    Definitions,
    NodeSessions,
    EmitOutput,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ToolDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub expected_behavior: String,
    pub recommended_usage: String,
    pub entrypoint: Entrypoint,
    pub outputs: Vec<OutputDescriptor>,
    pub configuration: Vec<ConfigurationField>,
}

/// A package-declared MCP service that is delivered to an agent through the Harness.
/// It is intentionally distinct from Workflow trigger and action tools.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentMcpServerDescriptor {
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
pub(crate) struct AgentMcpCapabilityGroupDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentMcpGrantDescriptor {
    pub id: String,
    pub label: String,
    pub description: String,
    pub capability: String,
    pub transition: String,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AgentMcpToolDescriptor {
    pub id: String,
    pub name: String,
    pub description: String,
    pub capability: String,
    pub input_schema: Value,
    pub output_schema: Value,
    pub expected_behavior: String,
    pub recommended_usage: String,
    pub required_grants: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum Entrypoint {
    Mcp { input_schema: Value },
    SessionEvent { event: SessionEventKind },
    Action { uses_prompt: bool },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum SessionEventKind {
    InvocationTerminal,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OutputDescriptor {
    pub id: String,
    pub name: String,
    pub kind: OutputKind,
    pub schema: Value,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum OutputKind {
    Data,
    SessionRequest,
    SessionStopRequest,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ConfigurationField {
    pub key: String,
    pub label: String,
    pub choices: Vec<String>,
    pub default_value: String,
    pub when: Option<(String, String)>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceContext {
    pub node_id: String,
    pub node_name: String,
    pub session_id: String,
    pub invocation_id: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct InvocationContext {
    pub instance_id: String,
    pub occurrence_id: String,
    pub capability: CapabilityRef,
    pub source: Option<SourceContext>,
    pub connection_id: Option<String>,
    pub output_node_id: Option<String>,
}

#[derive(Clone, Debug)]
pub(crate) enum ToolInput {
    Mcp(Value),
    SessionEvent {
        status: String,
        output: String,
    },
    Action {
        configuration: Value,
        inputs: Vec<ResolvedInput>,
    },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ResolvedInput {
    pub reference: String,
    pub value: Value,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PromptPart {
    pub reference: String,
    pub text: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum SessionRequestTarget {
    New,
    Exact { session_id: String },
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionRequest {
    pub node_id: String,
    pub target: SessionRequestTarget,
    pub prompt: Vec<PromptPart>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SessionStopRequest {
    pub node_id: String,
    pub session_id: String,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct ToolResult {
    pub stop_requests: Vec<SessionStopRequest>,
    pub text: String,
    pub session_requests: Vec<SessionRequest>,
}

#[derive(Clone, Debug, Default)]
pub(crate) struct RoutingReceipt {
    pub stops: usize,
    pub deliveries: usize,
}

pub(crate) trait OtpPackage: Send + Sync {
    fn descriptor(&self) -> PackageDescriptor;
    fn validate_configuration(&self, tool: &str, configuration: &Value) -> Result<(), String>;
    fn validate_agent_mcp_configuration(
        &self,
        server: &str,
        configuration: &Value,
    ) -> Result<(), String> {
        let _ = (server, configuration);
        Ok(())
    }
    fn invoke(
        &self,
        context: &InvocationContext,
        input: ToolInput,
        host: &dyn super::OtpHost,
    ) -> Result<ToolResult, String>;
}

pub(crate) fn validate_json(schema: &Value, value: &Value) -> Result<(), String> {
    match schema.get("type").and_then(Value::as_str) {
        Some("object") => {
            let object = value.as_object().ok_or("Expected an object")?;
            let fields = schema.get("properties").and_then(Value::as_object);
            if let Some(required) = schema.get("required").and_then(Value::as_array) {
                for key in required.iter().filter_map(Value::as_str) {
                    if !object.contains_key(key) {
                        return Err(format!("Missing field {key}"));
                    }
                }
            }
            for (key, value) in object {
                if let Some(field) = fields.and_then(|fields| fields.get(key)) {
                    validate_json(field, value)?;
                } else if schema.get("additionalProperties") == Some(&Value::Bool(false)) {
                    return Err(format!("Unknown field {key}"));
                }
            }
        }
        Some("array") => {
            let items = value.as_array().ok_or("Expected an array")?;
            if let Some(item) = schema.get("items") {
                for value in items {
                    validate_json(item, value)?;
                }
            }
        }
        Some("string") if !value.is_string() => return Err("Expected a string".into()),
        _ => {}
    }
    Ok(())
}
