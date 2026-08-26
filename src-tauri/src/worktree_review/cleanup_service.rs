use super::{
    domain::{
        ArtifactSetId, CleanupDisposition, CleanupEffect, CleanupEligibility, CleanupJob,
        CleanupJobId, CleanupJobState, CleanupReceipt, CleanupResource, CleanupResourceId,
        CleanupTrigger, ContainmentRoot, RetentionPolicy, ReviewBuild, ReviewBuildId,
        ReviewOperationAttempt, WorkspaceLifecycle, WorkspaceOwnership,
    },
    retention::{self, RetentionDisposition},
    storage::{
        ArtifactSetRepository, CleanupRepository, OperationAttemptRepository,
        ReviewBuildRepository, ReviewSettingsRepository, StorageError, WorkspaceRepository,
        WorktreeReviewDatabase,
    },
};
use chrono::{DateTime, Utc};
use serde::Serialize;
use sha2::{Digest, Sha256};
use std::{
    collections::HashMap,
    error::Error,
    fmt, fs,
    path::{Component, Path, PathBuf},
    sync::Arc,
};

/// Narrow effect adapter for Worktree Review-owned AppData. Worktrees and runtime processes are
/// intentionally unsupported here: retaining a build worktree is independent from artifact
/// retention, and process cleanup requires its own opaque runtime authority.
pub(crate) struct AppDataCleanupEffects {
    review_root: PathBuf,
}

impl AppDataCleanupEffects {
    pub(crate) fn open(review_root: PathBuf) -> Result<Self, CleanupServiceError> {
        let review_root = review_root.canonicalize().map_err(|_| {
            CleanupServiceError::new(
                CleanupServiceErrorKind::InvalidRequest,
                "Worktree Review AppData root is unavailable for cleanup effects.",
            )
        })?;
        Ok(Self { review_root })
    }

    fn remove_storage_key(&self, key: &str) -> Result<CleanupEffectOutcome, String> {
        if !safe_storage_key(key) {
            return Err("The cleanup storage key is not a normalized AppData path.".into());
        }
        let target = self.review_root.join(key);
        let metadata = match fs::symlink_metadata(&target) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                return Ok(CleanupEffectOutcome::AlreadyAbsent)
            }
            Err(_) => return Err("The cleanup target could not be inspected.".into()),
        };
        if metadata.file_type().is_symlink() {
            return Err("Cleanup refuses an AppData target reached through a symlink.".into());
        }
        let resolved = target
            .canonicalize()
            .map_err(|_| "The cleanup target could not be resolved.".to_string())?;
        if !path_is_contained(&self.review_root, &resolved) {
            return Err("The cleanup target is outside Worktree Review AppData.".into());
        }
        if metadata.is_file() {
            fs::remove_file(&resolved).map_err(|_| {
                "The Worktree Review AppData file could not be removed.".to_string()
            })?;
        } else if metadata.is_dir() {
            fs::remove_dir_all(&resolved).map_err(|_| {
                "The Worktree Review AppData directory could not be removed.".to_string()
            })?;
        } else {
            return Err("Cleanup refuses a non-file AppData resource.".into());
        }
        Ok(CleanupEffectOutcome::Removed)
    }
}

