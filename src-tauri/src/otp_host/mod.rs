use crate::otp_api::*;
use std::{
    collections::{BTreeMap, BTreeSet},
    sync::Arc,
};

pub(crate) struct OtpRegistry {
    packages: BTreeMap<String, Arc<dyn OtpPackage>>,
}

impl OtpRegistry {
    pub(crate) fn import(imports: &[&str]) -> Result<Arc<Self>, String> {
        let mut packages = BTreeMap::new();
        for id in imports {
            let package = crate::otp_packages::instantiate(id)?;
            let descriptor = package.descriptor();
            if descriptor.contract_version != 1 {
                return Err("Unsupported OTP contract".into());
            }
            if packages.insert(descriptor.id.clone(), package).is_some() {
                return Err(format!("Duplicate OTP import: {id}"));
            }
        }
        Ok(Arc::new(Self { packages }))
    }

    pub(crate) fn catalogue(&self) -> Vec<PackageDescriptor> {
        self.packages.values().map(|p| p.descriptor()).collect()
    }

    pub(crate) fn tool(&self, reference: &CapabilityRef) -> Result<ToolDescriptor, String> {
        self.packages
            .get(&reference.package)
            .ok_or_else(|| format!("OTP package {} is not imported", reference.package))?
            .descriptor()
            .tools
            .into_iter()
            .find(|t| t.id == reference.tool)
            .ok_or_else(|| format!("OTP tool {} is unavailable", reference.tool))
    }

    pub(crate) fn agent_mcp_server(
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

    pub(crate) fn validate_agent_mcp_configuration(
        &self,
        package: &str,
        server: &str,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        self.packages
            .get(package)
            .ok_or_else(|| format!("OTP package {package} is not imported"))?
            .validate_agent_mcp_configuration(server, value)
    }

    pub(crate) fn validate_configuration(
        &self,
        reference: &CapabilityRef,
        value: &serde_json::Value,
    ) -> Result<(), String> {
        self.tool(reference)?;
        self.packages[&reference.package].validate_configuration(&reference.tool, value)
    }

    pub(crate) fn invoke(
        &self,
        context: &InvocationContext,
        input: ToolInput,
        host: &dyn OtpHost,
    ) -> Result<ToolResult, String> {
        let descriptor = self.tool(&context.capability)?;
        if let (Entrypoint::Mcp { input_schema }, ToolInput::Mcp(arguments)) =
            (&descriptor.entrypoint, &input)
        {
            validate_json(input_schema, arguments)?;
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

    pub(crate) fn mcp_tools(&self) -> BTreeMap<String, BTreeSet<String>> {
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

pub(crate) mod agent_mcp;
pub(crate) mod catalogue;
pub(crate) mod installations;
pub(crate) mod job_agent;
pub(crate) mod mcp;
pub(crate) mod session_control;
pub(crate) mod workflow;
