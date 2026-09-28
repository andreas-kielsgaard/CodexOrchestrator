//! Provider parts an Orchid host uses for its registered configurations. The host owns dispatch,
//! Session binding and transport; each provider owns its executable, native home and formats.
use crate::{
    contracts::{
        AgentRuntime, ProviderConfigurationRef, RuntimePortError,
    },
    protocol::RuntimeCapabilities,
};
use serde::{Deserialize, Deserializer, Serialize};
use serde_json::{json, Value};
use std::{collections::HashMap, path::PathBuf, sync::Arc};

/// One provider configuration registered on a host.
#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostProviderConfiguration {
    pub id: String,
    pub provider: String,
    /// Opaque provider-owned host configuration. Only the selected host provider decodes it.
    pub settings: Value,
}

fn codex_provider() -> String {
    crate::providers::codex::options::PROVIDER.into()
}

impl<'de> Deserialize<'de> for HostProviderConfiguration {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct Wire {
            id: String,
            #[serde(default = "codex_provider")]
            provider: String,
            #[serde(default)]
            settings: Option<Value>,
            #[serde(default)]
            executable: Option<String>,
            #[serde(default)]
            home: Option<PathBuf>,
        }
        let wire = Wire::deserialize(deserializer)?;
        let settings = match wire.settings {
            Some(settings) => settings,
            None if wire.provider == crate::providers::codex::options::PROVIDER => json!({
                "executable": wire.executable.ok_or_else(|| serde::de::Error::missing_field("settings.executable"))?,
                "home": wire.home.ok_or_else(|| serde::de::Error::missing_field("settings.home"))?,
            }),
            None => return Err(serde::de::Error::missing_field("settings")),
        };
        Ok(Self {
            id: wire.id,
            provider: wire.provider,
            settings,
        })
    }
}

pub trait HostProvider: Send + Sync {
    fn runtime(
        &self,
        configuration: &HostProviderConfiguration,
    ) -> Result<Arc<dyn AgentRuntime>, RuntimePortError>;

    /// Native environment for every launch of this configuration, such as its home folder.
    fn launch_environment(
        &self,
        configuration: &HostProviderConfiguration,
    ) -> Result<Vec<(String, String)>, RuntimePortError>;

    fn capabilities(
        &self,
        configuration: &HostProviderConfiguration,
        reference: ProviderConfigurationRef,
        working_directory: Option<PathBuf>,
    ) -> Result<RuntimeCapabilities, RuntimePortError>;

}

/// Providers an Orchid host can run, keyed by provider identity.
pub fn registered() -> HashMap<String, Arc<dyn HostProvider>> {
    HashMap::from([(
        crate::providers::codex::options::PROVIDER.to_string(),
        Arc::new(crate::providers::codex::host::CodexHostProvider) as Arc<dyn HostProvider>,
    )])
}
