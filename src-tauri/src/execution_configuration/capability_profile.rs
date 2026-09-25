use super::runtime_profile::{
    validate_identifier, validate_provider_options, CapabilitySet, RuntimeSelections,
};
use orchid_engine::contracts::ProviderNativeOptions;
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

pub(crate) const CAPABILITY_PROFILE_CONTRACT_VERSION: u32 = 1;

/// Capability group for MCP servers configured natively by the selected provider configuration.
pub(crate) const NATIVE_MCP_GROUP: &str = "native-mcps";
/// Capability group for skills discovered natively by the selected provider configuration.
pub(crate) const NATIVE_SKILL_GROUP: &str = "native-skills";
/// Capability group for Orchid's product-owned skill root.
pub(crate) const ORCHID_SKILL_GROUP: &str = "orchid-skills";

/// Capability group for the skills shipped by one installed OTP package.
pub(crate) fn otp_skill_group(package: &str) -> String {
    format!("otp:{package}:skills")
}

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
    /// Provider-native override, decoded only by the route's provider. Absence inherits the
    /// provider configuration's own default.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub(crate) provider_options: Option<ProviderNativeOptions>,
}

impl ProfileRoutePolicy {
    pub(crate) fn validate(&self) -> Result<(), String> {
        validate_identifier("Capability Profile route", "routeId", &self.route_id)?;
        self.execution.validate()?;
        validate_provider_options(
            &self.execution.configuration(),
            self.provider_options.as_ref(),
        )?;
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
            // The provider for a message follows from its device and model, so each device has
            // at most one route per provider and a model belongs to one route on a device.
            for (index, route) in self.route_policies.iter().enumerate() {
                for other in &self.route_policies[..index] {
                    if other.execution.device_id != route.execution.device_id {
                        continue;
                    }
                    if other.execution.provider == route.execution.provider {
                        return Err(format!(
                            "Capability Profile has two `{}` routes on device `{}`",
                            route.execution.provider, route.execution.device_name
                        ));
                    }
                    if let Some(model) = route.model_allowances.iter().find(|allowance| {
                        other
                            .model_allowances
                            .iter()
                            .any(|existing| existing.model_id == allowance.model_id)
                    }) {
                        return Err(format!(
                            "Capability Profile offers model `{}` on two routes of device `{}`",
                            model.model_id, route.execution.device_name
                        ));
                    }
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
        self.route_policies
            .iter()
            .find(|route| route.execution.route_ref() == execution.route_ref())
    }

    /// The route on a device that offers a model.
    pub(crate) fn route_for_model(
        &self,
        device_id: &str,
        model: &str,
    ) -> Option<&ProfileRoutePolicy> {
        self.route_policies.iter().find(|route| {
            route.execution.device_id == device_id
                && route
                    .model_allowances
                    .iter()
                    .any(|allowance| allowance.model_id == model)
        })
    }

    /// The route a resolved runtime configuration belongs to: the default route when it matches,
    /// otherwise the first route with that configuration.
    pub(crate) fn route_for_configuration(
        &self,
        configuration: &orchid_engine::contracts::ProviderConfigurationRef,
    ) -> Option<&ProfileRoutePolicy> {
        let matches = |route: &&ProfileRoutePolicy| &route.execution.configuration() == configuration;
        self.default_route()
            .filter(matches)
            .or_else(|| self.route_policies.iter().find(matches))
            .or_else(|| self.default_route())
    }

    pub(crate) fn contains_execution(
        &self,
        execution: &crate::execution_targets::domain::ExecutionBinding,
    ) -> bool {
        if self.route_policies.is_empty() {
            self.execution.route_ref() == execution.route_ref()
        } else {
            self.route_for_execution(execution).is_some()
        }
    }
}

#[cfg(test)]
mod route_tests {
    use super::*;
    use crate::execution_targets::domain::{ExecutionBinding, ExecutionConnection};

    fn route(id: &str, device: &str, provider: &str, models: &[&str]) -> ProfileRoutePolicy {
        ProfileRoutePolicy {
            route_id: id.into(),
            execution: ExecutionBinding {
                device_id: device.into(),
                device_name: device.into(),
                provider: provider.into(),
                configuration_ref: format!("{provider}-setup"),
                connection: ExecutionConnection::Local,
            },
            model_allowances: models
                .iter()
                .map(|model| ModelAllowance {
                    model_id: (*model).into(),
                    minimum_reasoning: "low".into(),
                    maximum_reasoning: "high".into(),
                })
                .collect(),
            mcp_groups: Default::default(),
            skill_groups: Default::default(),
            defaults: Default::default(),
            provider_options: None,
        }
    }

    fn profile(routes: Vec<ProfileRoutePolicy>) -> CapabilityProfile {
        CapabilityProfile {
            execution: routes[0].execution.clone(),
            defaults: Default::default(),
            default_route_id: Some(routes[0].route_id.clone()),
            route_policies: routes,
            contract_version: CAPABILITY_PROFILE_CONTRACT_VERSION,
            capability_profile_id: "profile".into(),
            name: "Profile".into(),
            revision: 1,
            allowed_capabilities: Default::default(),
        }
    }

    #[test]
    fn a_device_routes_each_model_to_its_provider() {
        let profile = profile(vec![
            route("codex", "laptop", "codex", &["gpt-5"]),
            route("claude", "laptop", "claude", &["opus"]),
        ]);
        profile.validate().unwrap();
        assert_eq!(profile.route_for_model("laptop", "opus").unwrap().route_id, "claude");
        assert_eq!(profile.route_for_model("laptop", "gpt-5").unwrap().route_id, "codex");
        assert!(profile.route_for_model("server", "opus").is_none());
    }

    #[test]
    fn a_device_has_one_route_per_provider_and_one_route_per_model() {
        let two_codex = profile(vec![
            route("one", "laptop", "codex", &["gpt-5"]),
            route("two", "laptop", "codex", &["gpt-5-mini"]),
        ]);
        assert!(two_codex.validate().unwrap_err().contains("two `codex` routes"));
        let shared_model = profile(vec![
            route("codex", "laptop", "codex", &["shared"]),
            route("claude", "laptop", "claude", &["shared"]),
        ]);
        assert!(shared_model.validate().unwrap_err().contains("model `shared`"));
        let other_device = profile(vec![
            route("laptop", "laptop", "codex", &["gpt-5"]),
            route("server", "server", "codex", &["gpt-5"]),
        ]);
        other_device.validate().unwrap();
    }
}
