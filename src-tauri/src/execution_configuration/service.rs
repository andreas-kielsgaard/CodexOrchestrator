use super::{
    capability_profile::{
        CapabilityProfile, ProfileRoutePolicy, CAPABILITY_PROFILE_CONTRACT_VERSION,
    },
    ports::{
        CapabilityProfileRepository, CapabilityProfileRepositoryError, SelectedRuntimeProfileSource,
    },
    runtime_profile::{validate_identifier, CapabilitySet, RuntimeProfileSnapshot},
};
use std::{error::Error, fmt, sync::Arc};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct CapabilityProfileService {
    repository: Arc<dyn CapabilityProfileRepository>,
    runtime_profile_source: Arc<dyn SelectedRuntimeProfileSource>,
    endpoints: Option<Arc<crate::execution_targets::endpoints::ExecutionEndpoints>>,
}

impl CapabilityProfileService {
    pub(crate) fn codex_skills_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<
        orchid_engine::codex::app_server::skills::CodexSkillCatalogue,
        CapabilityProfileServiceError,
    > {
        self.runtime_profile_source
            .discover_skills_for_configuration(reference, cwd)
            .map_err(|error| CapabilityProfileServiceError::RuntimeUnavailable(error.to_string()))
    }
    pub(crate) fn model_catalogue(
        &self,
        reference: &str,
    ) -> Result<super::ModelCatalogueView, CapabilityProfileServiceError> {
        validate_identifier("Model catalogue", "configurationRef", reference)
            .map_err(CapabilityProfileServiceError::InvalidInput)?;
        let observation = self
            .runtime_profile_source
            .refresh_quick_features_for_configuration(reference, None);
        let (stored, observation_error) = match observation {
            Ok(features) if !features.models.is_empty() => {
                let stored = super::StoredModelCatalogue {
                    observed_at: chrono::Utc::now().to_rfc3339(),
                    models: features.models,
                };
                self.repository.save_model_catalogue(reference, &stored)?;
                (Some(stored), None)
            }
            Ok(features) => (
                self.repository.model_catalogue(reference)?,
                Some(if features.limitations.is_empty() {
                    "The selected runtime did not report any models.".into()
                } else {
                    features.limitations.join(" ")
                }),
            ),
            Err(error) => (
                self.repository.model_catalogue(reference)?,
                Some(error.to_string()),
            ),
        };
        Ok(super::ModelCatalogueView {
            configuration_ref: reference.into(),
            observed_at: stored.as_ref().map(|value| value.observed_at.clone()),
            models: stored.map(|value| value.models).unwrap_or_default(),
            observation_error,
        })
    }
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
            .profile_for_configuration(&binding.configuration_ref, cwd)
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
        self.create_with_routes(
            capability_profile_id,
            name,
            allowed_capabilities,
            defaults,
            execution,
            Vec::new(),
            None,
        )
    }

    pub(crate) fn create_generated_with_routes(
        &self,
        name: String,
        allowed_capabilities: CapabilitySet,
        defaults: super::RuntimeSelections,
        execution: crate::execution_targets::domain::ExecutionBinding,
        route_policies: Vec<ProfileRoutePolicy>,
        default_route_id: Option<String>,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        self.create_with_routes(
            Uuid::new_v4().to_string(),
            name,
            allowed_capabilities,
            defaults,
            execution,
            route_policies,
            default_route_id,
        )
    }

    pub(crate) fn create_with_routes(
        &self,
        capability_profile_id: String,
        name: String,
        allowed_capabilities: CapabilitySet,
        defaults: super::RuntimeSelections,
        execution: crate::execution_targets::domain::ExecutionBinding,
        route_policies: Vec<ProfileRoutePolicy>,
        default_route_id: Option<String>,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        let execution = selected_route_execution(&route_policies, default_route_id.as_deref())
            .unwrap_or(execution);
        let capability_profile = CapabilityProfile {
            execution,
            contract_version: CAPABILITY_PROFILE_CONTRACT_VERSION,
            defaults,
            route_policies,
            default_route_id,
            capability_profile_id,
            name,
            revision: 1,
            allowed_capabilities,
        };
        capability_profile
            .validate()
            .map_err(CapabilityProfileServiceError::InvalidInput)?;
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
        self.update_with_routes(
            capability_profile_id,
            name,
            allowed_capabilities,
            defaults,
            execution,
            current.route_policies,
            current.default_route_id,
        )
    }

    pub(crate) fn update_with_routes(
        &self,
        capability_profile_id: &str,
        name: String,
        allowed_capabilities: CapabilitySet,
        defaults: super::RuntimeSelections,
        execution: crate::execution_targets::domain::ExecutionBinding,
        route_policies: Vec<ProfileRoutePolicy>,
        default_route_id: Option<String>,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        let current = self.read(capability_profile_id)?;
        let revision = current.revision.checked_add(1).ok_or_else(|| {
            CapabilityProfileServiceError::RevisionOverflow {
                capability_profile_id: capability_profile_id.into(),
            }
        })?;
        let replacement = CapabilityProfile {
            execution: selected_route_execution(&route_policies, default_route_id.as_deref())
                .unwrap_or(execution),
            contract_version: CAPABILITY_PROFILE_CONTRACT_VERSION,
            defaults,
            route_policies,
            default_route_id,
            capability_profile_id: current.capability_profile_id,
            name,
            revision,
            allowed_capabilities,
        };
        replacement
            .validate()
            .map_err(CapabilityProfileServiceError::InvalidInput)?;
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
}

