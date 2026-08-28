use super::runtime_profile::{CapabilitySet, RuntimeSelections};
use serde::{Deserialize, Serialize};

pub(crate) const NODE_PROFILE_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NodeProfile {
    pub(crate) contract_version: u32,
    pub(crate) allowed_capabilities: CapabilitySet,
    pub(crate) pinned_defaults: RuntimeSelections,
}

impl NodeProfile {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.contract_version != NODE_PROFILE_CONTRACT_VERSION {
            return Err(format!(
                "Node Profile contract version {} is unsupported",
                self.contract_version
            ));
        }
        self.allowed_capabilities
            .validate("Node Profile allowed capabilities")?;
        self.pinned_defaults
            .validate("Node Profile pinned defaults")
    }
}
