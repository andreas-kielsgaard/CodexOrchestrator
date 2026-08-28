use super::{
    capability_profile::CapabilityProfile,
    node_profile::NodeProfile,
    ports::{SelectedRuntimeProfileSource, SelectedRuntimeProfileSourceError},
    runtime_profile::{
        validate_identifier, validate_selection_availability, CapabilitySet,
        RuntimeProfileSnapshot, RuntimeSelections,
    },
    session_profile::SessionProfile,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{error::Error, fmt};

pub(crate) const SESSION_CREATION_REQUEST_CONTRACT_VERSION: u32 = 1;
pub(crate) const SESSION_CREATION_RESOLUTION_CONTRACT_VERSION: u32 = 1;
pub(crate) const DIRECT_USER_INVOCATION_REQUEST_CONTRACT_VERSION: u32 = 1;
pub(crate) const DIRECT_USER_INVOCATION_RESOLUTION_CONTRACT_VERSION: u32 = 1;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionCreationRequest {
    pub(crate) contract_version: u32,
    pub(crate) capability_profile: CapabilityProfile,
    pub(crate) node_profile: NodeProfile,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SessionCreationResolution {
    contract_version: u32,
    session_profile: SessionProfile,
    digest: String,
}

impl SessionCreationResolution {
    pub(crate) fn session_profile(&self) -> &SessionProfile {
        &self.session_profile
    }

    pub(crate) fn digest(&self) -> &str {
        &self.digest
    }

    pub(crate) fn verify_digest(&self) -> Result<(), ResolutionError> {
        if self.contract_version != SESSION_CREATION_RESOLUTION_CONTRACT_VERSION {
            return Err(ResolutionError::InvalidInput(format!(
                "Session creation resolution contract version {} is unsupported",
                self.contract_version
            )));
        }
        self.session_profile
            .validate()
            .map_err(ResolutionError::InvalidInput)?;
        let expected = session_profile_digest(self.contract_version, &self.session_profile)?;
        if expected == self.digest {
            Ok(())
        } else {
            Err(ResolutionError::DigestMismatch)
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DirectUserInvocationRequest {
    pub(crate) contract_version: u32,
    pub(crate) model: Option<String>,
    pub(crate) reasoning_mode: Option<String>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct DirectUserInvocationResolution {
    pub(crate) contract_version: u32,
    pub(crate) session_profile_digest: String,
    pub(crate) selections: RuntimeSelections,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum ResolutionError {
    SourceUnavailable(String),
    InvalidInput(String),
    CapabilityProfileWidensRuntime(String),
    NodeProfileWidensCapabilityProfile(String),
    LockedCapabilityExcluded(String),
    PinnedSelectionUnavailable(String),
    SelectionConflictsWithLocked(String),
    DirectUserSelectionUnavailable(String),
    RuntimeProfileChanged { expected: String, actual: String },
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
            Self::CapabilityProfileWidensRuntime(capability) => write!(
                formatter,
                "Capability Profile requests unavailable {capability}"
            ),
            Self::NodeProfileWidensCapabilityProfile(capability) => write!(
                formatter,
                "Node Profile requests {capability} outside its Capability Profile"
            ),
            Self::LockedCapabilityExcluded(capability) => write!(
                formatter,
                "Session Profile excludes runtime-locked {capability}"
            ),
            Self::PinnedSelectionUnavailable(capability) => {
                write!(formatter, "Node Profile pins unavailable {capability}")
            }
            Self::SelectionConflictsWithLocked(capability) => write!(
                formatter,
                "Pinned selection conflicts with runtime-locked {capability}"
            ),
            Self::DirectUserSelectionUnavailable(capability) => write!(
                formatter,
                "Direct-user invocation selects unavailable {capability}"
            ),
            Self::RuntimeProfileChanged { expected, actual } => write!(
                formatter,
                "Session Profile expects runtime profile `{expected}`, but `{actual}` is selected"
            ),
            Self::Encoding(message) => {
                write!(formatter, "Unable to encode Session Profile: {message}")
            }
            Self::DigestMismatch => formatter.write_str("Session Profile digest mismatch"),
        }
    }
}

impl Error for ResolutionError {}

impl From<SelectedRuntimeProfileSourceError> for ResolutionError {
    fn from(value: SelectedRuntimeProfileSourceError) -> Self {
        Self::SourceUnavailable(value.to_string())
    }
}

pub(crate) struct SessionProfileResolver;

impl SessionProfileResolver {
    pub(crate) fn resolve_creation(
        source: &dyn SelectedRuntimeProfileSource,
        request: SessionCreationRequest,
    ) -> Result<SessionCreationResolution, ResolutionError> {
        validate_creation_request(&request)?;
        let runtime_profile = source.selected_runtime_profile()?;
        runtime_profile
            .validate()
            .map_err(ResolutionError::InvalidInput)?;
        validate_narrowing(
            &runtime_profile,
            &request.capability_profile,
            &request.node_profile,
        )?;
        let pinned_defaults = resolve_pinned_defaults(
            &runtime_profile.locked,
            &request.node_profile.pinned_defaults,
            &request.node_profile.allowed_capabilities,
        )?;
        let session_profile = SessionProfile::resolved(
            runtime_profile.profile_ref,
            runtime_profile.exposure,
            runtime_profile.locked,
            request.capability_profile.capability_profile_id,
            request.capability_profile.revision,
            request.node_profile.allowed_capabilities,
            pinned_defaults,
        );
        let contract_version = SESSION_CREATION_RESOLUTION_CONTRACT_VERSION;
        let digest = session_profile_digest(contract_version, &session_profile)?;
        Ok(SessionCreationResolution {
            contract_version,
            session_profile,
            digest,
        })
    }

    pub(crate) fn validate_direct_user_invocation(
        source: &dyn SelectedRuntimeProfileSource,
        creation: &SessionCreationResolution,
        request: DirectUserInvocationRequest,
    ) -> Result<DirectUserInvocationResolution, ResolutionError> {
        validate_direct_user_request(&request)?;
        Self::validate_pinned_session(source, creation)?;
        let session_profile = creation.session_profile();
        let requested = RuntimeSelections {
            model: request
                .model
                .or_else(|| session_profile.pinned_defaults().model.clone()),
            reasoning_mode: request
                .reasoning_mode
                .or_else(|| session_profile.pinned_defaults().reasoning_mode.clone()),
            sandbox_mode: session_profile.pinned_defaults().sandbox_mode,
        };
        validate_selection_availability(
            &requested,
            session_profile.attached_runtime_capabilities(),
        )
        .map_err(ResolutionError::DirectUserSelectionUnavailable)?;
        let selections =
            resolve_locked_selections(session_profile.attached_runtime_locked(), &requested)?;
        Ok(DirectUserInvocationResolution {
            contract_version: DIRECT_USER_INVOCATION_RESOLUTION_CONTRACT_VERSION,
            session_profile_digest: creation.digest.clone(),
            selections,
        })
    }

    pub(crate) fn validate_pinned_session(
        source: &dyn SelectedRuntimeProfileSource,
        creation: &SessionCreationResolution,
    ) -> Result<(), ResolutionError> {
        creation.verify_digest()?;
        let runtime_profile = source.selected_runtime_profile()?;
        runtime_profile
            .validate()
            .map_err(ResolutionError::InvalidInput)?;
        let session_profile = creation.session_profile();
        if runtime_profile.profile_ref != session_profile.runtime_profile_ref() {
            return Err(ResolutionError::RuntimeProfileChanged {
                expected: session_profile.runtime_profile_ref().to_owned(),
                actual: runtime_profile.profile_ref,
            });
        }
        Ok(())
    }
}

fn validate_creation_request(request: &SessionCreationRequest) -> Result<(), ResolutionError> {
    if request.contract_version != SESSION_CREATION_REQUEST_CONTRACT_VERSION {
        return Err(ResolutionError::InvalidInput(format!(
            "Session creation request contract version {} is unsupported",
            request.contract_version
        )));
    }
    request
        .capability_profile
        .validate()
        .map_err(ResolutionError::InvalidInput)?;
    request
        .node_profile
        .validate()
        .map_err(ResolutionError::InvalidInput)
}

fn validate_direct_user_request(
    request: &DirectUserInvocationRequest,
) -> Result<(), ResolutionError> {
    if request.contract_version != DIRECT_USER_INVOCATION_REQUEST_CONTRACT_VERSION {
        return Err(ResolutionError::InvalidInput(format!(
            "Direct-user invocation request contract version {} is unsupported",
            request.contract_version
        )));
    }
    if let Some(model) = &request.model {
        validate_identifier("Direct-user invocation", "model", model)
            .map_err(ResolutionError::InvalidInput)?;
    }
    if let Some(reasoning_mode) = &request.reasoning_mode {
        validate_identifier("Direct-user invocation", "reasoningMode", reasoning_mode)
            .map_err(ResolutionError::InvalidInput)?;
    }
    Ok(())
}

fn validate_narrowing(
    runtime_profile: &RuntimeProfileSnapshot,
    capability_profile: &CapabilityProfile,
    node_profile: &NodeProfile,
) -> Result<(), ResolutionError> {
    if let Some(capability) = capability_profile
        .allowed_capabilities
        .first_capability_outside(&runtime_profile.exposure)
    {
        return Err(ResolutionError::CapabilityProfileWidensRuntime(capability));
    }
    if let Some(capability) = node_profile
        .allowed_capabilities
        .first_capability_outside(&capability_profile.allowed_capabilities)
    {
        return Err(ResolutionError::NodeProfileWidensCapabilityProfile(
            capability,
        ));
    }
    validate_selection_availability(&runtime_profile.locked, &node_profile.allowed_capabilities)
        .map_err(ResolutionError::LockedCapabilityExcluded)
}

fn resolve_pinned_defaults(
    locked: &RuntimeSelections,
    pinned: &RuntimeSelections,
    available: &CapabilitySet,
) -> Result<RuntimeSelections, ResolutionError> {
    validate_selection_availability(pinned, available)
        .map_err(ResolutionError::PinnedSelectionUnavailable)?;
    resolve_locked_selections(locked, pinned)
}

fn resolve_locked_selections(
    locked: &RuntimeSelections,
    requested: &RuntimeSelections,
) -> Result<RuntimeSelections, ResolutionError> {
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

fn session_profile_digest(
    contract_version: u32,
    session_profile: &SessionProfile,
) -> Result<String, ResolutionError> {
    let bytes = serde_json::to_vec(session_profile)
        .map_err(|error| ResolutionError::Encoding(error.to_string()))?;
    let mut digest = Sha256::new();
    digest.update(b"execution-configuration/session-profile\0");
    digest.update(contract_version.to_be_bytes());
    digest.update(bytes);
    Ok(format!("{:x}", digest.finalize()))
}
