use super::{LauncherDetailNavigationView, LauncherProofPresentationView, ProofTauriState};
use tauri::State;

#[tauri::command]
pub(crate) fn human_review_launcher_proof_navigation(
    state: State<'_, ProofTauriState>,
) -> Result<Option<String>, String> {
    state.0.launcher_navigation()
}

#[tauri::command]
pub(crate) fn human_review_launcher_detail_navigation(
    state: State<'_, ProofTauriState>,
) -> Result<Option<LauncherDetailNavigationView>, String> {
    state.0.launcher_detail_navigation()
}

#[tauri::command]
pub(crate) fn human_review_launcher_proof_presentation(
    state: State<'_, ProofTauriState>,
) -> Result<Option<LauncherProofPresentationView>, String> {
    state.0.launcher_presentation()
}
