use crate::otp_api::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub struct OtpRegistry {
    packages: BTreeMap<String, Arc<dyn OtpPackage>>,
    schemas: BTreeMap<(String, String, String), jsonschema::Validator>,
}

impl OtpRegistry {
    pub fn import(imports: &[&str]) -> Result<Arc<Self>, String> {
        let mut packages = BTreeMap::new();
        let mut schemas = BTreeMap::new();
        for id in imports {
            let package = crate::otp_packages::instantiate(id)?;
            let descriptor = package.descriptor();
            if descriptor.contract_version != 1 {
                return Err("Unsupported OTP contract".into());
            }
            let mut compile =
                |id: &str, slot: &str, schema: &serde_json::Value| -> Result<(), String> {
                    let validator = jsonschema::validator_for(schema).map_err(|error| {
                        format!("Invalid {}.{id} {slot} schema: {error}", descriptor.id)
                    })?;
                    if schemas
                        .insert((descriptor.id.clone(), id.into(), slot.into()), validator)
                        .is_some()
                    {
                        return Err(format!(
                            "Duplicate OTP schema: {}.{id} {slot}",
                            descriptor.id
                        ));
                    }
                    Ok(())
                };
            for node in &descriptor.node_capabilities {
                compile(&node.id, "node", &node.configuration_schema)?;
            }
            for tool in &descriptor.tools {
                if let Entrypoint::Mcp { input_schema } = &tool.entrypoint {
                    compile(&tool.id, "input", input_schema)?;
                }
                for output in &tool.outputs {
                    compile(&tool.id, &format!("output:{}", output.id), &output.schema)?;
                }
            }
            if packages.insert(descriptor.id.clone(), package).is_some() {
                return Err(format!("Duplicate OTP import: {id}"));
            }
        }
        Ok(Arc::new(Self { packages, schemas }))
    }

    fn validate_schema(
        &self,
        reference: &CapabilityRef,
        slot: &str,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        self.schemas
            .get(&(
                reference.package.clone(),
                reference.tool.clone(),
                slot.into(),
            ))
            .ok_or_else(|| {
                format!(
                    "Missing OTP schema: {}.{} {slot}",
                    reference.package, reference.tool
                )
            })?
            .validate(value)
            .map_err(|error| error.to_string())
    }

    pub fn validate_node_configuration(
        &self,
        reference: &CapabilityRef,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        self.validate_schema(reference, "node", value)
    }

    pub fn validate_output(
        &self,
        reference: &CapabilityRef,
        output: &str,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        self.validate_schema(reference, &format!("output:{output}"), value)
    }

    pub fn catalogue(&self) -> Vec<PackageDescriptor> {
        self.packages.values().map(|p| p.descriptor()).collect()
    }

    pub fn tool(&self, reference: &CapabilityRef) -> Result<ToolDescriptor, String> {
        self.packages
            .get(&reference.package)
            .ok_or_else(|| format!("OTP package {} is not imported", reference.package))?
            .descriptor()
            .tools
            .into_iter()
            .find(|t| t.id == reference.tool)
            .ok_or_else(|| format!("OTP tool {} is unavailable", reference.tool))
    }

    pub fn node_capability(
        &self,
        reference: &CapabilityRef,
    ) -> Result<NodeCapabilityDescriptor, String> {
        self.packages
            .get(&reference.package)
            .ok_or_else(|| format!("OTP package {} is not imported", reference.package))?
            .descriptor()
            .node_capabilities
            .into_iter()
            .find(|node| node.id == reference.tool)
            .ok_or_else(|| format!("Node capability {} is unavailable", reference.tool))
    }

    pub fn agent_mcp_server(
        &self,
        server_name: &str,
    ) -> Result<(String, AgentMcpServerDescriptor), String> {
        self.packages
            .iter()
            .find_map(|(package, value)| {
                value
                    .descriptor()
                    .agent_mcp_servers
                    .into_iter()
                    .find(|server| server.server_name == server_name)
                    .map(|server| (package.clone(), server))
            })
            .ok_or_else(|| format!("OTP agent MCP server {server_name} is unavailable"))
    }

    pub fn validate_configuration(
        &self,
        reference: &CapabilityRef,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        self.tool(reference)?;
        self.packages[&reference.package].validate_configuration(&reference.tool, value)
    }

    pub fn invoke(
        &self,
        context: &InvocationContext,
        input: ToolInput,
        host: &dyn OtpHost,
    ) -> Result<ToolResult, String> {
        let descriptor = self.tool(&context.capability)?;
        if let (Entrypoint::Mcp { .. }, ToolInput::Mcp(arguments)) =
            (&descriptor.entrypoint, &input)
        {
            self.validate_schema(&context.capability, "input", arguments)?;
        }
        let matches = matches!(
            (&descriptor.entrypoint, &input),
            (Entrypoint::Mcp { .. }, ToolInput::Mcp(_))
                | (
                    Entrypoint::SessionEvent { .. },
                    ToolInput::SessionEvent { .. }
                )
                | (Entrypoint::Action { .. }, ToolInput::Action { .. })
        );
        if !matches {
            return Err("OTP entrypoint does not accept this input".into());
        }
        self.packages[&context.capability.package].invoke(context, input, host)
    }

    pub fn mcp_tools(&self) -> BTreeMap<String, BTreeSet<String>> {
        let mut result = BTreeMap::new();
        for package in self.catalogue() {
            let tools = package
                .tools
                .into_iter()
                .filter(|tool| matches!(tool.entrypoint, Entrypoint::Mcp { .. }))
                .map(|tool| tool.id)
                .collect::<BTreeSet<_>>();
            if !tools.is_empty() {
                result.insert(package.id, tools);
            }
            for server in package.agent_mcp_servers {
                result.insert(
                    server.server_name,
                    server.tools.into_iter().map(|tool| tool.id).collect(),
                );
            }
        }
        result
    }
}

pub mod installations;
pub mod job_agent;
pub mod mcp;
pub mod session_control;
pub mod workflow;
