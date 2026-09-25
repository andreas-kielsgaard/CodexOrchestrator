use serde::{Deserialize, Serialize};
use serde_json::Value;

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
