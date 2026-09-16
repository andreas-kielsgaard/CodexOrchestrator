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
    fn default_profile(
        &self,
    ) -> Result<Option<CapabilityProfile>, CapabilityProfileRepositoryError>;
    fn default_profile_id(&self) -> Result<Option<String>, CapabilityProfileRepositoryError>;
    fn set_default_profile(&self, id: &str) -> Result<(), CapabilityProfileRepositoryError>;
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
    fn configuration_home(
        &self,
        _reference: &str,
    ) -> Result<std::path::PathBuf, SelectedRuntimeProfileSourceError> {
        Err(SelectedRuntimeProfileSourceError::unavailable(
            "Native configuration home is unavailable",
        ))
    }
    fn quick_features_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<super::RuntimeQuickFeatures, SelectedRuntimeProfileSourceError> {
        if reference != "selected" {
            return Err(SelectedRuntimeProfileSourceError::unavailable(
                "This provider does not expose quick features for named configurations.",
            ));
        }
        self.quick_features_at(cwd)
    }
    fn quick_features_at(
        &self,
        _cwd: Option<&str>,
    ) -> Result<super::RuntimeQuickFeatures, SelectedRuntimeProfileSourceError> {
        Err(SelectedRuntimeProfileSourceError::unavailable(
            "This provider does not expose quick features.",
        ))
    }
    fn resolve_configuration_ref(
        &self,
        reference: &str,
    ) -> Result<String, SelectedRuntimeProfileSourceError> {
        Ok(reference.into())
    }
    fn profile_for_configuration(
        &self,
        _reference: &str,
        cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        self.selected_runtime_profile_at(cwd)
    }
    fn inventory_for_configuration(
        &self,
        _reference: &str,
        _cwd: Option<&str>,
    ) -> Result<super::NativeCapabilityInventory, SelectedRuntimeProfileSourceError> {
        self.native_inventory()
    }
    fn native_inventory(
        &self,
    ) -> Result<super::NativeCapabilityInventory, SelectedRuntimeProfileSourceError> {
        Err(SelectedRuntimeProfileSourceError::unavailable(
            "Native inventory discovery is unavailable",
        ))
    }
    fn selected_runtime_profile_at(
        &self,
        _cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        self.selected_runtime_profile()
    }
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError>;
}

pub(crate) struct WorkingContextProfileSource<'a> {
    pub(crate) source: &'a dyn SelectedRuntimeProfileSource,
    pub(crate) cwd: Option<&'a str>,
}
impl SelectedRuntimeProfileSource for WorkingContextProfileSource<'_> {
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        self.source.selected_runtime_profile_at(self.cwd)
    }
}
