use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub(crate) const BINDING_CONTRACT_VERSION: &str = "harness-binding/v1";
pub(crate) const MEDIATION_PLAN_VERSION: &str = "harness-mediation-plan/v1";
pub(crate) const CONTROL_PROTOCOL_VERSION: &str = "harness-control/v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum HarnessDomainError {
    EmptyHarnessId,
    InvalidVersionNumber,
    EmptySessionId,
    CrossHarnessReplacement,
    SameVersionReplacement,
}

impl fmt::Display for HarnessDomainError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::EmptyHarnessId => "harness ID must be a non-empty opaque identifier",
            Self::InvalidVersionNumber => "Harness version number must be greater than zero",
            Self::EmptySessionId => "session-scoped Harness version requires a session ID",
            Self::CrossHarnessReplacement => {
                "Harness version replacement must remain within one Harness"
            }
            Self::SameVersionReplacement => {
                "Harness version replacement must target a different version"
            }
        })
    }
}

impl Error for HarnessDomainError {}

#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct HarnessId(String);

impl HarnessId {
    pub(crate) fn new(value: impl Into<String>) -> Result<Self, HarnessDomainError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(HarnessDomainError::EmptyHarnessId);
        }
        Ok(Self(value))
    }

    pub(crate) fn as_str(&self) -> &str {
        &self.0
    }
}

impl fmt::Display for HarnessId {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(self.as_str())
    }
}

impl<'de> Deserialize<'de> for HarnessId {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Copy, Debug, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(transparent)]
pub(crate) struct HarnessVersionNumber(u64);

impl HarnessVersionNumber {
    pub(crate) fn new(value: u64) -> Result<Self, HarnessDomainError> {
        if value == 0 {
            return Err(HarnessDomainError::InvalidVersionNumber);
        }
        Ok(Self(value))
    }

    pub(crate) fn get(self) -> u64 {
        self.0
    }
}

impl fmt::Display for HarnessVersionNumber {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}", self.get())
    }
}

impl<'de> Deserialize<'de> for HarnessVersionNumber {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = u64::deserialize(deserializer)?;
        Self::new(value).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Hash, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessVersionRef {
    harness_id: HarnessId,
    version: HarnessVersionNumber,
}

impl HarnessVersionRef {
    pub(crate) fn new(harness_id: HarnessId, version: HarnessVersionNumber) -> Self {
        Self {
            harness_id,
            version,
        }
    }

    pub(crate) fn harness_id(&self) -> &HarnessId {
        &self.harness_id
    }

    pub(crate) fn version(&self) -> HarnessVersionNumber {
        self.version
    }
}

impl fmt::Display for HarnessVersionRef {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}@v{}", self.harness_id, self.version)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum HarnessVersionScope {
    Reusable,
    SessionSpecific { session_id: String },
}

impl HarnessVersionScope {
    pub(crate) fn session_specific(
        session_id: impl Into<String>,
    ) -> Result<Self, HarnessDomainError> {
        let session_id = session_id.into();
        if session_id.trim().is_empty() {
            return Err(HarnessDomainError::EmptySessionId);
        }
        Ok(Self::SessionSpecific { session_id })
    }

    pub(crate) fn validate(&self) -> Result<(), HarnessDomainError> {
        match self {
            Self::Reusable => Ok(()),
            Self::SessionSpecific { session_id } if session_id.trim().is_empty() => {
                Err(HarnessDomainError::EmptySessionId)
            }
            Self::SessionSpecific { .. } => Ok(()),
        }
    }
}

impl<'de> Deserialize<'de> for HarnessVersionScope {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(
            tag = "kind",
            rename_all = "snake_case",
            rename_all_fields = "camelCase"
        )]
        enum SerializedScope {
            Reusable,
            SessionSpecific { session_id: String },
        }

        let scope = match SerializedScope::deserialize(deserializer)? {
            SerializedScope::Reusable => Self::Reusable,
            SerializedScope::SessionSpecific { session_id } => Self::SessionSpecific { session_id },
        };
        scope.validate().map_err(serde::de::Error::custom)?;
        Ok(scope)
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessVersionReplacement {
    source: HarnessVersionRef,
    target: HarnessVersionRef,
}

impl HarnessVersionReplacement {
    pub(crate) fn new(
        source: HarnessVersionRef,
        target: HarnessVersionRef,
    ) -> Result<Self, HarnessDomainError> {
        let replacement = Self { source, target };
        replacement.validate()?;
        Ok(replacement)
    }

