use super::{
    catalog::{ReviewWorktreeCatalog, ReviewWorktreeOption},
    comparison::WorktreeComparisonView,
    detail::{assemble, now_ms, DetailInput, ReviewInstanceDetailView, ReviewLifecycleEventView},
    progress::{ProgressHandle, ProgressRegistry, ReviewOperationProgressView},
    runtime_port::{
        IntoReviewInstanceRuntime, ReviewBuildOutcome, ReviewHealth, ReviewInstanceHandle,
        ReviewInstancePhase, ReviewInstanceRequest, ReviewInstanceRuntime, ReviewInstanceStatus,
        ReviewRuntimeError, ReviewRuntimeErrorKind, ReviewSourceRef,
    },
    source_history::{self, ReviewSourceHistoryView},
    store::{
        SqliteWorktreeReviewStore, StoredReviewLifecycleEvent, StoredReviewSession,
        WorktreeReviewStore,
    },
    worktree_build::git_text,
};
use crate::orchestration::initiated_sprint_git_authority::{
    BindInitiatedSprintGitAuthorityError, VerifiedRuntimeGitComparison,
    WorktreeRuntimeGitComparison,
};
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use std::{
    collections::{HashMap, HashSet},
    path::{Path, PathBuf},
    sync::{Arc, Mutex},
    thread,
};

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewSourceView {
    pub(crate) source_ref: String,
    pub(crate) label: String,
    pub(crate) branch: Option<String>,
    pub(crate) detached: bool,
    pub(crate) is_main: bool,
    pub(crate) is_current: bool,
    pub(crate) parent_source_ref: Option<String>,
    pub(crate) lineage_ambiguous: bool,
    pub(crate) relationship: String,
    pub(crate) ahead: usize,
    pub(crate) behind: usize,
    pub(crate) fork_revision: String,
    pub(crate) revision: String,
    pub(crate) compatibility: String,
    pub(crate) compatibility_message: String,
    pub(crate) details_state: String,
    pub(crate) attached: bool,
    pub(crate) ref_kind: String,
    pub(crate) merged_directly: bool,
    pub(crate) equivalent_patches: usize,
    pub(crate) comparison_branch: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewSettingsView {
    pub(crate) cleanup_detached_builds: bool,
}

impl WorktreeRuntimeGitComparison for WorktreeReviewService {
    fn resolve_verified_comparison(
        &self,
        runtime_instance_ref: &str,
    ) -> Result<VerifiedRuntimeGitComparison, BindInitiatedSprintGitAuthorityError> {
        let (handle, metadata) = self
            .resolve(runtime_instance_ref)
            .map_err(|_| BindInitiatedSprintGitAuthorityError::RuntimeSourceUnavailable)?;
        let verified = self
            .runtime
            .verified_source(&handle)
            .map_err(|error| match error.kind {
                ReviewRuntimeErrorKind::Conflict => {
                    BindInitiatedSprintGitAuthorityError::RuntimeSourceStale
                }
                ReviewRuntimeErrorKind::NotFound | ReviewRuntimeErrorKind::InvalidState => {
                    BindInitiatedSprintGitAuthorityError::RuntimeSourceUnavailable
                }
                _ => BindInitiatedSprintGitAuthorityError::Unavailable,
            })?;
        self.catalog
            .ensure_compatible(&metadata.source_ref)
            .map_err(|_| BindInitiatedSprintGitAuthorityError::RuntimeSourceIncompatible)?;
        if !verified.clean {
            return Err(BindInitiatedSprintGitAuthorityError::RuntimeSourceDirty);
        }
        let catalog_identity = self
            .catalog
            .comparison_identity(&metadata.source_ref)
            .map_err(|_| BindInitiatedSprintGitAuthorityError::RuntimeSourceUnavailable)?;
        let repository_root = catalog_identity
            .main_root
            .canonicalize()
            .map_err(|_| BindInitiatedSprintGitAuthorityError::RuntimeSourceUnavailable)?;
        let worktree_root = catalog_identity
            .selected_root
            .canonicalize()
            .map_err(|_| BindInitiatedSprintGitAuthorityError::RuntimeSourceUnavailable)?;
        if repository_root != catalog_identity.main_root
            || worktree_root != catalog_identity.selected_root
        {
            return Err(BindInitiatedSprintGitAuthorityError::RuntimeEvidenceMismatch);
        }
        let observed_worktree = verified
            .worktree_path
            .canonicalize()
            .map_err(|_| BindInitiatedSprintGitAuthorityError::RuntimeSourceUnavailable)?;
        if observed_worktree != worktree_root {
            return Err(BindInitiatedSprintGitAuthorityError::RuntimeEvidenceMismatch);
        }
        let repository_common_dir = common_dir(&repository_root)?;
        if repository_common_dir != catalog_identity.common_dir
            || repository_common_dir != common_dir(&worktree_root)?
        {
            return Err(BindInitiatedSprintGitAuthorityError::RuntimeEvidenceMismatch);
        }
        let baseline_object_id =
            full_commit(&repository_root, &catalog_identity.baseline_object_id)?;
        if baseline_object_id != catalog_identity.baseline_object_id.to_ascii_lowercase() {
            return Err(BindInitiatedSprintGitAuthorityError::RuntimeEvidenceMismatch);
        }
        let current_object_id = full_commit(&worktree_root, "HEAD")?;
        if current_object_id != verified.current_object_id.to_ascii_lowercase()
            || baseline_object_id == current_object_id
        {
            return Err(BindInitiatedSprintGitAuthorityError::RuntimeEvidenceMismatch);
        }
        let common_identity = normalized_path(&repository_common_dir);
        let worktree_identity = normalized_path(&worktree_root);
        Ok(VerifiedRuntimeGitComparison {
            repository_id: product_id("repository", &[&common_identity]),
            repository_root: repository_root.to_string_lossy().into_owned(),
            repository_common_dir: repository_common_dir.to_string_lossy().into_owned(),
            worktree_id: product_id("worktree", &[&common_identity, &worktree_identity]),
            worktree_root: worktree_root.to_string_lossy().into_owned(),
            baseline_object_id,
            current_object_id,
            runtime_instance_ref: runtime_instance_ref.to_owned(),
            runtime_source_ref: metadata.source_ref,
            source_fingerprint: verified.source_fingerprint.to_ascii_lowercase(),
        })
    }
}

fn common_dir(root: &Path) -> Result<PathBuf, BindInitiatedSprintGitAuthorityError> {
    let value = git_text(root, ["rev-parse", "--git-common-dir"])
        .map_err(|_| BindInitiatedSprintGitAuthorityError::ComparisonUnavailable)?;
    let path = PathBuf::from(value);
    let path = if path.is_absolute() {
        path
    } else {
        root.join(path)
    };
    path.canonicalize()
        .map_err(|_| BindInitiatedSprintGitAuthorityError::ComparisonUnavailable)
}

fn full_commit(
    root: &Path,
    revision: &str,
) -> Result<String, BindInitiatedSprintGitAuthorityError> {
    let commit = git_text(
        root,
        ["rev-parse", "--verify", &format!("{revision}^{{commit}}")],
    )
    .map_err(|_| BindInitiatedSprintGitAuthorityError::ComparisonUnavailable)?
    .to_ascii_lowercase();
    if (commit.len() != 40 && commit.len() != 64)
        || !commit.bytes().all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(BindInitiatedSprintGitAuthorityError::ComparisonUnavailable);
    }
    Ok(commit)
}

