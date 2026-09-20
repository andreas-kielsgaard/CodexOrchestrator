use super::{
    capability_profile::{CapabilityProfile, CAPABILITY_PROFILE_CONTRACT_VERSION},
    ports::{
        CapabilityProfileRepository, CapabilityProfileRepositoryError, SelectedRuntimeProfileSource,
    },
    runtime_profile::{
        validate_identifier, validate_selection_availability, CapabilitySet, RuntimeProfileSnapshot,
    },
};
use std::{error::Error, fmt, sync::Arc};

#[derive(Clone)]
pub(crate) struct CapabilityProfileService {
    repository: Arc<dyn CapabilityProfileRepository>,
    runtime_profile_source: Arc<dyn SelectedRuntimeProfileSource>,
    endpoints: Option<Arc<crate::execution_targets::endpoints::ExecutionEndpoints>>,
}

impl CapabilityProfileService {
    pub(crate) fn native_inventory(
        &self,
    ) -> Result<super::NativeCapabilityInventory, CapabilityProfileServiceError> {
        self.runtime_profile_source
            .native_inventory()
            .map_err(|e| CapabilityProfileServiceError::RuntimeUnavailable(e.to_string()))
    }
    pub(crate) fn native_inventory_for_configuration(
        &self,
        reference: &str,
    ) -> Result<super::NativeCapabilityInventory, CapabilityProfileServiceError> {
        self.runtime_profile_source
            .inventory_for_configuration(reference, None)
            .map_err(|e| CapabilityProfileServiceError::RuntimeUnavailable(e.to_string()))
    }
    pub(crate) fn default_profile_id(
        &self,
    ) -> Result<Option<String>, CapabilityProfileServiceError> {
        self.repository.default_profile_id().map_err(Into::into)
    }
    pub(crate) fn set_default_profile(
        &self,
        id: &str,
    ) -> Result<(), CapabilityProfileServiceError> {
        self.repository.set_default_profile(id).map_err(Into::into)
    }
    pub(crate) fn default_profile(
        &self,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        self.repository.default_profile()?.ok_or_else(|| {
            CapabilityProfileServiceError::InvalidInput(
                "Choose a default Capability Profile in Capabilities before starting a new session"
                    .into(),
            )
        })
    }
    pub(crate) fn new(
        repository: Arc<dyn CapabilityProfileRepository>,
        runtime_profile_source: Arc<dyn SelectedRuntimeProfileSource>,
    ) -> Self {
        Self {
            repository,
            runtime_profile_source,
            endpoints: None,
        }
    }

    pub(crate) fn with_endpoints(
        mut self,
        endpoints: Arc<crate::execution_targets::endpoints::ExecutionEndpoints>,
    ) -> Self {
        self.endpoints = Some(endpoints);
        self
    }

