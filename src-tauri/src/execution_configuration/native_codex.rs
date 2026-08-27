use super::{
    ports::{SelectedRuntimeProfileSource, SelectedRuntimeProfileSourceError},
    runtime_profile::{
        CapabilitySet, RuntimeProfileSnapshot, RuntimeSelections, SandboxMode,
        RUNTIME_PROFILE_CONTRACT_VERSION,
    },
};
use crate::native_profiles::{ExecutionMode, NativeProfileService};
use serde::{Deserialize, Serialize};
use std::sync::Arc;

/// Temporary advertised exposure for the foundation slice.
///
/// The selected native profile currently supplies only its opaque identity and execution mode.
/// Model, reasoning, MCP, and skill values are injected fixtures until their provider contracts
/// are designed; this adapter does not claim that the Codex CLI reported or enforces them.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NativeCodexCapabilityExposure {
    pub(crate) capabilities: CapabilitySet,
    pub(crate) locked: RuntimeSelections,
}

pub(crate) struct NativeCodexSelectedRuntimeProfileSource {
    service: Arc<NativeProfileService>,
    foundation_exposure: NativeCodexCapabilityExposure,
}

impl NativeCodexSelectedRuntimeProfileSource {
    pub(crate) fn new(
        service: Arc<NativeProfileService>,
        foundation_exposure: NativeCodexCapabilityExposure,
    ) -> Self {
        Self {
            service,
            foundation_exposure,
        }
    }
}

impl SelectedRuntimeProfileSource for NativeCodexSelectedRuntimeProfileSource {
    fn selected_runtime_profile(
        &self,
    ) -> Result<RuntimeProfileSnapshot, SelectedRuntimeProfileSourceError> {
        let selected = self
            .service
            .resolve_selected_home()
            .map_err(SelectedRuntimeProfileSourceError::unavailable)?;
        Ok(snapshot_for_selected_profile(
            selected.profile_id,
            selected.execution_mode,
            self.foundation_exposure.clone(),
        ))
    }
}

fn snapshot_for_selected_profile(
    profile_id: String,
    execution_mode: ExecutionMode,
    foundation_exposure: NativeCodexCapabilityExposure,
) -> RuntimeProfileSnapshot {
    let sandbox_mode = match execution_mode {
        ExecutionMode::WorkspaceWrite => SandboxMode::WorkspaceWrite,
        ExecutionMode::DangerFullAccess => SandboxMode::DangerFullAccess,
    };
    let mut exposure = foundation_exposure.capabilities;
    exposure.sandbox_modes = [sandbox_mode].into_iter().collect();
    let mut locked = foundation_exposure.locked;
    locked.sandbox_mode = Some(sandbox_mode);
    RuntimeProfileSnapshot {
        contract_version: RUNTIME_PROFILE_CONTRACT_VERSION,
        profile_ref: format!("native-codex:{profile_id}"),
        exposure,
        locked,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn selected_native_profile_locks_the_foundation_to_its_execution_mode() {
        let foundation_exposure = NativeCodexCapabilityExposure {
            capabilities: CapabilitySet {
                sandbox_modes: [SandboxMode::DangerFullAccess].into_iter().collect(),
                ..CapabilitySet::default()
            },
            locked: RuntimeSelections {
                sandbox_mode: Some(SandboxMode::DangerFullAccess),
                ..RuntimeSelections::default()
            },
        };
        let snapshot = snapshot_for_selected_profile(
            "selected".into(),
            ExecutionMode::WorkspaceWrite,
            foundation_exposure,
        );

        assert_eq!(snapshot.profile_ref, "native-codex:selected");
        assert_eq!(
            snapshot.exposure.sandbox_modes,
            [SandboxMode::WorkspaceWrite].into_iter().collect()
        );
        assert_eq!(
            snapshot.locked.sandbox_mode,
            Some(SandboxMode::WorkspaceWrite)
        );
    }
}
