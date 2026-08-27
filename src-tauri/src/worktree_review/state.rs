use super::domain::{ReviewBuildId, WorktreeId};
use crate::{
    repository_context::{
        RepositoryContext, RepositoryContextError, RepositoryContextErrorKind, RepositoryIdentity,
    },
    worktree_review::storage::{
        PersistedRepositorySelection, RepositorySelectionRepository, ReviewBuildRepository,
        WorkspaceRepository, WorktreeReviewDatabase,
    },
};
use serde::Serialize;
use std::{
    env,
    path::{Component, Path, PathBuf},
    sync::{Arc, Mutex},
};

pub(crate) const STATE_DATABASE_FILE: &str = "worktree-review.sqlite";
pub(crate) const WORKTREE_REVIEW_DATA_DIR_ENV: &str = "CODEX_ORCHESTRATOR_WORKTREE_REVIEW_DATA_DIR";
pub(crate) const ACTIVE_REVIEW_BUILD_ID_ENV: &str = "CODEX_ORCHESTRATOR_ACTIVE_REVIEW_BUILD_ID";
pub(crate) const ACTIVE_REVIEW_WORKTREE_ID_ENV: &str =
    "CODEX_ORCHESTRATOR_ACTIVE_REVIEW_WORKTREE_ID";

/// Resolves the one application-owned data root used by every Worktree Review process.
///
/// Reviewed applications receive private AppData. Their launch contract must therefore pass this
/// root explicitly; no process hierarchy, runtime layout, or existing database is treated as
/// storage authority.
#[derive(Clone, Debug, Eq, PartialEq)]
struct SharedWorktreeReviewDataRoot {
    root: PathBuf,
}

impl SharedWorktreeReviewDataRoot {
    fn resolve(requested_root: PathBuf) -> Result<Self, String> {
        Self::resolve_with_shared_root(
            requested_root,
            env::var_os(WORKTREE_REVIEW_DATA_DIR_ENV).map(PathBuf::from),
        )
    }

    fn resolve_with_shared_root(
        requested_root: PathBuf,
        shared_root: Option<PathBuf>,
    ) -> Result<Self, String> {
        require_absolute_normalized(&requested_root, "Worktree Review data root")?;
        let root = shared_root.unwrap_or(requested_root);
        require_absolute_normalized(&root, "Shared Worktree Review data root")?;
        Ok(Self { root })
    }

    fn prepare(mut self) -> Result<Self, String> {
        std::fs::create_dir_all(&self.root)
            .map_err(|_| "Worktree Review data storage could not be created.".to_string())?;
        self.root = self
            .root
            .canonicalize()
            .map_err(|_| "Worktree Review data storage could not be resolved.".to_string())?;
        Ok(self)
    }

    fn database_path(&self) -> PathBuf {
        self.root.join(STATE_DATABASE_FILE)
    }

    fn build_output_store_root(&self) -> &Path {
        &self.root
    }
}