    pub(crate) fn runtime_for_binding(
        &self,
        binding: &crate::execution_targets::domain::ExecutionBinding,
        cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, CapabilityProfileServiceError> {
        if let Some(endpoints) = &self.endpoints {
            return endpoints
                .describe_runtime(binding, cwd)
                .map(|runtime| runtime.runtime_profile)
                .map_err(CapabilityProfileServiceError::RuntimeUnavailable);
        }
        if binding.is_remote() {
            return Err(CapabilityProfileServiceError::RuntimeUnavailable(
                "Remote execution endpoints are not configured".into(),
            ));
        }
        self.runtime_profile_source
            .selected_runtime_profile_at(cwd)
            .map_err(|e| CapabilityProfileServiceError::RuntimeUnavailable(e.to_string()))
    }

    /// Returns the one runtime profile currently available to Execution Configuration.
    pub(crate) fn runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, CapabilityProfileServiceError> {
        let runtime_profile = self
            .runtime_profile_source
            .selected_runtime_profile()
            .map_err(|error| {
                CapabilityProfileServiceError::RuntimeUnavailable(error.to_string())
            })?;
        runtime_profile
            .validate()
            .map_err(CapabilityProfileServiceError::InvalidInput)?;
        Ok(runtime_profile)
    }

    pub(crate) fn list(&self) -> Result<Vec<CapabilityProfile>, CapabilityProfileServiceError> {
        self.repository.list().map_err(Into::into)
    }

    pub(crate) fn read(
        &self,
        capability_profile_id: &str,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        validate_identifier(
            "Capability Profile query",
            "capabilityProfileId",
            capability_profile_id,
        )
        .map_err(CapabilityProfileServiceError::InvalidInput)?;
        self.repository
            .find(capability_profile_id)
            .map_err(CapabilityProfileServiceError::from)?
            .ok_or_else(|| CapabilityProfileServiceError::NotFound(capability_profile_id.into()))
    }

    pub(crate) fn create(
        &self,
        capability_profile_id: String,
        name: String,
        allowed_capabilities: CapabilitySet,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        self.create_with_defaults(
            capability_profile_id,
            name,
            allowed_capabilities,
            Default::default(),
        )
    }
    pub(crate) fn create_with_defaults(
        &self,
        capability_profile_id: String,
        name: String,
        allowed_capabilities: CapabilitySet,
        defaults: super::RuntimeSelections,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        self.create_with_execution(
            capability_profile_id,
            name,
            allowed_capabilities,
            defaults,
            Default::default(),
        )
    }

    pub(crate) fn create_with_execution(
        &self,
        capability_profile_id: String,
        name: String,
        allowed_capabilities: CapabilitySet,
        defaults: super::RuntimeSelections,
        execution: crate::execution_targets::domain::ExecutionBinding,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        let capability_profile = CapabilityProfile {
            execution,
            contract_version: CAPABILITY_PROFILE_CONTRACT_VERSION,
            defaults,
            capability_profile_id,
            name,
            revision: 1,
            allowed_capabilities,
        };
        self.validate_for_selected_runtime(&capability_profile)?;
        self.repository.insert(&capability_profile)?;
        Ok(capability_profile)
    }

    pub(crate) fn update(
        &self,
        capability_profile_id: &str,
        name: String,
        allowed_capabilities: CapabilitySet,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        let current = self.read(capability_profile_id)?;
        self.update_with_defaults(
            capability_profile_id,
            name,
            allowed_capabilities,
            current.defaults,
        )
    }
    pub(crate) fn update_with_defaults(
        &self,
        capability_profile_id: &str,
        name: String,
        allowed_capabilities: CapabilitySet,
        defaults: super::RuntimeSelections,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        let current = self.read(capability_profile_id)?;
        self.update_with_execution(
            capability_profile_id,
            name,
            allowed_capabilities,
            defaults,
            current.execution,
        )
    }

    pub(crate) fn update_with_execution(
        &self,
        capability_profile_id: &str,
        name: String,
        allowed_capabilities: CapabilitySet,
        defaults: super::RuntimeSelections,
        execution: crate::execution_targets::domain::ExecutionBinding,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        let current = self.read(capability_profile_id)?;
        let revision = current.revision.checked_add(1).ok_or_else(|| {
            CapabilityProfileServiceError::RevisionOverflow {
                capability_profile_id: capability_profile_id.into(),
            }
        })?;
        let replacement = CapabilityProfile {
            execution,
            contract_version: CAPABILITY_PROFILE_CONTRACT_VERSION,
            defaults,
            capability_profile_id: current.capability_profile_id,
            name,
            revision,
            allowed_capabilities,
        };
        self.validate_for_selected_runtime(&replacement)?;
        self.repository.replace(&replacement, current.revision)?;
        Ok(replacement)
    }

    pub(crate) fn delete(
        &self,
        capability_profile_id: &str,
    ) -> Result<(), CapabilityProfileServiceError> {
        validate_identifier(
            "Capability Profile deletion",
            "capabilityProfileId",
            capability_profile_id,
        )
        .map_err(CapabilityProfileServiceError::InvalidInput)?;
        self.repository
            .remove(capability_profile_id)
            .map_err(Into::into)
    }

    fn validate_for_selected_runtime(
        &self,
        capability_profile: &CapabilityProfile,
    ) -> Result<(), CapabilityProfileServiceError> {
        capability_profile
            .validate()
            .map_err(CapabilityProfileServiceError::InvalidInput)?;
        let runtime_profile = self.runtime_for_binding(&capability_profile.execution, None)?;
        if let Some(capability) = capability_profile
            .allowed_capabilities
            .first_capability_outside(&runtime_profile.exposure)
        {
            return Err(CapabilityProfileServiceError::WidensRuntime(capability));
        }
        validate_selection_availability(
            &runtime_profile.locked,
            &capability_profile.allowed_capabilities,
        )
        .map_err(CapabilityProfileServiceError::ExcludesRuntimeLock)
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum CapabilityProfileServiceError {
    InvalidInput(String),
    RuntimeUnavailable(String),
    WidensRuntime(String),
    ExcludesRuntimeLock(String),
    AlreadyExists(String),
    NotFound(String),
    RevisionConflict {
        capability_profile_id: String,
        expected_revision: u64,
    },
    RevisionOverflow {
        capability_profile_id: String,
    },
    InvalidStoredProfile(String),
    Storage(String),
}

impl fmt::Display for CapabilityProfileServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => formatter.write_str(message),
            Self::RuntimeUnavailable(message) => {
                write!(formatter, "Selected runtime profile is unavailable: {message}")
            }
            Self::WidensRuntime(capability) => {
                write!(formatter, "Capability Profile requests unavailable {capability}")
            }
            Self::ExcludesRuntimeLock(capability) => write!(
                formatter,
                "Capability Profile excludes runtime-locked {capability}"
            ),
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
            Self::RevisionOverflow {
                capability_profile_id,
            } => write!(
                formatter,
                "Capability Profile `{capability_profile_id}` cannot create another revision"
            ),
            Self::InvalidStoredProfile(message) => {
                write!(formatter, "Stored Capability Profile is invalid: {message}")
            }
            Self::Storage(message) => formatter.write_str(message),
        }
    }
}

