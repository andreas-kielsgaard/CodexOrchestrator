//! Selected native environment discovery. Defaults remain absent so Codex resolves them at launch.
use super::{
    ports::{RuntimeSkillRoot, SelectedRuntimeProfileSource, SelectedRuntimeProfileSourceError},
    runtime_profile::{RuntimeProfileSnapshot, SandboxMode},
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
    orchid_skill_roots: Vec<std::path::PathBuf>,
    otp_skill_roots: std::collections::BTreeMap<String, Vec<std::path::PathBuf>>,
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
            orchid_skill_roots: Vec::new(),
            otp_skill_roots: Default::default(),
        }
    }
    pub(crate) fn with_skill_roots(mut self, roots: Vec<String>) -> Self {
        self.orchid_skill_roots = roots.iter().map(std::path::PathBuf::from).collect();
        self.refresh_skill_reader();
        self
    }
    pub(crate) fn with_otp_skill_roots(
        mut self,
        roots: std::collections::BTreeMap<String, Vec<String>>,
    ) -> Self {
        self.otp_skill_roots = roots
            .into_iter()
            .map(|(package, paths)| (package, paths.into_iter().map(Into::into).collect()))
            .collect();
        self.refresh_skill_reader();
        self
    }
    fn refresh_skill_reader(&mut self) {
        let paths = self
            .orchid_skill_roots
            .iter()
            .chain(self.otp_skill_roots.values().flatten())
            .map(|path| path.to_string_lossy().into_owned())
            .collect();
        self.reader = Arc::new(CodexEnvironmentReader::new("codex").with_skill_roots(paths));
    }
    #[cfg(test)]
    pub(crate) fn with_reader(mut self, reader: Arc<dyn CodexEnvironmentSource>) -> Self {
        self.reader = reader;
        self
    }
}

impl SelectedRuntimeProfileSource for NativeCodexSelectedRuntimeProfileSource {
    fn configuration_ref_for_session(
        &self,
        session_id: &str,
    ) -> Result<Option<String>, SelectedRuntimeProfileSourceError> {
        self.service
            .bound_profile_id(session_id)
            .map_err(SelectedRuntimeProfileSourceError::unavailable)
    }
    fn discover_skills_for_configuration(
        &self,
        reference: &str,
        cwd: Option<&str>,
    ) -> Result<
        crate::runtime::codex::app_server::skills::CodexSkillCatalogue,
        SelectedRuntimeProfileSourceError,
    > {
        let selected = self
            .service
            .resolve_configuration_home(reference)
            .map_err(SelectedRuntimeProfileSourceError::unavailable)?;
        self.reader
            .discover_skills(selected.home, cwd.map(Into::into))
            .map_err(|error| SelectedRuntimeProfileSourceError::unavailable(error.to_string()))
    }
    fn skill_roots_for_configuration(
        &self,
        reference: &str,
    ) -> Result<Vec<RuntimeSkillRoot>, SelectedRuntimeProfileSourceError> {
        self.service
            .resolve_configuration_home(reference)
            .map_err(SelectedRuntimeProfileSourceError::unavailable)?;
        let mut roots = Vec::new();
        roots.extend(
            self.orchid_skill_roots
                .iter()
                .cloned()
                .map(|path| RuntimeSkillRoot {
                    group_id: "orchid-skills".into(),
                    path,
                }),
        );
        for (package, paths) in &self.otp_skill_roots {
            roots.extend(paths.iter().cloned().map(|path| RuntimeSkillRoot {
                group_id: format!("otp:{package}:skills"),
                path,
            }));
        }
        Ok(roots)
    }
    fn configuration_home(
        &self,
        reference: &str,
    ) -> Result<std::path::PathBuf, SelectedRuntimeProfileSourceError> {
        self.service
            .resolve_configuration_home(reference)
            .map(|home| home.home)
            .map_err(SelectedRuntimeProfileSourceError::unavailable)
    }
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
        let cwd_path = cwd.map(std::path::PathBuf::from);
        let profile_ref = format!("native-codex:{}", selected.profile_id);
        let mut features = match self.reader.read(selected.home.clone(), cwd_path.clone()) {
            Ok(native) => quick_features::project(profile_ref, &native),
            Err(model_error) => {
                let catalogue = self
                    .reader
                    .discover_skills(selected.home, cwd_path)
                    .map_err(|error| {
                        SelectedRuntimeProfileSourceError::unavailable(error.to_string())
                    })?;
                let mut fallback = super::RuntimeQuickFeatures {
                    profile_ref,
                    ..Default::default()
                };
                fallback.limitations.push(format!(
                    "Model/configuration discovery is unavailable: {model_error}"
                ));
                quick_features::append_codex_skills(&mut fallback, &catalogue);
                fallback
            }
        };
        quick_features::append_owned_root_skills(
            &mut features,
            &self.orchid_skill_roots,
            "orchid-skills",
        );
        for (package, paths) in &self.otp_skill_roots {
            quick_features::append_owned_root_skills(
                &mut features,
                paths,
                &format!("otp:{package}:skills"),
            );
        }
        Ok(features)
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
        Ok(with_temporary_danger_full_access(
            orchid_engine::configuration::runtime_profile(
                &native,
                format!("native-codex:{}", selected.profile_id),
                self.product_tools.clone(),
            ),
        ))
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