    pub(crate) fn source(&self) -> &HarnessVersionRef {
        &self.source
    }

    pub(crate) fn target(&self) -> &HarnessVersionRef {
        &self.target
    }

    pub(crate) fn validate(&self) -> Result<(), HarnessDomainError> {
        if self.source.harness_id() != self.target.harness_id() {
            return Err(HarnessDomainError::CrossHarnessReplacement);
        }
        if self.source.version() == self.target.version() {
            return Err(HarnessDomainError::SameVersionReplacement);
        }
        Ok(())
    }
}

impl<'de> Deserialize<'de> for HarnessVersionReplacement {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        #[derive(Deserialize)]
        #[serde(rename_all = "camelCase")]
        struct SerializedReplacement {
            source: HarnessVersionRef,
            target: HarnessVersionRef,
        }

        let serialized = SerializedReplacement::deserialize(deserializer)?;
        Self::new(serialized.source, serialized.target).map_err(serde::de::Error::custom)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum HarnessMigrationOutcome {
    Current,
    ReplacementApplied { path: Vec<HarnessVersionRef> },
}

impl HarnessMigrationOutcome {
    pub(crate) fn was_replaced(&self) -> bool {
        matches!(self, Self::ReplacementApplied { .. })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct HarnessResolution {
    requested: HarnessVersionRef,
    resolved: HarnessVersionRef,
    outcome: HarnessMigrationOutcome,
}

impl HarnessResolution {
    pub(crate) fn current(reference: HarnessVersionRef) -> Self {
        Self {
            requested: reference.clone(),
            resolved: reference,
            outcome: HarnessMigrationOutcome::Current,
        }
    }

    pub(crate) fn replaced(
        requested: HarnessVersionRef,
        resolved: HarnessVersionRef,
        path: Vec<HarnessVersionRef>,
    ) -> Self {
        Self {
            requested,
            resolved,
            outcome: HarnessMigrationOutcome::ReplacementApplied { path },
        }
    }

    pub(crate) fn requested(&self) -> &HarnessVersionRef {
        &self.requested
    }

    pub(crate) fn resolved(&self) -> &HarnessVersionRef {
        &self.resolved
    }

    pub(crate) fn outcome(&self) -> &HarnessMigrationOutcome {
        &self.outcome
    }
}

#[cfg(test)]
mod harness_version_tests {
    use super::*;

    fn reference(harness_id: &str, version: u64) -> HarnessVersionRef {
        HarnessVersionRef::new(
            HarnessId::new(harness_id).unwrap(),
            HarnessVersionNumber::new(version).unwrap(),
        )
    }

    #[test]
    fn opaque_id_and_version_reject_empty_product_identity() {
        assert_eq!(
            HarnessId::new("  ").unwrap_err(),
            HarnessDomainError::EmptyHarnessId
        );
        assert_eq!(
            HarnessVersionNumber::new(0).unwrap_err(),
            HarnessDomainError::InvalidVersionNumber
        );
    }

    #[test]
    fn session_scope_requires_an_opaque_session_id() {
        assert_eq!(
            HarnessVersionScope::session_specific("\t").unwrap_err(),
            HarnessDomainError::EmptySessionId
        );
        assert_eq!(
            HarnessVersionScope::session_specific("session-1").unwrap(),
            HarnessVersionScope::SessionSpecific {
                session_id: "session-1".into(),
            }
        );
    }

    #[test]
    fn replacement_must_remain_within_one_harness() {
        let error = HarnessVersionReplacement::new(
            reference("epic-plan-builder", 1),
            reference("work-unit-implementer", 2),
        )
        .unwrap_err();

        assert_eq!(error, HarnessDomainError::CrossHarnessReplacement);
    }

    #[test]
    fn replacement_must_advance_to_a_different_version() {
        let error = HarnessVersionReplacement::new(
            reference("epic-plan-builder", 1),
            reference("epic-plan-builder", 1),
        )
        .unwrap_err();

        assert_eq!(error, HarnessDomainError::SameVersionReplacement);
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ManagedMcpUpstreamDescriptor {
    pub(crate) name: String,
    pub(crate) url: String,
    pub(crate) bearer_token: String,
    #[serde(default)]
    pub(crate) caller_context: bool,
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
    pub(crate) source_workflow_instance_id: String,
    pub(crate) source_node_id: String,
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
            source_workflow_instance_id: record.source_workflow_instance_id.clone(),
            source_node_id: record.source_node_id.clone(),
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
