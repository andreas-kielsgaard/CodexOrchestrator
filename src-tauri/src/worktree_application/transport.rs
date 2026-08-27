use super::{
    GitCommitId, PhysicalWorktreeApplication, PhysicalWorktreeAttachment,
    PhysicalWorktreeCheckoutRequest,
};
use crate::repository_context::{FullRefName, RepositoryContext, WorktreeLocation};
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

#[derive(Clone, Debug, Deserialize, Eq, PartialEq)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreatePhysicalWorktreeInput {
    pub(crate) repository_root: String,
    pub(crate) worktree_root: String,
    pub(crate) branch_name: String,
    pub(crate) start_point_ref: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct PhysicalWorktreeView {
    pub(crate) repository_root: String,
    pub(crate) worktree_root: String,
    pub(crate) commit_id: String,
    pub(crate) head_ref: String,
}

/// Task-shaped native adapter. Mutable ref resolution is completed before the exact mutating core
/// receives its request; the core itself accepts only the pinned commit identity.
#[tauri::command]
pub(crate) async fn create_physical_worktree(
    input: CreatePhysicalWorktreeInput,
) -> Result<PhysicalWorktreeView, String> {
    tauri::async_runtime::spawn_blocking(move || create_physical_worktree_blocking(input))
        .await
        .map_err(|_| "Physical worktree creation did not complete.".to_string())?
}

fn create_physical_worktree_blocking(
    input: CreatePhysicalWorktreeInput,
) -> Result<PhysicalWorktreeView, String> {
    let context = RepositoryContext::discover().map_err(|error| error.to_string())?;
    let repository = context
        .identities()
        .inspect(&PathBuf::from(&input.repository_root))
        .map_err(|error| error.to_string())?;
    let start = match input.start_point_ref {
        Some(reference) => {
            let reference = task_start_ref(reference)?;
            context
                .references()
                .resolve_commit(repository.top_level.path(), &reference)
                .map_err(|error| error.to_string())?
        }
        None => context
            .worktrees()
            .list(&repository.id, repository.top_level.path())
            .map_err(|error| error.to_string())?
            .into_iter()
            .find_map(|worktree| match worktree.location {
                WorktreeLocation::Available(location)
                    if location.path() == repository.top_level.path() =>
                {
                    Some(worktree.head)
                }
                _ => None,
            })
            .ok_or_else(|| "The repository HEAD is unavailable.".to_string())?,
    };
    let commit = GitCommitId::new(start.as_str()).map_err(|error| error.to_string())?;
    let attachment = PhysicalWorktreeAttachment::new_branch(input.branch_name)
        .map_err(|error| error.to_string())?;
    let request = PhysicalWorktreeCheckoutRequest::new(
        repository.top_level.path().to_path_buf(),
        PathBuf::from(input.worktree_root),
        commit,
        attachment,
    )
    .map_err(|error| error.to_string())?;
    let result = PhysicalWorktreeApplication
        .materialize_checkout(context.git_executable(), &request)
        .map_err(|error| error.to_string())?;
    Ok(PhysicalWorktreeView {
        repository_root: external_text(&result.repository_root),
        worktree_root: external_text(&result.worktree_root),
        commit_id: result.commit_id.as_str().to_owned(),
        head_ref: result
            .head_ref
            .ok_or_else(|| "The created named worktree is detached.".to_string())?,
    })
}

fn external_text(path: &std::path::Path) -> String {
    super::git::external_path(path)
        .to_string_lossy()
        .into_owned()
}

fn task_start_ref(value: String) -> Result<FullRefName, String> {
    let value = if value.starts_with("refs/") {
        value
    } else {
        format!("refs/heads/{value}")
    };
    FullRefName::parse(value).map_err(|error| error.to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn task_branch_start_is_normalized_before_core_resolution() {
        assert_eq!(
            task_start_ref("main".into()).unwrap().as_str(),
            "refs/heads/main"
        );
        assert_eq!(
            task_start_ref("refs/heads/release".into())
                .unwrap()
                .as_str(),
            "refs/heads/release"
        );
    }
}
