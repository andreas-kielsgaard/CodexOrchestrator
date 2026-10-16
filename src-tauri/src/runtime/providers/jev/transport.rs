use super::{credential_status, save_credential, remove_credential, CredentialStatus};

#[tauri::command]
pub(crate) fn load_jev_credential_status() -> Result<CredentialStatus, String> {
    credential_status()
}

#[tauri::command]
pub(crate) fn save_jev_api_key(key: String) -> Result<CredentialStatus, String> {
    save_credential(&key)
}

#[tauri::command]
pub(crate) fn remove_jev_api_key() -> Result<CredentialStatus, String> {
    remove_credential()
}
