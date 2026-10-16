use super::*;
use serde::Deserialize;
#[derive(Clone)]
pub(crate) struct OtpInstallationTauriState {
    service: std::sync::Arc<OtpInstallationService>,
}

impl OtpInstallationTauriState {
    pub(crate) fn new(service: std::sync::Arc<OtpInstallationService>) -> Self {
        Self { service }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct SaveJobAgentInstallationInput {
    installation: JobAgentInstallation,
}

#[tauri::command]
pub(crate) fn read_job_agent_otp_installation(
    state: tauri::State<'_, OtpInstallationTauriState>,
) -> Result<JobAgentInstallationStatus, String> {
    state.service.status()
}

#[tauri::command]
pub(crate) fn save_job_agent_otp_installation(
    state: tauri::State<'_, OtpInstallationTauriState>,
    input: SaveJobAgentInstallationInput,
) -> Result<JobAgentInstallationStatus, String> {
    state.service.save_configuration(input.installation)
}
