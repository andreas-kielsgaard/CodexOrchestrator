use super::NodeProfile;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

/// Inputs resolved only when a Session is created. The event layer carries these opaquely.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SessionCreationIntent {
    pub capability_profile_id: String,
    pub node_profile: NodeProfile,
    #[serde(default)]
    pub agent_mcp_configuration: BTreeMap<String, serde_json::Value>,
    pub working_directory: String,
    pub title: String,
}
