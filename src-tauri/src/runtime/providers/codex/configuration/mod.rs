//! Selected native environment discovery. Defaults remain absent so Codex resolves them at launch.
use crate::execution_configuration::{
    RuntimeProfileSnapshot, SandboxMode, ProviderConfigurationSource,
    ProviderConfigurationSourceError,
};
use orchid_engine::contracts::ProviderConfigurationRef;
use orchid_engine::providers::codex::{
    options::{CodexNativeOptions, PROVIDER as CODEX},
    runtime_profile,
};
use crate::runtime::providers::codex::{
    app_server::environment::{CodexEnvironmentReader, CodexEnvironmentSource},
    profiles::NativeProfileService,
};
use std::sync::{Arc, Mutex};
mod quick_features;

pub(crate) struct CodexConfigurationSource {
    service: Arc<NativeProfileService>,
    product_tools: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    reader: Arc<dyn CodexEnvironmentSource>,
    quick_feature_cache:
        Mutex<std::collections::BTreeMap<String, crate::execution_configuration::RuntimeQuickFeatures>>,
}

impl CodexConfigurationSource {
    pub(crate) fn new(
        service: Arc<NativeProfileService>,
        product_tools: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    ) -> Self {
        Self {
            service,
            product_tools,
            reader: Arc::new(CodexEnvironmentReader::new("codex")),
            quick_feature_cache: Mutex::new(Default::default()),
        }
    }
    /// Orchid-owned roots are offered by Orchid itself. Codex receives them only so its native
    /// inventory can report which discovered entries they own.
    pub(crate) fn with_product_skill_roots(
        mut self,
        roots: &crate::execution_configuration::ProductSkillRoots,
    ) -> Self {
        let paths = roots
            .roots()
            .iter()
            .map(|root| root.path.to_string_lossy().into_owned())
            .collect();
        self.reader = Arc::new(CodexEnvironmentReader::new("codex").with_skill_roots(paths));
        if let Ok(cache) = self.quick_feature_cache.get_mut() {
            cache.clear();
        }
        self
    }
    #[cfg(test)]
    pub(crate) fn with_reader(mut self, reader: Arc<dyn CodexEnvironmentSource>) -> Self {
        self.reader = reader;
        self
    }

    fn read_quick_features(
        &self,
        reference: &str,
        cwd: Option<&str>,
        refresh: bool,
    ) -> Result<crate::execution_configuration::RuntimeQuickFeatures, ProviderConfigurationSourceError> {
        let selected = self
            .service
            .resolve_configuration_home(reference)
            .map_err(ProviderConfigurationSourceError::unavailable)?;
        let cache_key = format!(
            "{}\0{}",
            selected.profile_id,
            cwd.unwrap_or("").replace('\\', "/").to_lowercase()
        );
        // Keep the context bucket locked through discovery so concurrent callers coalesce onto
        // the first observation instead of spawning parallel Codex app-server probes.
        let mut cache = self.quick_feature_cache.lock().map_err(|_| {
            ProviderConfigurationSourceError::unavailable("Quick-feature cache is unavailable")
        })?;
        if !refresh {
            if let Some(cached) = cache.get(&cache_key).cloned() {
                return Ok(cached);
            }
        }
        let cwd_path = cwd.map(std::path::PathBuf::from);
        let configuration = codex_configuration(&selected.profile_id);
        let features = match self.reader.read(selected.home.clone(), cwd_path.clone()) {
            Ok(native) => quick_features::project(configuration, &native),
            Err(model_error) => {
                let catalogue = self
                    .reader
                    .discover_skills(selected.home, cwd_path)
                    .map_err(|error| {
                        ProviderConfigurationSourceError::unavailable(error.to_string())
                    })?;
                let mut fallback = crate::execution_configuration::RuntimeQuickFeatures {
                    configuration: Some(configuration),
                    ..Default::default()
                };
                fallback.limitations.push(format!(
                    "Model/configuration discovery is unavailable: {model_error}"
                ));
                fallback.append_native_skills(&catalogue);
                fallback
            }
        };
        cache.insert(cache_key, features.clone());
        Ok(features)
    }

