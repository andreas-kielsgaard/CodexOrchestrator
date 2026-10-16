//! Provider setups registered on a device: a native folder and the CLI executable that uses it.
//! Each provider owns its setup storage, detection and readiness; this read model lets shared
//! screens list every provider's setups together and offer them as Capability Profile routes.
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderSetup {
    pub device_id: String,
    pub provider: String,
    /// The provider configuration a route names to use this setup.
    pub configuration_id: String,
    pub folder: String,
    pub executable: Option<String>,
    pub state: ProviderSetupState,
    pub detail: Option<String>,
    /// The provider's default setup on the device.
    pub selected: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ProviderSetupState {
    Ready,
    NeedsLogin,
    Unavailable,
}
