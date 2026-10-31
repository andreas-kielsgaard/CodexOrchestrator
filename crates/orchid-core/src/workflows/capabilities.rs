use super::{authoring::WorkflowNodeBinding, execution::WorkflowExecutionService};
use crate::otp_api::*;
use serde_json::Value;
use std::{
    collections::BTreeMap,
    sync::{Arc, RwLock},
};

/// Local implementations are installed by the application that owns the service.
pub trait WorkflowCapabilityRuntime: Send + Sync {
    fn allow_delivery_retry(
        &self,
        _context: &InvocationContext,
        _request: &SessionRequest,
    ) -> Result<bool, String> {
        Ok(true)
    }

    fn invoke(
        &self,
        context: &InvocationContext,
        configuration: &Value,
        payload: Value,
    ) -> Result<ToolResult, String>;
    fn delivered(
        &self,
        _context: &InvocationContext,
        _attempt_id: &str,
        _ordinal: usize,
        _request: &SessionRequest,
        _receipt: &crate::session_events::SessionEventResult,
    ) -> Result<(), String> {
        Ok(())
    }
}

#[derive(Default)]
pub struct CapabilityRuntimes(RwLock<BTreeMap<String, Arc<dyn WorkflowCapabilityRuntime>>>);

impl CapabilityRuntimes {
    pub fn register(
        &self,
        package: &str,
        runtime: Arc<dyn WorkflowCapabilityRuntime>,
    ) -> Result<(), String> {
        let mut runtimes = self
            .0
            .write()
            .map_err(|_| "Capability runtime registry unavailable")?;
        if runtimes.contains_key(package) {
            return Err(format!(
                "Capability runtime {package} is already registered"
            ));
        }
        runtimes.insert(package.into(), runtime);
        Ok(())
    }
    pub fn invoke(
        &self,
        context: &InvocationContext,
        configuration: &Value,
        payload: Value,
    ) -> Result<ToolResult, String> {
        let runtime = self
            .0
            .read()
            .map_err(|_| "Capability runtime registry unavailable")?
            .get(&context.capability.package)
            .cloned()
            .ok_or_else(|| {
                format!(
                    "Capability runtime {} is not installed on this device",
                    context.capability.package
                )
            })?;
        runtime.invoke(context, configuration, payload)
    }
    pub fn allow_delivery_retry(
        &self,
        context: &InvocationContext,
        request: &SessionRequest,
    ) -> Result<bool, String> {
        let runtime = self
            .0
            .read()
            .map_err(|_| "Capability registry unavailable")?
            .get(&context.capability.package)
            .cloned();
        runtime
            .map(|runtime| runtime.allow_delivery_retry(context, request))
            .unwrap_or(Ok(true))
    }
    pub fn delivered(
        &self,
        context: &InvocationContext,
        attempt_id: &str,
        ordinal: usize,
        request: &SessionRequest,
        receipt: &crate::session_events::SessionEventResult,
    ) -> Result<(), String> {
        let runtime = self
            .0
            .read()
            .map_err(|_| "Capability runtime registry unavailable")?
            .get(&context.capability.package)
            .cloned();
        if let Some(runtime) = runtime {
            runtime.delivered(context, attempt_id, ordinal, request, receipt)?;
        }
        Ok(())
    }
}

impl WorkflowExecutionService {
    pub fn emit_capability_output(
        &self,
        instance_id: &str,
        node_id: &str,
        occurrence_id: &str,
        output: &OutputRef,
        payload: Value,
    ) -> Result<RoutingReceipt, String> {
        if occurrence_id.trim().is_empty() {
            return Err("Capability occurrence identity is required".into());
        }
        let instance = self.instances.load(instance_id)?;
        let node = instance
            .recipe
            .nodes
            .iter()
            .find(|node| node.node_id == node_id)
            .ok_or("Capability node is missing")?;
        let WorkflowNodeBinding::Capability(binding) = &node.binding else {
            return Err("Service events require a capability node".into());
        };
        let descriptor = self.registry.node_capability(&binding.capability)?;
        if !descriptor.outputs.contains(output) {
            return Err("Capability node does not declare this output".into());
        }
        let context = InvocationContext {
            instance_id: instance_id.into(),
            occurrence_id: occurrence_id.into(),
            capability: output.capability.clone(),
            source: Some(SourceContext {
                node_id: node_id.into(),
                node_name: node.name.clone(),
                session_id: None,
                invocation_id: None,
            }),
            connection_id: None,
            output_node_id: None,
        };
        self.route_otp_output(&instance, &context, &output.output, payload)
    }
}
