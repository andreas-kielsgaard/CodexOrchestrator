//! Prepares current MCP connections for a durable Session tool policy.
use super::{
    domain::{HarnessBindingRecord, HarnessBindingStage, SidecarBindingRegistration},
    proxy::proxy_url,
    service::HarnessEngineService,
};
use crate::agent_sessions::{
    application::SessionHarnessLaunchAuthority,
    domain::{AgentInvocationId, AgentSessionId},
    ports::{RuntimeLaunchExtension, RuntimeManagedMcpServer},
};
use chrono::Utc;

impl HarnessEngineService {
    fn live_registration(
        &self,
        binding: &HarnessBindingRecord,
    ) -> Result<SidecarBindingRegistration, String> {
        let resolved = binding.parsed_plan()?.resolve(|name| {
            self.upstreams
                .resolve_for_session(Some(&binding.session_id), name)
        })?;
        SidecarBindingRegistration::from_record(binding, &resolved)
    }
    fn proxy_extension(
        &self,
        binding: &HarnessBindingRecord,
        mut extension: RuntimeLaunchExtension,
    ) -> Result<RuntimeLaunchExtension, String> {
        reject_caller_mcp_configuration(&extension)?;
        let token = binding
            .harness_token
            .as_deref()
            .ok_or_else(|| "Bound Harness has no Harness token.".to_string())?;
        let plan = binding.parsed_plan()?;
        let address = self.sidecar.proxy_address()?;
        for (index, exposure) in plan.exposures.iter().enumerate() {
            let name = &exposure.proxy_server_name;
            let url = proxy_url(address, token, index);
            extension.managed_mcp_servers.push(RuntimeManagedMcpServer {
                name: name.clone(),
                url,
            });
        }
        Ok(extension)
    }

    pub(super) fn complete_prepared_binding(
        &self,
        binding: &HarnessBindingRecord,
    ) -> Result<HarnessBindingRecord, String> {
        let token = self
            .sidecar
            .register_binding(self.live_registration(binding)?)?;
        match self
            .repository
            .mark_bound(&binding.id, &token, &Utc::now().to_rfc3339())
        {
            Ok(bound) => Ok(bound),
            Err(error) => {
                let _ = self.sidecar.retire_binding(&binding.id);
                Err(error)
            }
        }
    }
}

impl SessionHarnessLaunchAuthority for HarnessEngineService {
    fn prepare_launch(
        &self,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<Option<RuntimeLaunchExtension>, String> {
        let Some(mut binding) = self.repository.current_for_session(session_id.as_str())? else {
            return Ok(extension);
        };
        if binding.stage == HarnessBindingStage::Prepared {
            binding = self.complete_prepared_binding(&binding)?;
        }
        if binding.stage != HarnessBindingStage::Bound {
            return Err("The Session Harness binding is not available for invocation.".to_string());
        }
        binding.verify_digest()?;
        let registration = self.live_registration(&binding)?;
        self.sidecar.ensure_binding(registration)?;
        self.sidecar
            .prepare_invocation(&binding.id, invocation_id.as_str())?;
        self.proxy_extension(&binding, extension.unwrap_or_default())
            .map(Some)
    }
}

pub(super) fn reject_caller_mcp_configuration(
    extension: &RuntimeLaunchExtension,
) -> Result<(), String> {
    if !extension.managed_mcp_servers.is_empty()
        || extension
            .config_overrides
            .iter()
            .any(|argument| argument.to_ascii_lowercase().contains("mcp_servers"))
    {
        return Err(
            "A Harness-bound Session cannot accept caller-supplied MCP configuration.".to_string(),
        );
    }
    Ok(())
}
