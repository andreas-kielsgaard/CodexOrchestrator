use super::{
    catalog::{ReviewWorktreeCatalog, ReviewWorktreeOption},
    comparison::WorktreeComparisonView,
    detail::{assemble, now_ms, DetailInput, ReviewInstanceDetailView, ReviewLifecycleEventView},
    progress::{ProgressHandle, ProgressRegistry, ReviewOperationProgressView},
    proof_evidence::{self, ReviewBuildOperationEvidenceView},
    worktree_build::WorktreeBuildContextView,
};
use crate::worktree_runtime::{
    IsolatedTestRequest, TestActionOutcome, TestInstanceError, TestInstanceErrorKind,
    TestInstanceHandle, TestInstancePhase, TestInstanceStatus, TestSourceRef,
    WorktreeTestInstances,
};
use rusqlite::{params, Connection};
use serde::Serialize;
use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
};
use uuid::Uuid;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewSourceView {
    pub(crate) source_ref: String,
    pub(crate) label: String,
    pub(crate) revision: String,
    pub(crate) compatibility: String,
    pub(crate) compatibility_message: String,
}

impl From<&ReviewWorktreeOption> for ReviewSourceView {
    fn from(value: &ReviewWorktreeOption) -> Self {
        Self {
            source_ref: value.source_ref.clone(),
            label: value.label.clone(),
            revision: value.revision.clone(),
            compatibility: value.compatibility.clone(),
            compatibility_message: value.compatibility_message.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewInstanceView {
    pub(crate) instance_ref: String,
    pub(crate) name: String,
    pub(crate) source_label: String,
    pub(crate) phase: String,
    pub(crate) health: String,
    pub(crate) stale: bool,
    pub(crate) build: String,
    pub(crate) can_focus: bool,
    pub(crate) purpose: String,
    pub(crate) current_use: String,
    pub(crate) retention: String,
    pub(crate) cleanup: String,
    pub(crate) action_required: bool,
    pub(crate) action_summary: String,
    pub(crate) compatibility: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AcceptedReviewOperationView {
    pub(crate) operation_ref: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewOperationStatusView {
    pub(crate) progress: ReviewOperationProgressView,
    pub(crate) result: Option<ReviewInstanceView>,
    pub(crate) error: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LauncherDetailNavigationView {
    pub(crate) instance_ref: String,
    pub(crate) sequence: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct LauncherProofPresentationView {
    pub(crate) route: String,
    pub(crate) origin: String,
    pub(crate) instance_ref: Option<String>,
    pub(crate) operation_ref: Option<String>,
    pub(crate) source_ref: Option<String>,
    pub(crate) sequence: String,
}

#[derive(Clone)]
struct ReviewMetadata {
    name: String,
    source_ref: String,
    source_label: String,
}

#[derive(Clone)]
enum ReviewOperationResult {
    Pending,
    Succeeded(ReviewInstanceView),
    Failed(String),
}

pub(crate) struct HumanReviewLauncherService {
    runtime: Arc<dyn WorktreeTestInstances>,
    catalog: Arc<ReviewWorktreeCatalog>,
    instances: Mutex<HashMap<String, ReviewMetadata>>,
    built: Mutex<HashSet<String>>,
    store: Mutex<Connection>,
    progress: Arc<ProgressRegistry>,
    operation_results: Mutex<HashMap<String, ReviewOperationResult>>,
    launcher_proof_navigation: Mutex<Option<String>>,
    launcher_detail_navigation: Mutex<Option<LauncherDetailNavigationView>>,
    launcher_proof_presentation: Mutex<Option<LauncherProofPresentationView>>,
    instances_root: PathBuf,
}

impl HumanReviewLauncherService {
    pub(crate) fn new(
        runtime: Arc<dyn WorktreeTestInstances>,
        catalog: Arc<ReviewWorktreeCatalog>,
        store_path: &Path,
        instances_root: PathBuf,
    ) -> Result<Self, String> {
        let store = Connection::open(store_path)
            .map_err(|error| format!("open review launcher state: {error}"))?;
        store
            .execute_batch(
                "CREATE TABLE IF NOT EXISTS review_sessions (
                instance_ref TEXT PRIMARY KEY,
                name TEXT NOT NULL,
                source_ref TEXT NOT NULL DEFAULT '',
                source_label TEXT NOT NULL,
                built INTEGER NOT NULL CHECK (built IN (0, 1))
            );
            CREATE TABLE IF NOT EXISTS review_history (
                event_id INTEGER PRIMARY KEY AUTOINCREMENT,
                instance_ref TEXT NOT NULL,
                occurred_at_ms INTEGER NOT NULL,
                kind TEXT NOT NULL,
                summary TEXT NOT NULL
            );
            CREATE INDEX IF NOT EXISTS idx_review_history_instance
                ON review_history(instance_ref, event_id);",
            )
            .map_err(|error| format!("initialize review launcher state: {error}"))?;
        let has_source_ref = {
            let mut statement = store
                .prepare("PRAGMA table_info(review_sessions)")
                .map_err(|error| format!("inspect review launcher state: {error}"))?;
            let columns = statement
                .query_map([], |row| row.get::<_, String>(1))
                .map_err(|error| format!("inspect review launcher columns: {error}"))?;
            columns
                .collect::<Result<Vec<_>, _>>()
                .map_err(|error| format!("read review launcher columns: {error}"))?
                .iter()
                .any(|column| column == "source_ref")
        };
        if !has_source_ref {
            store
                .execute(
                    "ALTER TABLE review_sessions ADD COLUMN source_ref TEXT NOT NULL DEFAULT ''",
                    [],
                )
                .map_err(|error| format!("migrate review launcher state: {error}"))?;
        }
        let (instances, built) = load_sessions(&store)?;
        Ok(Self {
            runtime,
            catalog,
            instances: Mutex::new(instances),
            built: Mutex::new(built),
            store: Mutex::new(store),
            progress: Arc::new(ProgressRegistry::system()),
            operation_results: Mutex::new(HashMap::new()),
            launcher_proof_navigation: Mutex::new(None),
            launcher_detail_navigation: Mutex::new(None),
            launcher_proof_presentation: Mutex::new(None),
            instances_root,
        })
    }

    pub(crate) fn sources(&self) -> Vec<ReviewSourceView> {
        self.catalog
            .options()
            .iter()
            .map(ReviewSourceView::from)
            .collect()
    }

    pub(crate) fn instances(&self) -> Vec<ReviewInstanceView> {
        let refs = self
            .instances
            .lock()
            .map(|instances| instances.keys().cloned().collect::<Vec<_>>())
            .unwrap_or_default();
        refs.into_iter()
            .filter_map(|instance_ref| self.status(instance_ref).ok())
            .collect()
    }

    pub(crate) fn prepare(
        &self,
        operation_ref: String,
        source_ref: String,
        name: String,
    ) -> Result<ReviewInstanceView, String> {
        let progress = self.progress.begin(
            &operation_ref,
            format!("prepare:{source_ref}"),
            "prepare",
            "preparation",
            "Preparing isolated review material",
        )?;
        self.prepare_with_progress(progress, source_ref, name)
    }

    fn prepare_with_progress(
        &self,
        progress: ProgressHandle,
        source_ref: String,
        name: String,
    ) -> Result<ReviewInstanceView, String> {
        progress.update(
            "preparation",
            "Preparing isolated review material",
            Some("Resolving the selected worktree and projecting isolated mutable state."),
        );
        let result = (|| {
            let source_label = self
                .catalog
                .label(&source_ref)
                .ok_or_else(|| "The selected worktree is unavailable.".to_string())?;
            let requested = self
                .runtime
                .request(
                    IsolatedTestRequest::new(
                        TestSourceRef::new(source_ref.clone()).map_err(safe_error)?,
                        name.clone(),
                    )
                    .map_err(safe_error)?,
                )
                .map_err(safe_error)?;
            let instance_ref = requested.handle.opaque_ref().to_owned();
            self.instances
                .lock()
                .map_err(|_| "Review instance state is unavailable.".to_string())?
                .insert(
                    instance_ref.clone(),
                    ReviewMetadata {
                        name: name.clone(),
                        source_ref: source_ref.clone(),
                        source_label: source_label.clone(),
                    },
                );
            self.persist(&instance_ref, &name, &source_ref, &source_label, false)?;
            Ok(view(
                instance_ref,
                name,
                source_label,
                requested.status,
                "not-built",
                &self.catalog.compatibility(&source_ref).0,
            ))
        })();
        if let Ok(value) = &result {
            self.record_event(
                &value.instance_ref,
                "Prepared",
                "Reserved isolated mutable roots, ports, logs, application data, and lifecycle ownership.",
            )?;
        }
        finish_progress(&progress, result)
    }

    pub(crate) fn build(
        &self,
        operation_ref: String,
        instance_ref: String,
    ) -> Result<ReviewInstanceView, String> {
        let progress = self.progress.begin(
            &operation_ref,
            format!("build:{instance_ref}"),
            "build",
            "preparation",
            "Checking source and build inputs",
        )?;
        self.build_with_progress(progress, instance_ref)
    }

    fn build_with_progress(
        &self,
        progress: ProgressHandle,
        instance_ref: String,
    ) -> Result<ReviewInstanceView, String> {
        let result = (|| {
            let (handle, metadata) = self.resolve(&instance_ref)?;
            if let Err(error) = self.catalog.ensure_compatible(&metadata.source_ref) {
                progress.fail_with(
                    "compatibility",
                    "Selected worktree is not review-compatible",
                    Some(&error),
                );
                return Err(error);
            }
            let result = self
                .runtime
                .build_with_progress(&handle, &progress)
                .map_err(safe_error)?;
            let build = match result.outcome {
                TestActionOutcome::Passed => {
                    self.built
                        .lock()
                        .map_err(|_| "Review build state is unavailable.".to_string())?
                        .insert(instance_ref.clone());
                    self.persist(
                        &instance_ref,
                        &metadata.name,
                        &metadata.source_ref,
                        &metadata.source_label,
                        true,
                    )?;
                    "passed"
                }
                TestActionOutcome::Failed => "failed",
            };
            Ok(view(
                instance_ref,
                metadata.name,
                metadata.source_label,
                result.status,
                build,
                &self.catalog.compatibility(&metadata.source_ref).0,
            ))
        })();
        if let Ok(value) = &result {
            self.record_event(
                &value.instance_ref,
                "Built",
                "Verified the private executable and frontend output. Compilation is skipped only when the exact identity and artifact hashes already match.",
            )?;
        }
        finish_progress(&progress, result)
    }

    pub(crate) fn start(
        &self,
        operation_ref: String,
        instance_ref: String,
    ) -> Result<ReviewInstanceView, String> {
        let progress = self.progress.begin(
            &operation_ref,
            format!("start:{instance_ref}"),
            "start",
            "reservation",
            "Reserving the review instance",
        )?;
        self.start_with_progress(progress, instance_ref, true)
    }

    fn start_with_progress(
        &self,
        progress: ProgressHandle,
        instance_ref: String,
        activate_when_ready: bool,
    ) -> Result<ReviewInstanceView, String> {
        if !self
            .built
            .lock()
            .map_err(|_| "Review build state is unavailable.".to_string())?
            .contains(&instance_ref)
        {
            progress.fail();
            return Err("Build this review instance successfully before opening it.".into());
        }
        let (_, metadata) = self.resolve(&instance_ref)?;
        if let Err(error) = self.catalog.ensure_compatible(&metadata.source_ref) {
            progress.fail_with(
                "compatibility",
                "Selected worktree is not review-compatible",
                Some(&error),
            );
            return Err(error);
        }
        let result = self.lifecycle(instance_ref, |runtime, handle| {
            runtime.start_with_progress(handle, &progress)
        });
        let result = result.and_then(|view| {
            if activate_when_ready {
                self.focus(view.instance_ref.clone())
            } else {
                Ok(view)
            }
        });
        if let Ok(value) = &result {
            self.record_event(
                &value.instance_ref,
                "Opened",
                "Established the exact owned, titled, visible worktree-build window and rendered application readiness marker.",
            )?;
        }
        finish_progress(&progress, result)
    }

    pub(crate) fn begin_prepare(
        self: &Arc<Self>,
        source_ref: String,
        name: String,
    ) -> Result<AcceptedReviewOperationView, String> {
        let operation_ref = fresh_operation_ref();
        let progress = self.progress.begin(
            &operation_ref,
            format!("prepare:{source_ref}"),
            "prepare",
            "preparation",
            "Preparing isolated review material",
        )?;
        self.spawn_operation(operation_ref, move |service| {
            service.prepare_with_progress(progress, source_ref, name)
        })
    }

    pub(crate) fn begin_build(
        self: &Arc<Self>,
        instance_ref: String,
    ) -> Result<AcceptedReviewOperationView, String> {
        let operation_ref = fresh_operation_ref();
        let progress = self.progress.begin(
            &operation_ref,
            format!("build:{instance_ref}"),
            "build",
            "preparation",
            "Checking source and build inputs",
        )?;
        self.spawn_operation(operation_ref, move |service| {
            service.build_with_progress(progress, instance_ref)
        })
    }

    pub(crate) fn begin_open(
        self: &Arc<Self>,
        instance_ref: String,
        activate_when_ready: bool,
    ) -> Result<AcceptedReviewOperationView, String> {
        let operation_ref = fresh_operation_ref();
        let progress = self.progress.begin(
            &operation_ref,
            format!("start:{instance_ref}"),
            "start",
            "reservation",
            "Reserving the review instance",
        )?;
        self.spawn_operation(operation_ref, move |service| {
            service.start_with_progress(progress, instance_ref, activate_when_ready)
        })
    }

    fn spawn_operation(
        self: &Arc<Self>,
        operation_ref: String,
        operation: impl FnOnce(Arc<Self>) -> Result<ReviewInstanceView, String> + Send + 'static,
    ) -> Result<AcceptedReviewOperationView, String> {
        self.operation_results
            .lock()
            .map_err(|_| "Review operation state is unavailable.".to_string())?
            .insert(operation_ref.clone(), ReviewOperationResult::Pending);
        let service = self.clone();
        let result_ref = operation_ref.clone();
        let spawn = std::thread::Builder::new()
            .name("worktree-review-operation".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    operation(service.clone())
                }))
                .unwrap_or_else(|_| {
                    service.progress.fail_operation(&result_ref);
                    Err("The review operation ended unexpectedly.".into())
                });
                let terminal = match result {
                    Ok(value) => ReviewOperationResult::Succeeded(value),
                    Err(error) => ReviewOperationResult::Failed(error),
                };
                if let Ok(mut results) = service.operation_results.lock() {
                    results.insert(result_ref, terminal);
                }
            });
        if spawn.is_err() {
            self.progress.fail_operation(&operation_ref);
            self.operation_results
                .lock()
                .map_err(|_| "Review operation state is unavailable.".to_string())?
                .insert(
                    operation_ref,
                    ReviewOperationResult::Failed(
                        "The review operation could not be started.".into(),
                    ),
                );
            return Err("The review operation could not be started.".into());
        }
        Ok(AcceptedReviewOperationView { operation_ref })
    }

    pub(crate) fn status(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        self.lifecycle(instance_ref, |runtime, handle| runtime.status(handle))
    }

    pub(crate) fn focus(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        self.lifecycle(instance_ref, |runtime, handle| runtime.focus(handle))
    }

    pub(crate) fn stop(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        let result = self.lifecycle(instance_ref, |runtime, handle| runtime.stop(handle))?;
        self.record_event(
            &result.instance_ref,
            "Stopped",
            "Stopped only the exact owned child process tree; retained build outputs and isolated data remain.",
        )?;
        Ok(result)
    }

    pub(crate) fn recover(&self, instance_ref: String) -> Result<ReviewInstanceView, String> {
        let result = self.lifecycle(instance_ref, |runtime, handle| runtime.recover(handle))?;
        self.record_event(
            &result.instance_ref,
            "Recovered",
            "Reconciled stale lifecycle ownership without deleting retained build or application state.",
        )?;
        Ok(result)
    }

    pub(crate) fn operation_progress(
        &self,
        operation_ref: String,
    ) -> Result<ReviewOperationProgressView, String> {
        self.progress.get(&operation_ref)
    }

    pub(crate) fn operations(&self) -> Vec<ReviewOperationProgressView> {
        self.progress.list()
    }

    pub(crate) fn operation_status(
        &self,
        operation_ref: String,
    ) -> Result<ReviewOperationStatusView, String> {
        let progress = self.progress.get(&operation_ref)?;
        let result = self
            .operation_results
            .lock()
            .map_err(|_| "Review operation state is unavailable.".to_string())?
            .get(&operation_ref)
            .cloned()
            .ok_or_else(|| "The review operation is unavailable.".to_string())?;
        let (result, error) = match result {
            ReviewOperationResult::Pending => (None, None),
            ReviewOperationResult::Succeeded(value) => (Some(value), None),
            ReviewOperationResult::Failed(error) => (None, Some(error)),
        };
        Ok(ReviewOperationStatusView {
            progress,
            result,
            error,
        })
    }

    pub(crate) fn context(&self, instance_ref: String) -> Result<WorktreeBuildContextView, String> {
        let (_, metadata) = self.resolve(&instance_ref)?;
        self.catalog
            .scope(&metadata.source_ref, metadata.name)?
            .context()
    }

    pub(crate) fn comparison(
        &self,
        instance_ref: String,
    ) -> Result<WorktreeComparisonView, String> {
        let (_, metadata) = self.resolve(&instance_ref)?;
        super::comparison::comparison(&self.catalog.scope(&metadata.source_ref, metadata.name)?)
    }

    pub(crate) fn detail(&self, instance_ref: String) -> Result<ReviewInstanceDetailView, String> {
        let (_, metadata) = self.resolve(&instance_ref)?;
        let status = self.status(instance_ref.clone())?;
        let context = self
            .catalog
            .scope(&metadata.source_ref, metadata.name.clone())?
            .context()?;
        let (compatibility, compatibility_message) =
            self.catalog.compatibility(&metadata.source_ref);
        Ok(assemble(DetailInput {
            instance_ref: instance_ref.clone(),
            name: metadata.name,
            source_label: metadata.source_label,
            phase: status.phase,
            health: status.health,
            stale: status.stale,
            build: status.build,
            compatibility,
            compatibility_message,
            context,
            instance_root: self.instances_root.join(&instance_ref),
            lifecycle_history: self.history(&instance_ref)?,
            operations: self.progress.history_for_instance(&instance_ref),
        }))
    }

    pub(crate) fn proof_navigate(&self, instance_ref: String, route: &str) -> Result<(), String> {
        self.resolve(&instance_ref)?;
        if !matches!(
            route,
            "application"
                | "widget-expanded"
                | "widget-minimized"
                | "widget-restored"
                | "widget-build-details"
                | "worktree-details"
                | "file-review"
        ) {
            return Err("That proof surface is unavailable.".into());
        }
        let target = self
            .instances_root
            .join(&instance_ref)
            .join("app-data")
            .join("debug-proof-navigation.json");
        let parent = target
            .parent()
            .ok_or_else(|| "Proof navigation storage is unavailable.".to_string())?;
        fs::create_dir_all(parent)
            .map_err(|_| "Proof navigation storage is unavailable.".to_string())?;
        let temporary = target.with_extension("pending");
        let body = serde_json::json!({
            "route": route,
            "sequence": Uuid::new_v4().simple().to_string(),
        });
        fs::write(
            &temporary,
            serde_json::to_vec(&body)
                .map_err(|_| "Proof navigation could not be encoded.".to_string())?,
        )
        .and_then(|_| fs::rename(&temporary, &target))
        .map_err(|_| "Proof navigation could not be recorded.".to_string())
    }

    pub(crate) fn proof_navigate_launcher(&self) -> Result<(), String> {
        *self
            .launcher_proof_navigation
            .lock()
            .map_err(|_| "Launcher proof navigation is unavailable.".to_string())? =
            Some("worktree-review".into());
        self.set_launcher_presentation("overview", "launcher", None, None, None)?;
        Ok(())
    }

    pub(crate) fn launcher_proof_navigation(&self) -> Result<Option<String>, String> {
        self.launcher_proof_navigation
            .lock()
            .map(|route| route.clone())
            .map_err(|_| "Launcher proof navigation is unavailable.".to_string())
    }

    pub(crate) fn proof_navigate_launcher_detail(
        &self,
        instance_ref: String,
    ) -> Result<(), String> {
        self.resolve(&instance_ref)?;
        self.proof_navigate_launcher()?;
        *self
            .launcher_detail_navigation
            .lock()
            .map_err(|_| "Launcher detail proof navigation is unavailable.".to_string())? =
            Some(LauncherDetailNavigationView {
                instance_ref: instance_ref.clone(),
                sequence: Uuid::new_v4().simple().to_string(),
            });
        self.set_launcher_presentation(
            "details",
            "retained-build-card",
            Some(instance_ref),
            None,
            None,
        )?;
        Ok(())
    }

    pub(crate) fn proof_navigate_launcher_operation(
        &self,
        instance_ref: String,
        operation_ref: String,
    ) -> Result<(), String> {
        let detail = self.detail(instance_ref.clone())?;
        if !detail
            .operations
            .iter()
            .any(|operation| operation.operation_ref == operation_ref)
        {
            return Err("The retained operation output is unavailable for this build.".into());
        }
        self.proof_navigate_launcher()?;
        self.set_launcher_presentation(
            "details",
            "retained-operation-output",
            Some(instance_ref),
            Some(operation_ref),
            None,
        )
    }

    pub(crate) fn proof_select_launcher_source(&self, source_ref: String) -> Result<(), String> {
        if self.catalog.label(&source_ref).is_none() {
            return Err("The selected worktree is unavailable.".into());
        }
        self.proof_navigate_launcher()?;
        self.set_launcher_presentation(
            "overview",
            "selected-worktree",
            None,
            None,
            Some(source_ref),
        )
    }

    pub(crate) fn launcher_detail_navigation(
        &self,
    ) -> Result<Option<LauncherDetailNavigationView>, String> {
        self.launcher_detail_navigation
            .lock()
            .map(|route| route.clone())
            .map_err(|_| "Launcher detail proof navigation is unavailable.".to_string())
    }

    pub(crate) fn launcher_proof_presentation(
        &self,
    ) -> Result<Option<LauncherProofPresentationView>, String> {
        self.launcher_proof_presentation
            .lock()
            .map(|presentation| presentation.clone())
            .map_err(|_| "Launcher proof presentation is unavailable.".to_string())
    }

    pub(crate) fn proof_build_operation_evidence(
        &self,
        operation_ref: String,
    ) -> Result<ReviewBuildOperationEvidenceView, String> {
        let (instance_ref, operation) = self.progress.history(&operation_ref)?;
        let instance_ref = instance_ref.ok_or_else(|| {
            "The Build operation is not associated with a retained instance.".to_string()
        })?;
        let registry_path = self
            .instances_root
            .parent()
            .ok_or_else(|| "The isolated review registry location is invalid.".to_string())?
            .join("registry.sqlite");
        proof_evidence::assemble(registry_path, instance_ref, operation)
    }

    fn set_launcher_presentation(
        &self,
        route: &str,
        origin: &str,
        instance_ref: Option<String>,
        operation_ref: Option<String>,
        source_ref: Option<String>,
    ) -> Result<(), String> {
        *self
            .launcher_proof_presentation
            .lock()
            .map_err(|_| "Launcher proof presentation is unavailable.".to_string())? =
            Some(LauncherProofPresentationView {
                route: route.into(),
                origin: origin.into(),
                instance_ref,
                operation_ref,
                source_ref,
                sequence: Uuid::new_v4().simple().to_string(),
            });
        Ok(())
    }

    fn lifecycle(
        &self,
        instance_ref: String,
        operation: impl FnOnce(
            &dyn WorktreeTestInstances,
            &TestInstanceHandle,
        ) -> Result<TestInstanceStatus, TestInstanceError>,
    ) -> Result<ReviewInstanceView, String> {
        let (handle, metadata) = self.resolve(&instance_ref)?;
        let status = operation(self.runtime.as_ref(), &handle).map_err(safe_error)?;
        let built = self
            .built
            .lock()
            .map_err(|_| "Review build state is unavailable.".to_string())?
            .contains(&instance_ref);
        let build = if !built {
            "not-built"
        } else if !status.source_current {
            "superseded"
        } else if !status.build_reusable {
            "rebuild-required"
        } else {
            "passed"
        };
        Ok(view(
            instance_ref,
            metadata.name,
            metadata.source_label,
            status,
            build,
            &self.catalog.compatibility(&metadata.source_ref).0,
        ))
    }

    fn resolve(&self, instance_ref: &str) -> Result<(TestInstanceHandle, ReviewMetadata), String> {
        let metadata = self
            .instances
            .lock()
            .map_err(|_| "Review instance state is unavailable.".to_string())?
            .get(instance_ref)
            .map(|value| ReviewMetadata {
                name: value.name.clone(),
                source_ref: value.source_ref.clone(),
                source_label: value.source_label.clone(),
            })
            .ok_or_else(|| {
                "Prepare this review instance again in the current launcher session.".to_string()
            })?;
        Ok((
            TestInstanceHandle::from_opaque(instance_ref.to_owned()).map_err(safe_error)?,
            metadata,
        ))
    }

    fn persist(
        &self,
        instance_ref: &str,
        name: &str,
        source_ref: &str,
        source_label: &str,
        built: bool,
    ) -> Result<(), String> {
        self.store
            .lock()
            .map_err(|_| "Review launcher state is unavailable.".to_string())?
            .execute(
                "INSERT INTO review_sessions (instance_ref, name, source_ref, source_label, built)
                 VALUES (?1, ?2, ?3, ?4, ?5)
                 ON CONFLICT(instance_ref) DO UPDATE SET
                    name = excluded.name,
                    source_ref = excluded.source_ref,
                    source_label = excluded.source_label,
                    built = excluded.built",
                params![
                    instance_ref,
                    name,
                    source_ref,
                    source_label,
                    i64::from(built)
                ],
            )
            .map_err(|_| "Review launcher state could not be saved.".to_string())?;
        Ok(())
    }

    fn record_event(&self, instance_ref: &str, kind: &str, summary: &str) -> Result<(), String> {
        self.store
            .lock()
            .map_err(|_| "Review launcher history is unavailable.".to_string())?
            .execute(
                "INSERT INTO review_history (instance_ref, occurred_at_ms, kind, summary)
                 VALUES (?1, ?2, ?3, ?4)",
                params![instance_ref, now_ms() as i64, kind, summary],
            )
            .map_err(|_| "Review launcher history could not be saved.".to_string())?;
        Ok(())
    }

    fn history(&self, instance_ref: &str) -> Result<Vec<ReviewLifecycleEventView>, String> {
        let store = self
            .store
            .lock()
            .map_err(|_| "Review launcher history is unavailable.".to_string())?;
        let mut statement = store
            .prepare(
                "SELECT occurred_at_ms, kind, summary FROM review_history
                 WHERE instance_ref = ?1 ORDER BY event_id",
            )
            .map_err(|_| "Review launcher history is unavailable.".to_string())?;
        let rows = statement
            .query_map([instance_ref], |row| {
                Ok(ReviewLifecycleEventView {
                    occurred_at_ms: row.get::<_, i64>(0)?.max(0) as u64,
                    kind: row.get(1)?,
                    summary: row.get(2)?,
                })
            })
            .map_err(|_| "Review launcher history is unavailable.".to_string())?;
        rows.collect::<Result<Vec<_>, _>>()
            .map_err(|_| "Review launcher history is unavailable.".to_string())
    }
}

fn finish_progress(
    progress: &super::progress::ProgressHandle,
    result: Result<ReviewInstanceView, String>,
) -> Result<ReviewInstanceView, String> {
    if result.is_ok() {
        progress.succeed();
    } else {
        progress.fail_with(
            "failed",
            "The operation ended before all required evidence was established",
            result.as_ref().err().map(String::as_str),
        );
    }
    result
}

fn load_sessions(
    store: &Connection,
) -> Result<(HashMap<String, ReviewMetadata>, HashSet<String>), String> {
    let mut statement = store
        .prepare("SELECT instance_ref, name, source_ref, source_label, built FROM review_sessions")
        .map_err(|error| format!("read review launcher state: {error}"))?;
    let rows = statement
        .query_map([], |row| {
            Ok((
                row.get::<_, String>(0)?,
                ReviewMetadata {
                    name: row.get(1)?,
                    source_ref: row.get(2)?,
                    source_label: row.get(3)?,
                },
                row.get::<_, i64>(4)? != 0,
            ))
        })
        .map_err(|error| format!("read review launcher sessions: {error}"))?;
    let mut instances = HashMap::new();
    let mut built = HashSet::new();
    for row in rows {
        let (instance_ref, metadata, was_built) =
            row.map_err(|error| format!("decode review launcher session: {error}"))?;
        if was_built {
            built.insert(instance_ref.clone());
        }
        instances.insert(instance_ref, metadata);
    }
    Ok((instances, built))
}

fn fresh_operation_ref() -> String {
    format!("review-operation-{}", Uuid::new_v4().simple())
}

fn view(
    instance_ref: String,
    name: String,
    source_label: String,
    status: TestInstanceStatus,
    build: &str,
    compatibility: &str,
) -> ReviewInstanceView {
    let (current_use, action_required, action_summary) =
        instance_guidance(&status, build, compatibility);
    ReviewInstanceView {
        instance_ref,
        name,
        source_label,
        phase: phase(status.phase).into(),
        health: format!("{:?}", status.health).to_lowercase(),
        stale: status.stale,
        build: build.into(),
        can_focus: status.phase == TestInstancePhase::Running
            && status.health == crate::worktree_runtime::HealthState::Healthy
            && !status.stale,
        purpose: "A retained isolated build for human review of one selected worktree.".into(),
        current_use,
        retention: "Retained".into(),
        cleanup: "Stop closes only its owned process tree. Outputs and isolated data remain until deliberate developer cleanup; automatic pruning is not implemented.".into(),
        action_required,
        action_summary,
        compatibility: compatibility.into(),
    }
}

fn instance_guidance(
    status: &TestInstanceStatus,
    build: &str,
    compatibility: &str,
) -> (String, bool, String) {
    if compatibility == "incompatible" {
        return (
            "Source incompatible".into(),
            true,
            "Choose or update to a worktree with the required review child contract.".into(),
        );
    }
    if build == "superseded" {
        return (
            "Source changed since this build".into(),
            true,
            "Prepare a fresh instance for the selected worktree; this retained build remains inspectable but cannot be opened as current source.".into(),
        );
    }
    if build == "rebuild-required" {
        return (
            "Build verification expired".into(),
            true,
            "Run Build again to verify the exact private executable and frontend output before Open.".into(),
        );
    }
    if status.stale || status.health == crate::worktree_runtime::HealthState::Unhealthy {
        return (
            "Needs recovery".into(),
            true,
            "Recover this instance before attempting another Open.".into(),
        );
    }
    if status.phase == TestInstancePhase::Running {
        return (
            "Human review window open".into(),
            false,
            "Review or Focus the child; Stop closes only this instance.".into(),
        );
    }
    if build == "passed" {
        return (
            "Verified build retained".into(),
            false,
            "Open the verified build, or Build to verify exact reuse.".into(),
        );
    }
    (
        "Prepared, not running".into(),
        false,
        "Build the selected source before Open becomes available.".into(),
    )
}

fn phase(value: TestInstancePhase) -> &'static str {
    match value {
        TestInstancePhase::Prepared => "prepared",
        TestInstancePhase::Starting => "starting",
        TestInstancePhase::Running => "running",
        TestInstancePhase::Stopping => "stopping",
        TestInstancePhase::Stopped => "stopped",
        TestInstancePhase::Recovering => "recovering",
        TestInstancePhase::Recovered => "recovered",
    }
}

fn safe_error(error: TestInstanceError) -> String {
    let readiness_failure = error.kind == TestInstanceErrorKind::Unavailable && {
        let message = error.message.to_ascii_lowercase();
        message.contains("window") || message.contains("readiness")
    };
    if readiness_failure {
        return "The owned process or supporting services did not establish the exact titled, visible, useful-size worktree-build window and rendered application marker. The verified build remains reusable; Stop or Recover the owned tree, then retry Open."
            .into();
    }
    match error.kind {
        TestInstanceErrorKind::InvalidRequest => "The review request is invalid.".into(),
        TestInstanceErrorKind::NotFound => "The review instance is no longer available.".into(),
        TestInstanceErrorKind::Unauthorized => {
            "Review instance ownership could not be verified.".into()
        }
        TestInstanceErrorKind::InvalidState => {
            "That action is not available in the current lifecycle state.".into()
        }
        TestInstanceErrorKind::OperationInProgress => {
            "Another lifecycle action is still in progress.".into()
        }
        TestInstanceErrorKind::Conflict => {
            "The selected source or review instance changed; prepare a fresh instance.".into()
        }
        TestInstanceErrorKind::BuildRequired => {
            "This retained build is no longer an exact verified artifact. Run Build again; its isolated data and prior output remain available.".into()
        }
        TestInstanceErrorKind::Unavailable => {
            "The isolated review runtime could not complete this action. Check its private logs."
                .into()
        }
    }
}

#[cfg(test)]
mod guidance_tests {
    use super::*;

