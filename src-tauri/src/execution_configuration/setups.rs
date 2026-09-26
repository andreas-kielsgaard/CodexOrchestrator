//! Provider setups registered on a device: a native folder and the CLI executable that uses it.
//! Each provider owns its setup storage, detection and readiness; this read model lets shared
//! screens list every provider's setups together and offer them as Capability Profile routes.
use serde::Serialize;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProviderSetup {
    pub(crate) device_id: String,
    pub(crate) provider: String,
    /// The provider configuration a route names to use this setup.
    pub(crate) configuration_id: String,
    pub(crate) folder: String,
    pub(crate) executable: Option<String>,
    pub(crate) state: ProviderSetupState,
    pub(crate) detail: Option<String>,
    /// The provider's default setup on the device.
    pub(crate) selected: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderSetupState {
    Ready,
    NeedsLogin,
    Unavailable,
}
