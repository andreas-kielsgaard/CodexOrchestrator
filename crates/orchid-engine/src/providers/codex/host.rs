//! Codex on an Orchid host: the app-server runtime, `CODEX_HOME`, native discovery and thread
//! transfer. The host owns dispatch and transport.
use super::{
    app_server::{
        continuation,
        environment::{CodexEnvironmentReader, CodexEnvironmentSource},
        CodexAppServerRuntime,
    },
    runtime_profile,
};
use crate::{
    contracts::{
        provider::ProviderContinuationPayload, AgentRuntime, ExternalRuntimeContextId,
        ProviderConfigurationRef, RuntimePortError,
    },
    host::providers::{HostProvider, HostProviderConfiguration},
    protocol::RuntimeCapabilities,
};
use std::{path::PathBuf, sync::Arc};

pub struct CodexHostProvider;

impl HostProvider for CodexHostProvider {
    fn runtime(&self, configuration: &HostProviderConfiguration) -> Arc<dyn AgentRuntime> {
        Arc::new(CodexAppServerRuntime::system(&configuration.executable))
    }

    fn launch_environment(&self, configuration: &HostProviderConfiguration) -> Vec<(String, String)> {
        vec![(
            "CODEX_HOME".into(),
            configuration.home.to_string_lossy().into_owned(),
        )]
    }

    fn capabilities(
        &self,
        configuration: &HostProviderConfiguration,
        reference: ProviderConfigurationRef,
        working_directory: Option<PathBuf>,
    ) -> Result<RuntimeCapabilities, RuntimePortError> {
        let reader = CodexEnvironmentReader::new(&configuration.executable);
        let native = reader.read(configuration.home.clone(), working_directory.clone())?;
        let profile = runtime_profile::runtime_profile(&native, reference, Default::default());
        let inventory = reader.inventory(configuration.home.clone(), working_directory)?;
        Ok(RuntimeCapabilities { profile, inventory })
    }

    fn export_continuation(
        &self,
        configuration: &HostProviderConfiguration,
        external_context_id: &ExternalRuntimeContextId,
    ) -> Result<ProviderContinuationPayload, RuntimePortError> {
        continuation::encode(continuation::export(
            &configuration.executable,
            &configuration.home,
            external_context_id.as_str(),
        )?)
    }

    fn continuation_context(
        &self,
        payload: &ProviderContinuationPayload,
    ) -> Result<ExternalRuntimeContextId, RuntimePortError> {
        let native = continuation::decode(payload)?;
        ExternalRuntimeContextId::new(&native.thread_id).map_err(|error| {
            RuntimePortError::new(
                crate::contracts::RuntimePortErrorKind::Unavailable,
                error.to_string(),
            )
        })
    }

    fn install_continuation(
        &self,
        configuration: &HostProviderConfiguration,
        payload: &ProviderContinuationPayload,
    ) -> Result<(), RuntimePortError> {
        continuation::install(
            &configuration.executable,
            &configuration.home,
            &continuation::decode(payload)?,
        )
    }
}
