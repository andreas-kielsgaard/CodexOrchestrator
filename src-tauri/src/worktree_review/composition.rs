use super::{catalog::ReviewWorktreeCatalog, service::HumanReviewLauncherService};
use crate::worktree_runtime::{
    AuthoritySecret, RuntimeSettings, SqliteInstanceRegistry, SystemActionExecutor,
    SystemSourceInspector, TcpHealthProbe, ToolchainPrograms, WorktreeRuntimeApplication,
    WorktreeTestInstanceFacade,
};
use std::{fs, fs::OpenOptions, io::Write, path::Path, sync::Arc};
use uuid::Uuid;

pub(crate) fn compose(
    current_source: &Path,
    review_root: &Path,
) -> Result<HumanReviewLauncherService, String> {
    fs::create_dir_all(review_root)
        .map_err(|error| format!("create review runtime root: {error}"))?;
    let programs = ToolchainPrograms::discover().map_err(|error| error.to_string())?;
    let catalog = Arc::new(ReviewWorktreeCatalog::discover(
        current_source,
        &programs.git,
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
    HumanReviewLauncherService::new(facade, catalog, &review_root.join("launcher.sqlite"))
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
