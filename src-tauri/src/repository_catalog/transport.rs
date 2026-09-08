use super::{
    domain::{
        RegisteredRepositoryView, RepositoryCatalogOverviewView, ResolvedRepoBranchWorktreeTarget,
    },
    RepositoryCatalog,
};
use serde::Deserialize;
use std::{path::PathBuf, sync::Arc};
use tauri::State;

pub(crate) struct RepositoryCatalogTauriState {
    catalog: Arc<RepositoryCatalog>,
}

impl RepositoryCatalogTauriState {
    pub(crate) fn new(catalog: Arc<RepositoryCatalog>) -> Self {
        Self { catalog }
    }
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryDirectoryRegistrationInput {
    repository_root: String,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CodexRepositoryRegistrationInput {
    repository_id: String,
}

#[tauri::command]
pub(crate) async fn repository_catalog_overview(
    state: State<'_, RepositoryCatalogTauriState>,
) -> Result<RepositoryCatalogOverviewView, String> {
    let catalog = state.catalog.clone();
    blocking("overview", move || catalog.overview()).await
}

#[tauri::command]
pub(crate) async fn register_repository_directory(
    state: State<'_, RepositoryCatalogTauriState>,
    input: RepositoryDirectoryRegistrationInput,
) -> Result<RegisteredRepositoryView, String> {
    let repository_root = input.repository_root.trim();
    if repository_root.is_empty() {
        return Err("Enter the root of a Git repository.".into());
    }
    let catalog = state.catalog.clone();
    let repository_root = PathBuf::from(repository_root);
    blocking("directory registration", move || {
        catalog.register_directory(repository_root)
    })
    .await
}

#[tauri::command]
pub(crate) async fn register_codex_repository(
    state: State<'_, RepositoryCatalogTauriState>,
    input: CodexRepositoryRegistrationInput,
) -> Result<RegisteredRepositoryView, String> {
    let catalog = state.catalog.clone();
    blocking("Codex registration", move || {
        catalog.register_codex_repository(&input.repository_id)
    })
    .await
}

#[tauri::command]
pub(crate) async fn list_registered_repository_worktree_targets(
    state: State<'_, RepositoryCatalogTauriState>,
) -> Result<Vec<ResolvedRepoBranchWorktreeTarget>, String> {
    let catalog = state.catalog.clone();
    blocking("worktree target discovery", move || {
        catalog.list_worktree_targets()
    })
    .await
}

async fn blocking<T: Send + 'static>(
    label: &'static str,
    operation: impl FnOnce() -> Result<T, String> + Send + 'static,
) -> Result<T, String> {
    tauri::async_runtime::spawn_blocking(operation)
        .await
        .map_err(|error| format!("Repository catalog {label} task failed: {error}"))?
}
