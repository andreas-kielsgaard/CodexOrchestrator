use super::{capability_profile::CapabilityProfile, runtime_profile::RuntimeProfileSnapshot};
use std::{error::Error, fmt};

/// A product-owned skill folder whose skills a route's capability group makes eligible for the
/// session manifest.
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct RuntimeSkillRoot {
    pub(crate) group_id: String,
    pub(crate) path: std::path::PathBuf,
}

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
    fn model_catalogue(
        &self,
        configuration_ref: &str,
    ) -> Result<Option<super::StoredModelCatalogue>, CapabilityProfileRepositoryError>;
    fn save_model_catalogue(
        &self,
        configuration_ref: &str,
        catalogue: &super::StoredModelCatalogue,
    ) -> Result<(), CapabilityProfileRepositoryError>;
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
pub(crate) struct ProviderConfigurationSourceError {
    message: String,
}

impl ProviderConfigurationSourceError {
    pub(crate) fn unavailable(message: impl Into<String>) -> Self {
        Self {
            message: message.into(),
        }
    }
}

impl fmt::Display for ProviderConfigurationSourceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for ProviderConfigurationSourceError {}

/// One agent provider's registered configurations. Every operation is addressed by a
/// configuration ID within that provider; the provider registry selects the implementation.
pub(crate) trait ProviderConfigurationSource: Send + Sync {
    /// The configuration a provider bound to a Session outside Orchid's pinned profile, if any.
    fn configuration_ref_for_session(
        &self,
        _session_id: &str,
    ) -> Result<Option<String>, ProviderConfigurationSourceError> {
        Ok(None)
    }
    /// Resolves a provider alias, such as its current default, to a concrete configuration ID.
    fn resolve_configuration_ref(
        &self,
        reference: &str,
    ) -> Result<String, ProviderConfigurationSourceError> {
        Ok(reference.into())
    }
    fn profile_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError>;
    /// Skills the provider configuration discovers natively. Product skill roots are not
    /// provider concerns; see `ProductSkillRoots`.
    fn native_skills_for_configuration(
        &self,
        _reference: &str,
        _cwd: Option<&str>,
    ) -> Result<orchid_engine::contracts::ProviderSkillCatalogue, ProviderConfigurationSourceError>
    {
        Err(ProviderConfigurationSourceError::unavailable(
            "Native skill discovery is unavailable",
        ))
    }
    fn quick_features_for_configuration(
        &self,
        _reference: &str,
        _cwd: Option<&str>,
    ) -> Result<super::RuntimeQuickFeatures, ProviderConfigurationSourceError> {
        Err(ProviderConfigurationSourceError::unavailable(
            "This provider does not expose quick features.",
        ))
    }
    fn refresh_quick_features_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<super::RuntimeQuickFeatures, ProviderConfigurationSourceError> {
        self.quick_features_for_configuration(reference, cwd)
    }
    fn inventory_for_configuration(
        &self,
        _reference: &str,
        _cwd: Option<&str>,
    ) -> Result<super::NativeCapabilityInventory, ProviderConfigurationSourceError> {
        Err(ProviderConfigurationSourceError::unavailable(
            "Native inventory discovery is unavailable",
        ))
    }
}
