use super::{
    capability_profile::{
        CapabilityProfile, ProfileRoutePolicy, CAPABILITY_PROFILE_CONTRACT_VERSION,
    },
    ports::{
        CapabilityProfileRepository, CapabilityProfileRepositoryError, ProviderConfigurationSource,
    },
    runtime_profile::{validate_identifier, CapabilitySet, RuntimeProfileSnapshot},
};
use crate::execution_targets::domain::ExecutionRouteRef;
use orchid_engine::contracts::ProviderConfigurationRef;
use std::{error::Error, fmt, sync::Arc};
use uuid::Uuid;

#[derive(Clone)]
pub(crate) struct CapabilityProfileService {
    repository: Arc<dyn CapabilityProfileRepository>,
    endpoints: Option<Arc<crate::execution_targets::endpoints::ExecutionEndpoints>>,
}

impl CapabilityProfileService {
    fn configuration_source(
        &self,
        provider: &str,
    ) -> Result<Arc<dyn ProviderConfigurationSource>, CapabilityProfileServiceError> {
        self.endpoints
            .as_ref()
            .ok_or("Execution endpoints are not configured".to_string())
            .and_then(|endpoints| endpoints.configuration_source(provider))
            .map_err(CapabilityProfileServiceError::RuntimeUnavailable)
    }
    pub(crate) fn native_skills_for_configuration(
        &self,
        configuration: &ProviderConfigurationRef,
        cwd: Option<&str>,
    ) -> Result<orchid_engine::contracts::ProviderSkillCatalogue, CapabilityProfileServiceError>
    {
        self.configuration_source(&configuration.provider)?
            .native_skills_for_configuration(&configuration.configuration_id, cwd)
            .map_err(|error| CapabilityProfileServiceError::RuntimeUnavailable(error.to_string()))
    }
    /// Observes a route's models, keeping the last complete observation for offline editing.
    pub(crate) fn model_catalogue(
        &self,
        route: &ExecutionRouteRef,
    ) -> Result<super::ModelCatalogueView, CapabilityProfileServiceError> {
        validate_identifier("Model catalogue", "configurationRef", &route.configuration_ref)
            .map_err(CapabilityProfileServiceError::InvalidInput)?;
        let observation = if route.device_id == "local" {
            self.endpoints
                .as_ref()
                .ok_or("Execution endpoints are not configured".to_string())
                .and_then(|endpoints| endpoints.configuration_source(&route.provider))
                .and_then(|source| {
                    source
                        .refresh_quick_features_for_configuration(&route.configuration_ref, None)
                        .map_err(|error| error.to_string())
                })
        } else {
            Err("Model discovery is unavailable for remote devices.".to_string())
        };
        let (stored, observation_error) = match observation {
            Ok(features) if !features.models.is_empty() => {
                let stored = super::StoredModelCatalogue {
                    observed_at: chrono::Utc::now().to_rfc3339(),
                    models: features.models,
                };
                self.repository.save_model_catalogue(route, &stored)?;
                (Some(stored), None)
            }
            Ok(features) => (
                self.repository.model_catalogue(route)?,
                Some(if features.limitations.is_empty() {
                    "The selected runtime did not report any models.".into()
                } else {
                    features.limitations.join(" ")
                }),
            ),
            Err(error) => (self.repository.model_catalogue(route)?, Some(error)),
        };
        Ok(super::ModelCatalogueView {
            route: route.clone(),
            observed_at: stored.as_ref().map(|value| value.observed_at.clone()),
            models: stored.map(|value| value.models).unwrap_or_default(),
            observation_error,
        })
    }
    pub(crate) fn native_inventory_for_configuration(
        &self,
        configuration: &ProviderConfigurationRef,
    ) -> Result<super::NativeCapabilityInventory, CapabilityProfileServiceError> {
        self.configuration_source(&configuration.provider)?
            .inventory_for_configuration(&configuration.configuration_id, None)
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
    pub(crate) fn new(repository: Arc<dyn CapabilityProfileRepository>) -> Self {
        Self {
            repository,
            endpoints: None,
        }
    }

    /// Test composition: `source` serves the `codex` provider through local endpoints.
    #[cfg(test)]
    pub(crate) fn with_configuration_source(
        self,
        source: Arc<dyn ProviderConfigurationSource>,
    ) -> Self {
        let mut providers =
            crate::runtime::providers::registrations::ProviderRegistrations::default();
        providers.configurations.replace("codex", source);
        self.with_endpoints(Arc::new(
            crate::execution_targets::endpoints::ExecutionEndpoints::new(providers),
        ))
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
        self.endpoints
            .as_ref()
            .ok_or("Execution endpoints are not configured".to_string())
            .and_then(|endpoints| endpoints.describe_runtime(binding, cwd))
            .map(|runtime| runtime.runtime_profile)
            .map_err(CapabilityProfileServiceError::RuntimeUnavailable)
    }

    /// Returns the runtime profile of the default local execution binding.
    pub(crate) fn runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, CapabilityProfileServiceError> {
        let runtime_profile = self.runtime_for_binding(
            &crate::execution_targets::domain::ExecutionBinding::default(),
            None,
        )?;
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

    /// Resolves an unsent draft against the current profile revision without silently changing
    /// its selected execution route.
    pub(crate) fn resolve_draft_selection(
        &self,
        capability_profile_id: &str,
        execution: &crate::execution_targets::domain::ExecutionBinding,
    ) -> Result<CapabilityProfile, CapabilityProfileServiceError> {
        let profile = self.read(capability_profile_id)?;
        if !profile.contains_execution(execution) {
            return Err(CapabilityProfileServiceError::InvalidInput(
                "The selected execution route is no longer part of this Capability Profile. Choose a route again."
                    .into(),
            ));
        }
        Ok(profile)
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
        ports::ProviderConfigurationSourceError,
        repository::{InMemoryCapabilityProfileRepository, SqliteCapabilityProfileRepository},
        runtime_profile::{RuntimeSelections, SandboxMode, RUNTIME_PROFILE_CONTRACT_VERSION},
        ModelAllowance, ProfileRoutePolicy,
    };
    use std::collections::BTreeSet;

    struct FixedRuntimeSource(RuntimeProfileSnapshot);

    impl ProviderConfigurationSource for FixedRuntimeSource {
        fn profile_for_configuration(
        &self,
        _reference: &str,
        _cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError> {
            Ok(self.0.clone())
        }
    }

    fn set(values: &[&str]) -> BTreeSet<String> {
        values.iter().map(|value| (*value).to_owned()).collect()
    }

    fn runtime() -> RuntimeProfileSnapshot {
        RuntimeProfileSnapshot {
            contract_version: RUNTIME_PROFILE_CONTRACT_VERSION,
            configuration: orchid_engine::contracts::ProviderConfigurationRef::new("codex", "selected"),
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
            provider_options: None,
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
        CapabilityProfileService::new(Arc::new(InMemoryCapabilityProfileRepository::default())).with_configuration_source(Arc::new(FixedRuntimeSource(runtime())))
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
            mcp_groups: set(&["native-mcps"]),
            skill_groups: set(&["native-skills"]),
            defaults: RuntimeSelections {
                model: Some("codex-a".into()),
                reasoning_mode: Some("high".into()),
                sandbox_mode: Some(SandboxMode::WorkspaceWrite),
            },
            provider_options: None,
        }
    }

    #[test]
    fn generated_route_profile_persists_default_route_and_group_policy() {
        let directory = tempfile::tempdir().unwrap();
        let database = directory.path().join("capability-profile-flow.sqlite");
        let repository = Arc::new(SqliteCapabilityProfileRepository::open(&database).unwrap());
        let service = CapabilityProfileService::new(repository.clone()).with_configuration_source(Arc::new(FixedRuntimeSource(runtime())));
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
        let mut persisted = reopened
            .find(&created.capability_profile_id)
            .unwrap()
            .unwrap();
        persisted.execution.device_name = updated.execution.device_name.clone();
        for (persisted_route, updated_route) in
            persisted.route_policies.iter_mut().zip(&updated.route_policies)
        {
            persisted_route.execution.device_name = updated_route.execution.device_name.clone();
        }
        assert_eq!(persisted, updated);
    }

    #[test]
    fn draft_selection_follows_rename_but_never_switches_a_removed_route() {
        let service = service();
        let selected = route("local", "selected");
        let created = service
            .create_generated_with_routes(
                "Original name".into(),
                allowed(&["codex-a"]),
                RuntimeSelections::default(),
                Default::default(),
                vec![selected.clone()],
                Some(selected.route_id.clone()),
            )
            .unwrap();
        let draft_execution = selected.execution.clone();

        let renamed = service
            .update_with_routes(
                &created.capability_profile_id,
                "Renamed".into(),
                allowed(&["codex-a"]),
                RuntimeSelections::default(),
                Default::default(),
                vec![selected],
                Some("local".into()),
            )
            .unwrap();
        let resolved = service
            .resolve_draft_selection(&created.capability_profile_id, &draft_execution)
            .unwrap();
        assert_eq!(resolved.revision, renamed.revision);
        assert_eq!(resolved.name, "Renamed");

        let replacement = route("replacement", "other-home");
        service
            .update_with_routes(
                &created.capability_profile_id,
                "Renamed".into(),
                allowed(&["codex-a"]),
                RuntimeSelections::default(),
                Default::default(),
                vec![replacement.clone()],
                Some(replacement.route_id),
            )
            .unwrap();
        assert!(matches!(
            service.resolve_draft_selection(&created.capability_profile_id, &draft_execution),
            Err(CapabilityProfileServiceError::InvalidInput(message))
                if message.contains("route is no longer part")
        ));
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
        impl ProviderConfigurationSource for FlakyModels {
            fn profile_for_configuration(
        &self,
        _reference: &str,
        _cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError> {
                Err(ProviderConfigurationSourceError::unavailable(
                    "runtime offline",
                ))
            }
            fn quick_features_for_configuration(
                &self,
                _: &str,
                _: Option<&str>,
            ) -> Result<
                crate::execution_configuration::RuntimeQuickFeatures,
                ProviderConfigurationSourceError,
            > {
                if self.0.load(std::sync::atomic::Ordering::SeqCst) {
                    return Err(ProviderConfigurationSourceError::unavailable(
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
        let service = CapabilityProfileService::new(Arc::new(InMemoryCapabilityProfileRepository::default())).with_configuration_source(source.clone());
        let route = crate::execution_targets::domain::ExecutionRouteRef {
            device_id: "local".into(),
            provider: "codex".into(),
            configuration_ref: "home-one".into(),
        };
        let empty = service.model_catalogue(&route).unwrap();
        assert!(empty.observed_at.is_none());
        assert!(empty.models.is_empty());
        source.0.store(false, std::sync::atomic::Ordering::SeqCst);
        let observed = service.model_catalogue(&route).unwrap();
        assert_eq!(observed.models[0].id, "known-model");
        let observed_at = observed.observed_at;
        source.0.store(true, std::sync::atomic::Ordering::SeqCst);
        let stale = service.model_catalogue(&route).unwrap();
        assert_eq!(stale.observed_at, observed_at);
        assert_eq!(stale.models[0].id, "known-model");
        assert_eq!(stale.observation_error.as_deref(), Some("runtime offline"));
    }
}
