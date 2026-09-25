use crate::{
    agent_sessions::domain::ExternalRuntimeContextId,
    execution_configuration::ProviderConfigurationSource,
    runtime::providers::continuation::ProviderContinuationPort,
};
use orchid_engine::{
    contracts::provider::ProviderContinuationPayload,
    providers::codex::app_server::continuation,
};
use std::sync::Arc;

pub(crate) struct CodexContinuationPort {
    program: String,
    configurations: Arc<dyn ProviderConfigurationSource>,
}

impl CodexContinuationPort {
    pub(crate) fn new(
        program: impl Into<String>,
        configurations: Arc<dyn ProviderConfigurationSource>,
    ) -> Self {
        Self {
            program: program.into(),
            configurations,
        }
    }
}

impl ProviderContinuationPort for CodexContinuationPort {
    fn export(
        &self,
        configuration_ref: &str,
        external_context_id: &ExternalRuntimeContextId,
    ) -> Result<ProviderContinuationPayload, String> {
        let home = self
            .configurations
            .configuration_home(configuration_ref)
            .map_err(|error| error.to_string())?;
        continuation::encode(
            continuation::export(&self.program, &home, external_context_id.as_str())
                .map_err(|error| error.to_string())?,
        )
        .map_err(|error| error.to_string())
    }

    fn install(
        &self,
        configuration_ref: &str,
        payload: &ProviderContinuationPayload,
    ) -> Result<(), String> {
        let home = self
            .configurations
            .configuration_home(configuration_ref)
            .map_err(|error| error.to_string())?;
        let native = continuation::decode(payload).map_err(|error| error.to_string())?;
        continuation::install(&self.program, &home, &native).map_err(|error| error.to_string())
    }

    fn fork(
        &self,
        configuration_ref: &str,
        external_context_id: &ExternalRuntimeContextId,
        working_directory: &str,
    ) -> Result<ExternalRuntimeContextId, String> {
        let home = self
            .configurations
            .configuration_home(configuration_ref)
            .map_err(|error| error.to_string())?;
        let forked = continuation::fork(
            &self.program,
            &home,
            external_context_id.as_str(),
            working_directory,
        )
        .map_err(|error| error.to_string())?;
        ExternalRuntimeContextId::new(forked).map_err(|error| error.to_string())
    }
}
