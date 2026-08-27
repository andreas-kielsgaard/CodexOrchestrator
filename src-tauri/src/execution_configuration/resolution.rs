use super::{
    harness::HarnessDefinition,
    node_profile::{InstructionDelivery, NodeProfileDefinition},
    ports::{SelectedRuntimeProfileSource, SelectedRuntimeProfileSourceError},
    runtime_profile::{
        validate_selection_availability, CapabilitySet, InvocationPhase, RuntimeProfileSnapshot,
        RuntimeSelections,
    },
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub(crate) const RESOLUTION_REQUEST_CONTRACT_VERSION: u32 = 1;
pub(crate) const RESOLVED_CONFIGURATION_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolutionContext {
    pub(crate) phase: InvocationPhase,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolutionRequest {
    pub(crate) contract_version: u32,
    pub(crate) harness: HarnessDefinition,
    pub(crate) node_profile: NodeProfileDefinition,
    pub(crate) context: ResolutionContext,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolvedInstructionDelivery {
    pub(crate) recurring: Option<String>,
    pub(crate) start_only: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolvedExecutionConfigurationContent {
    pub(crate) profile_ref: String,
    pub(crate) harness_id: String,
    pub(crate) harness_revision: u64,
    pub(crate) node_profile_id: String,
    pub(crate) node_profile_revision: u64,
    pub(crate) phase: InvocationPhase,
    pub(crate) capabilities: CapabilitySet,
    pub(crate) selections: RuntimeSelections,
    pub(crate) instructions: ResolvedInstructionDelivery,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ResolvedExecutionConfiguration {
    pub(crate) contract_version: u32,
    pub(crate) content: ResolvedExecutionConfigurationContent,
    pub(crate) digest: String,
}

impl ResolvedExecutionConfiguration {
    pub(crate) fn verify_digest(&self) -> Result<(), ResolutionError> {
        if self.contract_version != RESOLVED_CONFIGURATION_CONTRACT_VERSION {
            return Err(ResolutionError::InvalidInput(format!(
                "Resolved configuration contract version {} is unsupported",
                self.contract_version
            )));
        }
        let expected = content_digest(self.contract_version, &self.content)?;
        if expected == self.digest {
            Ok(())
        } else {
            Err(ResolutionError::DigestMismatch)
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ResolutionError {
    SourceUnavailable(String),
    InvalidInput(String),
    HarnessWidensProfile(String),
    NodeProfileWidensHarness(String),
    LockedCapabilityExcluded(String),
    SelectionUnavailable(String),
    SelectionConflictsWithLocked(String),
    Encoding(String),
    DigestMismatch,
}

impl fmt::Display for ResolutionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::SourceUnavailable(message) => {
                write!(
                    formatter,
                    "Selected runtime profile is unavailable: {message}"
                )
            }
            Self::InvalidInput(message) => formatter.write_str(message),
            Self::HarnessWidensProfile(capability) => {
                write!(formatter, "Harness requests unavailable {capability}")
            }
            Self::NodeProfileWidensHarness(capability) => {
                write!(
                    formatter,
                    "Node Profile requests {capability} outside its Harness"
                )
            }
            Self::LockedCapabilityExcluded(capability) => write!(
                formatter,
                "Resolved capability set excludes profile-locked {capability}"
            ),
            Self::SelectionUnavailable(capability) => {
                write!(formatter, "Node Profile selects unavailable {capability}")
            }
            Self::SelectionConflictsWithLocked(capability) => write!(
                formatter,
                "Node Profile selection conflicts with profile-locked {capability}"
            ),
            Self::Encoding(message) => {
                write!(
                    formatter,
                    "Unable to encode resolved configuration: {message}"
                )
            }
            Self::DigestMismatch => formatter.write_str("Resolved configuration digest mismatch"),
        }
    }
}

impl Error for ResolutionError {}

impl From<SelectedRuntimeProfileSourceError> for ResolutionError {
    fn from(value: SelectedRuntimeProfileSourceError) -> Self {
        Self::SourceUnavailable(value.to_string())
    }
}

pub(crate) struct ExecutionConfigurationResolver;

impl ExecutionConfigurationResolver {
    pub(crate) fn resolve(
        source: &dyn SelectedRuntimeProfileSource,
        request: ResolutionRequest,
    ) -> Result<ResolvedExecutionConfiguration, ResolutionError> {
        if request.contract_version != RESOLUTION_REQUEST_CONTRACT_VERSION {
            return Err(ResolutionError::InvalidInput(format!(
                "Resolution request contract version {} is unsupported",
                request.contract_version
            )));
        }
        request
            .harness
            .validate()
            .map_err(ResolutionError::InvalidInput)?;
        request
            .node_profile
            .validate()
            .map_err(ResolutionError::InvalidInput)?;
        let profile = source.selected_runtime_profile()?;
        profile.validate().map_err(ResolutionError::InvalidInput)?;
        validate_narrowing(&profile, &request.harness, &request.node_profile)?;
        let selections = resolve_selections(
            &profile.locked,
            &request.node_profile.selections,
            &request.node_profile.allowed_capabilities,
        )?;
        let content = ResolvedExecutionConfigurationContent {
            profile_ref: profile.profile_ref,
            harness_id: request.harness.harness_id,
            harness_revision: request.harness.revision,
            node_profile_id: request.node_profile.node_profile_id,
            node_profile_revision: request.node_profile.revision,
            phase: request.context.phase,
            capabilities: request.node_profile.allowed_capabilities,
            selections,
            instructions: resolve_instructions(
                request.context.phase,
                request.node_profile.instructions,
            ),
        };
        Ok(ResolvedExecutionConfiguration {
            contract_version: RESOLVED_CONFIGURATION_CONTRACT_VERSION,
            digest: content_digest(RESOLVED_CONFIGURATION_CONTRACT_VERSION, &content)?,
            content,
        })
    }
}

fn validate_narrowing(
    profile: &RuntimeProfileSnapshot,
    harness: &HarnessDefinition,
    node_profile: &NodeProfileDefinition,
) -> Result<(), ResolutionError> {
    if let Some(capability) = harness
        .allowed_capabilities
        .first_capability_outside(&profile.exposure)
    {
        return Err(ResolutionError::HarnessWidensProfile(capability));
    }
    if let Some(capability) = node_profile
        .allowed_capabilities
        .first_capability_outside(&harness.allowed_capabilities)
    {
        return Err(ResolutionError::NodeProfileWidensHarness(capability));
    }
    validate_selection_availability(&profile.locked, &node_profile.allowed_capabilities)
        .map_err(ResolutionError::LockedCapabilityExcluded)
}

fn resolve_selections(
    locked: &RuntimeSelections,
    requested: &RuntimeSelections,
    available: &CapabilitySet,
) -> Result<RuntimeSelections, ResolutionError> {
    validate_selection_availability(requested, available)
        .map_err(ResolutionError::SelectionUnavailable)?;
    Ok(RuntimeSelections {
        model: select_locked("model", &locked.model, &requested.model)?,
        reasoning_mode: select_locked(
            "reasoning mode",
            &locked.reasoning_mode,
            &requested.reasoning_mode,
        )?,
        sandbox_mode: select_locked(
            "sandbox mode",
            &locked.sandbox_mode,
            &requested.sandbox_mode,
        )?,
    })
}

fn select_locked<T: Clone + Eq + fmt::Debug>(
    label: &str,
    locked: &Option<T>,
    requested: &Option<T>,
) -> Result<Option<T>, ResolutionError> {
    match (locked, requested) {
        (Some(locked), Some(requested)) if locked != requested => {
            Err(ResolutionError::SelectionConflictsWithLocked(format!(
                "{label} `{requested:?}`; required `{locked:?}`"
            )))
        }
        (Some(locked), _) => Ok(Some(locked.clone())),
        (None, requested) => Ok(requested.clone()),
    }
}

fn resolve_instructions(
    phase: InvocationPhase,
    instructions: InstructionDelivery,
) -> ResolvedInstructionDelivery {
    ResolvedInstructionDelivery {
        recurring: instructions.recurring,
        start_only: match phase {
            InvocationPhase::Start => instructions.start_only,
            InvocationPhase::Resume => None,
        },
    }
}

fn content_digest(
    contract_version: u32,
    content: &ResolvedExecutionConfigurationContent,
) -> Result<String, ResolutionError> {
    let bytes = serde_json::to_vec(content)
        .map_err(|error| ResolutionError::Encoding(error.to_string()))?;
    let mut digest = Sha256::new();
    digest.update(b"execution-configuration/resolved\0");
    digest.update(contract_version.to_be_bytes());
    digest.update(bytes);
    Ok(format!("{:x}", digest.finalize()))
}