    /// The Codex home a configuration resolves to. Only Codex continuation uses it.
    pub(crate) fn configuration_home(
        &self,
        reference: &str,
    ) -> Result<std::path::PathBuf, ProviderConfigurationSourceError> {
        self.service
            .resolve_configuration_home(reference)
            .map(|home| home.home)
            .map_err(ProviderConfigurationSourceError::unavailable)
    }
}

fn codex_configuration(profile_id: &str) -> ProviderConfigurationRef {
    ProviderConfigurationRef::new(CODEX, profile_id)
}

impl ProviderConfigurationSource for CodexConfigurationSource {
    fn configuration_ref_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, ProviderConfigurationSourceError> {
        self.service
            .bound_profile_id(session_id)
            .map_err(ProviderConfigurationSourceError::unavailable)
    }
    fn native_skills_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<orchid_engine::contracts::ProviderSkillCatalogue, ProviderConfigurationSourceError>
    {
        let selected = self
            .service
            .resolve_configuration_home(reference)
            .map_err(ProviderConfigurationSourceError::unavailable)?;
        self.reader
            .discover_skills(selected.home, cwd.map(Into::into))
            .map_err(|error| ProviderConfigurationSourceError::unavailable(error.to_string()))
    }
    fn quick_features_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<crate::execution_configuration::RuntimeQuickFeatures, ProviderConfigurationSourceError> {
        self.read_quick_features(reference, cwd, false)
    }
    fn refresh_quick_features_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<crate::execution_configuration::RuntimeQuickFeatures, ProviderConfigurationSourceError> {
        self.read_quick_features(reference, cwd, true)
    }
    fn setups(
        &self,
    ) -> Result<Vec<crate::execution_configuration::ProviderSetup>, ProviderConfigurationSourceError>
    {
        self.service
            .provider_setups()
            .map_err(ProviderConfigurationSourceError::unavailable)
    }
    fn resolve_configuration_ref(
        &self,
        reference: &str,
    ) -> Result<String, ProviderConfigurationSourceError> {
        self.service
            .resolve_configuration_home(reference)
            .map(|home| home.profile_id)
            .map_err(ProviderConfigurationSourceError::unavailable)
    }
    fn inventory_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<crate::execution_configuration::NativeCapabilityInventory, ProviderConfigurationSourceError> {
        let selected = self
            .service
            .resolve_configuration_home(reference)
            .map_err(ProviderConfigurationSourceError::unavailable)?;
        self.reader
            .inventory(selected.home, cwd.map(std::path::PathBuf::from))
            .map_err(|e| ProviderConfigurationSourceError::unavailable(e.to_string()))
    }
    fn profile_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, ProviderConfigurationSourceError> {
        let selected = self
            .service
            .resolve_configuration_home(reference)
            .map_err(ProviderConfigurationSourceError::unavailable)?;
        let native = self
            .reader
            .read(selected.home, cwd.map(std::path::PathBuf::from))
            .map_err(|e| ProviderConfigurationSourceError::unavailable(e.to_string()))?;
        let quick = quick_features::project(codex_configuration(&selected.profile_id), &native);
        let cache_key = format!(
            "{}\0{}",
            selected.profile_id,
            cwd.unwrap_or("").replace('\\', "/").to_lowercase()
        );
        self.quick_feature_cache
            .lock()
            .map_err(|_| {
                ProviderConfigurationSourceError::unavailable("Quick-feature cache is unavailable")
            })?
            .insert(cache_key, quick);
        let mut profile = runtime_profile::runtime_profile(
            &native,
            codex_configuration(&selected.profile_id),
            self.product_tools.clone(),
        );
        profile.provider_options = self
            .service
            .profile_personality(&selected.profile_id)
            .map_err(ProviderConfigurationSourceError::unavailable)?
            .map(|personality| {
                CodexNativeOptions {
                    personality: Some(personality),
                }
                .encode()
            });
        Ok(with_temporary_danger_full_access(profile))
    }
}

/// Sandboxing moves to Capability Profiles in a later slice. Until then, Orchid launches its
/// native Codex harnesses with one explicit full-access policy rather than pretending this is a
/// CODEX_HOME setting.
fn with_temporary_danger_full_access(
    mut profile: RuntimeProfileSnapshot,
) -> RuntimeProfileSnapshot {
    profile.exposure.sandbox_modes = [SandboxMode::DangerFullAccess].into_iter().collect();
    profile.locked.sandbox_mode = Some(SandboxMode::DangerFullAccess);
    profile
}
