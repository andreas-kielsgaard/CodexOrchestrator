use super::runtime_profile::{validate_identifier, CapabilitySet, RuntimeSelections};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const CAPABILITY_PROFILE_CONTRACT_VERSION: u32 = 1;

/// A named execution choice inside a Capability Profile. The IDs are durable transport identity;
/// UI surfaces the device/harness/source labels from the route catalogue instead.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ProfileRoutePolicy {
    pub(crate) route_id: String,
    pub(crate) execution: crate::execution_targets::domain::ExecutionBinding,
    #[serde(default)]
    pub(crate) model_allowances: Vec<ModelAllowance>,
    #[serde(default)]
    pub(crate) mcp_groups: BTreeSet<String>,
    #[serde(default)]
    pub(crate) skill_groups: BTreeSet<String>,
    #[serde(default)]
    pub(crate) defaults: RuntimeSelections,
}

impl ProfileRoutePolicy {
    pub(crate) fn validate(&self) -> Result<(), String> {
        validate_identifier("Capability Profile route", "routeId", &self.route_id)?;
        self.execution.validate()?;
        for allowance in &self.model_allowances {
            allowance.validate()?;
        }
        let mut model_ids = BTreeSet::new();
        for allowance in &self.model_allowances {
            if !model_ids.insert(&allowance.model_id) {
                return Err(format!(
                    "Capability Profile route repeats model `{}`",
                    allowance.model_id
                ));
            }
        }
        for group in self.mcp_groups.iter().chain(self.skill_groups.iter()) {
            validate_identifier("Capability Profile capability group", "id", group)?;
        }
        self.defaults.validate("Capability Profile route defaults")
    }
}

/// The contiguous reasoning range enabled for one model on one execution route.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ModelAllowance {
    pub(crate) model_id: String,
    pub(crate) minimum_reasoning: String,
    pub(crate) maximum_reasoning: String,
}

impl ModelAllowance {
    fn validate(&self) -> Result<(), String> {
        validate_identifier("Capability Profile model", "modelId", &self.model_id)?;
        validate_identifier(
            "Capability Profile model",
            "minimumReasoning",
            &self.minimum_reasoning,
        )?;
        validate_identifier(
            "Capability Profile model",
            "maximumReasoning",
            &self.maximum_reasoning,
        )?;
        const ORDER: &[&str] = &[
            "none", "minimal", "low", "medium", "high", "xhigh", "max", "ultra",
        ];
        if let (Some(minimum), Some(maximum)) = (
            ORDER
                .iter()
                .position(|value| *value == self.minimum_reasoning),
            ORDER
                .iter()
                .position(|value| *value == self.maximum_reasoning),
        ) {
            if minimum > maximum {
                return Err(format!(
                    "Capability Profile model `{}` has a reversed reasoning range",
                    self.model_id
                ));
            }
        }
        Ok(())
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CapabilityProfile {
    #[serde(default)]
    pub(crate) execution: crate::execution_targets::domain::ExecutionBinding,
    #[serde(default)]
    pub(crate) defaults: super::runtime_profile::RuntimeSelections,
    /// The route-policy representation is additive while existing persisted profiles retain the
    /// single execution/flat capability payload they were created with.
    #[serde(default)]
    pub(crate) route_policies: Vec<ProfileRoutePolicy>,
    #[serde(default)]
    pub(crate) default_route_id: Option<String>,
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
        if !self.route_policies.is_empty() {
            let mut route_ids = BTreeSet::new();
            for route in &self.route_policies {
                route.validate()?;
                if !route_ids.insert(&route.route_id) {
                    return Err(format!(
                        "Capability Profile repeats route `{}`",
                        route.route_id
                    ));
                }
            }
            let default_route = self.default_route_id.as_deref().ok_or_else(|| {
                "Capability Profile route policies require a default route".to_string()
            })?;
            if !self
                .route_policies
                .iter()
                .any(|route| route.route_id == default_route)
            {
                return Err("Capability Profile default route is not configured".into());
            }
        } else if self.default_route_id.is_some() {
            return Err("Capability Profile default route requires route policies".into());
        }
        Ok(())
    }

    pub(crate) fn default_route(&self) -> Option<&ProfileRoutePolicy> {
        self.default_route_id.as_deref().and_then(|id| {
            self.route_policies
                .iter()
                .find(|route| route.route_id == id)
        })
    }

    pub(crate) fn route_for_execution(
        &self,
        execution: &crate::execution_targets::domain::ExecutionBinding,
    ) -> Option<&ProfileRoutePolicy> {
        self.route_policies.iter().find(|route| {
            route.execution.device_id == execution.device_id
                && route.execution.provider == execution.provider
                && route.execution.configuration_ref == execution.configuration_ref
                && route.execution.connection == execution.connection
        })
    }

    pub(crate) fn contains_execution(
        &self,
        execution: &crate::execution_targets::domain::ExecutionBinding,
    ) -> bool {
        if self.route_policies.is_empty() {
            self.execution.device_id == execution.device_id
                && self.execution.provider == execution.provider
                && self.execution.configuration_ref == execution.configuration_ref
                && self.execution.connection == execution.connection
        } else {
            self.route_for_execution(execution).is_some()
        }
    }
}
