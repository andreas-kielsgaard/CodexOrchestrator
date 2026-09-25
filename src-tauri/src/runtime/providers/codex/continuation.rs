use crate::{
    agent_sessions::domain::ExternalRuntimeContextId,
    execution_configuration::SelectedRuntimeProfileSource,
    runtime::providers::continuation::ProviderContinuationPort,
};
use orchid_engine::{
    contracts::provider::ProviderContinuationPayload,
    providers::codex::app_server::continuation,
};
use std::sync::Arc;

pub(crate) struct CodexContinuationPort {
    program: String,
    configurations: Arc<dyn SelectedRuntimeProfileSource>,
}

impl CodexContinuationPort {
    pub(crate) fn new(
        program: impl Into<String>,
        configurations: Arc<dyn SelectedRuntimeProfileSource>,
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
}