fn normalized_path(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

fn product_id(kind: &str, parts: &[&str]) -> String {
    let mut hash = Sha256::new();
    hash.update(b"worktree-runtime-git-authority/v1");
    for part in parts {
        hash.update((part.len() as u64).to_be_bytes());
        hash.update(part.as_bytes());
    }
    format!("{kind}-{}", &format!("{:x}", hash.finalize())[..24])
}

impl From<&ReviewWorktreeOption> for ReviewSourceView {
    fn from(value: &ReviewWorktreeOption) -> Self {
        Self {
            source_ref: value.source_ref.clone(),
            label: value.label.clone(),
            branch: value.branch.clone(),
            detached: value.detached,
            is_main: value.is_main,
            is_current: value.is_current,
            parent_source_ref: value.parent_source_ref.clone(),
            lineage_ambiguous: value.lineage_ambiguous,
            relationship: value.relationship.clone(),
            ahead: value.ahead,
            behind: value.behind,
            fork_revision: value.fork_revision.clone(),
            revision: value.revision.clone(),
            compatibility: value.compatibility.clone(),
            compatibility_message: value.compatibility_message.clone(),
            details_state: value.details_state.clone(),
            attached: value.attached,
            ref_kind: value.ref_kind.clone(),
            merged_directly: value.merged_directly,
            equivalent_patches: value.equivalent_patches,
            comparison_branch: value.comparison_branch.clone(),
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewInstanceView {
    pub(crate) instance_ref: String,
    pub(crate) name: String,
    pub(crate) source_ref: String,
    pub(crate) source_label: String,
    pub(crate) prepared_revision: Option<String>,
    pub(crate) current_revision: Option<String>,
    pub(crate) source_state: String,
    pub(crate) outdated_by_commits: Option<usize>,
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

#[derive(Clone)]
struct ReviewMetadata {
    name: String,
    source_ref: String,
    source_label: String,
}

struct ReviewBuildFreshness {
    prepared_revision: Option<String>,
    current_revision: Option<String>,
    state: String,
    outdated_by_commits: Option<usize>,
}

struct SourceCatalogState {
    sources: Vec<ReviewSourceView>,
    generation: u64,
    refreshing: bool,
    requested_include_detached: bool,
}

pub(crate) struct WorktreeReviewService {
    runtime: Arc<dyn ReviewInstanceRuntime>,
    catalog: Arc<ReviewWorktreeCatalog>,
    instances: Mutex<HashMap<String, ReviewMetadata>>,
    built: Mutex<HashSet<String>>,
    store: Arc<dyn WorktreeReviewStore>,
    source_catalog: Mutex<SourceCatalogState>,
    pub(super) progress: Arc<ProgressRegistry>,
    pub(super) instances_root: PathBuf,
    attachments_root: PathBuf,
}

/// Transitional composition alias. Product-facing code should use `WorktreeReviewService`.
impl WorktreeReviewService {
    pub(crate) fn new(
        runtime: impl IntoReviewInstanceRuntime,
        catalog: Arc<ReviewWorktreeCatalog>,
        store_path: &Path,
        instances_root: PathBuf,
    ) -> Result<Self, String> {
        let store: Arc<dyn WorktreeReviewStore> =
            Arc::new(SqliteWorktreeReviewStore::open(store_path)?);
        let (instances, built) = load_sessions(store.as_ref())?;
        let attachments_root = instances_root
            .parent()
            .unwrap_or(&instances_root)
            .join("attached-worktrees");
        let fingerprint = catalog.cache_fingerprint();
        let sources = load_source_cache(store.as_ref(), &fingerprint)?.unwrap_or_else(|| {
            catalog
                .options()
                .iter()
                .map(ReviewSourceView::from)
                .collect()
        });
        Ok(Self {
            runtime: runtime.into_review_instance_runtime(),
            catalog,
            instances: Mutex::new(instances),
            built: Mutex::new(built),
            store,
            source_catalog: Mutex::new(SourceCatalogState {
                sources,
                generation: 0,
                refreshing: false,
                requested_include_detached: false,
            }),
            progress: Arc::new(ProgressRegistry::system()),
            instances_root,
            attachments_root,
        })
    }

    pub(crate) fn sources(&self) -> Vec<ReviewSourceView> {
        self.source_catalog
            .lock()
            .map(|catalog| catalog.sources.clone())
            .unwrap_or_default()
    }

    pub(crate) fn repository_sources(&self) -> Result<Vec<ReviewSourceView>, String> {
        self.catalog
            .repository_options()
            .map(|options| options.iter().map(ReviewSourceView::from).collect())
    }

    pub(crate) fn source_snapshot(
        self: &Arc<Self>,
        include_detached: bool,
        force_refresh: bool,
    ) -> Vec<ReviewSourceView> {
        self.request_source_refresh(include_detached, force_refresh);
        self.sources()
    }

    pub(crate) fn source_history(
        &self,
        source_ref: String,
    ) -> Result<ReviewSourceHistoryView, String> {
        source_history::read(&self.catalog, &source_ref)
    }

    pub(crate) fn attach_review_worktree(
        &self,
        source_ref: String,
    ) -> Result<ReviewSourceView, String> {
        let option = self
            .catalog
            .attach_review_worktree(&source_ref, &self.attachments_root)?;
        let view = ReviewSourceView::from(&option);
        if let Ok(mut state) = self.source_catalog.lock() {
            if let Some(current) = state
                .sources
                .iter_mut()
                .find(|current| current.source_ref == source_ref)
            {
                *current = view.clone();
            } else {
                state.sources.push(view.clone());
            }
        }
        Ok(view)
    }

    pub(crate) fn settings(&self) -> Result<ReviewSettingsView, String> {
        self.store
            .cleanup_detached_builds()
            .map(|cleanup_detached_builds| ReviewSettingsView {
                cleanup_detached_builds,
            })
    }

    pub(crate) fn update_settings(
        &self,
        settings: ReviewSettingsView,
    ) -> Result<ReviewSettingsView, String> {
        self.store
            .set_cleanup_detached_builds(settings.cleanup_detached_builds)?;
        Ok(settings)
    }

    fn request_source_refresh(self: &Arc<Self>, include_detached: bool, force_refresh: bool) {
        let (generation, requested_include_detached) = {
            let Ok(mut state) = self.source_catalog.lock() else {
                return;
            };
            state.requested_include_detached |= include_detached;
            if state.refreshing {
                return;
            }
            let needs_refresh = force_refresh
                || state.sources.iter().any(|source| {
                    (!source.detached || include_detached)
                        && matches!(source.details_state.as_str(), "pending" | "cached")
                });
            if !needs_refresh {
                return;
            }
            if force_refresh {
                for source in &mut state.sources {
                    if (!source.detached || include_detached) && source.details_state == "failed" {
                        source.details_state = "pending".into();
                    }
                }
            }
            state.generation = state.generation.saturating_add(1);
            state.refreshing = true;
            (state.generation, state.requested_include_detached)
        };
        let service = Arc::clone(self);
        if thread::Builder::new()
            .name("worktree-review-source-refresh".into())
            .spawn(move || service.refresh_sources(generation))
            .is_err()
        {
            self.fail_source_refresh(generation, requested_include_detached);
        }
    }

    fn refresh_sources(self: &Arc<Self>, generation: u64) {
        let include_detached = self
            .source_catalog
            .lock()
            .map(|state| state.requested_include_detached)
            .unwrap_or(false);
        let result = self
            .catalog
            .live_options_progressive(include_detached, |option| {
                self.publish_source(generation, option)
            });
        match result {
            Ok(options) => {
                let fingerprint = ReviewWorktreeCatalog::options_fingerprint(&options);
                let completed = {
                    let Ok(mut state) = self.source_catalog.lock() else {
                        return;
                    };
                    if state.generation != generation {
                        return;
                    }
                    let previous = state
                        .sources
                        .iter()
                        .map(|source| (source.source_ref.clone(), source.clone()))
                        .collect::<HashMap<_, _>>();
                    state.sources = options
                        .iter()
                        .map(ReviewSourceView::from)
                        .map(|source| {
                            if source.details_state == "pending" {
                                previous
                                    .get(&source.source_ref)
                                    .filter(|cached| cached.details_state != "pending")
                                    .cloned()
                                    .unwrap_or(source)
                            } else {
                                source
                            }
                        })
                        .collect();
                    state.refreshing = false;
                    state.sources.clone()
                };
                let _ = self.persist_source_cache(&fingerprint, &completed);
                self.cleanup_detached_sources(&options);
                let queued_detached =
                    self.source_catalog
                        .lock()
                        .map(|state| {
                            state.requested_include_detached
                                && state.sources.iter().any(|source| {
                                    source.detached && source.details_state == "pending"
                                })
                        })
                        .unwrap_or(false);
                if queued_detached && !include_detached {
                    self.request_source_refresh(true, false);
                }
            }
            Err(_) => self.fail_source_refresh(generation, include_detached),
        }
    }

    fn publish_source(&self, generation: u64, option: &ReviewWorktreeOption) {
        let Ok(mut state) = self.source_catalog.lock() else {
            return;
        };
        if state.generation != generation {
            return;
        }
        let source = ReviewSourceView::from(option);
        if let Some(current) = state
            .sources
            .iter_mut()
            .find(|current| current.source_ref == source.source_ref)
        {
            *current = source;
        } else {
            state.sources.push(source);
        }
    }

    fn fail_source_refresh(&self, generation: u64, include_detached: bool) {
        let Ok(mut state) = self.source_catalog.lock() else {
            return;
        };
        if state.generation != generation {
            return;
        }
        state.refreshing = false;
        for source in &mut state.sources {
            if (!source.detached || include_detached)
                && matches!(source.details_state.as_str(), "pending" | "cached")
            {
                source.details_state = "failed".into();
            }
        }
    }

    fn persist_source_cache(
        &self,
        fingerprint: &str,
        sources: &[ReviewSourceView],
    ) -> Result<(), String> {
        let payload = serde_json::to_string(sources)
            .map_err(|_| "Review source cache could not be encoded.".to_string())?;
        self.store.replace_source_cache(fingerprint, &payload)
    }

    fn cleanup_detached_sources(&self, options: &[ReviewWorktreeOption]) {
        if !self
            .settings()
            .map(|settings| settings.cleanup_detached_builds)
            .unwrap_or(false)
        {
            return;
        }
        let detached = options
            .iter()
            .filter(|source| source.detached)
            .map(|source| source.source_ref.as_str())
            .collect::<HashSet<_>>();
        let refs = self
            .instances
            .lock()
            .map(|instances| {
                instances
                    .iter()
                    .filter(|(_, metadata)| detached.contains(metadata.source_ref.as_str()))
                    .map(|(instance_ref, _)| instance_ref.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for instance_ref in refs {
            let _ = self.cleanup_instance(&instance_ref);
        }
    }

    fn cleanup_other_source_instances(&self, source_ref: &str, keep: &str) {
        let refs = self
            .instances
            .lock()
            .map(|instances| {
                instances
                    .iter()
                    .filter(|(instance_ref, metadata)| {
                        instance_ref.as_str() != keep && metadata.source_ref == source_ref
                    })
                    .map(|(instance_ref, _)| instance_ref.clone())
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default();
        for instance_ref in refs {
            let _ = self.cleanup_instance(&instance_ref);
        }
    }

    fn cleanup_instance(&self, instance_ref: &str) -> Result<(), String> {
        let handle =
            ReviewInstanceHandle::from_opaque(instance_ref.to_owned()).map_err(safe_error)?;
        self.runtime.cleanup(&handle).map_err(safe_error)?;
        self.store.delete_session(instance_ref)?;
        if let Ok(mut instances) = self.instances.lock() {
            instances.remove(instance_ref);
        }
        if let Ok(mut built) = self.built.lock() {
            built.remove(instance_ref);
        }
        Ok(())
    }

    fn build_freshness(
        &self,
        handle: &ReviewInstanceHandle,
        source_ref: &str,
        status: &ReviewInstanceStatus,
    ) -> ReviewBuildFreshness {
        let Ok(retained) = self.runtime.retained_source(handle) else {
            return ReviewBuildFreshness::unknown();
        };
        let Ok(value) = self
            .catalog
            .source_freshness(source_ref, &retained.current_object_id)
        else {
            return ReviewBuildFreshness {
                prepared_revision: Some(abbreviated_revision(&retained.current_object_id)),
                current_revision: None,
                state: "unavailable".into(),
                outdated_by_commits: None,
            };
        };
        ReviewBuildFreshness {
            prepared_revision: Some(value.prepared_revision),
            current_revision: Some(value.current_revision),
            state: if !status.source_current && value.state == "current" {
                "changed".into()
            } else {
                value.state
            },
            outdated_by_commits: value.outdated_by_commits,
        }
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
                    ReviewInstanceRequest::new(
                        ReviewSourceRef::new(source_ref.clone()).map_err(safe_error)?,
                        name.clone(),
                    )
                    .map_err(safe_error)?,
                )
                .map_err(safe_error)?;
            let instance_ref = requested.handle.opaque_ref().to_owned();
            let freshness = self.build_freshness(&requested.handle, &source_ref, &requested.status);
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
                source_ref.clone(),
                source_label,
                requested.status,
                "not-built",
                &self.catalog.compatibility(&source_ref).0,
                freshness,
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
                ReviewBuildOutcome::Passed => {
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
                    self.cleanup_other_source_instances(&metadata.source_ref, &instance_ref);
                    "passed"
                }
                ReviewBuildOutcome::Failed => "failed",
            };
            let freshness = self.build_freshness(&handle, &metadata.source_ref, &result.status);
            Ok(view(
                instance_ref,
                metadata.name,
                metadata.source_ref.clone(),
                metadata.source_label,
                result.status,
                build,
                &self.catalog.compatibility(&metadata.source_ref).0,
                freshness,
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

    #[cfg(debug_assertions)]
    pub(crate) fn start_in_background(
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
        self.start_with_progress(progress, instance_ref, false)
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

    #[cfg(debug_assertions)]
    pub(crate) fn context(
        &self,
        instance_ref: String,
    ) -> Result<super::worktree_build::WorktreeBuildContextView, String> {
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

    fn lifecycle(
        &self,
        instance_ref: String,
        operation: impl FnOnce(
            &dyn ReviewInstanceRuntime,
            &ReviewInstanceHandle,
        ) -> Result<ReviewInstanceStatus, ReviewRuntimeError>,
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
        let freshness = self.build_freshness(&handle, &metadata.source_ref, &status);
        Ok(view(
            instance_ref,
            metadata.name,
            metadata.source_ref.clone(),
            metadata.source_label,
            status,
            build,
            &self.catalog.compatibility(&metadata.source_ref).0,
            freshness,
        ))
    }

    fn resolve(
        &self,
        instance_ref: &str,
    ) -> Result<(ReviewInstanceHandle, ReviewMetadata), String> {
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
            ReviewInstanceHandle::from_opaque(instance_ref.to_owned()).map_err(safe_error)?,
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
        self.store.upsert_session(&StoredReviewSession {
            instance_ref: instance_ref.to_owned(),
            name: name.to_owned(),
            source_ref: source_ref.to_owned(),
            source_label: source_label.to_owned(),
            built,
        })
    }

    fn record_event(&self, instance_ref: &str, kind: &str, summary: &str) -> Result<(), String> {
        self.store.append_lifecycle_event(
            instance_ref,
            &StoredReviewLifecycleEvent {
                occurred_at_ms: now_ms(),
                kind: kind.to_owned(),
                summary: summary.to_owned(),
            },
        )
    }

    fn history(&self, instance_ref: &str) -> Result<Vec<ReviewLifecycleEventView>, String> {
        self.store.lifecycle_history(instance_ref).map(|events| {
            events
                .into_iter()
                .map(|event| ReviewLifecycleEventView {
                    occurred_at_ms: event.occurred_at_ms,
                    kind: event.kind,
                    summary: event.summary,
                })
                .collect()
        })
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
    store: &dyn WorktreeReviewStore,
) -> Result<(HashMap<String, ReviewMetadata>, HashSet<String>), String> {
    let mut instances = HashMap::new();
    let mut built = HashSet::new();
    for session in store.sessions()? {
        if session.built {
            built.insert(session.instance_ref.clone());
        }
        instances.insert(
            session.instance_ref,
            ReviewMetadata {
                name: session.name,
                source_ref: session.source_ref,
                source_label: session.source_label,
            },
        );
    }
    Ok((instances, built))
}

fn load_source_cache(
    store: &dyn WorktreeReviewStore,
    fingerprint: &str,
) -> Result<Option<Vec<ReviewSourceView>>, String> {
    let payload = store.source_cache(fingerprint)?;
    let Some(payload) = payload else {
        return Ok(None);
    };
    let Ok(mut sources) = serde_json::from_str::<Vec<ReviewSourceView>>(&payload) else {
        return Ok(None);
    };
    if sources.is_empty() {
        return Ok(None);
    }
    for source in &mut sources {
        if source.details_state != "pending" {
            source.details_state = "cached".into();
        }
    }
    Ok(Some(sources))
}

fn view(
    instance_ref: String,
    name: String,
    source_ref: String,
    source_label: String,
    status: ReviewInstanceStatus,
    build: &str,
    compatibility: &str,
    freshness: ReviewBuildFreshness,
) -> ReviewInstanceView {
    let (current_use, action_required, action_summary) =
        instance_guidance(&status, build, compatibility);
    ReviewInstanceView {
        instance_ref,
        name,
        source_ref,
        source_label,
        prepared_revision: freshness.prepared_revision,
        current_revision: freshness.current_revision,
        source_state: freshness.state,
        outdated_by_commits: freshness.outdated_by_commits,
        phase: phase(status.phase).into(),
        health: format!("{:?}", status.health).to_lowercase(),
        stale: status.stale,
        build: build.into(),
        can_focus: status.phase == ReviewInstancePhase::Running
            && status.health == ReviewHealth::Healthy
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

impl ReviewBuildFreshness {
    fn unknown() -> Self {
        Self {
            prepared_revision: None,
            current_revision: None,
            state: "unknown".into(),
            outdated_by_commits: None,
        }
    }
}

fn abbreviated_revision(value: &str) -> String {
    value.chars().take(12).collect()
}

fn instance_guidance(
    status: &ReviewInstanceStatus,
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
    if status.stale || status.health == ReviewHealth::Unhealthy {
        return (
            "Needs recovery".into(),
            true,
            "Recover this instance before attempting another Open.".into(),
        );
    }
    if status.phase == ReviewInstancePhase::Running {
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

fn phase(value: ReviewInstancePhase) -> &'static str {
    match value {
        ReviewInstancePhase::Prepared => "prepared",
        ReviewInstancePhase::Starting => "starting",
        ReviewInstancePhase::Running => "running",
        ReviewInstancePhase::Stopping => "stopping",
        ReviewInstancePhase::Stopped => "stopped",
        ReviewInstancePhase::Recovering => "recovering",
        ReviewInstancePhase::Recovered => "recovered",
    }
}

fn safe_error(error: ReviewRuntimeError) -> String {
    let readiness_failure = error.kind == ReviewRuntimeErrorKind::Unavailable && {
        let message = error.message.to_ascii_lowercase();
        message.contains("window") || message.contains("readiness")
    };
    if readiness_failure {
        return "The owned process or supporting services did not establish the exact titled, visible, useful-size worktree-build window and rendered application marker. The verified build remains reusable; Stop or Recover the owned tree, then retry Open."
            .into();
    }
    match error.kind {
        ReviewRuntimeErrorKind::InvalidRequest => "The review request is invalid.".into(),
        ReviewRuntimeErrorKind::NotFound => "The review instance is no longer available.".into(),
        ReviewRuntimeErrorKind::Unauthorized => {
            "Review instance ownership could not be verified.".into()
        }
        ReviewRuntimeErrorKind::InvalidState => {
            "That action is not available in the current lifecycle state.".into()
        }
        ReviewRuntimeErrorKind::OperationInProgress => {
            "Another lifecycle action is still in progress.".into()
        }
        ReviewRuntimeErrorKind::Conflict => {
            "The selected source or review instance changed; prepare a fresh instance.".into()
        }
        ReviewRuntimeErrorKind::BuildRequired => {
            "This retained build is no longer an exact verified artifact. Run Build again; its isolated data and prior output remain available.".into()
        }
        ReviewRuntimeErrorKind::Unavailable => {
            "The isolated review runtime could not complete this action. Check its private logs."
                .into()
        }
    }
}

#[cfg(test)]
mod guidance_tests {
    use super::*;
    use std::{fs, process::Command};

    #[test]
    fn window_readiness_failure_explains_missing_evidence_reuse_and_recovery() {
        let message = safe_error(ReviewRuntimeError {
            kind: ReviewRuntimeErrorKind::Unavailable,
            message: "owned processes were observed, but a usable application-ready worktree-build window did not appear before the readiness limit".into(),
        });

        assert!(message.contains("exact titled, visible, useful-size"));
        assert!(message.contains("rendered application marker"));
        assert!(message.contains("verified build remains reusable"));
        assert!(message.contains("Stop or Recover"));
        assert!(!message.contains("private logs"));
    }

    #[test]
    fn retained_invalidated_builds_name_the_safe_next_action() {
        let status = ReviewInstanceStatus {
            phase: ReviewInstancePhase::Stopped,
            health: ReviewHealth::Closed,
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

    #[test]
    fn matching_source_cache_is_reused_as_refreshable_presentation_data() {
        let directory = tempfile::tempdir().unwrap();
        let database_path = directory.path().join("review.sqlite");
        let store = SqliteWorktreeReviewStore::open(&database_path).unwrap();
        let source = ReviewSourceView {
            source_ref: "source-main".into(),
            label: "main".into(),
            branch: Some("main".into()),
            detached: false,
            is_main: true,
            is_current: true,
            parent_source_ref: None,
            lineage_ambiguous: false,
            relationship: "related".into(),
            ahead: 0,
            behind: 0,
            fork_revision: "111111111111".into(),
            revision: "111111111111".into(),
            compatibility: "compatible".into(),
            compatibility_message: "Compatible.".into(),
            details_state: "ready".into(),
            attached: true,
            ref_kind: "local_branch".into(),
            merged_directly: false,
            equivalent_patches: 0,
            comparison_branch: "main".into(),
        };
        store
            .replace_source_cache("matching", &serde_json::to_string(&[source]).unwrap())
            .unwrap();

        let cached = load_source_cache(&store, "matching")
            .unwrap()
            .expect("matching cache");
        assert_eq!(cached[0].details_state, "cached");
        assert!(load_source_cache(&store, "different").unwrap().is_none());
        rusqlite::Connection::open(database_path)
            .unwrap()
            .execute(
                "UPDATE review_source_cache SET payload = 'not-json' WHERE singleton = 1",
                [],
            )
            .unwrap();
        assert!(load_source_cache(&store, "matching").unwrap().is_none());
    }

    #[test]
    fn prepared_runtime_source_yields_catalog_owned_immutable_comparison() {
        let directory = tempfile::tempdir().unwrap();
        let main = directory.path().join("main");
        let selected = directory.path().join("selected");
        fs::create_dir_all(main.join("src-tauri")).unwrap();
        git(directory.path(), &["init", main.to_str().unwrap()]);
        git(&main, &["config", "user.email", "test@example.invalid"]);
        git(&main, &["config", "user.name", "Test"]);
        fs::write(main.join("package-lock.json"), "{}").unwrap();
        fs::write(main.join("src-tauri/Cargo.lock"), "").unwrap();
        fs::write(
            main.join("src-tauri/Cargo.toml"),
            "[package]\nname='fixture'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(
            main.join("src-tauri/worktree-review-contract.json"),
            r#"{"version":1,"readiness":"owned-window-and-rendered-application","provenance":"worktree-build-details-v1"}"#,
        )
        .unwrap();
        fs::write(main.join("source.txt"), "baseline\n").unwrap();
        git(&main, &["add", "."]);
        git(&main, &["commit", "-m", "baseline"]);
        let baseline = git_text(&main, ["rev-parse", "HEAD"]).unwrap();
        git(&main, &["branch", "feature"]);
        git(
            &main,
            &["worktree", "add", selected.to_str().unwrap(), "feature"],
        );
        fs::write(selected.join("source.txt"), "current\n").unwrap();
        git(&selected, &["add", "."]);
        git(&selected, &["commit", "-m", "current"]);
        let current = git_text(&selected, ["rev-parse", "HEAD"]).unwrap();

        let review = Arc::new(
            crate::worktree_review::compose(&main, &directory.path().join("runtime"))
                .expect("compose runtime"),
        );
        assert!(
            !review
                .settings()
                .expect("default settings")
                .cleanup_detached_builds
        );
        assert!(
            review
                .update_settings(ReviewSettingsView {
                    cleanup_detached_builds: true,
                })
                .expect("persist cleanup setting")
                .cleanup_detached_builds
        );
        assert!(
            review
                .settings()
                .expect("reloaded settings")
                .cleanup_detached_builds
        );
        let source = review
            .sources()
            .into_iter()
            .find(|source| source.revision == current[..12])
            .expect("selected source");
        let prepared = review
            .prepare(
                "operation-prepare".into(),
                source.source_ref,
                "Sprint review".into(),
            )
            .expect("prepare source");
        fs::write(main.join("advanced-main.txt"), "later main\n").unwrap();
        git(&main, &["add", "."]);
        git(&main, &["commit", "-m", "advance machine main"]);
        let advanced_main = git_text(&main, ["rev-parse", "HEAD"]).unwrap();
        assert_ne!(advanced_main, baseline);
        let comparison = review
            .resolve_verified_comparison(&prepared.instance_ref)
            .expect("verified comparison");

        assert_eq!(comparison.baseline_object_id, baseline);
        assert_eq!(comparison.current_object_id, current);
        assert_eq!(comparison.runtime_instance_ref, prepared.instance_ref);
        assert_eq!(
            PathBuf::from(comparison.worktree_root),
            selected.canonicalize().unwrap()
        );
        assert_eq!(comparison.source_fingerprint.len(), 64);

        let database_path = directory.path().join("active.sqlite");
        let connection = rusqlite::Connection::open(&database_path).unwrap();
        crate::storage::configure_sqlite_connection(&connection).unwrap();
        crate::storage::initialize_active_database(&connection).unwrap();
        connection
            .pragma_update(None, "foreign_keys", false)
            .unwrap();
        connection.execute_batch("INSERT INTO epic_initiation_provenance (id,command_id,result_id,event_id,recorded_at) VALUES ('runtime-provenance','runtime-command','runtime-result','runtime-event','t'); INSERT INTO epic_initiations (id,command_id,result_id,event_id,provenance_id,draft_id,proposal_revision_id,material_snapshot_id,epic_id,recorded_at) VALUES ('runtime-initiation','runtime-command','runtime-result','runtime-event','runtime-provenance','runtime-draft','runtime-revision','runtime-snapshot','runtime-epic','t'); INSERT INTO initiated_sprints (id,epic_id,ordinal,title,intended_movement,concern_summaries_json,sprint_plan_id,sprint_plan_revision_id) VALUES ('runtime-sprint','runtime-epic',0,'Runtime Sprint','Move','[]','runtime-plan','runtime-plan-revision');").unwrap();
        connection
            .pragma_update(None, "foreign_keys", true)
            .unwrap();
        let repository = Arc::new(
            crate::orchestration::repository::SqliteOrchestrationRepository::new(connection)
                .unwrap(),
        );
        let transition = crate::orchestration::initiated_sprint_git_authority::InitiatedSprintGitAuthorityService::new(
            repository.clone(),
            review.clone(),
        );
        let bound = transition
            .bind(crate::orchestration::initiated_sprint_git_authority::BindInitiatedSprintGitAuthorityRequest {
                sprint_id: "runtime-sprint".into(),
                runtime_instance_ref: prepared.instance_ref.clone(),
                idempotency_key: "runtime-request".into(),
            })
            .expect("bind runtime comparison");
        let durable = repository
            .load_initiated_sprint_git_authority(&bound.authority_ref)
            .unwrap()
            .unwrap();
        assert_eq!(durable.epic_id, "runtime-epic");
        assert_eq!(durable.runtime_instance_ref, prepared.instance_ref);
        assert_eq!(durable.baseline_object_id, baseline);
        assert_eq!(durable.current_object_id, current);
        assert_eq!(
            rusqlite::Connection::open(&database_path)
                .unwrap()
                .query_row(
                    "SELECT COUNT(*) FROM file_review_git_capture_authorizations",
                    [],
                    |row| row.get::<_, i64>(0),
                )
                .unwrap(),
            0
        );

        fs::write(selected.join("source.txt"), "dirty\n").unwrap();
        assert_eq!(
            review.resolve_verified_comparison(&prepared.instance_ref),
            Err(BindInitiatedSprintGitAuthorityError::RuntimeSourceStale)
        );
        let dirty = review
            .prepare(
                "operation-dirty".into(),
                comparison.runtime_source_ref,
                "Dirty Sprint review".into(),
            )
            .expect("prepare dirty source");
        assert_eq!(
            review.resolve_verified_comparison(&dirty.instance_ref),
            Err(BindInitiatedSprintGitAuthorityError::RuntimeSourceDirty)
        );
        assert_eq!(
            review.resolve_verified_comparison("wt-missing"),
            Err(BindInitiatedSprintGitAuthorityError::RuntimeSourceUnavailable)
        );

        let displaced = directory.path().join("selected-original");
        fs::rename(&selected, &displaced).unwrap();
        fs::create_dir_all(selected.join("src-tauri")).unwrap();
        git(directory.path(), &["init", selected.to_str().unwrap()]);
        git(&selected, &["config", "user.email", "test@example.invalid"]);
        git(&selected, &["config", "user.name", "Test"]);
        fs::write(selected.join("package-lock.json"), "{}").unwrap();
        fs::write(selected.join("src-tauri/Cargo.lock"), "").unwrap();
        fs::write(
            selected.join("src-tauri/Cargo.toml"),
            "[package]\nname='replacement'\nversion='0.1.0'\n",
        )
        .unwrap();
        fs::write(selected.join("replacement.txt"), "replacement\n").unwrap();
        git(&selected, &["add", "."]);
        git(&selected, &["commit", "-m", "replacement root"]);
        assert_eq!(
            review.resolve_verified_comparison(&prepared.instance_ref),
            Err(BindInitiatedSprintGitAuthorityError::RuntimeSourceStale)
        );
    }

    fn git(root: &Path, arguments: &[&str]) {
        let output = Command::new("git")
            .arg("-C")
            .arg(root)
            .args(arguments)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "git {:?}: {}",
            arguments,
            String::from_utf8_lossy(&output.stderr)
        );
    }
}
