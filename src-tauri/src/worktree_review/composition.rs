use super::{catalog::ReviewWorktreeCatalog, service::HumanReviewLauncherService};
use crate::worktree_runtime::{
    AuthoritySecret, RuntimeSettings, SqliteInstanceRegistry, SystemActionExecutor,
    SystemSourceInspector, TcpHealthProbe, ToolchainPrograms, WorktreeRuntimeApplication,
    WorktreeTestInstanceFacade,
};
use serde::{Deserialize, Serialize};
use std::{fs, fs::OpenOptions, io::Write, path::Path, process::Command, sync::Arc};
use uuid::Uuid;

pub(crate) fn compose(
    current_source: &Path,
    review_root: &Path,
) -> Result<HumanReviewLauncherService, String> {
    fs::create_dir_all(review_root)
        .map_err(|error| format!("create review runtime root: {error}"))?;
    let programs = ToolchainPrograms::discover().map_err(|error| error.to_string())?;
    let comparison_branch =
        load_or_infer_comparison_branch(review_root, current_source, &programs.git)?;
    let catalog = Arc::new(ReviewWorktreeCatalog::discover_with_comparison(
        current_source,
        &programs.git,
        Some(&comparison_branch),
    )?);
    let registry = Arc::new(
        SqliteInstanceRegistry::open(review_root.join("registry.sqlite"))
            .map_err(|error| error.to_string())?,
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
                instances_root: review_root.join("instances"),
                shared_cache_root: review_root.join("shared-cache"),
                port_start: 18200,
                port_end: 18399,
            },
            programs,
            load_or_create_authority(review_root)?,
        )
        .map_err(|error| error.to_string())?,
    );
    HumanReviewLauncherService::new(
        facade,
        catalog,
        &review_root.join("launcher.sqlite"),
        review_root.join("instances"),
    )
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
    let remote_default = git_output(
        current_source,
        git,
        &[
            "symbolic-ref",
            "--quiet",
            "--short",
            "refs/remotes/origin/HEAD",
        ],
    );
    let current = git_output(
        current_source,
        git,
        &["symbolic-ref", "--quiet", "--short", "HEAD"],
    );
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

fn git_output(root: &Path, git: &Path, args: &[&str]) -> Option<String> {
    let output = Command::new(git)
        .arg("-C")
        .arg(root)
        .args(args)
        .env("GIT_OPTIONAL_LOCKS", "0")
        .output()
        .ok()?;
    output
        .status
        .success()
        .then(|| String::from_utf8_lossy(&output.stdout).trim().to_owned())
        .filter(|value| !value.is_empty())
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
