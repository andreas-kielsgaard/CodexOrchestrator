use super::repository::Binding;
use orchid_coordinator::protocol::Device;
use serde::{Deserialize, Serialize};
#[derive(Clone, Serialize, Deserialize, Default)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Settings {
    pub external_url: Option<String>,
    pub credential_ref: Option<String>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SettingsInput {
    pub external_url: Option<String>,
    pub token: Option<String>,
}
#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct Overview {
    pub mode: String,
    pub endpoint: Option<String>,
    pub coordinator_id: Option<String>,
    pub error: Option<String>,
    pub devices: Vec<Device>,
    pub bindings: Vec<Binding>,
    pub legacy_devices: Vec<crate::execution_devices::ExecutionDeviceDto>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RegisterInput {
    pub credential_id: String,
    pub server_id: u64,
    pub display_name: String,
    pub execution_device_id: Option<String>,
}
