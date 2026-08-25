pub(crate) mod controller;
pub(super) mod evidence;
pub(crate) mod transport;

use super::{
    progress::ReviewOperationProgressView,
    service::{ReviewInstanceView, WorktreeReviewService},
};
use serde::Serialize;
use std::{
    collections::HashMap,
    fs,
    path::PathBuf,
    sync::{Arc, Mutex},
    thread,
};
use uuid::Uuid;

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
enum ReviewOperationResult {
    Pending,
    Succeeded(ReviewInstanceView),
    Failed(String),
}

pub(crate) struct ProofSupport {
    service: Arc<WorktreeReviewService>,
    review_root: PathBuf,
    operation_results: Mutex<HashMap<String, ReviewOperationResult>>,
    launcher_navigation: Mutex<Option<String>>,
    launcher_detail_navigation: Mutex<Option<LauncherDetailNavigationView>>,
    launcher_presentation: Mutex<Option<LauncherProofPresentationView>>,
}

impl ProofSupport {
    pub(crate) fn new(service: Arc<WorktreeReviewService>, review_root: PathBuf) -> Self {
        Self {
            service,
            review_root,
            operation_results: Mutex::new(HashMap::new()),
            launcher_navigation: Mutex::new(None),
            launcher_detail_navigation: Mutex::new(None),
            launcher_presentation: Mutex::new(None),
        }
    }

    pub(super) fn service(&self) -> &Arc<WorktreeReviewService> {
        &self.service
    }

    pub(super) fn begin_prepare(
        self: &Arc<Self>,
        source_ref: String,
        name: String,
    ) -> Result<AcceptedReviewOperationView, String> {
        self.spawn_operation(move |service, operation_ref| {
            service.prepare(operation_ref, source_ref, name)
        })
    }

    pub(super) fn begin_build(
        self: &Arc<Self>,
        instance_ref: String,
    ) -> Result<AcceptedReviewOperationView, String> {
        self.spawn_operation(move |service, operation_ref| {
            service.build(operation_ref, instance_ref)
        })
    }

    pub(super) fn begin_open(
        self: &Arc<Self>,
        instance_ref: String,
    ) -> Result<AcceptedReviewOperationView, String> {
        self.spawn_operation(move |service, operation_ref| {
            service.start_in_background(operation_ref, instance_ref)
        })
    }

    fn spawn_operation(
        self: &Arc<Self>,
        operation: impl FnOnce(Arc<WorktreeReviewService>, String) -> Result<ReviewInstanceView, String>
            + Send
            + 'static,
    ) -> Result<AcceptedReviewOperationView, String> {
        let operation_ref = fresh_operation_ref();
        self.operation_results
            .lock()
            .map_err(|_| "Review operation state is unavailable.".to_string())?
            .insert(operation_ref.clone(), ReviewOperationResult::Pending);
        let proof = self.clone();
        let result_ref = operation_ref.clone();
        let spawn = thread::Builder::new()
            .name("worktree-review-proof-operation".into())
            .spawn(move || {
                let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    operation(proof.service.clone(), result_ref.clone())
                }))
                .unwrap_or_else(|_| Err("The review operation ended unexpectedly.".into()));
                let terminal = match result {
                    Ok(value) => ReviewOperationResult::Succeeded(value),
                    Err(error) => ReviewOperationResult::Failed(error),
                };
                if let Ok(mut results) = proof.operation_results.lock() {
                    results.insert(result_ref, terminal);
                }
            });
        if spawn.is_err() {
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

    pub(super) fn operation(
        &self,
        operation_ref: String,
    ) -> Result<ReviewOperationStatusView, String> {
        let progress = self.service.operation_progress(operation_ref.clone())?;
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

    pub(super) fn navigate_child(&self, instance_ref: String, route: &str) -> Result<(), String> {
        self.service.status(instance_ref.clone())?;
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
            .service
            .instances_root
            .join(instance_ref)
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

    pub(super) fn navigate_launcher(&self) -> Result<(), String> {
        *self
            .launcher_navigation
            .lock()
            .map_err(|_| "Launcher proof navigation is unavailable.".to_string())? =
            Some("worktree-review".into());
        self.set_launcher_presentation("overview", "launcher", None, None, None)
    }

    pub(super) fn launcher_navigation(&self) -> Result<Option<String>, String> {
        self.launcher_navigation
            .lock()
            .map(|route| route.clone())
            .map_err(|_| "Launcher proof navigation is unavailable.".to_string())
    }

    pub(super) fn navigate_launcher_detail(&self, instance_ref: String) -> Result<(), String> {
        self.service.status(instance_ref.clone())?;
        self.navigate_launcher()?;
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
        )
    }

    pub(super) fn navigate_launcher_operation(
        &self,
        instance_ref: String,
        operation_ref: String,
    ) -> Result<(), String> {
        let detail = self.service.detail(instance_ref.clone())?;
        if !detail
            .operations
            .iter()
            .any(|operation| operation.operation_ref == operation_ref)
        {
            return Err("The retained operation output is unavailable for this build.".into());
        }
        self.navigate_launcher()?;
        self.set_launcher_presentation(
            "details",
            "retained-operation-output",
            Some(instance_ref),
            Some(operation_ref),
            None,
        )
    }

    pub(super) fn select_launcher_source(&self, source_ref: String) -> Result<(), String> {
        if !self
            .service
            .sources()
            .iter()
            .any(|source| source.source_ref == source_ref)
        {
            return Err("The selected worktree is unavailable.".into());
        }
        self.navigate_launcher()?;
        self.set_launcher_presentation(
            "overview",
            "selected-worktree",
            None,
            None,
            Some(source_ref),
        )
    }

    pub(super) fn launcher_detail_navigation(
        &self,
    ) -> Result<Option<LauncherDetailNavigationView>, String> {
        self.launcher_detail_navigation
            .lock()
            .map(|route| route.clone())
            .map_err(|_| "Launcher detail proof navigation is unavailable.".to_string())
    }

    pub(super) fn launcher_presentation(
        &self,
    ) -> Result<Option<LauncherProofPresentationView>, String> {
        self.launcher_presentation
            .lock()
            .map(|presentation| presentation.clone())
            .map_err(|_| "Launcher proof presentation is unavailable.".to_string())
    }

    pub(super) fn build_evidence(
        &self,
        operation_ref: String,
    ) -> Result<evidence::ReviewBuildOperationEvidenceView, String> {
        for instance in self.service.instances() {
            let detail = self.service.detail(instance.instance_ref.clone())?;
            if let Some(operation) = detail
                .operations
                .into_iter()
                .find(|operation| operation.operation_ref == operation_ref)
            {
                return evidence::assemble(
                    self.review_root.join("registry.sqlite"),
                    instance.instance_ref,
                    operation,
                );
            }
        }
        Err("Review operation evidence is unavailable.".into())
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
            .launcher_presentation
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
}

#[derive(Clone)]
pub(crate) struct ProofTauriState(pub(crate) Arc<ProofSupport>);

impl ProofTauriState {
    pub(crate) fn new(proof: Arc<ProofSupport>) -> Self {
        Self(proof)
    }
}

fn fresh_operation_ref() -> String {
    format!("review-operation-{}", Uuid::new_v4().simple())
}