fn require_absolute_normalized(path: &Path, label: &str) -> Result<(), String> {
    if !path.is_absolute()
        || path
            .components()
            .any(|component| matches!(component, Component::CurDir | Component::ParentDir))
    {
        return Err(format!("{label} must be absolute and normalized."));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum CapabilityReadinessStatus {
    Ready,
    NeedsRepository,
    NotEvaluated,
    MissingTool,
    RepositoryUnavailable,
    StorageUnavailable,
    RuntimeUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CapabilityReadinessView {
    pub(crate) status: CapabilityReadinessStatus,
    pub(crate) message: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SelectedRepositoryView {
    pub(crate) repository_id: String,
    pub(crate) root: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeReviewCapabilitiesView {
    pub(crate) repository_browsing: CapabilityReadinessView,
    pub(crate) build_runtime: CapabilityReadinessView,
    pub(crate) build_output_storage: CapabilityReadinessView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeReviewOverviewView {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) selected_repository: Option<SelectedRepositoryView>,
    pub(crate) capabilities: WorktreeReviewCapabilitiesView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SelectRepositoryResultView {
    pub(crate) selection: CapabilityReadinessView,
    pub(crate) overview: WorktreeReviewOverviewView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ActiveBuildContextView {
    pub(crate) build_id: String,
    pub(crate) worktree_id: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct WorktreeReviewUnavailable {
    pub(crate) status: CapabilityReadinessStatus,
    pub(crate) message: String,
}

struct ApplicationState {
    selection: Option<PersistedRepositorySelection>,
    storage_available: bool,
    build_runtime: CapabilityReadinessView,
}

pub(crate) struct WorktreeReviewApplication {
    review_root: PathBuf,
    database: Result<Arc<WorktreeReviewDatabase>, WorktreeReviewUnavailable>,
    repository_context: Result<RepositoryContext, WorktreeReviewUnavailable>,
    active_build_context: Option<ActiveBuildContextView>,
    state: Mutex<ApplicationState>,
}

impl WorktreeReviewApplication {
    /// Opens a stable product application root. Environmental failures become capability facts.
    pub(crate) fn open(review_root: PathBuf) -> Self {
        let requested_root = review_root;
        let data_root = SharedWorktreeReviewDataRoot::resolve(requested_root.clone())
            .and_then(SharedWorktreeReviewDataRoot::prepare);
        let review_root = data_root
            .as_ref()
            .map(|root| root.build_output_store_root().to_path_buf())
            .unwrap_or(requested_root);
        let database = data_root
            .map_err(|_| storage_unavailable())
            .and_then(|root| {
                WorktreeReviewDatabase::open(root.database_path())
                    .map(Arc::new)
                    .map_err(|_| storage_unavailable())
            });
        let (selection, storage_available) = match &database {
            Ok(database) => match database.selection().load() {
                Ok(selection) => (selection, true),
                Err(_) => (None, false),
            },
            Err(_) => (None, false),
        };
        let repository_context = RepositoryContext::discover().map_err(map_repository_error);
        let active_build_context = database
            .as_ref()
            .ok()
            .and_then(|database| load_active_build_context(database));
        Self {
            review_root,
            database,
            repository_context,
            active_build_context,
            state: Mutex::new(ApplicationState {
                selection,
                storage_available,
                build_runtime: readiness(
                    CapabilityReadinessStatus::NotEvaluated,
                    "Build tooling is checked when a build operation needs it.",
                ),
            }),
        }
    }

    pub(crate) fn overview(&self) -> WorktreeReviewOverviewView {
        let state = match self.state.lock() {
            Ok(state) => state,
            Err(_) => return unavailable_overview(),
        };
        let build_output_storage = if state.storage_available {
            readiness(
                CapabilityReadinessStatus::Ready,
                "Worktree Review storage is ready.",
            )
        } else {
            readiness(
                CapabilityReadinessStatus::StorageUnavailable,
                "Worktree Review storage is unavailable.",
            )
        };
        let (selected_repository, repository_browsing) = match &state.selection {
            None => (
                None,
                readiness(
                    CapabilityReadinessStatus::NeedsRepository,
                    "Select a Git repository to browse branches and worktrees.",
                ),
            ),
            Some(selection) => match self.verify_selection(selection) {
                Ok(repository) => (
                    Some(view(&repository)),
                    readiness(
                        CapabilityReadinessStatus::Ready,
                        "The selected repository is ready.",
                    ),
                ),
                Err(error) => (
                    Some(SelectedRepositoryView {
                        repository_id: selection.repository_id.clone(),
                        root: selection.repository_root.to_string_lossy().into_owned(),
                    }),
                    readiness(error.status, error.message),
                ),
            },
        };
        WorktreeReviewOverviewView {
            selected_repository,
            capabilities: WorktreeReviewCapabilitiesView {
                repository_browsing,
                build_runtime: state.build_runtime.clone(),
                build_output_storage,
            },
        }
    }

    pub(crate) fn review_root(&self) -> &Path {
        &self.review_root
    }

    pub(crate) fn active_build_context(&self) -> Option<ActiveBuildContextView> {
        self.active_build_context.clone()
    }

    pub(crate) fn database(
        &self,
    ) -> Result<Arc<WorktreeReviewDatabase>, WorktreeReviewUnavailable> {
        self.database.clone()
    }

    pub(crate) fn select_repository(&self, candidate: PathBuf) -> SelectRepositoryResultView {
        let repository = match self.repository_context() {
            Ok(context) => match context.identities().inspect(&candidate) {
                Ok(repository) => repository,
                Err(error) => return self.selection_failed(map_repository_error(error)),
            },
            Err(error) => return self.selection_failed(error),
        };
        let selection = PersistedRepositorySelection {
            repository_id: repository.id.as_str().to_owned(),
            repository_root: repository.top_level.path().to_path_buf(),
        };
        if self
            .database()
            .and_then(|database| {
                database
                    .selection()
                    .save(&selection)
                    .map_err(|_| storage_unavailable())
            })
            .is_err()
        {
            if let Ok(mut state) = self.state.lock() {
                state.storage_available = false;
            }
            return self.selection_failed(WorktreeReviewUnavailable {
                status: CapabilityReadinessStatus::StorageUnavailable,
                message: "The repository selection could not be saved.".into(),
            });
        }
        if let Ok(mut state) = self.state.lock() {
            state.storage_available = true;
            state.selection = Some(selection);
            state.build_runtime = readiness(
                CapabilityReadinessStatus::NotEvaluated,
                "Build tooling is checked when a build operation needs it.",
            );
        }
        SelectRepositoryResultView {
            selection: readiness(
                CapabilityReadinessStatus::Ready,
                "The repository selection was saved.",
            ),
            overview: self.overview(),
        }
    }

    pub(crate) fn selected_repository(
        &self,
    ) -> Result<RepositoryIdentity, WorktreeReviewUnavailable> {
        let selection = self
            .state
            .lock()
            .map_err(|_| unavailable())?
            .selection
            .clone()
            .ok_or_else(|| WorktreeReviewUnavailable {
                status: CapabilityReadinessStatus::NeedsRepository,
                message: "Select a Git repository first.".into(),
            })?;
        self.verify_selection(&selection)
    }

    pub(crate) fn repository_context(
        &self,
    ) -> Result<RepositoryContext, WorktreeReviewUnavailable> {
        self.repository_context.clone()
    }

    fn verify_selection(
        &self,
        selection: &PersistedRepositorySelection,
    ) -> Result<RepositoryIdentity, WorktreeReviewUnavailable> {
        let repository = self
            .repository_context()?
            .identities()
            .inspect(&selection.repository_root)
            .map_err(map_repository_error)?;
        if repository.id.as_str() != selection.repository_id {
            return Err(WorktreeReviewUnavailable {
                status: CapabilityReadinessStatus::RepositoryUnavailable,
                message: "The saved repository identity no longer matches this location.".into(),
            });
        }
        Ok(repository)
    }

    fn selection_failed(&self, error: WorktreeReviewUnavailable) -> SelectRepositoryResultView {
        SelectRepositoryResultView {
            selection: readiness(error.status, error.message),
            overview: self.overview(),
        }
    }
}

fn load_active_build_context(database: &WorktreeReviewDatabase) -> Option<ActiveBuildContextView> {
    let build_id = ReviewBuildId::new(env::var(ACTIVE_REVIEW_BUILD_ID_ENV).ok()?).ok()?;
    let worktree_id = WorktreeId::new(env::var(ACTIVE_REVIEW_WORKTREE_ID_ENV).ok()?).ok()?;
    let build = database.builds().find(&build_id).ok()??;
    let workspace = database.workspaces().find(&build.workspace_id).ok()??;
    (workspace.worktree_id == worktree_id).then(|| ActiveBuildContextView {
        build_id: build_id.as_str().to_owned(),
        worktree_id: worktree_id.as_str().to_owned(),
    })
}

fn view(repository: &RepositoryIdentity) -> SelectedRepositoryView {
    SelectedRepositoryView {
        repository_id: repository.id.as_str().to_owned(),
        root: repository.top_level.path().to_string_lossy().into_owned(),
    }
}

fn readiness(
    status: CapabilityReadinessStatus,
    message: impl Into<String>,
) -> CapabilityReadinessView {
    CapabilityReadinessView {
        status,
        message: message.into(),
    }
}

fn map_repository_error(error: RepositoryContextError) -> WorktreeReviewUnavailable {
    let kind = error.kind;
    WorktreeReviewUnavailable {
        status: match kind {
            RepositoryContextErrorKind::MissingGit => CapabilityReadinessStatus::MissingTool,
            _ => CapabilityReadinessStatus::RepositoryUnavailable,
        },
        message: match kind {
            RepositoryContextErrorKind::MissingGit => "Git is required for Worktree Review.",
            _ => "The selected folder is not an available Git repository.",
        }
        .into(),
    }
}

fn unavailable_overview() -> WorktreeReviewOverviewView {
    let unavailable = readiness(
        CapabilityReadinessStatus::RuntimeUnavailable,
        "Worktree Review state is unavailable.",
    );
    WorktreeReviewOverviewView {
        selected_repository: None,
        capabilities: WorktreeReviewCapabilitiesView {
            repository_browsing: unavailable.clone(),
            build_runtime: unavailable.clone(),
            build_output_storage: unavailable,
        },
    }
}

fn unavailable() -> WorktreeReviewUnavailable {
    WorktreeReviewUnavailable {
        status: CapabilityReadinessStatus::RuntimeUnavailable,
        message: "Worktree Review state is unavailable.".into(),
    }
}

fn storage_unavailable() -> WorktreeReviewUnavailable {
    WorktreeReviewUnavailable {
        status: CapabilityReadinessStatus::StorageUnavailable,
        message: "Worktree Review storage is unavailable.".into(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::process::Command;

    #[test]
    fn host_process_uses_the_supplied_shared_data_root() {
        let directory = tempfile::tempdir().unwrap();
        let requested = directory.path().join("review");

        let resolved =
            SharedWorktreeReviewDataRoot::resolve_with_shared_root(requested.clone(), None)
                .unwrap()
                .prepare()
                .unwrap();

        assert_eq!(
            resolved.build_output_store_root(),
            requested.canonicalize().unwrap()
        );
        assert_eq!(
            resolved.database_path(),
            requested.canonicalize().unwrap().join(STATE_DATABASE_FILE)
        );
    }

    #[test]
    fn reviewed_build_uses_the_explicit_shared_data_root() {
        let directory = tempfile::tempdir().unwrap();
        let shared_root = directory.path().join("review");
        let private_root = directory.path().join("private-app-data/worktree-review");

        let resolved = SharedWorktreeReviewDataRoot::resolve_with_shared_root(
            private_root.clone(),
            Some(shared_root.clone()),
        )
        .unwrap()
        .prepare()
        .unwrap();

        assert_eq!(
            resolved.build_output_store_root(),
            shared_root.canonicalize().unwrap()
        );
        assert_eq!(
            resolved.database_path(),
            shared_root
                .canonicalize()
                .unwrap()
                .join(STATE_DATABASE_FILE)
        );
        assert!(!private_root.join(STATE_DATABASE_FILE).exists());
    }

    #[test]
    fn shared_data_root_contract_rejects_relative_paths() {
        let directory = tempfile::tempdir().unwrap();
        let result = SharedWorktreeReviewDataRoot::resolve_with_shared_root(
            directory.path().join("private-app-data/worktree-review"),
            Some(PathBuf::from("relative-review-data")),
        );

        assert!(result.is_err());
        assert!(!directory.path().join(STATE_DATABASE_FILE).exists());
    }

    #[test]
    fn missing_selection_is_a_recoverable_product_state() {
        let directory = tempfile::tempdir().unwrap();
        let application = WorktreeReviewApplication::open(directory.path().join("review"));
        assert_eq!(
            application
                .overview()
                .capabilities
                .repository_browsing
                .status,
            CapabilityReadinessStatus::NeedsRepository
        );
    }

    #[test]
    fn selection_is_replaced_durably_without_removing_repository_roots() {
        let directory = tempfile::tempdir().unwrap();
        let review_root = directory.path().join("review");
        std::fs::create_dir_all(&review_root).unwrap();
        let first = PersistedRepositorySelection {
            repository_id: "repository-first".into(),
            repository_root: directory.path().join("first"),
        };
        let second = PersistedRepositorySelection {
            repository_id: "repository-second".into(),
            repository_root: directory.path().join("second"),
        };
        let database = WorktreeReviewDatabase::open(review_root.join(STATE_DATABASE_FILE)).unwrap();
        database.selection().save(&first).unwrap();
        database.selection().save(&second).unwrap();
        assert_eq!(database.selection().load().unwrap(), Some(second));
        assert!(!review_root
            .join("repositories")
            .join("repository-first")
            .exists());
    }

    #[test]
    fn selected_repository_survives_restart_without_composing_build_tooling() {
        let directory = tempfile::tempdir().unwrap();
        let repository_root = directory.path().join("repository");
        std::fs::create_dir(&repository_root).unwrap();
        assert!(Command::new("git")
            .args(["init", "--quiet"])
            .current_dir(&repository_root)
            .status()
            .unwrap()
            .success());
        let review_root = directory.path().join("review");
        let application = WorktreeReviewApplication::open(review_root.clone());
        let selected = application.select_repository(repository_root.clone());
        assert_eq!(
            selected.overview.capabilities.repository_browsing.status,
            CapabilityReadinessStatus::Ready
        );
        assert_eq!(
            selected.overview.capabilities.build_runtime.status,
            CapabilityReadinessStatus::NotEvaluated
        );

        let reopened = WorktreeReviewApplication::open(review_root);
        let repository = reopened.selected_repository().unwrap();
        assert_eq!(
            repository.top_level.path(),
            repository_root.canonicalize().unwrap()
        );
    }
}
