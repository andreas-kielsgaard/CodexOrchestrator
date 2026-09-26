use serde::{Deserialize, Serialize};
use serde_json::Value;
use std::fmt;

/// One registered configuration of one agent provider. Product code stores and compares this
/// pair; it never parses provider identity out of a configuration string.
#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProviderConfigurationRef {
    pub provider: String,
    pub configuration_id: String,
}

impl ProviderConfigurationRef {
    pub fn new(provider: impl Into<String>, configuration_id: impl Into<String>) -> Self {
        Self {
            provider: provider.into(),
            configuration_id: configuration_id.into(),
        }
    }

    pub fn validate(&self) -> Result<(), String> {
        for (field, value) in [
            ("provider", &self.provider),
            ("configurationId", &self.configuration_id),
        ] {
            if value.is_empty() || value.trim() != value {
                return Err(format!(
                    "Provider configuration {field} must be a non-empty trimmed value"
                ));
            }
        }
        Ok(())
    }
}

/// Label and opaque-key form. It is not a serialization format and is never parsed back.
impl fmt::Display for ProviderConfigurationRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.provider, self.configuration_id)
    }
}

/// An adapter-owned setting envelope. Product code may persist and route it but only the named
/// provider may decode `settings`.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderNativeOptions {
    pub provider: String,
    pub settings: Value,
}

/// Provider-scoped native conversation state. Device transport treats the payload as opaque.
#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderContinuationPayload {
    pub provider: String,
    pub format: String,
    pub payload: Value,
}
