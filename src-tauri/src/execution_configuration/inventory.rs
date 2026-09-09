//! Observed native inventory is descriptive; it is not a product tool allowlist.
use serde::Serialize;

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeCapabilityEntry {
    pub(crate) name: String,
    pub(crate) kind: String,
    pub(crate) origin: String,
    pub(crate) state: String,
    pub(crate) support: String,
}

#[derive(Clone, Debug, Default, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct NativeCapabilityInventory {
    pub(crate) entries: Vec<NativeCapabilityEntry>,
    pub(crate) limitations: Vec<String>,
}
