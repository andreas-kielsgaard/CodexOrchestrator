use super::OtpRegistry;
use std::sync::Arc;

pub(crate) struct OtpCatalogueTauriState {
    registry: Arc<OtpRegistry>,
}

impl OtpCatalogueTauriState {
    pub(crate) fn new(registry: Arc<OtpRegistry>) -> Self {
        Self { registry }
    }
}

#[tauri::command]
pub(crate) fn list_otp_catalogue(
    state: tauri::State<'_, OtpCatalogueTauriState>,
) -> Vec<crate::otp_api::PackageDescriptor> {
    state.registry.catalogue()
}
