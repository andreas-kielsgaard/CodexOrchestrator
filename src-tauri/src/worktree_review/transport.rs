use super::{
    comparison::WorktreeComparisonView,
    detail::ReviewInstanceDetailView,
    progress::ReviewOperationProgressView,
    service::{ReviewInstanceView, ReviewSettingsView, ReviewSourceView, WorktreeReviewService},
    source_history::ReviewSourceHistoryView,
    state::{WorktreeReviewReadinessView, WorktreeReviewState},
};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Arc;
use tauri::State;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeReviewCommandError {
    pub(crate) code: &'static str,
    pub(crate) message: String,
}

impl WorktreeReviewCommandError {
    fn from_message(message: String) -> Self {
        let lower = message.to_ascii_lowercase();
        let code = if lower.contains("select a git repository")
            || lower.contains("repository is selected")
        {
            "worktree_review_not_ready"
        } else if lower.contains("invalid") || lower.contains("already used") {
            "worktree_review_invalid_request"
        } else if lower.contains("unavailable") || lower.contains("could not") {
            "worktree_review_unavailable"
        } else {
            "worktree_review_operation_failed"
        };
        let contains_opaque_details = message.contains('\n')
            || message.contains("fatal:")
            || message.contains("\\\\")
            || message.contains(":/");
        Self {
            code,
            message: if contains_opaque_details {
                "Worktree Review could not complete this action.".into()
            } else {
                message
            },
        }
    }

    fn task_failed() -> Self {
        Self {
            code: "worktree_review_unavailable",
            message: "The Worktree Review background task could not complete.".into(),
        }
    }
}

impl From<String> for WorktreeReviewCommandError {
    fn from(message: String) -> Self {
        Self::from_message(message)
    }
}

type CommandResult<T> = Result<T, WorktreeReviewCommandError>;

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
pub(crate) struct ReviewSourceInput {
    source_ref: String,
}

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewSourceListInput {
    include_detached: bool,
    refresh: bool,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewSettingsInput {
    cleanup_detached_builds: bool,
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

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SelectWorktreeReviewRepositoryInput {
    repository_root: PathBuf,
}

#[tauri::command]
pub(crate) async fn worktree_review_readiness(
    state: State<'_, WorktreeReviewState>,
) -> CommandResult<WorktreeReviewReadinessView> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.readiness())
        .await
        .map_err(|_| WorktreeReviewCommandError::task_failed())
}

#[tauri::command]
pub(crate) async fn select_worktree_review_repository(
    state: State<'_, WorktreeReviewState>,
    input: SelectWorktreeReviewRepositoryInput,
) -> CommandResult<WorktreeReviewReadinessView> {
    let state = state.inner().clone();
    tauri::async_runtime::spawn_blocking(move || state.select_repository(input.repository_root))
        .await
        .map_err(|_| WorktreeReviewCommandError::task_failed())
}

#[tauri::command]
pub(crate) fn list_human_review_worktrees(
    state: State<'_, WorktreeReviewState>,
    input: Option<ReviewSourceListInput>,
) -> CommandResult<Vec<ReviewSourceView>> {
    let input = input.unwrap_or_default();
    Ok(state
        .service()?
        .source_snapshot(input.include_detached, input.refresh))
}

