use crate::contracts::provider::ProviderNativeOptions;
use crate::contracts::runtime::{RuntimePortError, RuntimePortErrorKind};
use serde::{Deserialize, Serialize};

pub const PROVIDER: &str = "codex";

/// Codex-native thread personality. Absence preserves the selected native home's default.
#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CodexPersonality {
    None,
    Friendly,
    Pragmatic,
}

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CodexNativeOptions {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub personality: Option<CodexPersonality>,
}

impl CodexNativeOptions {
    pub fn encode(self) -> ProviderNativeOptions {
        ProviderNativeOptions {
            provider: PROVIDER.into(),
            settings: serde_json::to_value(self).expect("Codex native options serialize"),
        }
    }

    pub fn decode(options: &ProviderNativeOptions) -> Result<Self, RuntimePortError> {
        if options.provider != PROVIDER {
            return Err(RuntimePortError::new(
                RuntimePortErrorKind::UnsupportedOptions,
                format!(
                    "Codex cannot apply native options for provider `{}`",
                    options.provider
                ),
            ));
        }
        serde_json::from_value(options.settings.clone()).map_err(|error| {
            RuntimePortError::new(
                RuntimePortErrorKind::UnsupportedOptions,
                format!("Invalid Codex native options: {error}"),
            )
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rejects_another_providers_native_options() {
        let error = CodexNativeOptions::decode(&ProviderNativeOptions {
            provider: "test-provider".into(),
            settings: serde_json::json!({"personality":"friendly"}),
        })
        .unwrap_err();
        assert_eq!(error.kind, RuntimePortErrorKind::UnsupportedOptions);
    }
}
