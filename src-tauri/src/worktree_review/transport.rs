use super::{
    branch_first::{
        AssociateWorktreeInput, AssociatedWorktreeView, BranchDetailView, BranchFirstReviewService,
        BranchHistoryPageView, ProductOverviewView,
    },
    build_service::{CreateBuildInput, ReviewBuildView},
    state::{CapabilityReadinessStatus, WorktreeReviewApplication},
};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tauri::State;

pub(crate) struct WorktreeReviewTauriState {
    application: Arc<WorktreeReviewApplication>,
    product: Result<Arc<BranchFirstReviewService>, String>,
}

impl WorktreeReviewTauriState {
    pub(crate) fn new(application: Arc<WorktreeReviewApplication>) -> Self {
        let product = BranchFirstReviewService::open(application.clone()).map(Arc::new);
        Self {
            application,
            product,
        }
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
pub(crate) struct RepositoryConnectionInput {
    repository_root: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchSelectionInput {
    repository_id: String,
    branch_ref: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchHistoryInput {
    repository_id: String,
    branch_ref: String,
    cursor: Option<String>,
    page_size: Option<usize>,
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
pub(crate) async fn connect_worktree_review_repository(
    state: State<'_, WorktreeReviewTauriState>,
    input: RepositoryConnectionInput,
) -> Result<ProductOverviewView, String> {
    let root = input.repository_root.trim();
    if root.is_empty() {
        return Err("Enter the root of a Git repository.".into());
    }
    let application = state.application.clone();
    let product = state.product_arc()?;
    let root = PathBuf::from(root);
    blocking("repository connection", move || {
        let result = application.select_repository(root);
        if result.selection.status != CapabilityReadinessStatus::Ready {
            return Err(result.selection.message);
        }
        product.overview()
    })
    .await
}

#[tauri::command]
pub(crate) async fn worktree_review_branch_detail(
    state: State<'_, WorktreeReviewTauriState>,
    input: BranchSelectionInput,
) -> Result<BranchDetailView, String> {
    let product = state.product_arc()?;
    blocking("branch detail", move || {
        product.branch_detail(&input.repository_id, &input.branch_ref)
    })
    .await
}

#[tauri::command]
pub(crate) async fn worktree_review_branch_history(
    state: State<'_, WorktreeReviewTauriState>,
    input: BranchHistoryInput,
) -> Result<BranchHistoryPageView, String> {
    let page_size = input.page_size.unwrap_or(50);
    if !(1..=100).contains(&page_size) {
        return Err("Branch history pages must contain between 1 and 100 commits.".into());
    }
    let product = state.product_arc()?;
    blocking("branch history", move || {
        product.branch_history(
            &input.repository_id,
            &input.branch_ref,
            input.cursor.as_deref(),
            page_size,
        )
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
    state: State<'_, WorktreeReviewTauriState>,
    input: CreateBuildInput,
) -> Result<ReviewBuildView, String> {
    let product = state.product_arc()?;
    blocking("build", move || product.create_build(input)).await
}

#[tauri::command]
pub(crate) async fn worktree_review_open_build(
    state: State<'_, WorktreeReviewTauriState>,
    input: OpenBuildInput,
) -> Result<(), String> {
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
