use super::{
    compose_scoped, composition::WorktreeReviewCompositionError, service::WorktreeReviewService,
};
use crate::orchestration::initiated_sprint_git_authority::{
    BindInitiatedSprintGitAuthorityError, VerifiedRuntimeGitComparison,
    WorktreeRuntimeGitComparison,
};
use crate::repository_context::{RepositoryContext, RepositoryContextErrorKind};
use rusqlite::{params, Connection, OptionalExtension};
use serde::Serialize;
use std::{
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};

const SELECTION_DATABASE_FILE: &str = "worktree-review.sqlite";

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeReviewReadinessView {
    pub(crate) status: WorktreeReviewReadinessStatus,
    pub(crate) message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) repository_root: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) enum WorktreeReviewReadinessStatus {
    Ready,
    NeedsRepository,
    MissingTool,
    StorageUnavailable,
    RepositoryUnavailable,
    RuntimeUnavailable,
}

impl WorktreeReviewReadinessView {
    pub(super) fn runtime_unavailable() -> Self {
        Self::unavailable(
            WorktreeReviewReadinessStatus::RuntimeUnavailable,
            "Worktree Review state is unavailable.",
            None,
        )
    }

    fn needs_repository() -> Self {
        Self {
            status: WorktreeReviewReadinessStatus::NeedsRepository,
            message: "Select a Git repository to use Worktree Review.".into(),
            repository_root: None,
        }
    }

    fn unavailable(
        status: WorktreeReviewReadinessStatus,
        message: impl Into<String>,
        repository_root: Option<&Path>,
    ) -> Self {
        Self {
            status,
            message: message.into(),
            repository_root: repository_root.map(|path| path.to_string_lossy().into_owned()),
        }
    }

