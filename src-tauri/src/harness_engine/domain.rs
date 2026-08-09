use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};

pub(crate) const BINDING_CONTRACT_VERSION: &str = "harness-binding/v1";
pub(crate) const MEDIATION_PLAN_VERSION: &str = "harness-mediation-plan/v1";
pub(crate) const CONTROL_PROTOCOL_VERSION: &str = "harness-control/v1";

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManagedMcpUpstreamDescriptor {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) bearer_token: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum HarnessToolAccess {
    EntireServer,
    SelectedTools { tool_names: Vec<String> },
}

impl HarnessToolAccess {
    pub(crate) fn allows(&self, tool_name: &str) -> bool {
        match self {
            Self::EntireServer => true,
            Self::SelectedTools { tool_names } => {
                tool_names.iter().any(|candidate| candidate == tool_name)
            }
        }
    }

    pub(crate) fn selected_tools(&self) -> Option<&[String]> {
        match self {
            Self::EntireServer => None,
            Self::SelectedTools { tool_names } => Some(tool_names),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessMcpExposurePlan {
    pub(crate) configured_server_name: String,
    pub(crate) proxy_server_name: String,
    pub(crate) upstream: ManagedMcpUpstreamDescriptor,
    pub(crate) access: HarnessToolAccess,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessMediationPlan {
    pub(crate) contract_version: String,
    pub(crate) exposures: Vec<HarnessMcpExposurePlan>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum HarnessBindingStage {
    Prepared,
    Bound,
    Retired,
}

impl HarnessBindingStage {
    pub(crate) fn parse(value: &str) -> Result<Self, String> {
        match value {
            "prepared" => Ok(Self::Prepared),
            "bound" => Ok(Self::Bound),
            "retired" => Ok(Self::Retired),
            _ => Err(format!("Unknown Harness binding stage {value}.")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct HarnessBindingRecord {
    pub(crate) id: String,
    pub(crate) session_id: String,
    pub(crate) runtime_instance_id: String,
    pub(crate) session_instance_token: String,
    pub(crate) stage: HarnessBindingStage,
    pub(crate) harness_snapshot: String,
    pub(crate) mediation_plan: String,
    pub(crate) configuration_digest: String,
    pub(crate) harness_token: Option<String>,
    pub(crate) source_workflow_instance_id: String,
    pub(crate) source_recipe_id: String,
    pub(crate) source_node_id: String,
    pub(crate) prepared_at: String,
    pub(crate) bound_at: Option<String>,
    pub(crate) retired_at: Option<String>,
}

impl HarnessBindingRecord {
    pub(crate) fn verify_digest(&self) -> Result<(), String> {
        let expected = binding_digest(&self.harness_snapshot, &self.mediation_plan);
        if expected == self.configuration_digest {
            Ok(())
        } else {
            Err(format!(
                "Harness binding {} failed immutable digest verification.",
                self.id
            ))
        }
    }

    pub(crate) fn parsed_plan(&self) -> Result<HarnessMediationPlan, String> {
        self.verify_digest()?;
        serde_json::from_str(&self.mediation_plan)
            .map_err(|error| format!("Harness binding mediation plan is invalid: {error}"))
    }
}

pub(crate) fn binding_digest(harness_snapshot: &str, mediation_plan: &str) -> String {
    let mut digest = Sha256::new();
    digest.update(BINDING_CONTRACT_VERSION.as_bytes());
    digest.update([0]);
    digest.update(harness_snapshot.as_bytes());
    digest.update([0]);
    digest.update(mediation_plan.as_bytes());
    format!("sha256:{:x}", digest.finalize())
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SidecarBindingRegistration {
    pub(crate) binding_id: String,
    pub(crate) session_id: String,
    pub(crate) runtime_instance_id: String,
    pub(crate) session_instance_token: String,
    pub(crate) harness_snapshot: String,
    pub(crate) mediation_plan: String,
    pub(crate) configuration_digest: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) harness_token: Option<String>,
}

impl SidecarBindingRegistration {
    pub(crate) fn from_record(record: &HarnessBindingRecord) -> Result<Self, String> {
        record.verify_digest()?;
        Ok(Self {
            binding_id: record.id.clone(),
            session_id: record.session_id.clone(),
            runtime_instance_id: record.runtime_instance_id.clone(),
            session_instance_token: record.session_instance_token.clone(),
            harness_snapshot: record.harness_snapshot.clone(),
            mediation_plan: record.mediation_plan.clone(),
            configuration_digest: record.configuration_digest.clone(),
            harness_token: record.harness_token.clone(),
        })
    }

    pub(crate) fn verify_digest(&self) -> Result<(), String> {
        let expected = binding_digest(&self.harness_snapshot, &self.mediation_plan);
        if expected == self.configuration_digest {
            Ok(())
        } else {
            Err(format!(
                "Harness sidecar registration {} failed digest verification.",
                self.binding_id
            ))
        }
    }
}
