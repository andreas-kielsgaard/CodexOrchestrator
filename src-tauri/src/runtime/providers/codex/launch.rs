//! Codex launch preparation: attaches the configuration's Codex home and keeps each Session bound
//! to the home its native conversation lives in.
use crate::agent_sessions::{
    application::ProviderLaunchPreparation,
    domain::{AgentInvocationId, AgentSessionId},
    ports::RuntimeLaunchExtension,
};
use crate::runtime::providers::codex::profiles::NativeProfileService;
use orchid_engine::contracts::ProviderConfigurationRef;
use std::sync::Arc;

pub(crate) struct CodexLaunchPreparation(pub(crate) Arc<NativeProfileService>);

/// Codex prepares only Codex configurations; another provider's reference is never reinterpreted.
fn codex_configuration_id(configuration: &ProviderConfigurationRef) -> Result<&str, String> {
    if configuration.provider != orchid_engine::providers::codex::options::PROVIDER {
        return Err(format!(
            "Codex cannot prepare a launch for agent provider `{}`",
            configuration.provider
        ));
    }
    Ok(&configuration.configuration_id)
}

impl ProviderLaunchPreparation for CodexLaunchPreparation {
    fn prepare_launch(
        &self,
        configuration: &ProviderConfigurationRef,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        resuming: bool,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<RuntimeLaunchExtension, String> {
        self.0.prepare_configured_agent_session_launch(
            codex_configuration_id(configuration)?,
            session_id.as_str(),
            invocation_id.as_str(),
            resuming,
            extension,
        )
    }

    fn prepare_destination_launch(
        &self,
        configuration: &ProviderConfigurationRef,
        session_id: &AgentSessionId,
        invocation_id: &AgentInvocationId,
        resuming: bool,
        extension: Option<RuntimeLaunchExtension>,
    ) -> Result<RuntimeLaunchExtension, String> {
        self.0.prepare_destination_native_launch(
            codex_configuration_id(configuration)?,
            session_id.as_str(),
            invocation_id.as_str(),
            resuming,
            extension,
        )
    }

    fn commit_destination(
        &self,
        configuration: &ProviderConfigurationRef,
        session_id: &AgentSessionId,
    ) -> Result<(), String> {
        self.0
            .bind_session_home(codex_configuration_id(configuration)?, session_id.as_str())
    }

    fn bound_configuration_ref(&self, session_id: &AgentSessionId) -> Result<Option<String>, String> {
        self.0.bound_profile_id(session_id.as_str())
    }
}