    fn ready(repository_root: &Path) -> Self {
        Self::unavailable(
            WorktreeReviewReadinessStatus::Ready,
            "Worktree Review is ready.",
            Some(repository_root),
        )
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
struct SelectedRepository {
    repository_id: String,
    repository_root: PathBuf,
}

struct WorktreeReviewStateInner {
    service: Option<Arc<WorktreeReviewService>>,
    selection: Option<SelectedRepository>,
    readiness: WorktreeReviewReadinessView,
}

#[derive(Clone)]
pub(crate) struct WorktreeReviewState {
    review_root: PathBuf,
    inner: Arc<Mutex<WorktreeReviewStateInner>>,
}

impl WorktreeReviewState {
    pub(crate) fn open(review_root: PathBuf) -> Self {
        let loaded = load_selection(&review_root);
        let (selection, readiness) = match loaded {
            Ok(Some(selection)) => {
                let readiness = WorktreeReviewReadinessView::unavailable(
                    WorktreeReviewReadinessStatus::Ready,
                    "Worktree Review repository is selected.",
                    Some(&selection.repository_root),
                );
                (Some(selection), readiness)
            }
            Ok(None) => (None, WorktreeReviewReadinessView::needs_repository()),
            Err(()) => (
                None,
                WorktreeReviewReadinessView::unavailable(
                    WorktreeReviewReadinessStatus::StorageUnavailable,
                    "Worktree Review storage is unavailable.",
                    None,
                ),
            ),
        };
        Self {
            review_root,
            inner: Arc::new(Mutex::new(WorktreeReviewStateInner {
                service: None,
                selection,
                readiness,
            })),
        }
    }

    pub(crate) fn readiness(&self) -> WorktreeReviewReadinessView {
        let _ = self.ensure_service();
        self.inner
            .lock()
            .map(|inner| inner.readiness.clone())
            .unwrap_or_else(|_| WorktreeReviewReadinessView::runtime_unavailable())
    }

    pub(crate) fn service(&self) -> Result<Arc<WorktreeReviewService>, String> {
        self.ensure_service()?;
        let (service, message) = self
            .inner
            .lock()
            .map(|inner| (inner.service.clone(), inner.readiness.message.clone()))
            .map_err(|_| "Worktree Review state is unavailable.".to_string())?;
        service.ok_or(message)
    }

    pub(crate) fn select_repository(
        &self,
        repository_root: PathBuf,
    ) -> WorktreeReviewReadinessView {
        let selection = match identify_repository(repository_root) {
            Ok(selection) => selection,
            Err(readiness) => return readiness,
        };
        let service = match compose_scoped(
            &selection.repository_root,
            &self.review_root,
            &self.repository_runtime_root(&selection.repository_id),
        ) {
            Ok(service) => Arc::new(service),
            Err(error) => {
                return readiness_for_composition_error(error, Some(&selection.repository_root))
            }
        };
        if save_selection(&self.review_root, &selection).is_err() {
            return WorktreeReviewReadinessView::unavailable(
                WorktreeReviewReadinessStatus::StorageUnavailable,
                "Worktree Review storage is unavailable.",
                Some(&selection.repository_root),
            );
        }
        let readiness = WorktreeReviewReadinessView::ready(&selection.repository_root);
        if let Ok(mut inner) = self.inner.lock() {
            inner.service = Some(service);
            inner.selection = Some(selection);
            inner.readiness = readiness.clone();
        }
        readiness
    }

    fn ensure_service(&self) -> Result<(), String> {
        let selection = {
            let inner = self
                .inner
                .lock()
                .map_err(|_| "Worktree Review state is unavailable.".to_string())?;
            if inner.service.is_some() {
                return Ok(());
            }
            inner
                .selection
                .clone()
                .ok_or_else(|| inner.readiness.message.clone())?
        };
        let verified_selection = match identify_repository(selection.repository_root.clone()) {
            Ok(verified) if verified.repository_id == selection.repository_id => verified,
            Ok(_) => {
                let readiness = WorktreeReviewReadinessView::unavailable(
                    WorktreeReviewReadinessStatus::RepositoryUnavailable,
                    "The saved Worktree Review repository identity no longer matches.",
                    Some(&selection.repository_root),
                );
                let message = readiness.message.clone();
                if let Ok(mut inner) = self.inner.lock() {
                    inner.readiness = readiness;
                }
                return Err(message);
            }
            Err(readiness) => {
                let message = readiness.message.clone();
                if let Ok(mut inner) = self.inner.lock() {
                    inner.readiness = readiness;
                }
                return Err(message);
            }
        };
        let result = compose_scoped(
            &verified_selection.repository_root,
            &self.review_root,
            &self.repository_runtime_root(&verified_selection.repository_id),
        );
        let mut inner = self
            .inner
            .lock()
            .map_err(|_| "Worktree Review state is unavailable.".to_string())?;
        match result {
            Ok(service) => {
                inner.service = Some(Arc::new(service));
                inner.readiness =
                    WorktreeReviewReadinessView::ready(&verified_selection.repository_root);
                Ok(())
            }
            Err(error) => {
                inner.readiness = readiness_for_composition_error(
                    error,
                    Some(&verified_selection.repository_root),
                );
                Err(inner.readiness.message.clone())
            }
        }
    }

    fn repository_runtime_root(&self, repository_id: &str) -> PathBuf {
        self.review_root.join("repositories").join(repository_id)
    }
}

impl WorktreeRuntimeGitComparison for WorktreeReviewState {
    fn resolve_verified_comparison(
        &self,
        runtime_instance_ref: &str,
    ) -> Result<VerifiedRuntimeGitComparison, BindInitiatedSprintGitAuthorityError> {
        self.service()
            .map_err(|_| BindInitiatedSprintGitAuthorityError::RuntimeSourceUnavailable)?
            .resolve_verified_comparison(runtime_instance_ref)
    }
}

fn identify_repository(
    repository_root: PathBuf,
) -> Result<SelectedRepository, WorktreeReviewReadinessView> {
    let context = RepositoryContext::discover_git()
        .map_err(|error| readiness_for_repository_error(error.kind, &repository_root))?;
    let repository = context
        .repository(&repository_root)
        .map_err(|error| readiness_for_repository_error(error.kind, &repository_root))?;
    Ok(SelectedRepository {
        repository_id: repository.id.as_str().to_owned(),
        repository_root: repository.top_level.path().to_path_buf(),
    })
}

fn readiness_for_repository_error(
    kind: RepositoryContextErrorKind,
    root: &Path,
) -> WorktreeReviewReadinessView {
    if kind == RepositoryContextErrorKind::MissingGit {
        WorktreeReviewReadinessView::unavailable(
            WorktreeReviewReadinessStatus::MissingTool,
            "Git is required for Worktree Review.",
            Some(root),
        )
    } else {
        WorktreeReviewReadinessView::unavailable(
            WorktreeReviewReadinessStatus::RepositoryUnavailable,
            "The selected folder is not an available Git repository.",
            Some(root),
        )
    }
}

fn open_selection_database(review_root: &Path) -> Result<Connection, ()> {
    std::fs::create_dir_all(review_root).map_err(|_| ())?;
    let connection = Connection::open(review_root.join(SELECTION_DATABASE_FILE)).map_err(|_| ())?;
    connection
        .execute_batch(
            "CREATE TABLE IF NOT EXISTS worktree_review_selection (
                singleton INTEGER PRIMARY KEY CHECK (singleton = 1),
                repository_id TEXT NOT NULL,
                repository_root TEXT NOT NULL
            );",
        )
        .map_err(|_| ())?;
    Ok(connection)
}

fn load_selection(review_root: &Path) -> Result<Option<SelectedRepository>, ()> {
    open_selection_database(review_root)?
        .query_row(
            "SELECT repository_id, repository_root
             FROM worktree_review_selection WHERE singleton = 1",
            [],
            |row| {
                Ok(SelectedRepository {
                    repository_id: row.get(0)?,
                    repository_root: PathBuf::from(row.get::<_, String>(1)?),
                })
            },
        )
        .optional()
        .map_err(|_| ())
}

fn save_selection(review_root: &Path, selection: &SelectedRepository) -> Result<(), ()> {
    open_selection_database(review_root)?
        .execute(
            "INSERT INTO worktree_review_selection (singleton, repository_id, repository_root)
             VALUES (1, ?1, ?2)
             ON CONFLICT(singleton) DO UPDATE SET
                repository_id = excluded.repository_id,
                repository_root = excluded.repository_root",
            params![
                &selection.repository_id,
                selection.repository_root.to_string_lossy().as_ref()
            ],
        )
        .map(|_| ())
        .map_err(|_| ())
}

fn readiness_for_composition_error(
    error: WorktreeReviewCompositionError,
    repository_root: Option<&Path>,
) -> WorktreeReviewReadinessView {
    let status = match error {
        WorktreeReviewCompositionError::MissingTool => WorktreeReviewReadinessStatus::MissingTool,
        WorktreeReviewCompositionError::RepositoryUnavailable => {
            WorktreeReviewReadinessStatus::RepositoryUnavailable
        }
        WorktreeReviewCompositionError::StorageUnavailable => {
            WorktreeReviewReadinessStatus::StorageUnavailable
        }
        WorktreeReviewCompositionError::RuntimeUnavailable => {
            WorktreeReviewReadinessStatus::RuntimeUnavailable
        }
    };
    WorktreeReviewReadinessView::unavailable(status, error.to_string(), repository_root)
}

#[cfg(test)]
mod tests {
    use super::{
        load_selection, save_selection, SelectedRepository, WorktreeReviewReadinessStatus,
        WorktreeReviewState,
    };

    #[test]
    fn missing_selection_is_a_recoverable_product_state() {
        let directory = tempfile::tempdir().expect("review root");
        let state = WorktreeReviewState::open(directory.path().join("runtime"));

        assert_eq!(
            state.readiness().status,
            WorktreeReviewReadinessStatus::NeedsRepository
        );
        assert!(state.service().is_err());
    }

    #[test]
    fn selection_storage_replaces_the_active_repository_without_losing_history_roots() {
        let directory = tempfile::tempdir().expect("review root");
        let review_root = directory.path().join("runtime");
        let first = SelectedRepository {
            repository_id: "repository-first".into(),
            repository_root: directory.path().join("first"),
        };
        let second = SelectedRepository {
            repository_id: "repository-second".into(),
            repository_root: directory.path().join("second"),
        };

        save_selection(&review_root, &first).expect("first selection");
        save_selection(&review_root, &second).expect("replacement selection");

        assert_eq!(load_selection(&review_root).unwrap(), Some(second));
        let state = WorktreeReviewState::open(review_root.clone());
        assert_eq!(
            state.repository_runtime_root(&first.repository_id),
            review_root.join("repositories").join(first.repository_id)
        );
    }
}
