use super::compiled_plan::{WorkflowCompiledPlan, WorkflowConnectionPromptInput};
use crate::{
    otp_api::{Entrypoint, OutputKind},
    otp_host::OtpRegistry,
};
use std::collections::BTreeSet;

pub struct WorkflowCompiler;
impl WorkflowCompiler {
    pub fn compile(
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
            if let super::compiled_plan::WorkflowSessionCreation::Capability(binding) =
                &node.session_creation
            {
                registry
                    .validate_node_configuration(&binding.capability, &binding.configuration)?;
            }
        }
        for edge in &plan.connections {
            if !nodes.contains(edge.source_node.identity().id())
                || !nodes.contains(edge.destination_node.identity().id())
            {
                return Err("Connection references a missing node".into());
            }
            let producer = registry.tool(&edge.trigger.capability)?;
            let source = plan
                .nodes
                .iter()
                .find(|node| node.reference == edge.source_node)
                .ok_or("Source node is missing")?;
            let destination = plan
                .nodes
                .iter()
                .find(|node| node.reference == edge.destination_node)
                .ok_or("Destination node is missing")?;
            match &source.session_creation {
                super::compiled_plan::WorkflowSessionCreation::Capability(binding) => {
                    let descriptor = registry.node_capability(&binding.capability)?;
                    if !descriptor.outputs.contains(&edge.trigger) {
                        return Err("Capability node does not offer this output".into());
                    }
                }
                _ if matches!(
                    producer.entrypoint,
                    Entrypoint::Action { .. } | Entrypoint::ServiceEvent
                ) =>
                {
                    return Err("Agent nodes require an MCP or Session Event producer".into());
                }
                _ => {}
            }
            if let super::compiled_plan::WorkflowSessionCreation::Capability(binding) =
                &destination.session_creation
            {
                if !registry
                    .node_capability(&binding.capability)?
                    .actions
                    .contains(&edge.action)
                {
                    return Err("Capability node does not offer this action".into());
                }
            }
            let output = producer
                .outputs
                .iter()
                .find(|o| o.id == edge.trigger.output && o.kind == OutputKind::Data)
                .ok_or("The selected capability does not offer this data output")?;
            validate_action(
                registry,
                &edge.action,
                &edge.configuration,
                matches!(
                    destination.session_creation,
                    super::compiled_plan::WorkflowSessionCreation::Capability(_)
                ),
            )?;
            if matches!(
                registry.tool(&edge.action)?.entrypoint,
                Entrypoint::Action { uses_prompt: true }
            ) && edge.prompt_inputs.is_empty()
                && edge.prompt_text.trim().is_empty()
            {
                return Err("Prompt actions require at least one prompt source".into());
            }
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

fn validate_action(
    registry: &OtpRegistry,
    reference: &crate::otp_api::CapabilityRef,
    configuration: &serde_json::Value,
    capability_node: bool,
) -> Result<(), String> {
    let action = registry.tool(reference)?;
    if !matches!(action.entrypoint, Entrypoint::Action { .. })
        || (!capability_node
            && !action.outputs.iter().any(|o| {
                matches!(
                    o.kind,
                    OutputKind::SessionRequest | OutputKind::SessionStopRequest
                )
            }))
    {
        return Err("Node delivery requires a declared Session Request action".into());
    }
    registry.validate_configuration(reference, configuration)
}
