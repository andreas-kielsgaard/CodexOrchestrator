//! Provider parts an Orchid host uses for its registered configurations. The host owns dispatch,
//! Session binding and transport; each provider owns its executable, native home and formats.
use crate::{
    contracts::{
        provider::ProviderContinuationPayload, AgentRuntime, ExternalRuntimeContextId,
        ProviderConfigurationRef, RuntimePortError, RuntimePortErrorKind,
    },
    protocol::RuntimeCapabilities,
};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

/// One provider configuration registered on a host.
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct HostProviderConfiguration {
    pub id: String,
    /// Host files written before provider identity existed configure Codex.
    #[serde(default = "codex_provider")]
    pub provider: String,
    pub executable: String,
    /// The provider's native home folder for this configuration.
    pub home: PathBuf,
}

fn codex_provider() -> String {
    crate::providers::codex::options::PROVIDER.into()
}

pub trait HostProvider: Send + Sync {
    fn runtime(&self, configuration: &HostProviderConfiguration) -> Arc<dyn AgentRuntime>;

    /// Native environment for every launch of this configuration, such as its home folder.
    fn launch_environment(&self, configuration: &HostProviderConfiguration) -> Vec<(String, String)>;

    fn capabilities(
        &self,
        configuration: &HostProviderConfiguration,
        reference: ProviderConfigurationRef,
        working_directory: Option<PathBuf>,
    ) -> Result<RuntimeCapabilities, RuntimePortError>;

    fn export_continuation(
        &self,
        _configuration: &HostProviderConfiguration,
        _external_context_id: &ExternalRuntimeContextId,
    ) -> Result<ProviderContinuationPayload, RuntimePortError> {
        Err(no_continuation())
    }

    /// The native conversation a continuation payload installs.
    fn continuation_context(
        &self,
        _continuation: &ProviderContinuationPayload,
    ) -> Result<ExternalRuntimeContextId, RuntimePortError> {
        Err(no_continuation())
    }

    fn install_continuation(
        &self,
        _configuration: &HostProviderConfiguration,
        _continuation: &ProviderContinuationPayload,
    ) -> Result<(), RuntimePortError> {
        Err(no_continuation())
    }
}

fn no_continuation() -> RuntimePortError {
    RuntimePortError::new(
        RuntimePortErrorKind::UnsupportedOptions,
        "This agent provider does not transfer native conversations",
    )
}

/// Providers an Orchid host can run, keyed by provider identity.
pub fn registered() -> HashMap<String, Arc<dyn HostProvider>> {
    HashMap::from([(
        crate::providers::codex::options::PROVIDER.to_string(),
        Arc::new(crate::providers::codex::host::CodexHostProvider) as Arc<dyn HostProvider>,
    )])
}