    #[test]
    fn window_readiness_failure_explains_missing_evidence_reuse_and_recovery() {
        let message = safe_error(TestInstanceError::new(
            TestInstanceErrorKind::Unavailable,
            "owned processes were observed, but a usable application-ready worktree-build window did not appear before the readiness limit",
        ));

        assert!(message.contains("exact titled, visible, useful-size"));
        assert!(message.contains("rendered application marker"));
        assert!(message.contains("verified build remains reusable"));
        assert!(message.contains("Stop or Recover"));
        assert!(!message.contains("private logs"));
    }

    #[test]
    fn retained_invalidated_builds_name_the_safe_next_action() {
        let status = TestInstanceStatus {
            phase: TestInstancePhase::Stopped,
            health: crate::worktree_runtime::HealthState::Closed,
            stale: false,
            source_current: false,
            build_reusable: false,
        };
        let (use_summary, action_required, action) =
            instance_guidance(&status, "superseded", "compatible");
        assert_eq!(use_summary, "Source changed since this build");
        assert!(action_required);
        assert!(action.contains("Prepare a fresh instance"));

        let (use_summary, action_required, action) =
            instance_guidance(&status, "rebuild-required", "compatible");
        assert_eq!(use_summary, "Build verification expired");
        assert!(action_required);
        assert!(action.contains("Run Build again"));
    }
}
