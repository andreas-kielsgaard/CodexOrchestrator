use serde::{Deserialize, Serialize};

pub const VERSION: u32 = 1;
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Health {
    pub contract_version: u32,
    pub coordinator_id: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Server {
    pub id: u64,
    pub name: String,
    pub status: String,
    pub address: Option<String>,
    pub location: Option<String>,
}
#[derive(Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Connect {
    pub token: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Discovery {
    pub credential_id: String,
    pub servers: Vec<Server>,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Register {
    pub credential_id: String,
    pub server_id: u64,
    pub display_name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Device {
    pub id: String,
    pub display_name: String,
    pub server: Server,
    pub observed_at: String,
    pub credential_configured: bool,
    pub operation: Option<Operation>,
    pub observation_error: Option<String>,
}
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "snake_case")]
pub enum PowerCommand {
    Start,
    Stop,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct PowerRequest {
    pub operation_id: String,
    pub command: PowerCommand,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Operation {
    pub id: String,
    pub device_id: String,
    pub command: PowerCommand,
    pub status: String,
    pub provider_action_id: Option<u64>,
    pub requested_at: String,
    pub detail: Option<String>,
}
impl Operation {
    pub fn active(&self) -> bool {
        matches!(self.status.as_str(), "accepted" | "running")
    }
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct Rename {
    pub name: String,
}
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct ApiError {
    pub message: String,
}
