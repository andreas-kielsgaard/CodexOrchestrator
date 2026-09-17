use super::compiled_plan::{
    WorkflowCompiledPlan, WorkflowConnectionPromptInput, WorkflowSessionCreation,
};
use crate::{
    otp_api::{Entrypoint, OutputKind},
    otp_host::OtpRegistry,
};
use serde_json::json;
use std::collections::BTreeSet;

pub(crate) struct WorkflowCompiler;
impl WorkflowCompiler {
    pub(crate) fn compile(
        plan: WorkflowCompiledPlan,
        registry: &OtpRegistry,
    ) -> Result<WorkflowCompiledPlan, String> {
        let nodes: BTreeSet<_> = plan
            .nodes
            .iter()
            .map(|n| n.reference.identity().id())
            .collect();
        if nodes.len() != plan.nodes.len() || !nodes.contains(plan.starting_node.identity().id()) {
            return Err("Invalid Workflow node bindings".into());
        }
        for node in &plan.nodes {
            validate_agent_mcp_configuration(registry, node)?;
        }
        validate_action(registry, &plan.entry_action, &plan.entry_configuration)?;
        for edge in &plan.connections {
            if !nodes.contains(edge.source_node.identity().id())
                || !nodes.contains(edge.destination_node.identity().id())
            {
                return Err("Connection references a missing node".into());
            }
            let producer = registry.tool(&edge.trigger.capability)?;
            if matches!(producer.entrypoint, Entrypoint::Action { .. }) {
                return Err(
                    "A connection trigger requires an MCP or Session Event producer".into(),
                );
            }
            let output = producer
                .outputs
                .iter()
                .find(|o| o.id == edge.trigger.output && o.kind == OutputKind::Data)
                .ok_or("The selected capability does not offer this data output")?;
            validate_action(registry, &edge.action, &edge.configuration)?;
            for input in &edge.prompt_inputs {
                match input {
                    WorkflowConnectionPromptInput::OutputField { field } => {
                        if output
                            .schema
                            .get("properties")
                            .and_then(|p| p.get(field))
                            .is_none()
                        {
                            return Err(format!("Output does not offer field {field}"));
                        }
                    }
                    WorkflowConnectionPromptInput::NodeFiles { node_id, .. } => {
                        if !nodes.contains(node_id.as_str()) {
                            return Err(format!("File input references missing node {node_id}"));
                        }
                    }
                    WorkflowConnectionPromptInput::FileContent { path } => {
                        if path.trim().is_empty() {
                            return Err("File input requires a path".into());
                        }
                    }
                }
            }
        }
        Ok(plan)
    }
}

fn validate_agent_mcp_configuration(
    registry: &OtpRegistry,
    node: &super::compiled_plan::WorkflowCompiledNode,
) -> Result<(), String> {
    let (capabilities, configuration) = match &node.session_creation {
        WorkflowSessionCreation::ResolvedInput(request) => (
            &request.node_profile.allowed_capabilities,
            &request.agent_mcp_configuration,
        ),
        WorkflowSessionCreation::AtBirth(intent) => (
            &intent.node_profile.allowed_capabilities,
            &intent.agent_mcp_configuration,
        ),
    };
    for server in capabilities.mcp_tools.keys() {
        let Ok((package, _)) = registry.agent_mcp_server(server) else {
            continue;
        };
        let value = configuration
            .get(&package)
            .and_then(serde_json::Value::as_object)
            .and_then(|servers| servers.get(server))
            .cloned()
            .unwrap_or_else(|| json!({}));
        registry.validate_agent_mcp_configuration(&package, server, &value)?;
    }
    for package in configuration.keys() {
        if !registry
            .catalogue()
            .iter()
            .any(|candidate| candidate.id == *package)
        {
            return Err(format!(
                "Workflow node configures unavailable OTP package {package}"
            ));
        }
    }
    Ok(())
}

fn validate_action(
    registry: &OtpRegistry,
    reference: &crate::otp_api::CapabilityRef,
    configuration: &serde_json::Value,
) -> Result<(), String> {
    let action = registry.tool(reference)?;
    if !matches!(action.entrypoint, Entrypoint::Action { .. })
        || !action.outputs.iter().any(|o| {
            matches!(
                o.kind,
                OutputKind::SessionRequest | OutputKind::SessionStopRequest
            )
        })
    {
        return Err("Node delivery requires a declared Session Request action".into());
    }
    registry.validate_configuration(reference, configuration)
}
