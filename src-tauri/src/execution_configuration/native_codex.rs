//! Selected native environment discovery. Defaults remain absent so Codex resolves them at launch.
use super::{
    ports::{SelectedRuntimeProfileSource, SelectedRuntimeProfileSourceError},
    runtime_profile::RuntimeProfileSnapshot,
};
use crate::{
    native_profiles::NativeProfileService,
    runtime::codex::app_server::environment::{CodexEnvironmentReader, CodexEnvironmentSource},
};
use std::sync::Arc;
mod quick_features;

pub(crate) struct NativeCodexSelectedRuntimeProfileSource {
    service: Arc<NativeProfileService>,
    product_tools: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    reader: Arc<dyn CodexEnvironmentSource>,
}

impl NativeCodexSelectedRuntimeProfileSource {
    pub(crate) fn new(
        service: Arc<NativeProfileService>,
        product_tools: std::collections::BTreeMap<String, std::collections::BTreeSet<String>>,
    ) -> Self {
        Self {
            service,
            product_tools,
            reader: Arc::new(CodexEnvironmentReader::new("codex")),
        }
    }
    pub(crate) fn with_skill_roots(mut self, roots: Vec<String>) -> Self {
        self.reader = Arc::new(CodexEnvironmentReader::new("codex").with_skill_roots(roots));
        self
    }
    #[cfg(test)]
    pub(crate) fn with_reader(mut self, reader: Arc<dyn CodexEnvironmentSource>) -> Self {
        self.reader = reader;
        self
    }
}

impl SelectedRuntimeProfileSource for NativeCodexSelectedRuntimeProfileSource {
    fn quick_features_at(
        &self,
        cwd: Option<&str>,
    ) -> Result<super::RuntimeQuickFeatures, SelectedRuntimeProfileSourceError> {
        self.quick_features_for_configuration("selected", cwd)
    }
    fn quick_features_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<super::RuntimeQuickFeatures, SelectedRuntimeProfileSourceError> {
        let selected = self
            .service
            .resolve_configuration_home(reference)
            .map_err(SelectedRuntimeProfileSourceError::unavailable)?;
        let native = self
            .reader
            .read(selected.home, cwd.map(std::path::PathBuf::from))
            .map_err(|e| SelectedRuntimeProfileSourceError::unavailable(e.to_string()))?;
        Ok(quick_features::project(
            format!("native-codex:{}", selected.profile_id),
            &native,
        ))
    }
    fn resolve_configuration_ref(
        &self,
        reference: &str,
    ) -> Result<String, SelectedRuntimeProfileSourceError> {
        self.service
            .resolve_configuration_home(reference)
            .map(|home| home.profile_id)
            .map_err(SelectedRuntimeProfileSourceError::unavailable)
    }
    fn inventory_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<super::NativeCapabilityInventory, SelectedRuntimeProfileSourceError> {
        let selected = self
            .service
            .resolve_configuration_home(reference)
            .map_err(SelectedRuntimeProfileSourceError::unavailable)?;
        self.reader
            .inventory(selected.home, cwd.map(std::path::PathBuf::from))
            .map_err(|e| SelectedRuntimeProfileSourceError::unavailable(e.to_string()))
    }
    fn native_inventory(
        &self,
    ) -> Result<super::NativeCapabilityInventory, SelectedRuntimeProfileSourceError> {
        let selected = self
            .service
            .resolve_session_home()
            .map_err(SelectedRuntimeProfileSourceError::unavailable)?;
        self.reader
            .inventory(selected.home, None)
            .map_err(|e| SelectedRuntimeProfileSourceError::unavailable(e.to_string()))
    }
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        self.selected_runtime_profile_at(None)
    }
    fn selected_runtime_profile_at(
        &self,
        cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        self.profile_for_configuration("selected", cwd)
    }
    fn profile_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        let selected = self
            .service
            .resolve_configuration_home(reference)
            .map_err(SelectedRuntimeProfileSourceError::unavailable)?;
        let native = self
            .reader
            .read(selected.home, cwd.map(std::path::PathBuf::from))
            .map_err(|e| SelectedRuntimeProfileSourceError::unavailable(e.to_string()))?;
        Ok(orchid_engine::configuration::runtime_profile(
            &native,
            format!("native-codex:{}", selected.profile_id),
            self.product_tools.clone(),
        ))
    }
}
