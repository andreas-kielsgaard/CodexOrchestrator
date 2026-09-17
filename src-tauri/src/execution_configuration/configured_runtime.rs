use super::runtime_profile::{
    CapabilitySet, RuntimeProfileSnapshot, RuntimeSelections, SandboxMode,
    RUNTIME_PROFILE_CONTRACT_VERSION,
};

/// Product-declared execution capabilities used to author and resolve Session Profiles.
///
/// This is deliberately independent of a currently usable Codex home. Native-profile
/// resolution remains a launch concern, after a Session has been created.
pub(crate) fn configured_runtime_profile(
    mut capabilities: CapabilitySet,
) -> RuntimeProfileSnapshot {
    capabilities.sandbox_modes = [SandboxMode::WorkspaceWrite].into_iter().collect();
    RuntimeProfileSnapshot {
        contract_version: RUNTIME_PROFILE_CONTRACT_VERSION,
        profile_ref: "orchestration:configured-runtime/v1".into(),
        exposure: capabilities,
        locked: RuntimeSelections {
            sandbox_mode: Some(SandboxMode::WorkspaceWrite),
            ..RuntimeSelections::default()
        },
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn declares_workspace_write_without_reading_a_native_profile() {
        let snapshot = configured_runtime_profile(CapabilitySet::default());

        assert_eq!(snapshot.profile_ref, "orchestration:configured-runtime/v1");
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
