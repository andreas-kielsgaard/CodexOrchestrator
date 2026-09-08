use super::{CapabilityProfile, CapabilityProfileService, CapabilitySet, RuntimeProfileSnapshot};
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

pub(crate) struct CapabilityProfileTauriState {
    service: Arc<CapabilityProfileService>,
}

impl CapabilityProfileTauriState {
    pub(crate) fn new(service: Arc<CapabilityProfileService>) -> Self {
        Self { service }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CapabilityProfileIdInput {
    capability_profile_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateCapabilityProfileInput {
    capability_profile_id: String,
    name: String,
    allowed_capabilities: CapabilitySet,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateCapabilityProfileInput {
    capability_profile_id: String,
    name: String,
    allowed_capabilities: CapabilitySet,
}

#[tauri::command]
pub(crate) fn load_selected_runtime_profile(
    state: State<'_, CapabilityProfileTauriState>,
) -> Result<RuntimeProfileSnapshot, String> {
    state
        .service
        .runtime_profile()
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn list_capability_profiles(
    state: State<'_, CapabilityProfileTauriState>,
) -> Result<Vec<CapabilityProfile>, String> {
    state.service.list().map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn load_capability_profile(
    state: State<'_, CapabilityProfileTauriState>,
    input: CapabilityProfileIdInput,
) -> Result<CapabilityProfile, String> {
    state
        .service
        .read(&input.capability_profile_id)
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn create_capability_profile(
    state: State<'_, CapabilityProfileTauriState>,
    input: CreateCapabilityProfileInput,
) -> Result<CapabilityProfile, String> {
    state
        .service
        .create(
            input.capability_profile_id,
            input.name,
            input.allowed_capabilities,
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn update_capability_profile(
    state: State<'_, CapabilityProfileTauriState>,
    input: UpdateCapabilityProfileInput,
) -> Result<CapabilityProfile, String> {
    state
        .service
        .update(
            &input.capability_profile_id,
            input.name,
            input.allowed_capabilities,
        )
        .map_err(|error| error.to_string())
}

#[tauri::command]
pub(crate) fn delete_capability_profile(
    state: State<'_, CapabilityProfileTauriState>,
    input: CapabilityProfileIdInput,
) -> Result<(), String> {
    state
        .service
        .delete(&input.capability_profile_id)
        .map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_inputs_reject_legacy_harness_fields() {
        assert!(
            serde_json::from_value::<CreateCapabilityProfileInput>(serde_json::json!({
                "capabilityProfileId": "capability-review",
                "name": "Review",
                "allowedCapabilities": {
                    "models": [],
                    "reasoningModes": [],
                    "sandboxModes": [],
                    "mcpTools": {},
                    "skills": []
                },
                "promptPrefix": "You are a reviewer"
            }))
            .is_err()
        );
    }
}
