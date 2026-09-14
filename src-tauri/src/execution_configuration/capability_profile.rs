use super::runtime_profile::{validate_identifier, CapabilitySet};
use serde::{Deserialize, Serialize};

pub(crate) const CAPABILITY_PROFILE_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CapabilityProfile {
    #[serde(default)]
    pub(crate) execution: crate::execution_targets::domain::ExecutionBinding,
    #[serde(default)]
    pub(crate) defaults: super::runtime_profile::RuntimeSelections,
    pub(crate) contract_version: u32,
    pub(crate) capability_profile_id: String,
    pub(crate) name: String,
    pub(crate) revision: u64,
    pub(crate) allowed_capabilities: CapabilitySet,
}

impl CapabilityProfile {
    pub(crate) fn validate(&self) -> Result<(), String> {
        self.execution.validate()?;
        if self.contract_version != CAPABILITY_PROFILE_CONTRACT_VERSION {
            return Err(format!(
                "Capability Profile contract version {} is unsupported",
                self.contract_version
            ));
        }
        validate_identifier(
            "Capability Profile",
            "capabilityProfileId",
            &self.capability_profile_id,
        )?;
        validate_identifier("Capability Profile", "name", &self.name)?;
        if self.revision == 0 {
            return Err("Capability Profile revision must be positive".into());
        }
        self.allowed_capabilities
            .validate("Capability Profile allowed capabilities")?;
        self.defaults.validate("Capability Profile defaults")?;
        super::runtime_profile::validate_selection_availability(
            &self.defaults,
            &self.allowed_capabilities,
        )
    }
}
