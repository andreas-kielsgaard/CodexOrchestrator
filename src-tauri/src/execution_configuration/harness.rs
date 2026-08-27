use super::runtime_profile::{validate_identifier, CapabilitySet};
use serde::{Deserialize, Serialize};

pub(crate) const HARNESS_DEFINITION_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessDefinition {
    pub(crate) contract_version: u32,
    pub(crate) harness_id: String,
    pub(crate) revision: u64,
    pub(crate) allowed_capabilities: CapabilitySet,
}

impl HarnessDefinition {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.contract_version != HARNESS_DEFINITION_CONTRACT_VERSION {
            return Err(format!(
                "Harness definition contract version {} is unsupported",
                self.contract_version
            ));
        }
        validate_identifier("Harness definition", "harnessId", &self.harness_id)?;
        if self.revision == 0 {
            return Err("Harness definition revision must be positive".into());
        }
        self.allowed_capabilities
            .validate("Harness allowed capabilities")
    }
}
