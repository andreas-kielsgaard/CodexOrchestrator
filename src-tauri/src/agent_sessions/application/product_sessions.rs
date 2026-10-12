use super::*;
use crate::agent_sessions::ports::InvocationContent;
use crate::execution_configuration::{NodeProfile, SessionCreationRequest, SessionProfileResolver};
use std::collections::{BTreeMap, BTreeSet};

#[derive(Default)]
pub(crate) struct ProductSessionOptions {
    pub(crate) defaults: crate::execution_configuration::RuntimeSelections,
    pub(crate) fast_mode: Option<bool>,
}

impl AgentSessionApplication {
    /// Creates an ordinary profiled Session with explicitly selected product tools.
    pub(crate) fn create_product_session(
        &self,
        title: String,
        directory: Option<String>,
        profile_id: &str,
        tools: BTreeMap<String, BTreeSet<String>>,
    ) -> Result<crate::agent_sessions::domain::AgentSession, String> {
        self.create_product_session_configured(title, directory, profile_id, tools, ProductSessionOptions::default())
    }

    pub(crate) fn create_product_session_configured(
        &self,
        title: String,
        directory: Option<String>,
        profile_id: &str,
        tools: BTreeMap<String, BTreeSet<String>>,
        options: ProductSessionOptions,
    ) -> Result<crate::agent_sessions::domain::AgentSession, String> {
        let profiles = self
            .capability_profiles
            .as_ref()
            .ok_or("Capability Profiles unavailable")?;
        let mut capability = profiles.read(profile_id).map_err(|e| e.to_string())?;
        if capability.execution.is_remote() {
            return Err("Choose a laptop Capability Profile for this conversation".into());
        }
        for route in &mut capability.route_policies {
            route
                .mcp_groups
                .extend(tools.keys().map(|name| format!("otp:{name}:mcps")));
        }
        for (name, names) in &tools {
            capability
                .allowed_capabilities
                .mcp_tools
                .entry(name.clone())
                .or_default()
                .extend(names.clone());
        }
        let mut runtime = profiles
            .runtime_for_binding(&capability.execution, directory.as_deref())
            .map_err(|e| e.to_string())?;
        if let Some(fast) = options.fast_mode {
            if capability.execution.provider != "codex" { return Err("Fast mode requires the Codex provider".into()); }
            let route = capability.route_policies.iter_mut().find(|route| route.execution == capability.execution);
            let base = route.as_ref().and_then(|route| route.provider_options.as_ref()).or(runtime.provider_options.as_ref());
            let mut native = base.map(orchid_engine::providers::codex::options::CodexNativeOptions::decode)
                .transpose().map_err(|e| e.to_string())?.unwrap_or_default();
            native.fast_mode = Some(fast);
            let envelope = native.encode();
            if let Some(route) = route { route.provider_options = Some(envelope.clone()); }
            runtime.provider_options = Some(envelope);
        }
        let skills = self.compile_capability_skill_inputs(
            &capability,
            &capability.execution,
            directory.as_deref(),
        )?;
        let mut allowed = configuration::default_node_capabilities(&capability);
        allowed.mcp_tools.extend(tools);
        let resolution = SessionProfileResolver::resolve_snapshot(
            runtime,
            SessionCreationRequest {
                contract_version: 1,
                capability_profile: capability,
                node_profile: NodeProfile {
                    contract_version: 1,
                    allowed_capabilities: allowed,
                    pinned_defaults: options.defaults,
                },
                agent_mcp_configuration: Default::default(),
                session_skill_inputs: skills,
            },
        )
        .map_err(|e| e.to_string())?;
        self.create_session_with_ownership(
            CreateAgentSessionCommand {
                title: Some(title),
                working_directory: directory,
                requested_options: runtime_options(resolution.session_profile().pinned_defaults()),
            },
            AgentSessionOwnership {
                session_profile: Some(resolution),
                ..Default::default()
            },
        )
        .map_err(|e| e.to_string())
    }

    pub(crate) fn deliver_product_content(
        &self,
        session_id: crate::agent_sessions::domain::AgentSessionId,
        provenance: crate::agent_sessions::domain::AgentInvocationInputProvenance,
        content: InvocationContent,
    ) -> Result<SendAgentSessionMessageLaunchResult, AgentSessionApplicationError> {
        let acknowledgement = self.accept_queued_content(
            super::preparation::PreparedMessageInput {
                session_id: Some(session_id),
                submission_id: self.ids.invocation_id(),
                submitted_text: content.primary_query,
                context: content.context,
                attachments: vec![],
                title: None,
                working_directory: None,
                execution_selection: None,
                model: None,
                reasoning_mode: None,
                sandbox_mode: None,
                folder_target: None,
            },
            provenance,
            true,
        )?;
        Ok(SendAgentSessionMessageLaunchResult {
            acknowledgement,
            launch_accepted: false,
        })
    }

    pub(super) fn run_pinned_preparation(
        &self,
        mut preparation: crate::agent_sessions::preparation::SessionPreparation,
        cancel: &std::sync::atomic::AtomicBool,
    ) -> Result<(), AgentSessionApplicationError> {
        use crate::agent_sessions::preparation::{
            PreparationPhase, PreparationStep, PreparationStepStatus,
        };
        Self::check_preparation_cancel(cancel)?;
        let invocation = self
            .repository
            .get_invocation(&preparation.invocation_id)
            .map_err(AgentSessionApplicationError::repository)?
            .ok_or_else(|| {
                AgentSessionApplicationError::not_found("Queued invocation not found")
            })?;
        preparation.phase = PreparationPhase::Preparing;
        preparation.delivery_started = true;
        self.save_progress(&preparation)?;
        let result = self.delivery().deliver(InvocationDeliveryIntent {
            invocation_id: Some(invocation.id),
            provenance: invocation.input_provenance,
            session_id: Some(preparation.session_id.clone()),
            title: None,
            working_directory: None,
            requested_options: Some(invocation.requested_options),
            content: InvocationContent {
                primary_query: invocation.submitted_text,
                context: preparation.context.clone(),
            },
            attachments: invocation.attachments,
            launch_extension: None,
            prepared_only: true,
        })?;
        preparation.phase = PreparationPhase::Ready;
        preparation.steps = vec![PreparationStep {
            id: "delivery".into(),
            label: "Deliver submitted prompt".into(),
            status: if result.launch_accepted {
                PreparationStepStatus::Completed
            } else {
                PreparationStepStatus::Failed
            },
            error: None,
        }];
        self.save_progress(&preparation)
    }
}