impl CleanupEffectPort for AppDataCleanupEffects {
    fn apply(&self, resource: &CleanupResource) -> Result<CleanupEffectOutcome, String> {
        match resource {
            CleanupResource::ArtifactSet { storage_key, .. }
            | CleanupResource::AttemptLogs { storage_key, .. }
            | CleanupResource::BuildScratch { storage_key, .. } => {
                self.remove_storage_key(storage_key.as_str())
            }
            CleanupResource::Workspace { .. } => {
                Err("Build worktrees are retained by Worktree Review policy.".into())
            }
            CleanupResource::RuntimeInstance { .. } | CleanupResource::PortLease { .. } => {
                Err("Runtime cleanup requires the isolated runtime authority adapter.".into())
            }
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CleanupServiceErrorKind {
    NotFound,
    InvalidRequest,
    StorageUnavailable,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CleanupServiceError {
    pub(crate) kind: CleanupServiceErrorKind,
    pub(crate) message: String,
}

impl CleanupServiceError {
    fn new(kind: CleanupServiceErrorKind, message: impl Into<String>) -> Self {
        Self {
            kind,
            message: message.into(),
        }
    }
}

impl fmt::Display for CleanupServiceError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.message)
    }
}

impl Error for CleanupServiceError {}

impl From<StorageError> for CleanupServiceError {
    fn from(error: StorageError) -> Self {
        Self::new(CleanupServiceErrorKind::StorageUnavailable, error.message)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum CleanupEffectOutcome {
    Removed,
    AlreadyAbsent,
}

/// Performs one already-authorized effect. Implementations must use idempotent Git/runtime or
/// AppData operations and must revalidate their opaque external identity before mutation.
pub(crate) trait CleanupEffectPort: Send + Sync {
    fn apply(&self, resource: &CleanupResource) -> Result<CleanupEffectOutcome, String>;
}

pub(crate) trait CleanupClock: Send + Sync {
    fn now(&self) -> DateTime<Utc>;
}

struct SystemCleanupClock;

impl CleanupClock for SystemCleanupClock {
    fn now(&self) -> DateTime<Utc> {
        Utc::now()
    }
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct CleanupRequest {
    pub(crate) build_id: ReviewBuildId,
    pub(crate) resources: Vec<CleanupResource>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupResourcePresentation {
    pub(crate) resource_id: CleanupResourceId,
    pub(crate) disposition: Option<CleanupDisposition>,
    pub(crate) detail: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CleanupPresentation {
    pub(crate) job_id: CleanupJobId,
    pub(crate) build_id: ReviewBuildId,
    pub(crate) eligibility: CleanupEligibility,
    pub(crate) state: CleanupJobState,
    pub(crate) resources: Vec<CleanupResourcePresentation>,
    pub(crate) action_required: bool,
    pub(crate) summary: String,
    pub(crate) completed_at: Option<DateTime<Utc>>,
}

pub(crate) struct WorktreeReviewCleanupService {
    database: Arc<WorktreeReviewDatabase>,
    effects: Arc<dyn CleanupEffectPort>,
    clock: Arc<dyn CleanupClock>,
    review_root: PathBuf,
    containment_root: ContainmentRoot,
}

impl WorktreeReviewCleanupService {
    pub(crate) fn open_appdata(
        database: Arc<WorktreeReviewDatabase>,
        review_root: PathBuf,
    ) -> Result<Self, CleanupServiceError> {
        let effects = Arc::new(AppDataCleanupEffects::open(review_root.clone())?);
        Self::open(database, effects, review_root)
    }

    /// Opens the coordinator and reconciles any job interrupted before a durable receipt.
    pub(crate) fn open(
        database: Arc<WorktreeReviewDatabase>,
        effects: Arc<dyn CleanupEffectPort>,
        review_root: PathBuf,
    ) -> Result<Self, CleanupServiceError> {
        let service = Self::new(database, effects, Arc::new(SystemCleanupClock), review_root)?;
        service.reconcile_unfinished()?;
        Ok(service)
    }

    fn new(
        database: Arc<WorktreeReviewDatabase>,
        effects: Arc<dyn CleanupEffectPort>,
        clock: Arc<dyn CleanupClock>,
        review_root: PathBuf,
    ) -> Result<Self, CleanupServiceError> {
        let review_root = review_root.canonicalize().map_err(|_| {
            CleanupServiceError::new(
                CleanupServiceErrorKind::InvalidRequest,
                "Worktree Review AppData root is unavailable for cleanup authorization.",
            )
        })?;
        let containment_root = containment_identity(&review_root)?;
        Ok(Self {
            database,
            effects,
            clock,
            review_root,
            containment_root,
        })
    }

    pub(crate) fn containment_root(&self) -> &ContainmentRoot {
        &self.containment_root
    }

    /// Explicit retention entrypoint. It never runs during refresh or stop handling.
    pub(crate) fn cleanup_superseded_appdata_resources(
        &self,
        request: CleanupRequest,
    ) -> Result<CleanupPresentation, CleanupServiceError> {
        if request.resources.is_empty() {
            return Err(CleanupServiceError::new(
                CleanupServiceErrorKind::InvalidRequest,
                "Cleanup requires an explicit owned-resource ledger.",
            ));
        }
        let build = self
            .database
            .builds()
            .find(&request.build_id)?
            .ok_or_else(|| {
                CleanupServiceError::new(
                    CleanupServiceErrorKind::NotFound,
                    "The review build is unavailable.",
                )
            })?;
        let eligibility = self.eligibility(&build, &request.resources)?;
        let now = self.clock.now();
        let job = CleanupJob {
            id: CleanupJobId::random(),
            build_id: build.id,
            trigger: CleanupTrigger::RetentionPolicy,
            eligibility,
            state: if eligibility == CleanupEligibility::Eligible {
                CleanupJobState::Planned
            } else {
                CleanupJobState::NotEligible
            },
            resources: request.resources,
            created_at: now,
            started_at: None,
            settled_at: None,
        };
        self.database.cleanup().save_job(&job)?;
        if eligibility == CleanupEligibility::Eligible {
            self.run(job)
        } else {
            self.presentation(&job.id)
        }
    }

    pub(crate) fn reconcile_unfinished(
        &self,
    ) -> Result<Vec<CleanupPresentation>, CleanupServiceError> {
        self.database
            .cleanup()
            .list_unsettled()?
            .into_iter()
            .map(|job| self.run(job))
            .collect()
    }

    pub(crate) fn presentation(
        &self,
        job_id: &CleanupJobId,
    ) -> Result<CleanupPresentation, CleanupServiceError> {
        let job = self.database.cleanup().find_job(job_id)?.ok_or_else(|| {
            CleanupServiceError::new(
                CleanupServiceErrorKind::NotFound,
                "The cleanup record is unavailable.",
            )
        })?;
        let effects = self.database.cleanup().effects(job_id)?;
        let receipt = self.database.cleanup().find_receipt(job_id)?;
        Ok(present(&job, &effects, receipt.as_ref()))
    }

    fn eligibility(
        &self,
        build: &ReviewBuild,
        resources: &[CleanupResource],
    ) -> Result<CleanupEligibility, CleanupServiceError> {
        if matches!(build.lifecycle, super::domain::BuildLifecycle::Cleaned) {
            return Ok(CleanupEligibility::AlreadyCleaned);
        }
        if matches!(
            build.lifecycle,
            super::domain::BuildLifecycle::UnverifiedLegacy
        ) {
            return Ok(CleanupEligibility::SourceUnverified);
        }
        let builds = self
            .database
            .builds()
            .list_for_branch(&build.source.repository_id, &build.source.branch_ref)?;
        let mut latest_attempts = HashMap::<ReviewBuildId, ReviewOperationAttempt>::new();
        for candidate in &builds {
            if let Some(latest) = self
                .database
                .attempts()
                .list_for_build(&candidate.id)?
                .into_iter()
                .next()
            {
                latest_attempts.insert(candidate.id.clone(), latest);
            }
        }
        let keep_successful = self
            .database
            .settings()
            .load()?
            .map(|settings| match settings.retention_policy {
                RetentionPolicy::KeepNewestSuccessfulPerLogicalSource { count } => count as usize,
            })
            .unwrap_or(1);
        let decisions = retention::evaluate(&builds, &latest_attempts, keep_successful);
        let target = decisions
            .iter()
            .find(|decision| decision.build_id == build.id)
            .map(|decision| decision.disposition)
            .unwrap_or(RetentionDisposition::Unverified);
        match target {
            RetentionDisposition::LeaveRunning => return Ok(CleanupEligibility::BuildRunning),
            RetentionDisposition::KeepNewestSuccessful => {
                return Ok(CleanupEligibility::RetentionProtected)
            }
            RetentionDisposition::Unverified => return Ok(CleanupEligibility::SourceUnverified),
            RetentionDisposition::EligibleTerminal => {}
        }
        let has_successful_successor = decisions.iter().any(|decision| {
            decision.disposition == RetentionDisposition::KeepNewestSuccessful
                && builds.iter().any(|candidate| {
                    candidate.id == decision.build_id
                        && candidate.retention_key == build.retention_key
                        && candidate.created_at >= build.created_at
                })
        });
        if !has_successful_successor {
            return Ok(CleanupEligibility::RetentionProtected);
        }
        for resource in resources {
            if matches!(resource, CleanupResource::Workspace { .. }) {
                return Ok(CleanupEligibility::RetentionProtected);
            }
            match self.authorize(build, resource)? {
                ResourceAuthority::Authorized => {}
                ResourceAuthority::Borrowed => return Ok(CleanupEligibility::BorrowedResource),
                ResourceAuthority::Unverified => return Ok(CleanupEligibility::SourceUnverified),
            }
        }
        Ok(CleanupEligibility::Eligible)
    }

    fn authorize(
        &self,
        build: &ReviewBuild,
        resource: &CleanupResource,
    ) -> Result<ResourceAuthority, CleanupServiceError> {
        if !resource.removable_for_build(&build.id) {
            return Ok(ResourceAuthority::Borrowed);
        }
        match resource {
            CleanupResource::Workspace {
                workspace_id,
                ownership,
                containment_root,
                ..
            } => {
                let Some(workspace) = self.database.workspaces().find(workspace_id)? else {
                    return Ok(ResourceAuthority::Unverified);
                };
                if workspace.ownership != *ownership {
                    return Ok(ResourceAuthority::Unverified);
                }
                if !matches!(ownership, WorkspaceOwnership::OwnedBuildWorktree { build_id } if build_id == &build.id)
                {
                    return Ok(ResourceAuthority::Borrowed);
                }
                if containment_root.as_ref() != Some(&self.containment_root)
                    || !path_is_contained(&self.review_root, Path::new(workspace.location.as_str()))
                    || workspace.lifecycle == WorkspaceLifecycle::Unverified
                {
                    return Ok(ResourceAuthority::Unverified);
                }
            }
            CleanupResource::ArtifactSet {
                artifact_set_id,
                storage_key,
                containment_root,
                ..
            } => {
                if containment_root != &self.containment_root
                    || !safe_storage_key(storage_key.as_str())
                    || !self.artifact_matches(build, artifact_set_id, storage_key)?
                {
                    return Ok(ResourceAuthority::Unverified);
                }
            }
            CleanupResource::AttemptLogs {
                attempt_id,
                storage_key,
                containment_root,
                ..
            } => {
                if containment_root != &self.containment_root
                    || !safe_storage_key(storage_key.as_str())
                    || self
                        .database
                        .attempts()
                        .find(attempt_id)?
                        .is_none_or(|attempt| attempt.build_id != build.id)
                {
                    return Ok(ResourceAuthority::Unverified);
                }
            }
            CleanupResource::BuildScratch {
                build_id,
                storage_key,
                containment_root,
                ..
            } => {
                if build_id != &build.id
                    || containment_root != &self.containment_root
                    || !safe_storage_key(storage_key.as_str())
                {
                    return Ok(ResourceAuthority::Unverified);
                }
            }
            CleanupResource::RuntimeInstance { build_id, .. }
            | CleanupResource::PortLease { build_id, .. } => {
                if build_id != &build.id {
                    return Ok(ResourceAuthority::Unverified);
                }
            }
        }
        Ok(ResourceAuthority::Authorized)
    }

    fn artifact_matches(
        &self,
        build: &ReviewBuild,
        artifact_set_id: &ArtifactSetId,
        storage_key: &super::domain::ArtifactStorageKey,
    ) -> Result<bool, CleanupServiceError> {
        Ok(self
            .database
            .artifacts()
            .find(artifact_set_id)?
            .is_some_and(|artifacts| {
                artifacts.build_id == build.id && artifacts.storage_key == *storage_key
            }))
    }

    fn run(&self, mut job: CleanupJob) -> Result<CleanupPresentation, CleanupServiceError> {
        if job.eligibility != CleanupEligibility::Eligible {
            return self.presentation(&job.id);
        }
        if job.state == CleanupJobState::Planned {
            job.state = CleanupJobState::Running;
            job.started_at = Some(self.clock.now());
            self.database.cleanup().save_job(&job)?;
        }
        let existing = self
            .database
            .cleanup()
            .effects(&job.id)?
            .into_iter()
            .map(|effect| (effect.resource_id.clone(), effect))
            .collect::<HashMap<_, _>>();
        let mut effects = Vec::with_capacity(job.resources.len());
        for resource in &job.resources {
            if let Some(effect) = existing.get(resource.id()) {
                if effect.disposition.is_terminal() {
                    effects.push(effect.clone());
                    continue;
                }
            }
            let effect = match self.authorize_for_reconciliation(&job, resource)? {
                ResourceAuthority::Authorized => match self.effects.apply(resource) {
                    Ok(CleanupEffectOutcome::Removed) => effect(
                        resource,
                        CleanupDisposition::Removed,
                        None,
                        self.clock.now(),
                    ),
                    Ok(CleanupEffectOutcome::AlreadyAbsent) => effect(
                        resource,
                        CleanupDisposition::AlreadyAbsent,
                        None,
                        self.clock.now(),
                    ),
                    Err(message) => effect(
                        resource,
                        CleanupDisposition::Failed,
                        Some(message),
                        self.clock.now(),
                    ),
                },
                ResourceAuthority::Borrowed => effect(
                    resource,
                    CleanupDisposition::Failed,
                    Some("Cleanup authority changed: the resource is borrowed.".into()),
                    self.clock.now(),
                ),
                ResourceAuthority::Unverified => effect(
                    resource,
                    CleanupDisposition::Failed,
                    Some("Cleanup authority could not be reverified.".into()),
                    self.clock.now(),
                ),
            };
            self.database.cleanup().record_effect(&job.id, &effect)?;
            effects.push(effect);
        }
        let receipt = CleanupReceipt {
            job_id: job.id.clone(),
            build_id: job.build_id,
            effects,
            completed_at: self.clock.now(),
        };
        self.database.cleanup().finish(&receipt)?;
        self.presentation(&job.id)
    }

    fn authorize_for_reconciliation(
        &self,
        job: &CleanupJob,
        resource: &CleanupResource,
    ) -> Result<ResourceAuthority, CleanupServiceError> {
        let build = self.database.builds().find(&job.build_id)?.ok_or_else(|| {
            CleanupServiceError::new(
                CleanupServiceErrorKind::NotFound,
                "Cleanup build disappeared before reconciliation.",
            )
        })?;
        self.authorize(&build, resource)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ResourceAuthority {
    Authorized,
    Borrowed,
    Unverified,
}

fn effect(
    resource: &CleanupResource,
    disposition: CleanupDisposition,
    detail: Option<String>,
    recorded_at: DateTime<Utc>,
) -> CleanupEffect {
    CleanupEffect {
        resource_id: resource.id().clone(),
        disposition,
        detail,
        recorded_at,
    }
}

fn present(
    job: &CleanupJob,
    effects: &[CleanupEffect],
    receipt: Option<&CleanupReceipt>,
) -> CleanupPresentation {
    let by_resource = effects
        .iter()
        .map(|effect| (&effect.resource_id, effect))
        .collect::<HashMap<_, _>>();
    let resources = job
        .resources
        .iter()
        .map(|resource| {
            let effect = by_resource.get(resource.id()).copied();
            CleanupResourcePresentation {
                resource_id: resource.id().clone(),
                disposition: effect.map(|effect| effect.disposition),
                detail: effect.and_then(|effect| effect.detail.clone()),
            }
        })
        .collect();
    let action_required = job.state == CleanupJobState::AttentionRequired
        || job.eligibility == CleanupEligibility::SourceUnverified;
    let summary = match (job.eligibility, job.state) {
        (CleanupEligibility::Eligible, CleanupJobState::Planned) => {
            "Cleanup is durably planned.".into()
        }
        (CleanupEligibility::Eligible, CleanupJobState::Running) => {
            "Cleanup is reconciling owned resources.".into()
        }
        (CleanupEligibility::Eligible, CleanupJobState::Completed) => {
            "Owned resources were cleaned and the receipt was retained.".into()
        }
        (CleanupEligibility::Eligible, CleanupJobState::AttentionRequired) => {
            "Cleanup stopped with retained failure evidence.".into()
        }
        (CleanupEligibility::BuildRunning, _) => "The running build was left unchanged.".into(),
        (CleanupEligibility::RetentionProtected, _) => {
            "Retention policy protects this build.".into()
        }
        (CleanupEligibility::BorrowedResource, _) => {
            "Cleanup was refused because the ledger includes a borrowed worktree.".into()
        }
        (CleanupEligibility::AlreadyCleaned, _) => {
            "This build already has a cleanup disposition.".into()
        }
        (CleanupEligibility::SourceUnverified, _) => {
            "Cleanup was refused because resource ownership could not be verified.".into()
        }
        (_, _) => "Cleanup state is retained.".into(),
    };
    CleanupPresentation {
        job_id: job.id.clone(),
        build_id: job.build_id.clone(),
        eligibility: job.eligibility,
        state: job.state,
        resources,
        action_required,
        summary,
        completed_at: receipt.map(|receipt| receipt.completed_at),
    }
}

fn containment_identity(root: &Path) -> Result<ContainmentRoot, CleanupServiceError> {
    let normalized = normalized_path(root);
    ContainmentRoot::new(format!(
        "appdata-{}",
        format!("{:x}", Sha256::digest(normalized.as_bytes()))
    ))
    .map_err(|error| {
        CleanupServiceError::new(CleanupServiceErrorKind::InvalidRequest, error.to_string())
    })
}

fn safe_storage_key(value: &str) -> bool {
    let path = Path::new(value);
    !path.as_os_str().is_empty()
        && !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_)))
}

fn path_is_contained(root: &Path, candidate: &Path) -> bool {
    if !candidate.is_absolute()
        || candidate
            .components()
            .any(|component| matches!(component, Component::ParentDir | Component::CurDir))
    {
        return false;
    }
    let observed = candidate
        .canonicalize()
        .unwrap_or_else(|_| candidate.to_path_buf());
    let root = normalized_path(root);
    let observed = normalized_path(&observed);
    observed != root
        && observed
            .strip_prefix(&root)
            .is_some_and(|suffix| suffix.starts_with('/'))
}

fn normalized_path(path: &Path) -> String {
    let value = path.to_string_lossy().replace('\\', "/");
    if cfg!(windows) {
        value.to_ascii_lowercase()
    } else {
        value
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::worktree_review::{
        domain::{
            ArtifactStorageKey, BranchRef, BuildLifecycle, GitObjectId, OperationAttemptId,
            OperationExecutionState, OperationVerdict, RepositoryId, RetentionKey, ReviewBranch,
            ReviewBuildName, ReviewOperationKind, ReviewRepository, ReviewSourceSelection,
            ReviewWorkspace, SourceBinding, SourceFingerprint, WorkspaceId, WorktreeAssociationId,
            WorktreeId, WorktreeLocation,
        },
        storage::{ReviewRepositoryRepository, WorkspaceRepository},
    };
    use std::sync::Mutex;

    struct FixedClock(DateTime<Utc>);

    impl CleanupClock for FixedClock {
        fn now(&self) -> DateTime<Utc> {
            self.0
        }
    }

    struct RecordingEffects {
        database: Arc<WorktreeReviewDatabase>,
        applied: Mutex<Vec<CleanupResourceId>>,
    }

    impl CleanupEffectPort for RecordingEffects {
        fn apply(&self, resource: &CleanupResource) -> Result<CleanupEffectOutcome, String> {
            let running = self
                .database
                .cleanup()
                .list_unsettled()
                .unwrap()
                .into_iter()
                .any(|job| job.state == CleanupJobState::Running);
            if !running {
                return Err("effect ran before durable running state".into());
            }
            self.applied.lock().unwrap().push(resource.id().clone());
            Ok(CleanupEffectOutcome::Removed)
        }
    }

    fn object(character: char) -> GitObjectId {
        GitObjectId::new(character.to_string().repeat(40)).unwrap()
    }

    fn service_fixture() -> (
        tempfile::TempDir,
        Arc<WorktreeReviewDatabase>,
        WorktreeReviewCleanupService,
        Arc<RecordingEffects>,
        ReviewBuild,
    ) {
        let directory = tempfile::tempdir().unwrap();
        let review_root = directory.path().join("review");
        std::fs::create_dir(&review_root).unwrap();
        let database = Arc::new(
            WorktreeReviewDatabase::open(review_root.join("worktree-review.sqlite")).unwrap(),
        );
        let now = Utc::now();
        let repository = ReviewRepository {
            id: RepositoryId::new("repository").unwrap(),
            label: "Repository".into(),
            first_seen_at: now,
            last_seen_at: now,
        };
        let branch = ReviewBranch {
            repository_id: repository.id.clone(),
            branch_ref: BranchRef::new("refs/heads/main").unwrap(),
            observed_tip: object('b'),
            observed_at: now,
        };
        database
            .repositories()
            .save_repository(&repository)
            .unwrap();
        database.repositories().save_branch(&branch).unwrap();
        let old = seed_build(&database, &repository, &branch, "old", object('a'), now);
        let new = seed_build(
            &database,
            &repository,
            &branch,
            "new",
            object('b'),
            now + chrono::Duration::seconds(1),
        );
        seed_passed_attempt(&database, &old, now);
        seed_passed_attempt(&database, &new, now + chrono::Duration::seconds(1));
        let effects = Arc::new(RecordingEffects {
            database: database.clone(),
            applied: Mutex::new(Vec::new()),
        });
        let service = WorktreeReviewCleanupService::new(
            database.clone(),
            effects.clone(),
            Arc::new(FixedClock(now + chrono::Duration::seconds(2))),
            review_root,
        )
        .unwrap();
        (directory, database, service, effects, old)
    }

    fn seed_build(
        database: &WorktreeReviewDatabase,
        repository: &ReviewRepository,
        branch: &ReviewBranch,
        suffix: &str,
        selected: GitObjectId,
        now: DateTime<Utc>,
    ) -> ReviewBuild {
        let id = ReviewBuildId::new(format!("build-{suffix}")).unwrap();
        let workspace_id = WorkspaceId::new(format!("workspace-{suffix}")).unwrap();
        let workspace = ReviewWorkspace {
            id: workspace_id.clone(),
            repository_id: repository.id.clone(),
            worktree_id: WorktreeId::new(format!("worktree-{suffix}")).unwrap(),
            location: WorktreeLocation::new(format!("C:/unused/{suffix}")).unwrap(),
            ownership: WorkspaceOwnership::OwnedBuildWorktree {
                build_id: id.clone(),
            },
            lifecycle: WorkspaceLifecycle::Ready,
            created_at: now,
            updated_at: now,
        };
        database.workspaces().save(&workspace).unwrap();
        let build = ReviewBuild {
            id,
            name: ReviewBuildName::new(format!("Build {suffix}")).unwrap(),
            source: SourceBinding {
                repository_id: repository.id.clone(),
                branch_ref: branch.branch_ref.clone(),
                selection: ReviewSourceSelection::BranchCommit {
                    selected_object: selected.clone(),
                },
                workspace_id: workspace_id.clone(),
                materialized_object: selected,
                materialized_state_fingerprint: SourceFingerprint::new(format!(
                    "fingerprint-{suffix}"
                ))
                .unwrap(),
            },
            workspace_id,
            retention_key: RetentionKey::new("logical-source").unwrap(),
            current_artifact_set_id: None,
            lifecycle: BuildLifecycle::Active,
            created_at: now,
            updated_at: now,
        };
        database.builds().save(&build).unwrap();
        build
    }

    fn seed_passed_attempt(
        database: &WorktreeReviewDatabase,
        build: &ReviewBuild,
        now: DateTime<Utc>,
    ) {
        database
            .attempts()
            .save(&ReviewOperationAttempt {
                id: OperationAttemptId::new(format!("attempt-{}", build.id.as_str())).unwrap(),
                build_id: build.id.clone(),
                kind: ReviewOperationKind::Build,
                execution: OperationExecutionState::Completed,
                verdict: OperationVerdict::Passed,
                active_stage: None,
                failure: None,
                requested_at: now,
                started_at: Some(now),
                completed_at: Some(now),
            })
            .unwrap();
    }

    #[test]
    fn persists_the_job_before_applying_an_owned_effect() {
        let (_directory, _database, service, effects, old) = service_fixture();
        let resource = CleanupResource::BuildScratch {
            id: CleanupResourceId::new("scratch").unwrap(),
            build_id: old.id.clone(),
            storage_key: ArtifactStorageKey::new("builds/old/scratch").unwrap(),
            containment_root: service.containment_root().clone(),
        };
        let presentation = service
            .cleanup_superseded_appdata_resources(CleanupRequest {
                build_id: old.id,
                resources: vec![resource],
            })
            .unwrap();
        assert_eq!(presentation.state, CleanupJobState::Completed);
        assert_eq!(effects.applied.lock().unwrap().len(), 1);
        assert!(presentation.completed_at.is_some());
    }

    #[test]
    fn every_build_workspace_is_retained_without_an_effect() {
        let (_directory, database, service, effects, old) = service_fixture();
        let association_id = WorktreeAssociationId::new("association").unwrap();
        let workspace = ReviewWorkspace {
            id: WorkspaceId::new("borrowed-workspace").unwrap(),
            repository_id: old.source.repository_id.clone(),
            worktree_id: WorktreeId::new("borrowed-worktree").unwrap(),
            location: WorktreeLocation::new("C:/borrowed").unwrap(),
            ownership: WorkspaceOwnership::BorrowedExternal {
                association_id: association_id.clone(),
            },
            lifecycle: WorkspaceLifecycle::Ready,
            created_at: Utc::now(),
            updated_at: Utc::now(),
        };
        database.workspaces().save(&workspace).unwrap();
        let presentation = service
            .cleanup_superseded_appdata_resources(CleanupRequest {
                build_id: old.id,
                resources: vec![CleanupResource::Workspace {
                    id: CleanupResourceId::new("borrowed").unwrap(),
                    workspace_id: workspace.id,
                    ownership: WorkspaceOwnership::BorrowedExternal { association_id },
                    containment_root: None,
                }],
            })
            .unwrap();
        assert_eq!(
            presentation.eligibility,
            CleanupEligibility::RetentionProtected
        );
        assert_eq!(presentation.state, CleanupJobState::NotEligible);
        assert!(effects.applied.lock().unwrap().is_empty());
    }

    #[test]
    fn rejects_an_appdata_key_that_escapes_the_product_root() {
        let (_directory, _database, service, effects, old) = service_fixture();
        let presentation = service
            .cleanup_superseded_appdata_resources(CleanupRequest {
                build_id: old.id.clone(),
                resources: vec![CleanupResource::BuildScratch {
                    id: CleanupResourceId::new("escape").unwrap(),
                    build_id: old.id,
                    storage_key: ArtifactStorageKey::new("../foreign").unwrap(),
                    containment_root: service.containment_root().clone(),
                }],
            })
            .unwrap();
        assert_eq!(
            presentation.eligibility,
            CleanupEligibility::SourceUnverified
        );
        assert!(effects.applied.lock().unwrap().is_empty());
    }

    #[test]
    fn restart_reconciliation_skips_a_terminal_effect_and_finishes_its_receipt() {
        let (_directory, database, service, effects, old) = service_fixture();
        let resource = CleanupResource::BuildScratch {
            id: CleanupResourceId::new("already-recorded").unwrap(),
            build_id: old.id.clone(),
            storage_key: ArtifactStorageKey::new("builds/old/already-recorded").unwrap(),
            containment_root: service.containment_root().clone(),
        };
        let now = Utc::now();
        let job = CleanupJob {
            id: CleanupJobId::new("interrupted-cleanup").unwrap(),
            build_id: old.id,
            trigger: CleanupTrigger::RetentionPolicy,
            eligibility: CleanupEligibility::Eligible,
            state: CleanupJobState::Running,
            resources: vec![resource.clone()],
            created_at: now,
            started_at: Some(now),
            settled_at: None,
        };
        database.cleanup().save_job(&job).unwrap();
        database
            .cleanup()
            .record_effect(
                &job.id,
                &effect(&resource, CleanupDisposition::Removed, None, now),
            )
            .unwrap();

        let presentations = service.reconcile_unfinished().unwrap();

        assert_eq!(presentations.len(), 1);
        assert_eq!(presentations[0].state, CleanupJobState::Completed);
        assert!(effects.applied.lock().unwrap().is_empty());
        assert!(database.cleanup().find_receipt(&job.id).unwrap().is_some());
    }
}
