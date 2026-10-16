use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub const BINDING_CONTRACT_VERSION: &str = "harness-binding/v1";
pub const MEDIATION_PLAN_VERSION: &str = "harness-mediation-plan/v1";
pub const CONTROL_PROTOCOL_VERSION: &str = "harness-control/v1";

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum HarnessDomainError {
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
pub struct HarnessId(String);

impl HarnessId {
    pub fn new(value: impl Into<String>) -> Result<Self, HarnessDomainError> {
        let value = value.into();
        if value.trim().is_empty() {
            return Err(HarnessDomainError::EmptyHarnessId);
        }
        Ok(Self(value))
    }

    pub fn as_str(&self) -> &str {
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
pub struct HarnessVersionNumber(u64);

impl HarnessVersionNumber {
    pub fn new(value: u64) -> Result<Self, HarnessDomainError> {
        if value == 0 {
            return Err(HarnessDomainError::InvalidVersionNumber);
        }
        Ok(Self(value))
    }

    pub fn get(self) -> u64 {
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
pub struct HarnessVersionRef {
    harness_id: HarnessId,
    version: HarnessVersionNumber,
}

impl HarnessVersionRef {
    pub fn new(harness_id: HarnessId, version: HarnessVersionNumber) -> Self {
        Self {
            harness_id,
            version,
        }
    }

    pub fn harness_id(&self) -> &HarnessId {
        &self.harness_id
    }

    pub fn version(&self) -> HarnessVersionNumber {
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
pub enum HarnessVersionScope {
    Reusable,
    SessionSpecific { session_id: String },
}

impl HarnessVersionScope {
    pub fn session_specific(session_id: impl Into<String>) -> Result<Self, HarnessDomainError> {
        let session_id = session_id.into();
        if session_id.trim().is_empty() {
            return Err(HarnessDomainError::EmptySessionId);
        }
        Ok(Self::SessionSpecific { session_id })
    }

    pub fn validate(&self) -> Result<(), HarnessDomainError> {
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
pub struct HarnessVersionReplacement {
    source: HarnessVersionRef,
    target: HarnessVersionRef,
}

impl HarnessVersionReplacement {
    pub fn new(
        source: HarnessVersionRef,
        target: HarnessVersionRef,
    ) -> Result<Self, HarnessDomainError> {
        let replacement = Self { source, target };
        replacement.validate()?;
        Ok(replacement)
    }

    pub fn source(&self) -> &HarnessVersionRef {
        &self.source
    }

    pub fn target(&self) -> &HarnessVersionRef {
        &self.target
    }

    pub fn validate(&self) -> Result<(), HarnessDomainError> {
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
pub enum HarnessMigrationOutcome {
    Current,
    ReplacementApplied { path: Vec<HarnessVersionRef> },
}

impl HarnessMigrationOutcome {
    pub fn was_replaced(&self) -> bool {
        matches!(self, Self::ReplacementApplied { .. })
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessResolution {
    requested: HarnessVersionRef,
    resolved: HarnessVersionRef,
    outcome: HarnessMigrationOutcome,
}

impl HarnessResolution {
    pub fn current(reference: HarnessVersionRef) -> Self {
        Self {
            requested: reference.clone(),
            resolved: reference,
            outcome: HarnessMigrationOutcome::Current,
        }
    }

    pub fn replaced(
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

    pub fn requested(&self) -> &HarnessVersionRef {
        &self.requested
    }

    pub fn resolved(&self) -> &HarnessVersionRef {
        &self.resolved
    }

    pub fn outcome(&self) -> &HarnessMigrationOutcome {
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
pub struct ManagedMcpUpstreamDescriptor {
    pub name: String,
    pub url: String,
    pub bearer_token: String,
    #[serde(default)]
    pub workflow_tool_name: Option<String>,
    #[serde(default)]
    pub workflow_prepare_url: Option<String>,
    /// The Harness injects its trusted Session and invocation identity for this product-owned
    /// endpoint. The caller may never provide those values.
    #[serde(default)]
    pub caller_context: bool,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub enum HarnessToolAccess {
    EntireServer,
    SelectedTools { tool_names: Vec<String> },
}

impl HarnessToolAccess {
    pub fn allows(&self, tool_name: &str) -> bool {
        match self {
            Self::EntireServer => true,
            Self::SelectedTools { tool_names } => {
                tool_names.iter().any(|candidate| candidate == tool_name)
            }
        }
    }

    pub fn selected_tools(&self) -> Option<&[String]> {
        match self {
            Self::EntireServer => None,
            Self::SelectedTools { tool_names } => Some(tool_names),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessMcpExposurePlan {
    pub configured_server_name: String,
    pub proxy_server_name: String,
    pub upstream: ManagedMcpUpstreamDescriptor,
    pub access: HarnessToolAccess,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HarnessMediationPlan {
    pub contract_version: String,
    pub exposures: Vec<HarnessMcpExposurePlan>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum HarnessBindingStage {
    Prepared,
    Bound,
    Retired,
}

impl HarnessBindingStage {
    pub fn parse(value: &str) -> Result<Self, String> {
        match value {
            "prepared" => Ok(Self::Prepared),
            "bound" => Ok(Self::Bound),
            "retired" => Ok(Self::Retired),
            _ => Err(format!("Unknown Harness binding stage {value}.")),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct HarnessBindingRecord {
    pub id: String,
    pub session_id: String,
    pub runtime_instance_id: String,
    pub session_instance_token: String,
    pub stage: HarnessBindingStage,
    pub harness_snapshot: String,
    pub mediation_plan: String,
    pub configuration_digest: String,
    pub harness_token: Option<String>,
    pub source_workflow_instance_id: String,
    pub source_recipe_id: String,
    pub source_node_id: String,
    pub prepared_at: String,
    pub bound_at: Option<String>,
    pub retired_at: Option<String>,
}

impl HarnessBindingRecord {
    pub fn verify_digest(&self) -> Result<(), String> {
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

    pub fn parsed_plan(&self) -> Result<super::exposure::HarnessExposurePolicy, String> {
        self.verify_digest()?;
        serde_json::from_str(&self.mediation_plan)
            .map_err(|error| format!("Harness binding mediation plan is invalid: {error}"))
    }
}

pub fn binding_digest(harness_snapshot: &str, mediation_plan: &str) -> String {
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
pub struct SidecarBindingRegistration {
    pub binding_id: String,
    pub session_id: String,
    pub runtime_instance_id: String,
    pub session_instance_token: String,
    pub harness_snapshot: String,
    pub mediation_plan: String,
    pub configuration_digest: String,
    pub source_workflow_instance_id: String,
    pub source_node_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub harness_token: Option<String>,
}

impl SidecarBindingRegistration {
    pub fn from_record(
        record: &HarnessBindingRecord,
        resolved: &HarnessMediationPlan,
    ) -> Result<Self, String> {
        record.verify_digest()?;
        if record.parsed_plan()? != super::exposure::HarnessExposurePolicy::from_resolved(resolved)
        {
            return Err("Resolved Harness connection changed the recorded tool policy".into());
        }
        let mediation_plan = serde_json::to_string(resolved).map_err(|e| e.to_string())?;
        Ok(Self {
            binding_id: record.id.clone(),
            session_id: record.session_id.clone(),
            runtime_instance_id: record.runtime_instance_id.clone(),
            session_instance_token: record.session_instance_token.clone(),
            harness_snapshot: record.harness_snapshot.clone(),
            configuration_digest: binding_digest(&record.harness_snapshot, &mediation_plan),
            mediation_plan,
            source_workflow_instance_id: record.source_workflow_instance_id.clone(),
            source_node_id: record.source_node_id.clone(),
            harness_token: record.harness_token.clone(),
        })
    }

    pub fn verify_digest(&self) -> Result<(), String> {
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
