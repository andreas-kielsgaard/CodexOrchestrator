use super::{
    CapabilityProfile, CapabilityProfileService, CapabilitySet, ProfileRoutePolicy,
    RuntimeProfileSnapshot,
};
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
pub(crate) struct NativeProfileInventoryInput {
    configuration: orchid_engine::contracts::ProviderConfigurationRef,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NativeProfileSkillInput {
    configuration: orchid_engine::contracts::ProviderConfigurationRef,
    #[serde(default)]
    working_directory: Option<String>,
}

#[tauri::command]
pub(crate) async fn load_native_profile_skills(
    state: State<'_, CapabilityProfileTauriState>,
    input: NativeProfileSkillInput,
) -> Result<orchid_engine::contracts::ProviderSkillCatalogue, String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .native_skills_for_configuration(
                &input.configuration,
                input.working_directory.as_deref(),
            )
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct ModelCatalogueInput {
    route: crate::execution_targets::domain::ExecutionRouteRef,
}

#[tauri::command]
pub(crate) async fn load_profile_model_catalogue(
    state: State<'_, CapabilityProfileTauriState>,
    input: ModelCatalogueInput,
) -> Result<super::ModelCatalogueView, String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .model_catalogue(&input.route)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateCapabilityProfileInput {
    #[serde(default)]
    execution: crate::execution_targets::domain::ExecutionBinding,
    name: String,
    allowed_capabilities: CapabilitySet,
    #[serde(default)]
    defaults: super::RuntimeSelections,
    #[serde(default)]
    route_policies: Vec<ProfileRoutePolicy>,
    #[serde(default)]
    default_route_id: Option<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct UpdateCapabilityProfileInput {
    #[serde(default)]
    execution: crate::execution_targets::domain::ExecutionBinding,
    capability_profile_id: String,
    name: String,
    allowed_capabilities: CapabilitySet,
    #[serde(default)]
    defaults: super::RuntimeSelections,
    #[serde(default)]
    route_policies: Vec<ProfileRoutePolicy>,
    #[serde(default)]
    default_route_id: Option<String>,
}

#[tauri::command]
pub(crate) async fn load_native_profile_capability_inventory(
    state: State<'_, CapabilityProfileTauriState>,
    input: NativeProfileInventoryInput,
) -> Result<super::NativeCapabilityInventory, String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .native_inventory_for_configuration(&input.configuration)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn list_provider_setups(
    state: State<'_, CapabilityProfileTauriState>,
) -> Result<Vec<super::ProviderSetup>, String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.provider_setups().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn load_selected_runtime_profile(
    state: State<'_, CapabilityProfileTauriState>,
) -> Result<RuntimeProfileSnapshot, String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.runtime_profile().map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn list_capability_profiles(
    state: State<'_, CapabilityProfileTauriState>,
) -> Result<Vec<CapabilityProfile>, String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || service.list().map_err(|error| error.to_string()))
        .await
        .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn load_default_capability_profile(
    state: State<'_, CapabilityProfileTauriState>,
) -> Result<Option<String>, String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service.default_profile_id().map_err(|e| e.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn set_default_capability_profile(
    state: State<'_, CapabilityProfileTauriState>,
    input: CapabilityProfileIdInput,
) -> Result<(), String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .set_default_profile(&input.capability_profile_id)
            .map_err(|e| e.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn load_capability_profile(
    state: State<'_, CapabilityProfileTauriState>,
    input: CapabilityProfileIdInput,
) -> Result<CapabilityProfile, String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .read(&input.capability_profile_id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn create_capability_profile(
    state: State<'_, CapabilityProfileTauriState>,
    input: CreateCapabilityProfileInput,
) -> Result<CapabilityProfile, String> {
    let service = state.service.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .create_generated_with_routes(
                input.name,
                input.allowed_capabilities,
                input.defaults,
                input.execution,
                input.route_policies,
                input.default_route_id,
            )
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())?
}

#[tauri::command]
pub(crate) async fn update_capability_profile(
    state: State<'_, CapabilityProfileTauriState>,
    synchronization:State<'_,crate::device_connection::DefinitionSyncState>,
    input: UpdateCapabilityProfileInput,
) -> Result<CapabilityProfile, String> {
    let service = state.service.clone();
    let result=tauri::async_runtime::spawn_blocking(move || {
        service
            .update_with_routes(
                &input.capability_profile_id,
                input.name,
                input.allowed_capabilities,
                input.defaults,
                input.execution,
                input.route_policies,
                input.default_route_id,
            )
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;
    synchronization.changed("capability_profile",&result.capability_profile_id,serde_json::to_value(&result).map_err(|e|e.to_string())?);
    Ok(result)
}

#[tauri::command]
pub(crate) async fn delete_capability_profile(
    state: State<'_, CapabilityProfileTauriState>,
    synchronization:State<'_,crate::device_connection::DefinitionSyncState>,
    input: CapabilityProfileIdInput,
) -> Result<(), String> {
    let service = state.service.clone();
    let id=input.capability_profile_id.clone();
    tauri::async_runtime::spawn_blocking(move || {
        service
            .delete(&input.capability_profile_id)
            .map_err(|error| error.to_string())
    })
    .await
    .map_err(|error| error.to_string())??;
    synchronization.changed("capability_profile",&id,serde_json::Value::Null);Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn profile_inputs_reject_legacy_harness_fields() {
        assert!(
            serde_json::from_value::<CreateCapabilityProfileInput>(serde_json::json!({
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