impl Error for CapabilityProfileServiceError {}

impl From<CapabilityProfileRepositoryError> for CapabilityProfileServiceError {
    fn from(error: CapabilityProfileRepositoryError) -> Self {
        match error {
            CapabilityProfileRepositoryError::AlreadyExists(id) => Self::AlreadyExists(id),
            CapabilityProfileRepositoryError::NotFound(id) => Self::NotFound(id),
            CapabilityProfileRepositoryError::RevisionConflict {
                capability_profile_id,
                expected_revision,
            } => Self::RevisionConflict {
                capability_profile_id,
                expected_revision,
            },
            CapabilityProfileRepositoryError::InvalidStoredProfile(message) => {
                Self::InvalidStoredProfile(message)
            }
            CapabilityProfileRepositoryError::Storage(message) => Self::Storage(message),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::execution_configuration::{
        ports::SelectedRuntimeProfileSourceError,
        repository::InMemoryCapabilityProfileRepository,
        runtime_profile::{RuntimeSelections, SandboxMode, RUNTIME_PROFILE_CONTRACT_VERSION},
    };
    use std::collections::BTreeSet;

    struct FixedRuntimeSource(RuntimeProfileSnapshot);

    impl SelectedRuntimeProfileSource for FixedRuntimeSource {
        fn selected_runtime_profile(
            &self,
        ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
            Ok(self.0.clone())
        }
    }

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn runtime() -> RuntimeProfileSnapshot {
        RuntimeProfileSnapshot {
            contract_version: RUNTIME_PROFILE_CONTRACT_VERSION,
            profile_ref: "native-codex:selected".into(),
            exposure: CapabilitySet {
                models: set(&["codex-a", "codex-b"]),
                reasoning_modes: set(&["medium", "high"]),
                sandbox_modes: [SandboxMode::WorkspaceWrite].into_iter().collect(),
                skills: set(&["review"]),
                ..CapabilitySet::default()
            },
            locked: RuntimeSelections {
                sandbox_mode: Some(SandboxMode::WorkspaceWrite),
                ..RuntimeSelections::default()
            },
        }
    }

    fn allowed(models: &[&str]) -> CapabilitySet {
        CapabilitySet {
            models: set(models),
            reasoning_modes: set(&["high"]),
            sandbox_modes: [SandboxMode::WorkspaceWrite].into_iter().collect(),
            skills: set(&["review"]),
            ..CapabilitySet::default()
        }
    }

    fn service() -> CapabilityProfileService {
        CapabilityProfileService::new(
            Arc::new(InMemoryCapabilityProfileRepository::default()),
            Arc::new(FixedRuntimeSource(runtime())),
        )
    }

    #[test]
    fn owns_crud_and_revision_increment() {
        let service = service();
        assert_eq!(service.runtime_profile().unwrap(), runtime());

        let created = service
            .create("review".into(), "Review".into(), allowed(&["codex-a"]))
            .unwrap();
        assert_eq!(created.revision, 1);
        assert_eq!(service.read("review").unwrap(), created);
        assert_eq!(service.list().unwrap(), vec![created]);

        let updated = service
            .update(
                "review",
                "Architecture review".into(),
                allowed(&["codex-a", "codex-b"]),
            )
            .unwrap();
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.name, "Architecture review");
        assert_eq!(service.read("review").unwrap(), updated);

        service.delete("review").unwrap();
        assert!(service.list().unwrap().is_empty());
        assert!(matches!(
            service.read("review"),
            Err(CapabilityProfileServiceError::NotFound(id)) if id == "review"
        ));
    }

    #[test]
    fn rejects_profiles_that_widen_runtime_or_exclude_a_runtime_lock() {
        let service = service();
        assert!(matches!(
            service.create("wide".into(), "Wide".into(), allowed(&["unavailable"])),
            Err(CapabilityProfileServiceError::WidensRuntime(capability))
                if capability == "model `unavailable`"
        ));

        let mut excluding_lock = allowed(&["codex-a"]);
        excluding_lock.sandbox_modes.clear();
        assert!(matches!(
            service.create("locked".into(), "Locked".into(), excluding_lock),
            Err(CapabilityProfileServiceError::ExcludesRuntimeLock(capability))
                if capability.contains("sandbox mode")
        ));
    }

    #[test]
    fn reports_duplicate_and_missing_targets() {
        let service = service();
        service
            .create("review".into(), "Review".into(), allowed(&["codex-a"]))
            .unwrap();
        assert!(matches!(
            service.create("review".into(), "Review".into(), allowed(&["codex-a"])),
            Err(CapabilityProfileServiceError::AlreadyExists(id)) if id == "review"
        ));
        assert!(matches!(
            service.update("missing", "Missing".into(), allowed(&["codex-a"])),
            Err(CapabilityProfileServiceError::NotFound(id)) if id == "missing"
        ));
        assert!(matches!(
            service.delete("missing"),
            Err(CapabilityProfileServiceError::NotFound(id)) if id == "missing"
        ));
    }
}
