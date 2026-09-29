//! Codex on an Orchid host: the app-server runtime, `CODEX_HOME`, and native discovery.
use super::{
    app_server::{
        environment::{CodexEnvironmentReader, CodexEnvironmentSource},
        CodexAppServerRuntime,
    },
    runtime_profile,
};
use crate::{
    contracts::{
        AgentRuntime, ProviderConfigurationRef, RuntimePortError,
    },
    host::providers::{HostProvider, HostProviderConfiguration},
    protocol::RuntimeCapabilities,
};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};

pub struct CodexHostProvider;

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
struct CodexHostConfiguration {
    executable: String,
    home: PathBuf,
}

fn decode(
    configuration: &HostProviderConfiguration,
) -> Result<CodexHostConfiguration, RuntimePortError> {
    serde_json::from_value(configuration.settings.clone()).map_err(|error| {
        RuntimePortError::new(
            crate::contracts::RuntimePortErrorKind::Unavailable,
            format!("Invalid Codex host configuration `{}`: {error}", configuration.id),
        )
    })
}

impl HostProvider for CodexHostProvider {
    fn runtime(
        &self,
        configuration: &HostProviderConfiguration,
    ) -> Result<Arc<dyn AgentRuntime>, RuntimePortError> {
        Ok(Arc::new(CodexAppServerRuntime::system(
            decode(configuration)?.executable,
        )))
    }

    fn launch_environment(
        &self,
        configuration: &HostProviderConfiguration,
    ) -> Result<Vec<(String, String)>, RuntimePortError> {
        let configuration = decode(configuration)?;
        Ok(vec![(
            "CODEX_HOME".into(),
            configuration.home.to_string_lossy().into_owned(),
        )])
    }

    fn capabilities(
        &self,
        configuration: &HostProviderConfiguration,
        reference: ProviderConfigurationRef,
        working_directory: Option<PathBuf>,
    ) -> Result<RuntimeCapabilities, RuntimePortError> {
        let configuration = decode(configuration)?;
        let reader = CodexEnvironmentReader::new(&configuration.executable);
        let native = reader.read(configuration.home.clone(), working_directory.clone())?;
        let profile = runtime_profile::runtime_profile(&native, reference, Default::default());
        let inventory = reader.inventory(configuration.home.clone(), working_directory)?;
        Ok(RuntimeCapabilities { profile, inventory })
    }

}