fn selected_route_execution(
    routes: &[ProfileRoutePolicy],
    default_route_id: Option<&str>,
) -> Option<crate::execution_targets::domain::ExecutionBinding> {
    routes
        .iter()
        .find(|route| Some(route.route_id.as_str()) == default_route_id)
        .map(|route| route.execution.clone())
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
        repository::{InMemoryCapabilityProfileRepository, SqliteCapabilityProfileRepository},
        runtime_profile::{RuntimeSelections, SandboxMode, RUNTIME_PROFILE_CONTRACT_VERSION},
        ModelAllowance, ProfileRoutePolicy,
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

    fn route(id: &str, configuration_ref: &str) -> ProfileRoutePolicy {
        ProfileRoutePolicy {
            route_id: id.into(),
            execution: crate::execution_targets::domain::ExecutionBinding {
                configuration_ref: configuration_ref.into(),
                ..Default::default()
            },
            model_allowances: vec![ModelAllowance {
                model_id: "codex-a".into(),
                minimum_reasoning: "medium".into(),
                maximum_reasoning: "high".into(),
            }],
            mcp_groups: set(&["codex-profile-mcps"]),
            skill_groups: set(&["codex-profile-skills"]),
            defaults: RuntimeSelections {
                model: Some("codex-a".into()),
                reasoning_mode: Some("high".into()),
                sandbox_mode: Some(SandboxMode::WorkspaceWrite),
            },
        }
    }

    #[test]
    fn generated_route_profile_persists_default_route_and_group_policy() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("capability-profile-flow.sqlite");
        let repository = Arc::new(SqliteCapabilityProfileRepository::open(&database).unwrap());
        let service = CapabilityProfileService::new(
            repository.clone(),
            Arc::new(FixedRuntimeSource(runtime())),
        );
        let local = route("local", "selected");
        let alternate = route("alternate", "other-codex-home");

        let created = service
            .create_generated_with_routes(
                "Disposable route flow".into(),
                allowed(&["codex-a"]),
                RuntimeSelections::default(),
                Default::default(),
                vec![local.clone(), alternate.clone()],
                Some(alternate.route_id.clone()),
            )
            .unwrap();
        assert!(Uuid::parse_str(&created.capability_profile_id).is_ok());
        assert_eq!(created.default_route_id.as_deref(), Some("alternate"));
        assert_eq!(created.execution.configuration_ref, "other-codex-home");
        assert_eq!(
            created.default_route().unwrap().mcp_groups,
            alternate.mcp_groups
        );
        assert_eq!(
            created.default_route().unwrap().skill_groups,
            alternate.skill_groups
        );

        let updated = service
            .update_with_routes(
                &created.capability_profile_id,
                created.name.clone(),
                allowed(&["codex-a"]),
                RuntimeSelections::default(),
                Default::default(),
                vec![local.clone(), alternate],
                Some(local.route_id.clone()),
            )
            .unwrap();
        assert_eq!(updated.revision, 2);
        assert_eq!(updated.default_route_id.as_deref(), Some("local"));
        assert_eq!(updated.execution.configuration_ref, "selected");

        drop(service);
        drop(repository);
        let reopened = SqliteCapabilityProfileRepository::open(&database).unwrap();
        let persisted = reopened
            .find(&created.capability_profile_id)
            .unwrap()
            .unwrap();
        assert_eq!(persisted, updated);
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
    fn saves_design_time_choices_without_observing_runtime() {
        let service = service();
        let profile = service
            .create("wide".into(), "Wide".into(), allowed(&["unavailable"]))
            .unwrap();
        assert!(profile.allowed_capabilities.models.contains("unavailable"));

        let mut excluding_lock = allowed(&["codex-a"]);
        excluding_lock.sandbox_modes.clear();
        assert!(service
            .create("locked".into(), "Locked".into(), excluding_lock)
            .is_ok());
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

    #[test]
    fn failed_model_refresh_preserves_the_last_complete_route_catalogue() {
        struct FlakyModels(std::sync::atomic::AtomicBool);
        impl SelectedRuntimeProfileSource for FlakyModels {
            fn selected_runtime_profile(
                &self,
            ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
                Err(SelectedRuntimeProfileSourceError::unavailable(
                    "runtime offline",
                ))
            }
            fn quick_features_for_configuration(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<
                crate::execution_configuration::RuntimeQuickFeatures,
                SelectedRuntimeProfileSourceError,
            > {
                if self.0.load(std::sync::atomic::Ordering::SeqCst) {
                    return Err(SelectedRuntimeProfileSourceError::unavailable(
                        "runtime offline",
                    ));
                }
                Ok(crate::execution_configuration::RuntimeQuickFeatures {
                    models: vec![crate::execution_configuration::QuickModel {
                        id: "known-model".into(),
                        label: "Known model".into(),
                        description: String::new(),
                        default_reasoning_mode: Some("low".into()),
                        reasoning_modes: vec![],
                    }],
                    ..Default::default()
                })
            }
        }
        let source = Arc::new(FlakyModels(std::sync::atomic::AtomicBool::new(true)));
        let service = CapabilityProfileService::new(
            Arc::new(InMemoryCapabilityProfileRepository::default()),
            source.clone(),
        );
        let empty = service.model_catalogue("home-one").unwrap();
        assert!(empty.observed_at.is_none());
        assert!(empty.models.is_empty());
        source.0.store(false, std::sync::atomic::Ordering::SeqCst);
        let observed = service.model_catalogue("home-one").unwrap();
        assert_eq!(observed.models[0].id, "known-model");
        let observed_at = observed.observed_at;
        source.0.store(true, std::sync::atomic::Ordering::SeqCst);
        let stale = service.model_catalogue("home-one").unwrap();
        assert_eq!(stale.observed_at, observed_at);
        assert_eq!(stale.models[0].id, "known-model");
        assert_eq!(stale.observation_error.as_deref(), Some("runtime offline"));
    }
}
