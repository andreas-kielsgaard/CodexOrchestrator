//! Durable tool selection, independent of process-scoped MCP connections.

use super::domain::{
    HarnessMcpExposurePlan, HarnessMediationPlan, HarnessToolAccess, ManagedMcpUpstreamDescriptor,
    MEDIATION_PLAN_VERSION,
};
use serde::{Deserialize, Serialize};

pub(crate) const EXPOSURE_POLICY_VERSION: &str = "harness-exposure/v2";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessExposure {
    pub(crate) configured_server_name: String,
    pub(crate) proxy_server_name: String,
    pub(crate) access: HarnessToolAccess,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct HarnessExposurePolicy {
    pub(crate) contract_version: String,
    pub(crate) exposures: Vec<HarnessExposure>,
}

impl HarnessExposurePolicy {
    pub(crate) fn from_resolved(plan: &HarnessMediationPlan) -> Self {
        Self {
            contract_version: EXPOSURE_POLICY_VERSION.into(),
            exposures: plan
                .exposures
                .iter()
                .map(|entry| HarnessExposure {
                    configured_server_name: entry.configured_server_name.clone(),
                    proxy_server_name: entry.proxy_server_name.clone(),
                    access: entry.access.clone(),
                })
                .collect(),
        }
    }

    pub(crate) fn resolve(
        &self,
        resolve: impl Fn(&str) -> Result<ManagedMcpUpstreamDescriptor, String>,
    ) -> Result<HarnessMediationPlan, String> {
        if self.contract_version != EXPOSURE_POLICY_VERSION {
            return Err(format!(
                "Unsupported Harness exposure policy {}",
                self.contract_version
            ));
        }
        Ok(HarnessMediationPlan {
            contract_version: MEDIATION_PLAN_VERSION.into(),
            exposures: self
                .exposures
                .iter()
                .map(|entry| {
                    Ok(HarnessMcpExposurePlan {
                        configured_server_name: entry.configured_server_name.clone(),
                        proxy_server_name: entry.proxy_server_name.clone(),
                        access: entry.access.clone(),
                        upstream: resolve(&entry.configured_server_name)?,
                    })
                })
                .collect::<Result<_, String>>()?,
        })
    }
}
