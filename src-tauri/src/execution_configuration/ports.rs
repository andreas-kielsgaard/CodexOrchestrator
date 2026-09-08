use super::{capability_profile::CapabilityProfile, runtime_profile::RuntimeProfileSnapshot};
use std::{error::Error, fmt};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CapabilityProfileRepositoryError {
    AlreadyExists(String),
    NotFound(String),
    RevisionConflict {
        capability_profile_id: String,
        expected_revision: u64,
    },
    InvalidStoredProfile(String),
    Storage(String),
}

impl fmt::Display for CapabilityProfileRepositoryError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AlreadyExists(id) => {
                write!(formatter, "Capability Profile `{id}` already exists")
            }
            Self::NotFound(id) => write!(formatter, "Capability Profile `{id}` does not exist"),
            Self::RevisionConflict {
                capability_profile_id,
                expected_revision,
            } => write!(
                formatter,
                "Capability Profile `{capability_profile_id}` is no longer at revision {expected_revision}"
            ),
            Self::InvalidStoredProfile(message) => {
                write!(formatter, "Stored Capability Profile is invalid: {message}")
            }
            Self::Storage(message) => formatter.write_str(message),
        }
    }
}

impl Error for CapabilityProfileRepositoryError {}

/// Persistence owned by Execution Configuration.
///
/// `replace` uses the revision read by the application service so concurrent writers cannot
/// silently discard one another. Consumers should use `CapabilityProfileService`, not this port.
pub(crate) trait CapabilityProfileRepository: Send + Sync {
    fn list(&self) -> Result<Vec<CapabilityProfile>, CapabilityProfileRepositoryError>;

    fn find(
        &self,
        capability_profile_id: &str,
    ) -> Result<Option<CapabilityProfile>, CapabilityProfileRepositoryError>;

    fn insert(
        &self,
        capability_profile: &CapabilityProfile,
    ) -> Result<(), CapabilityProfileRepositoryError>;

    fn replace(
        &self,
        capability_profile: &CapabilityProfile,
        expected_revision: u64,
    ) -> Result<(), CapabilityProfileRepositoryError>;

    fn remove(&self, capability_profile_id: &str) -> Result<(), CapabilityProfileRepositoryError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SelectedRuntimeProfileSourceError {
    message: String,
}

impl SelectedRuntimeProfileSourceError {
    pub(crate) fn unavailable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for SelectedRuntimeProfileSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for SelectedRuntimeProfileSourceError {}

pub(crate) trait SelectedRuntimeProfileSource: Send + Sync {
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError>;
}
