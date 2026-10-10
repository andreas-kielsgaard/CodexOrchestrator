use super::credentials::{self, CredentialStatus};

#[tauri::command]
pub(crate) fn load_jev_credential_status() -> Result<CredentialStatus, String> {
    credentials::status()
}

#[tauri::command]
pub(crate) fn save_jev_api_key(key: String) -> Result<CredentialStatus, String> {
    credentials::save(&key)
}

#[tauri::command]
pub(crate) fn remove_jev_api_key() -> Result<CredentialStatus, String> {
    credentials::remove()
}