#[tauri::command]
pub(crate) async fn list_human_review_repository_history(
    state: State<'_, WorktreeReviewState>,
) -> CommandResult<Vec<ReviewSourceView>> {
    let service = state.service()?;
    tauri::async_runtime::spawn_blocking(move || service.repository_sources())
        .await
        .map_err(|_| WorktreeReviewCommandError::task_failed())?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn list_human_review_instances(
    state: State<'_, WorktreeReviewState>,
) -> CommandResult<Vec<ReviewInstanceView>> {
    Ok(state.service()?.instances())
}

#[tauri::command]
pub(crate) fn human_review_settings(
    state: State<'_, WorktreeReviewState>,
) -> CommandResult<ReviewSettingsView> {
    state.service()?.settings().map_err(Into::into)
}

#[tauri::command]
pub(crate) fn update_human_review_settings(
    state: State<'_, WorktreeReviewState>,
    input: ReviewSettingsInput,
) -> CommandResult<ReviewSettingsView> {
    state
        .service()?
        .update_settings(ReviewSettingsView {
            cleanup_detached_builds: input.cleanup_detached_builds,
        })
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn human_review_source_history(
    state: State<'_, WorktreeReviewState>,
    input: ReviewSourceInput,
) -> CommandResult<ReviewSourceHistoryView> {
    let service = state.service()?;
    tauri::async_runtime::spawn_blocking(move || service.source_history(input.source_ref))
        .await
        .map_err(|_| WorktreeReviewCommandError::task_failed())?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn attach_human_review_worktree(
    state: State<'_, WorktreeReviewState>,
    input: ReviewSourceInput,
) -> CommandResult<ReviewSourceView> {
    let service = state.service()?;
    tauri::async_runtime::spawn_blocking(move || service.attach_review_worktree(input.source_ref))
        .await
        .map_err(|_| WorktreeReviewCommandError::task_failed())?
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn prepare_human_review_instance(
    state: State<'_, WorktreeReviewState>,
    input: PrepareReviewInput,
) -> CommandResult<ReviewInstanceView> {
    run(state.service()?, move |service| {
        service.prepare(input.operation_ref, input.source_ref, input.name)
    })
    .await
}

#[tauri::command]
pub(crate) fn human_review_operation_progress(
    state: State<'_, WorktreeReviewState>,
    input: ReviewOperationInput,
) -> CommandResult<ReviewOperationProgressView> {
    state
        .service()?
        .operation_progress(input.operation_ref)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn list_human_review_operation_progress(
    state: State<'_, WorktreeReviewState>,
) -> CommandResult<Vec<ReviewOperationProgressView>> {
    Ok(state.service()?.operations())
}

#[tauri::command]
pub(crate) fn human_review_instance_detail(
    state: State<'_, WorktreeReviewState>,
    input: ReviewInstanceInput,
) -> CommandResult<ReviewInstanceDetailView> {
    state
        .service()?
        .detail(input.instance_ref)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn human_review_instance_comparison(
    state: State<'_, WorktreeReviewState>,
    input: ReviewInstanceInput,
) -> CommandResult<WorktreeComparisonView> {
    state
        .service()?
        .comparison(input.instance_ref)
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) fn mark_worktree_build_ready(window: tauri::Window) -> CommandResult<()> {
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
        .map_err(Into::into)
}

#[tauri::command]
pub(crate) async fn build_human_review_instance(
    state: State<'_, WorktreeReviewState>,
    input: ReviewInstanceOperationInput,
) -> CommandResult<ReviewInstanceView> {
    run(state.service()?, move |service| {
        service.build(input.operation_ref, input.instance_ref)
    })
    .await
}

#[tauri::command]
pub(crate) async fn start_human_review_instance(
    state: State<'_, WorktreeReviewState>,
    input: ReviewInstanceOperationInput,
) -> CommandResult<ReviewInstanceView> {
    run(state.service()?, move |service| {
        service.start(input.operation_ref, input.instance_ref)
    })
    .await
}

macro_rules! lifecycle_command {
    ($name:ident, $method:ident) => {
        #[tauri::command]
        pub(crate) async fn $name(
            state: State<'_, WorktreeReviewState>,
            input: ReviewInstanceInput,
        ) -> CommandResult<ReviewInstanceView> {
            run(state.service()?, move |service| {
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
    service: Arc<WorktreeReviewService>,
    operation: impl FnOnce(&WorktreeReviewService) -> Result<ReviewInstanceView, String>
        + Send
        + 'static,
) -> CommandResult<ReviewInstanceView> {
    tauri::async_runtime::spawn_blocking(move || operation(&service))
        .await
        .map_err(|_| WorktreeReviewCommandError::task_failed())?
        .map_err(Into::into)
}

#[cfg(test)]
mod tests {
    use super::WorktreeReviewCommandError;

    #[test]
    fn command_errors_are_structured_and_hide_opaque_process_details() {
        let error = WorktreeReviewCommandError::from(
            "fatal: repository unavailable at C:/private/repository".to_string(),
        );

        assert_eq!(error.code, "worktree_review_unavailable");
        assert_eq!(
            error.message,
            "Worktree Review could not complete this action."
        );
    }
}
