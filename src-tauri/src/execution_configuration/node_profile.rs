use super::runtime_profile::{validate_identifier, CapabilitySet, RuntimeSelections};
use serde::{Deserialize, Serialize};

pub(crate) const NODE_PROFILE_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Debug, Default, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct InstructionDelivery {
    pub(crate) recurring: Option<String>,
    pub(crate) start_only: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NodeProfileDefinition {
    pub(crate) contract_version: u32,
    pub(crate) node_profile_id: String,
    pub(crate) revision: u64,
    pub(crate) allowed_capabilities: CapabilitySet,
    pub(crate) selections: RuntimeSelections,
    pub(crate) instructions: InstructionDelivery,
}

impl NodeProfileDefinition {
    pub(crate) fn validate(&self) -> Result<(), String> {
        if self.contract_version != NODE_PROFILE_CONTRACT_VERSION {
            return Err(format!(
                "Node Profile contract version {} is unsupported",
                self.contract_version
            ));
        }
        validate_identifier(
            "Node Profile definition",
            "nodeProfileId",
            &self.node_profile_id,
        )?;
        if self.revision == 0 {
            return Err("Node Profile definition revision must be positive".into());
        }
        self.allowed_capabilities
            .validate("Node Profile allowed capabilities")?;
        self.selections.validate("Node Profile selections")?;
        validate_instruction("recurring instructions", &self.instructions.recurring)?;
        validate_instruction("start-only instructions", &self.instructions.start_only)
    }
}

fn validate_instruction(field: &str, value: &Option<String>) -> Result<(), String> {
    if value.as_ref().is_some_and(|value| value.trim().is_empty()) {
        Err(format!("Node Profile {field} must not be blank"))
    } else {
        Ok(())
    }
}
