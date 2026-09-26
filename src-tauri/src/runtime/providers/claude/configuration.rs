//! What a Claude setup offers Capability Profiles and sessions: the models and effort levels from
//! Claude's `initialize` report, and skills from the setup's and the project's skill folders.
//! Claude offers full access only.
use super::setups::{ClaudeSetup, ClaudeSetups};
use crate::execution_configuration::{
    CapabilitySet, NativeCapabilityEntry, NativeCapabilityInventory, ProviderConfigurationSource,
    ProviderConfigurationSourceError, ProviderSetup, QuickModel, QuickReasoningMode,
    RuntimeProfileSnapshot, RuntimeQuickFeatures, RuntimeSelections, SandboxMode,
};
use orchid_engine::{
    configuration::RUNTIME_PROFILE_CONTRACT_VERSION,
    contracts::{ProviderConfigurationRef, ProviderSkillCatalogue},
    providers::claude::{
        discovery::{self, ClaudeModel},
        PROVIDER,
    },
};
use std::{
    collections::{BTreeMap, BTreeSet},
    path::Path,
    sync::{Arc, Mutex},
};

/// Reads a setup's models from Claude. Replaced in tests.
pub(crate) type ModelReader =
    Arc<dyn Fn(&ClaudeSetup, &[(String, String)]) -> Result<Vec<ClaudeModel>, String> + Send + Sync>;

pub(crate) struct ClaudeConfigurationSource {
    setups: Arc<ClaudeSetups>,
    product_tools: BTreeMap<String, BTreeSet<String>>,
    read_models: ModelReader,
    /// Models by setup, read once and again on refresh.
    models: Mutex<BTreeMap<String, Vec<ClaudeModel>>>,
}

impl ClaudeConfigurationSource {
    pub(crate) fn new(
        setups: Arc<ClaudeSetups>,
        product_tools: BTreeMap<String, BTreeSet<String>>,
    ) -> Self {
        Self {
            setups,
            product_tools,
            read_models: Arc::new(|setup, environment| {
                discovery::models(&setup.executable, environment)
            }),
            models: Mutex::default(),
        }
    }

    #[cfg(test)]
    pub(crate) fn with_model_reader(mut self, read_models: ModelReader) -> Self {
        self.read_models = read_models;
        self
    }

    fn setup(&self, reference: &str) -> Result<ClaudeSetup, ProviderConfigurationSourceError> {
        self.setups
            .resolve(reference)
            .map_err(ProviderConfigurationSourceError::unavailable)
    }

    fn models(
        &self,
        setup: &ClaudeSetup,
        refresh: bool,
    ) -> Result<Vec<ClaudeModel>, ProviderConfigurationSourceError> {
        let mut cache = self.models.lock().map_err(|_| {
            ProviderConfigurationSourceError::unavailable("Claude model cache is unavailable")
        })?;
        if let Some(models) = cache.get(&setup.id).filter(|_| !refresh) {
            return Ok(models.clone());
        }
        let models = (self.read_models)(setup, &self.setups.environment(setup))
            .map_err(ProviderConfigurationSourceError::unavailable)?;
        cache.insert(setup.id.clone(), models.clone());
        Ok(models)
    }

    fn skills(&self, setup: &ClaudeSetup, cwd: Option<&str>) -> ProviderSkillCatalogue {
        discovery::skills(Path::new(&setup.folder), cwd.map(Path::new))
    }

    fn quick_features(
        &self,
        reference: &str,
        cwd: Option<&str>,
        refresh: bool,
    ) -> Result<RuntimeQuickFeatures, ProviderConfigurationSourceError> {
        let setup = self.setup(reference)?;
        let mut features = RuntimeQuickFeatures {
            configuration: Some(configuration(&setup)),
            ..Default::default()
        };
        match self.models(&setup, refresh) {
            Ok(models) => features.models = models.iter().map(quick_model).collect(),
            Err(error) => features
                .limitations
                .push(format!("Model discovery is unavailable: {error}")),
        }
        features.append_native_skills(&self.skills(&setup, cwd));
        Ok(features)
    }
}

fn configuration(setup: &ClaudeSetup) -> ProviderConfigurationRef {
    ProviderConfigurationRef::new(PROVIDER, &setup.id)
}

fn quick_model(model: &ClaudeModel) -> QuickModel {
    QuickModel {
        id: model.value.clone(),
        label: if model.display_name.is_empty() {
            model.value.clone()
        } else {
            model.display_name.clone()
        },
        description: model.description.clone(),
        default_reasoning_mode: None,
        reasoning_modes: model
            .supported_effort_levels
            .iter()
            .map(|level| QuickReasoningMode {
                id: level.clone(),
                description: String::new(),
            })
            .collect(),
    }
}

