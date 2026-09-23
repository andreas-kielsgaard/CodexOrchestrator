use super::runtime_profile::{
    validate_identifier, validate_selection_availability, CapabilitySet, RuntimeSelections,
};
use crate::agent_sessions::ports::RuntimeSkillInput;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub(crate) const SESSION_PROFILE_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionProfile {
    contract_version: u32,
    runtime_profile_ref: String,
    attached_runtime_capabilities: CapabilitySet,
    attached_runtime_locked: RuntimeSelections,
    capability_profile_id: String,
    capability_profile_revision: u64,
    node_capabilities: CapabilitySet,
    #[serde(default)]
    agent_mcp_configuration: BTreeMap<String, serde_json::Value>,
    /// Immutable skill manifest compiled at session instantiation.
    #[serde(default)]
    session_skill_inputs: Vec<RuntimeSkillInput>,
    pinned_defaults: RuntimeSelections,
    #[serde(default)]
    native_mcp_enabled: Option<bool>,
}

impl SessionProfile {
    pub(super) fn resolved(
        runtime_profile_ref: String,
        attached_runtime_capabilities: CapabilitySet,
        attached_runtime_locked: RuntimeSelections,
        capability_profile_id: String,
        capability_profile_revision: u64,
        node_capabilities: CapabilitySet,
        agent_mcp_configuration: BTreeMap<String, serde_json::Value>,
        session_skill_inputs: Vec<RuntimeSkillInput>,
        pinned_defaults: RuntimeSelections,
        native_mcp_enabled: Option<bool>,
    ) -> Self {
        Self {
            contract_version: SESSION_PROFILE_CONTRACT_VERSION,
            runtime_profile_ref,
            attached_runtime_capabilities,
            attached_runtime_locked,
            capability_profile_id,
            capability_profile_revision,
            node_capabilities,
            agent_mcp_configuration,
            session_skill_inputs,
            pinned_defaults,
            native_mcp_enabled,
        }
    }

    pub(crate) fn runtime_profile_ref(&self) -> &str {
        &self.runtime_profile_ref
    }

    pub(crate) fn attached_runtime_capabilities(&self) -> &CapabilitySet {
        &self.attached_runtime_capabilities
    }

    pub(crate) fn attached_runtime_locked(&self) -> &RuntimeSelections {
        &self.attached_runtime_locked
    }

    pub(crate) fn capability_profile_id(&self) -> &str {
        &self.capability_profile_id
    }

    pub(crate) fn capability_profile_revision(&self) -> u64 {
        self.capability_profile_revision
    }

    pub(crate) fn node_capabilities(&self) -> &CapabilitySet {
        &self.node_capabilities
    }

    pub(crate) fn agent_mcp_configuration(&self) -> &BTreeMap<String, serde_json::Value> {
        &self.agent_mcp_configuration
    }

    pub(crate) fn session_skill_inputs(&self) -> &[RuntimeSkillInput] {
        &self.session_skill_inputs
    }

    pub(crate) fn pinned_defaults(&self) -> &RuntimeSelections {
        &self.pinned_defaults
    }

    pub(crate) fn native_mcp_enabled(&self) -> Option<bool> {
        self.native_mcp_enabled
    }

    pub(super) fn validate(&self) -> Result<(), String> {
        if self.contract_version != SESSION_PROFILE_CONTRACT_VERSION {
            return Err(format!(
                "Session Profile contract version {} is unsupported",
                self.contract_version
            ));
        }
        validate_identifier(
            "Session Profile",
            "runtimeProfileRef",
            &self.runtime_profile_ref,
        )?;
        validate_identifier(
            "Session Profile",
            "capabilityProfileId",
            &self.capability_profile_id,
        )?;
        self.attached_runtime_capabilities
            .validate("Session Profile attached runtime capabilities")?;
        self.attached_runtime_locked
            .validate("Session Profile attached runtime locked selections")?;
        validate_selection_availability(
            &self.attached_runtime_locked,
            &self.attached_runtime_capabilities,
        )
        .map_err(|capability| {
            format!("Session Profile attached runtime locks unavailable {capability}")
        })?;
        if self.capability_profile_revision == 0 {
            return Err("Session Profile capability revision must be positive".into());
        }
        self.node_capabilities
            .validate("Session Profile node capabilities")?;
        self.pinned_defaults
            .validate("Session Profile pinned defaults")?;
        validate_selection_availability(&self.pinned_defaults, &self.attached_runtime_capabilities)
            .map_err(|capability| format!("Session Profile pins unavailable {capability}"))
    }
}
