//! Observed native inventory is descriptive; it is not a product tool allowlist.
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCapabilityEntry {
    pub name: String,
    pub kind: String,
    pub origin: String,
    pub state: String,
    pub support: String,
}

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct NativeCapabilityInventory {
    pub entries: Vec<NativeCapabilityEntry>,
    pub limitations: Vec<String>,
}
