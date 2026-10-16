use super::*;
use orchid_engine::providers::codex::options::CodexPersonality;
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;
pub(crate) struct NativeProfileTauriState {
    service: Arc<NativeProfileService>,
}
impl NativeProfileTauriState {
    pub(crate) fn new(service: Arc<NativeProfileService>) -> Self {
        Self { service }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct RegisterNativeProfileInput {
    home_path: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NativeProfileIdInput {
    profile_id: String,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NativeExecutionModeInput {
    profile_id: String,
    mode: ExecutionMode,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct NativePersonalityInput {
    profile_id: String,
    personality: Option<CodexPersonality>,
}

#[tauri::command]
pub(crate) fn load_native_profile_query(
    state: State<'_, NativeProfileTauriState>,
) -> Result<NativeProfileQueryDto, String> {
    state.service.query()
}

#[tauri::command]
pub(crate) fn discover_native_codex_homes(
    state: State<'_, NativeProfileTauriState>,
) -> Result<Vec<DiscoveredNativeCodexHome>, String> {
    state.service.discover_homes()
}
#[tauri::command]
pub(crate) fn register_native_profile(
    state: State<'_, NativeProfileTauriState>,
    input: RegisterNativeProfileInput,
) -> Result<NativeProfileDto, String> {
    state.service.register_existing(&input.home_path)
}
#[tauri::command]
pub(crate) fn create_dedicated_native_profile(
    state: State<'_, NativeProfileTauriState>,
) -> Result<NativeProfileDto, String> {
    state.service.create_dedicated()
}
#[tauri::command]
pub(crate) fn select_native_profile(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.select(&input.profile_id)
}
#[tauri::command]
pub(crate) fn select_native_profile_execution_mode(
    state: State<'_, NativeProfileTauriState>,
    input: NativeExecutionModeInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .select_execution_mode(&input.profile_id, input.mode)
}
#[tauri::command]
pub(crate) fn set_native_profile_personality(
    state: State<'_, NativeProfileTauriState>,
    input: NativePersonalityInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .set_personality(&input.profile_id, input.personality)
}
#[tauri::command]
pub(crate) fn authorize_native_profile_danger_full_access(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .authorize_danger_full_access(&input.profile_id)
}
#[tauri::command]
pub(crate) fn revoke_native_profile_danger_full_access(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.revoke_danger_full_access(&input.profile_id)
}
#[tauri::command]
pub(crate) fn request_native_profile_login(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.request_login(&input.profile_id)
}
#[tauri::command]
pub(crate) fn refresh_native_profile_readiness(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.refresh_readiness(&input.profile_id)
}

#[tauri::command]
pub(crate) fn open_native_profile_in_explorer(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<(), String> {
    state.service.open_in_explorer(&input.profile_id)
}
#[tauri::command]
pub(crate) fn request_native_profile_sandbox_initialization(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .request_sandbox_initialization(&input.profile_id)
}
#[tauri::command]
pub(crate) fn confirm_native_profile_sandbox_initialization(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .confirm_sandbox_initialization(&input.profile_id)
}
#[tauri::command]
pub(crate) fn verify_native_profile_preprovisioned_sandbox(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .verify_preprovisioned_sandbox(&input.profile_id)
}
#[tauri::command]
pub(crate) fn confirm_native_profile_preprovisioned_sandbox_adoption(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .confirm_preprovisioned_sandbox_adoption(&input.profile_id)
}
#[tauri::command]
pub(crate) fn run_native_profile_workspace_write_canary(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.run_workspace_write_canary(&input.profile_id)
}
#[tauri::command]
pub(crate) fn run_native_profile_danger_full_access_canary(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .run_danger_full_access_canary(&input.profile_id)
}
#[tauri::command]
pub(crate) fn probe_native_profile_mcp_reporting(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state.service.probe_mcp_reporting(&input.profile_id)
}

#[tauri::command]
pub(crate) fn reconcile_native_profile_mcp_reporting(
    state: State<'_, NativeProfileTauriState>,
    input: NativeProfileIdInput,
) -> Result<NativeProfileDto, String> {
    state
        .service
        .reconcile_pending_mcp_reporting(&input.profile_id)
}
