use super::{
    ports::{SelectedRuntimeProfileSource, SelectedRuntimeProfileSourceError},
    runtime_profile::{
        CapabilitySet, RuntimeProfileSnapshot, RuntimeSelections, SandboxMode,
        RUNTIME_PROFILE_CONTRACT_VERSION,
    },
};

/// Declared capability configuration used while authoring profiles and Workflows.
/// It contains no provider or MCP reachability claim.
pub(crate) fn configured_runtime_profile(
    mut capabilities: CapabilitySet,
) -> RuntimeProfileSnapshot {
    capabilities.sandbox_modes = [SandboxMode::DangerFullAccess].into_iter().collect();
    RuntimeProfileSnapshot {
        contract_version: RUNTIME_PROFILE_CONTRACT_VERSION,
        profile_ref: "orchestration:configured-runtime/v1".into(),
        exposure: capabilities,
        locked: RuntimeSelections {
            sandbox_mode: Some(SandboxMode::DangerFullAccess),
            ..RuntimeSelections::default()
        },
    }
}

pub(crate) struct ConfiguredRuntimeProfileSource {
    profile: RuntimeProfileSnapshot,
}

impl ConfiguredRuntimeProfileSource {
    pub(crate) fn new(profile: RuntimeProfileSnapshot) -> Self {
        Self { profile }
    }
}

impl SelectedRuntimeProfileSource for ConfiguredRuntimeProfileSource {
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        Ok(self.profile.clone())
    }
}
