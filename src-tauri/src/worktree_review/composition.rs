use super::{catalog::ReviewWorktreeCatalog, service::WorktreeReviewService};
use crate::repository_context::{FullRefName, RepositoryContext};
use crate::worktree_runtime::{
    AuthoritySecret, RuntimeSettings, SqliteInstanceRegistry, SystemActionExecutor,
    SystemSourceInspector, TcpHealthProbe, ToolchainPrograms, WorktreeRuntimeApplication,
    WorktreeTestInstanceFacade,
};
use serde::{Deserialize, Serialize};
use std::{fmt, fs, fs::OpenOptions, io::Write, path::Path, sync::Arc};
use uuid::Uuid;

#[derive(Debug)]
pub(crate) enum WorktreeReviewCompositionError {
    MissingTool,
    RepositoryUnavailable,
    StorageUnavailable,
    RuntimeUnavailable,
}

impl fmt::Display for WorktreeReviewCompositionError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::MissingTool => "A required Worktree Review tool is unavailable.",
            Self::RepositoryUnavailable => {
                "The selected Worktree Review repository is unavailable."
            }
            Self::StorageUnavailable => "Worktree Review storage is unavailable.",
            Self::RuntimeUnavailable => "The Worktree Review runtime is unavailable.",
        })
    }
}

#[cfg(test)]
pub(crate) fn compose(
    current_source: &Path,
    review_root: &Path,
) -> Result<WorktreeReviewService, WorktreeReviewCompositionError> {
    compose_scoped(current_source, review_root, review_root)
}

pub(crate) fn compose_scoped(
    current_source: &Path,
    review_root: &Path,
    repository_root: &Path,
) -> Result<WorktreeReviewService, WorktreeReviewCompositionError> {
    fs::create_dir_all(review_root)
        .map_err(|_| WorktreeReviewCompositionError::StorageUnavailable)?;
    fs::create_dir_all(repository_root)
        .map_err(|_| WorktreeReviewCompositionError::StorageUnavailable)?;
    let programs =
        ToolchainPrograms::discover().map_err(|_| WorktreeReviewCompositionError::MissingTool)?;
    let comparison_branch =
        load_or_infer_comparison_branch(repository_root, current_source, &programs.git)
            .map_err(|_| WorktreeReviewCompositionError::RepositoryUnavailable)?;
    let catalog = Arc::new(
        ReviewWorktreeCatalog::discover_with_comparison(
            current_source,
            &programs.git,
            Some(&comparison_branch),
        )
        .map_err(|_| WorktreeReviewCompositionError::RepositoryUnavailable)?,
    );
    let registry = Arc::new(
        SqliteInstanceRegistry::open(review_root.join("registry.sqlite"))
            .map_err(|_| WorktreeReviewCompositionError::StorageUnavailable)?,
    );
    #[cfg(windows)]
    let owner = Arc::new(crate::worktree_runtime::WindowsJobProcessOwner::default());
    #[cfg(not(windows))]
    let owner = Arc::new(crate::worktree_runtime::UnsupportedProcessOwner);
    let application = Arc::new(WorktreeRuntimeApplication::system(
        registry,
        owner,
        Arc::new(TcpHealthProbe::default()),
    ));
    let facade = Arc::new(
        WorktreeTestInstanceFacade::new(
            application,
            catalog.clone(),
            Arc::new(SystemSourceInspector),
            Arc::new(SystemActionExecutor),
            RuntimeSettings {
                instances_root: repository_root.join("instances"),
                shared_cache_root: review_root.join("shared-cache"),
                port_start: 18200,
                port_end: 18399,
            },
            programs,
            load_or_create_authority(review_root)
                .map_err(|_| WorktreeReviewCompositionError::StorageUnavailable)?,
        )
        .map_err(|_| WorktreeReviewCompositionError::RuntimeUnavailable)?,
    );
    WorktreeReviewService::new(
        facade,
        catalog,
        &repository_root.join("launcher.sqlite"),
        repository_root.join("instances"),
    )
    .map_err(|_| WorktreeReviewCompositionError::StorageUnavailable)
}

#[derive(Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct RepositorySettings {
    comparison_branch: String,
}

fn load_or_infer_comparison_branch(
    review_root: &Path,
    current_source: &Path,
    git: &Path,
) -> Result<String, String> {
    let path = review_root.join("repository-settings.json");
    if path.exists() {
        let settings: RepositorySettings = serde_json::from_slice(
            &fs::read(&path).map_err(|error| format!("read repository settings: {error}"))?,
        )
        .map_err(|error| format!("parse repository settings: {error}"))?;
        return Ok(settings.comparison_branch);
    }
    let repository = RepositoryContext::with_git(git).map_err(|error| error.to_string())?;
    let origin_head =
        FullRefName::parse("refs/remotes/origin/HEAD").map_err(|error| error.to_string())?;
    let remote_default = repository
        .symbolic_ref_short(current_source, &origin_head)
        .unwrap_or(None);
    let current = repository.current_branch(current_source).unwrap_or(None);
    let comparison_branch = remote_default.or(current).unwrap_or_else(|| "main".into());
    fs::write(
        &path,
        serde_json::to_vec_pretty(&RepositorySettings {
            comparison_branch: comparison_branch.clone(),
        })
        .map_err(|error| format!("encode repository settings: {error}"))?,
    )
    .map_err(|error| format!("persist repository settings: {error}"))?;
    Ok(comparison_branch)
}

fn load_or_create_authority(root: &Path) -> Result<AuthoritySecret, String> {
    let path = root.join("authority.secret");
    if path.exists() {
        return fs::read_to_string(&path)
            .map_err(|error| format!("read review authority: {error}"))
            .and_then(|value| {
                AuthoritySecret::new(value.trim().to_owned()).map_err(|error| error.to_string())
            });
    }
    let value = format!("{}{}", Uuid::new_v4().simple(), Uuid::new_v4().simple());
    let mut file = OpenOptions::new()
        .write(true)
        .create_new(true)
        .open(&path)
        .map_err(|error| format!("create review authority: {error}"))?;
    file.write_all(value.as_bytes())
        .map_err(|error| format!("write review authority: {error}"))?;
    AuthoritySecret::new(value).map_err(|error| error.to_string())
}
