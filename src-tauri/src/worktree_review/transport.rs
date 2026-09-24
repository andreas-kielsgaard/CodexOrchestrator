use super::{
    branch_first::{
        AssociateWorktreeInput, AssociatedWorktreeView, BranchDetailView, BranchFirstReviewService,
        DetachedWorktreeView, ProductOverviewView,
    },
    branch_graph::BranchGraphView,
    branch_history::{CommitHistoryPageView, CommitHistoryQuery},
    build_service::{CreateBuildInput, OpenBuildOutcomeView, ReviewBuildView},
    domain::ReviewTarget,
    state::WorktreeReviewApplication,
    worktree_activity::WorktreeActivityView,
};
use serde::Deserialize;
use std::sync::Arc;
use tauri::{AppHandle, Emitter, State};

pub(crate) const BUILD_TERMINAL_EVENT: &str = "worktree-review://build-terminal";

pub(crate) struct WorktreeReviewTauriState {
    product: Result<Arc<BranchFirstReviewService>, String>,
}

impl WorktreeReviewTauriState {
    pub(crate) fn new(application: Arc<WorktreeReviewApplication>) -> Self {
        let product = BranchFirstReviewService::open(application.clone()).map(Arc::new);
        Self { product }
    }

    fn product_arc(&self) -> Result<Arc<BranchFirstReviewService>, String> {
        self.product.as_ref().cloned().map_err(Clone::clone)
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositorySelectionInput {
    repository_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitHistoryInput {
    query: CommitHistoryQuery,
    cursor: Option<String>,
    page_size: Option<usize>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchGraphInput {
    snapshot_id: Option<String>,
    repository_id: String,
    limit: Option<usize>,
}
#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeActivityInput {
    repository_id: String,
    worktree_ids: Vec<String>,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateWorktreeInput {
    repository_id: String,
    branch_ref: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct OpenBuildInput {
    build_id: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct BuildLogInput {
    build_id: String,
    attempt_id: String,
    offset: u64,
}

#[tauri::command]
pub(crate) async fn worktree_review_build_log(
    state: State<'_, WorktreeReviewTauriState>,
    input: BuildLogInput,
) -> Result<super::build_service::BuildLogChunkView, String> {
    let product = state.product_arc()?;
    blocking("build log", move || {
        product.read_build_log(&input.build_id, &input.attempt_id, input.offset)
    })
    .await
}

#[tauri::command]
pub(crate) async fn worktree_review_overview(
    state: State<'_, WorktreeReviewTauriState>,
) -> Result<ProductOverviewView, String> {
    let product = state.product_arc()?;
    blocking("overview", move || product.overview()).await
}

#[tauri::command]
pub(crate) async fn select_worktree_review_repository(
    state: State<'_, WorktreeReviewTauriState>,
    input: RepositorySelectionInput,
) -> Result<ProductOverviewView, String> {
    let product = state.product_arc()?;
    blocking("repository selection", move || {
        product.select_repository(&input.repository_id)
    })
    .await
}

#[tauri::command]
pub(crate) async fn worktree_review_target_detail(
    state: State<'_, WorktreeReviewTauriState>,
    input: ReviewTarget,
) -> Result<BranchDetailView, String> {
    let product = state.product_arc()?;
    blocking("target detail", move || product.target_detail(input)).await
}

#[tauri::command]
pub(crate) async fn worktree_review_commit_history(
    state: State<'_, WorktreeReviewTauriState>,
    input: CommitHistoryInput,
) -> Result<CommitHistoryPageView, String> {
    let product = state.product_arc()?;
    blocking("commit history", move || {
        product.commit_history(
            input.query,
            input.cursor.as_deref(),
            input.page_size.unwrap_or(50),
        )
    })
    .await
}

#[tauri::command]
pub(crate) async fn worktree_review_branch_graph(
    state: State<'_, WorktreeReviewTauriState>,
    input: BranchGraphInput,
) -> Result<BranchGraphView, String> {
    let product = state.product_arc()?;
    blocking("branch graph", move || {
        product.branch_graph(
            &input.repository_id,
            input.limit.unwrap_or(1200),
            input.snapshot_id.as_deref(),
        )
    })
    .await
}

#[tauri::command]
pub(crate) async fn worktree_review_worktree_activity(
    state: State<'_, WorktreeReviewTauriState>,
    input: WorktreeActivityInput,
) -> Result<Vec<WorktreeActivityView>, String> {
    let product = state.product_arc()?;
    blocking("worktree activity", move || {
        product.worktree_activity(&input.repository_id, &input.worktree_ids)
    })
    .await
}

#[tauri::command]
pub(crate) async fn worktree_review_detached_worktrees(
    state: State<'_, WorktreeReviewTauriState>,
    input: RepositorySelectionInput,
) -> Result<Vec<DetachedWorktreeView>, String> {
    let product = state.product_arc()?;
    blocking("detached worktrees", move || {
        product.detached_worktrees(&input.repository_id)
    })
    .await
}

#[tauri::command]
pub(crate) async fn associate_worktree_review_worktree(
    state: State<'_, WorktreeReviewTauriState>,
    input: AssociateWorktreeInput,
) -> Result<AssociatedWorktreeView, String> {
    let product = state.product_arc()?;
    blocking("worktree association", move || {
        product.associate_worktree(input)
    })
    .await
}

#[tauri::command]
pub(crate) async fn create_worktree_review_worktree(
    state: State<'_, WorktreeReviewTauriState>,
    input: CreateWorktreeInput,
) -> Result<AssociatedWorktreeView, String> {
    let product = state.product_arc()?;
    blocking("worktree creation", move || {
        product.create_worktree(&input.repository_id, &input.branch_ref)
    })
    .await
}

#[tauri::command]
pub(crate) async fn create_worktree_review_build(
    app: AppHandle,
    state: State<'_, WorktreeReviewTauriState>,
    input: CreateBuildInput,
) -> Result<ReviewBuildView, String> {
    let product = state.product_arc()?;
    let (sender, receiver) = tokio::sync::oneshot::channel();
    let sender = std::sync::Arc::new(std::sync::Mutex::new(Some(sender)));
    let callback_sender = sender.clone();
    tauri::async_runtime::spawn_blocking(move || {
        let result = product.create_build_with_started(input, move |started| {
            if let Ok(mut sender) = callback_sender.lock() {
                if let Some(sender) = sender.take() {
                    let _ = sender.send(Ok(started));
                }
            }
        });
        if let Ok(terminal) = &result {
            let _ = app.emit(BUILD_TERMINAL_EVENT, terminal);
        }
        if let Ok(mut sender) = sender.lock() {
            if let Some(sender) = sender.take() {
                let _ = sender.send(result);
            }
        }
    });
    receiver
        .await
        .map_err(|_| "Build task stopped before recording its start".to_string())?
}

#[tauri::command]
pub(crate) async fn worktree_review_open_build(
    state: State<'_, WorktreeReviewTauriState>,
    input: OpenBuildInput,
) -> Result<OpenBuildOutcomeView, String> {
    let product = state.product_arc()?;
    blocking("open build", move || product.open_build(&input.build_id)).await
}

async fn blocking<T: Send + 'static>(
    label: &'static str,
    operation: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| format!("Worktree Review {label} task failed: {error}"))?
}
