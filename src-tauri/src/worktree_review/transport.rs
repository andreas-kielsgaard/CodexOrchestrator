use super::{
    comparison::WorktreeComparisonView,
    detail::ReviewInstanceDetailView,
    progress::ReviewOperationProgressView,
    service::{
        HumanReviewLauncherService, LauncherDetailNavigationView, LauncherProofPresentationView,
        ReviewInstanceView, ReviewSourceView,
    },
};
use serde::Deserialize;
use std::sync::Arc;
use tauri::State;

pub(crate) struct HumanReviewLauncherTauriState(Arc<HumanReviewLauncherService>);

impl HumanReviewLauncherTauriState {
    pub(crate) fn new(service: Arc<HumanReviewLauncherService>) -> Self {
        Self(service)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PrepareReviewInput {
    operation_ref: String,
    source_ref: String,
    name: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewInstanceInput {
    instance_ref: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewOperationInput {
    operation_ref: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewInstanceOperationInput {
    operation_ref: String,
    instance_ref: String,
}

#[tauri::command]
pub(crate) fn list_human_review_worktrees(
    state: State<'_, HumanReviewLauncherTauriState>,
) -> Vec<ReviewSourceView> {
    state.0.sources()
}

#[tauri::command]
pub(crate) fn list_human_review_instances(
    state: State<'_, HumanReviewLauncherTauriState>,
) -> Vec<ReviewInstanceView> {
    state.0.instances()
}

#[tauri::command]
pub(crate) async fn prepare_human_review_instance(
    state: State<'_, HumanReviewLauncherTauriState>,
    input: PrepareReviewInput,
) -> Result<ReviewInstanceView, String> {
    run(state.0.clone(), move |service| {
        service.prepare(input.operation_ref, input.source_ref, input.name)
    })
    .await
}

#[tauri::command]
pub(crate) fn human_review_operation_progress(
    state: State<'_, HumanReviewLauncherTauriState>,
    input: ReviewOperationInput,
) -> Result<ReviewOperationProgressView, String> {
    state.0.operation_progress(input.operation_ref)
}

#[tauri::command]
pub(crate) fn list_human_review_operation_progress(
    state: State<'_, HumanReviewLauncherTauriState>,
) -> Vec<ReviewOperationProgressView> {
    state.0.operations()
}

#[tauri::command]
pub(crate) fn human_review_instance_detail(
    state: State<'_, HumanReviewLauncherTauriState>,
    input: ReviewInstanceInput,
) -> Result<ReviewInstanceDetailView, String> {
    state.0.detail(input.instance_ref)
}

#[tauri::command]
pub(crate) fn human_review_instance_comparison(
    state: State<'_, HumanReviewLauncherTauriState>,
    input: ReviewInstanceInput,
) -> Result<WorktreeComparisonView, String> {
    state.0.comparison(input.instance_ref)
}

#[tauri::command]
pub(crate) fn human_review_launcher_proof_navigation(
    state: State<'_, HumanReviewLauncherTauriState>,
) -> Result<Option<String>, String> {
    state.0.launcher_proof_navigation()
}

#[tauri::command]
pub(crate) fn human_review_launcher_detail_navigation(
    state: State<'_, HumanReviewLauncherTauriState>,
) -> Result<Option<LauncherDetailNavigationView>, String> {
    state.0.launcher_detail_navigation()
}

#[tauri::command]
pub(crate) fn human_review_launcher_proof_presentation(
    state: State<'_, HumanReviewLauncherTauriState>,
) -> Result<Option<LauncherProofPresentationView>, String> {
    state.0.launcher_proof_presentation()
}

#[tauri::command]
pub(crate) fn mark_worktree_build_ready(window: tauri::Window) -> Result<(), String> {
    let name = std::env::var("CODEX_ORCHESTRATOR_WORKTREE_BUILD_NAME")
        .map_err(|_| "This is not an isolated worktree build.".to_string())?;
    let expected = format!("Codex Orchestrator [Worktree build: {name}]");
    window
        .set_title(&expected)
        .and_then(|_| window.set_size(tauri::Size::Logical(tauri::LogicalSize::new(1280.0, 820.0))))
        .and_then(|_| {
            window.set_min_size(Some(tauri::Size::Logical(tauri::LogicalSize::new(
                960.0, 640.0,
            ))))
        })
        .map_err(|_| "Prepare the worktree-build window.".to_string())?;
    #[cfg(windows)]
    {
        use windows_sys::Win32::UI::WindowsAndMessaging::{ShowWindow, SW_SHOWNOACTIVATE};
        let handle = window
            .hwnd()
            .map_err(|_| "Read the worktree-build window.".to_string())?;
        unsafe {
            ShowWindow(handle.0 as _, SW_SHOWNOACTIVATE);
        }
    }
    #[cfg(not(windows))]
    window
        .show()
        .map_err(|_| "Show the worktree-build window.".to_string())?;
    let path = std::env::var_os("CODEX_ORCHESTRATOR_WORKTREE_READY_PATH")
        .map(std::path::PathBuf::from)
        .ok_or_else(|| "Worktree-build readiness storage is unavailable.".to_string())?;
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|_| "Worktree-build readiness storage is unavailable.".to_string())?;
    }
    let temporary = path.with_extension("pending");
    std::fs::write(&temporary, b"application-surface-rendered")
        .and_then(|_| std::fs::rename(&temporary, &path))
        .map_err(|_| "Worktree-build readiness could not be recorded.".to_string())
}

#[tauri::command]
pub(crate) async fn build_human_review_instance(
    state: State<'_, HumanReviewLauncherTauriState>,
    input: ReviewInstanceOperationInput,
) -> Result<ReviewInstanceView, String> {
    run(state.0.clone(), move |service| {
        service.build(input.operation_ref, input.instance_ref)
    })
    .await
}

#[tauri::command]
pub(crate) async fn start_human_review_instance(
    state: State<'_, HumanReviewLauncherTauriState>,
    input: ReviewInstanceOperationInput,
) -> Result<ReviewInstanceView, String> {
    run(state.0.clone(), move |service| {
        service.start(input.operation_ref, input.instance_ref)
    })
    .await
}

macro_rules! lifecycle_command {
    ($name:ident, $method:ident) => {
        #[tauri::command]
        pub(crate) async fn $name(
            state: State<'_, HumanReviewLauncherTauriState>,
            input: ReviewInstanceInput,
        ) -> Result<ReviewInstanceView, String> {
            run(state.0.clone(), move |service| {
                service.$method(input.instance_ref)
            })
            .await
        }
    };
}

lifecycle_command!(status_human_review_instance, status);
lifecycle_command!(focus_human_review_instance, focus);
lifecycle_command!(stop_human_review_instance, stop);
lifecycle_command!(recover_human_review_instance, recover);

async fn run(
    service: Arc<HumanReviewLauncherService>,
    operation: impl FnOnce(&HumanReviewLauncherService) -> Result<ReviewInstanceView, String>
        + Send
        + 'static,
) -> Result<ReviewInstanceView, String> {
    tauri::async_runtime::spawn_blocking(move || operation(&service))
        .await
        .map_err(|error| format!("Review runtime task failed: {error}"))?
}
