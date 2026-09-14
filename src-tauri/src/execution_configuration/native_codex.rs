//! Selected native environment discovery. Defaults remain absent so Codex resolves them at launch.
use super::{
    ports::{SelectedRuntimeProfileSource, SelectedRuntimeProfileSourceError},
    runtime_profile::{
        CapabilitySet, RuntimeProfileSnapshot, RuntimeSelections, SandboxMode,
        RUNTIME_PROFILE_CONTRACT_VERSION,
    },
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
        let selected = self
            .service
            .resolve_session_home()
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
        let selected = self
            .service
            .resolve_session_home()
            .map_err(SelectedRuntimeProfileSourceError::unavailable)?;
        let native = self
            .reader
            .read(selected.home, cwd.map(std::path::PathBuf::from))
            .map_err(|e| SelectedRuntimeProfileSourceError::unavailable(e.to_string()))?;
        let mut exposure = CapabilitySet {
            mcp_tools: self.product_tools.clone(),
            sandbox_modes: [
                SandboxMode::ReadOnly,
                SandboxMode::WorkspaceWrite,
                SandboxMode::DangerFullAccess,
            ]
            .into_iter()
            .collect(),
            ..Default::default()
        };
        if let Some(allowed) = native.requirements["requirements"]["allowedSandboxModes"].as_array()
        {
            exposure.sandbox_modes.retain(|mode| {
                allowed.iter().any(|value| {
                    value.as_str()
                        == Some(match mode {
                            SandboxMode::ReadOnly => "read-only",
                            SandboxMode::WorkspaceWrite => "workspace-write",
                            SandboxMode::DangerFullAccess => "danger-full-access",
                        })
                })
            });
        }
        if let Some(models) = native.models.as_array() {
            for model in models {
                if let Some(name) = model["model"].as_str().or(model["id"].as_str()) {
                    exposure.models.insert(name.into());
                }
                if let Some(modes) = model["supportedReasoningEfforts"].as_array() {
                    for mode in modes {
                        if let Some(effort) = mode["reasoningEffort"].as_str() {
                            exposure.reasoning_modes.insert(effort.into());
                        }
                    }
                }
            }
        }
        if let Some(data) = native.skills["data"].as_array() {
            for entry in data {
                if let Some(skills) = entry["skills"].as_array() {
                    for skill in skills {
                        if skill["enabled"] != false {
                            if let Some(name) = skill["name"].as_str() {
                                exposure.skills.insert(name.into());
                            }
                        }
                    }
                }
            }
        }
        Ok(RuntimeProfileSnapshot {
            contract_version: RUNTIME_PROFILE_CONTRACT_VERSION,
            profile_ref: format!("native-codex:{}", selected.profile_id),
            exposure,
            locked: RuntimeSelections::default(),
        })
    }
}