impl ProviderConfigurationSource for ClaudeConfigurationSource {
    fn resolve_configuration_ref(
        &self,
        reference: &str,
    ) -> Result<String, ProviderConfigurationSourceError> {
        self.setup(reference).map(|setup| setup.id)
    }

    fn profile_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError> {
        let setup = self.setup(reference)?;
        let models = self.models(&setup, false)?;
        let exposure = CapabilitySet {
            models: models.iter().map(|model| model.value.clone()).collect(),
            reasoning_modes: models
                .iter()
                .flat_map(|model| model.supported_effort_levels.iter().cloned())
                .collect(),
            sandbox_modes: [SandboxMode::DangerFullAccess].into_iter().collect(),
            mcp_tools: self.product_tools.clone(),
            skills: self
                .skills(&setup, cwd)
                .skills
                .into_iter()
                .map(|skill| skill.name)
                .collect(),
        };
        Ok(RuntimeProfileSnapshot {
            contract_version: RUNTIME_PROFILE_CONTRACT_VERSION,
            configuration: configuration(&setup),
            exposure,
            locked: RuntimeSelections {
                sandbox_mode: Some(SandboxMode::DangerFullAccess),
                ..Default::default()
            },
            provider_options: None,
        })
    }

    fn native_skills_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<ProviderSkillCatalogue, ProviderConfigurationSourceError> {
        Ok(self.skills(&self.setup(reference)?, cwd))
    }

    fn quick_features_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<RuntimeQuickFeatures, ProviderConfigurationSourceError> {
        self.quick_features(reference, cwd, false)
    }

    fn refresh_quick_features_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<RuntimeQuickFeatures, ProviderConfigurationSourceError> {
        self.quick_features(reference, cwd, true)
    }

    fn inventory_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<NativeCapabilityInventory, ProviderConfigurationSourceError> {
        let catalogue = self.skills(&self.setup(reference)?, cwd);
        Ok(NativeCapabilityInventory {
            entries: catalogue
                .skills
                .into_iter()
                .map(|skill| NativeCapabilityEntry {
                    name: skill.name,
                    kind: "skill".into(),
                    origin: skill.scope,
                    state: "enabled".into(),
                    support: "discovered in the Claude setup folder".into(),
                })
                .collect(),
            limitations: catalogue.limitations,
        })
    }

    fn setups(&self) -> Result<Vec<ProviderSetup>, ProviderConfigurationSourceError> {
        self.setups
            .provider_setups()
            .map_err(ProviderConfigurationSourceError::unavailable)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::runtime::providers::claude::setups::test_setups;

    fn model(value: &str, efforts: &[&str]) -> ClaudeModel {
        ClaudeModel {
            value: value.into(),
            display_name: value.to_uppercase(),
            description: String::new(),
            supported_effort_levels: efforts.iter().map(|effort| effort.to_string()).collect(),
        }
    }

    #[test]
    fn profiles_offer_reported_models_efforts_skills_and_full_access_only() {
        let folder = tempfile::tempdir().unwrap();
        std::fs::create_dir_all(folder.path().join("skills/review")).unwrap();
        std::fs::write(folder.path().join("skills/review/SKILL.md"), "Review").unwrap();
        let setups = Arc::new(test_setups(None));
        let setup = setups.add(folder.path(), None).unwrap();
        let reads = Arc::new(Mutex::new(0));
        let counter = reads.clone();
        let source = ClaudeConfigurationSource::new(setups, BTreeMap::new()).with_model_reader(
            Arc::new(move |_, _| {
                *counter.lock().unwrap() += 1;
                Ok(vec![model("sonnet", &["low", "high"]), model("haiku", &[])])
            }),
        );

        let profile = source.profile_for_configuration(&setup.id, None).unwrap();
        profile.validate().unwrap();
        assert_eq!(profile.configuration, ProviderConfigurationRef::new("claude", &setup.id));
        assert_eq!(profile.exposure.models, ["haiku", "sonnet"].map(String::from).into());
        assert_eq!(profile.exposure.reasoning_modes, ["high", "low"].map(String::from).into());
        assert_eq!(profile.exposure.sandbox_modes, [SandboxMode::DangerFullAccess].into());
        assert_eq!(profile.exposure.skills, ["review".to_string()].into());

        let features = source.quick_features_for_configuration(&setup.id, None).unwrap();
        assert_eq!(features.models[0].label, "SONNET");
        assert_eq!(features.models[0].reasoning_modes.len(), 2);
        assert_eq!(features.skills[0].invocation_text, "$review");
        assert_eq!(*reads.lock().unwrap(), 1);
        source.refresh_quick_features_for_configuration(&setup.id, None).unwrap();
        assert_eq!(*reads.lock().unwrap(), 2);
        assert!(source.profile_for_configuration("unknown", None).is_err());
    }
}
