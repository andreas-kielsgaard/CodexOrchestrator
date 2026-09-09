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
        self.catalogue()
            .into_iter()
            .filter_map(|p| {
                let names = p
                    .tools
                    .into_iter()
                    .filter(|t| matches!(t.entrypoint, Entrypoint::Mcp { .. }))
                    .map(|t| t.id)
                    .collect::<BTreeSet<_>>();
                (!names.is_empty()).then_some((p.id, names))
            })
            .collect()
    }
}
pub(crate) mod mcp;
pub(crate) mod workflow;

pub(crate) mod session_control;
