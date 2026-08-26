use super::{
    artifact_store::ArtifactStore,
    build_executor::{BuildExecutionFailure, ReviewBuildExecutor},
    build_presentation::{
        artifact_view, cleanup_presentation_view, cleanup_view, review_build_view, BuildSourceInput,
    },
    cleanup_service::{CleanupRequest, WorktreeReviewCleanupService},
    domain::{
        ArtifactStorageKey, BranchRef, BuildAttention, BuildAttentionCategory, BuildAttentionId,
        BuildLifecycle, CleanupJobState, CleanupResource, CleanupResourceId, OperationAttemptId,
        OperationExecutionState, OperationFailure, OperationFailureCategory, OperationStage,
        OperationVerdict, RepositoryId, RetentionKey, ReviewBuild, ReviewBuildId, ReviewBuildName,
        ReviewOperationAttempt, ReviewOperationKind, WorkspaceId,
    },
    source_materialization::SourceMaterializationService,
    state::{
        WorktreeReviewApplication, ACTIVE_REVIEW_BUILD_ID_ENV, ACTIVE_REVIEW_WORKTREE_ID_ENV,
        WORKTREE_REVIEW_DATA_DIR_ENV,
    },
    storage::{
        ArtifactSetRepository, BuildAttentionRepository, CleanupRepository,
        OperationAttemptRepository, ReviewBuildRepository, WorkspaceRepository,
        WorktreeAssociationRepository, WorktreeReviewDatabase,
    },
};
use crate::{
    repository_context::{BranchRef as ObservedBranch, RepositoryIdentity},
    worktree_application::{
        PhysicalWorktreeApplication, PhysicalWorktreeBuildResult, WorktreeApplicationLaunchContext,
    },
};
use chrono::Utc;
use sha2::{Digest, Sha256};
use std::{ffi::OsString, sync::Arc};

pub(crate) use super::build_presentation::{CreateBuildInput, ReviewBuildView};

pub(crate) struct ReviewBuildCoordinator {
    application: Arc<WorktreeReviewApplication>,
    database: Arc<WorktreeReviewDatabase>,
    cleanup: WorktreeReviewCleanupService,
    materialization: SourceMaterializationService,
}

impl ReviewBuildCoordinator {
    pub(crate) fn open(application: Arc<WorktreeReviewApplication>) -> Result<Self, String> {
        let database = application.database().map_err(|error| error.message)?;
        let cleanup = WorktreeReviewCleanupService::open_appdata(
            database.clone(),
            application.review_root().to_path_buf(),
        )
        .map_err(|error| error.message)?;
        let materialization = SourceMaterializationService::new(
            database.clone(),
            application.review_root().to_path_buf(),
        );
        let coordinator = Self {
            application,
            database,
            cleanup,
            materialization,
        };
        coordinator.reconcile_unsettled()?;
        Ok(coordinator)
    }

    pub(crate) fn create_build(&self, input: CreateBuildInput) -> Result<ReviewBuildView, String> {
        validate_build_name(&input.name)?;
        let context = self
            .application
            .repository_context()
            .map_err(|error| error.message)?;
        let repository = self
            .application
            .selected_repository()
            .map_err(|error| error.message)?;
        if repository.id.as_str() != input.repository_id {
            return Err("The repository selection changed; refresh Worktree Review.".into());
        }
        let branch =
            self.materialization
                .selected_branch(&context, &repository, &input.branch_ref)?;
        let build_id = ReviewBuildId::random();
        let workspace_id = WorkspaceId::random();
        let prepared = self.materialization.prepare(
            &context,
            &repository,
            &branch,
            &build_id,
            workspace_id,
            &input.source,
            &input.workspace_plan,
        )?;
        let now = Utc::now();
        let build = ReviewBuild {
            id: build_id.clone(),
            name: ReviewBuildName::new(input.name).map_err(|error| error.to_string())?,
            source: prepared.source().clone(),
            workspace_id: prepared.workspace().id.clone(),
            retention_key: retention_key(&repository, &branch, &input.source)?,
            current_artifact_set_id: None,
            lifecycle: BuildLifecycle::Active,
            created_at: now,
            updated_at: now,
        };
        let mut materialization_attempt = ReviewOperationAttempt {
            id: OperationAttemptId::random(),
            build_id: build_id.clone(),
            kind: ReviewOperationKind::MaterializeSource,
            execution: OperationExecutionState::Pending,
            verdict: OperationVerdict::Unknown,
            active_stage: Some(prepared.stage()),
            failure: None,
            requested_at: now,
            started_at: None,
            completed_at: None,
        };
        self.database
            .transaction(|transaction| {
                transaction.workspaces().save(prepared.workspace())?;
                transaction.builds().save(&build)?;
                transaction.attempts().save(&materialization_attempt)
            })
            .map_err(|error| error.to_string())?;

        materialization_attempt.execution = OperationExecutionState::Running;
        materialization_attempt.started_at = Some(Utc::now());
        self.database
            .attempts()
            .save(&materialization_attempt)
            .map_err(|error| error.to_string())?;

        let materialized =
            match self
                .materialization
                .materialize(&context, &repository, &branch, prepared)
            {
                Ok(materialized) => materialized,
                Err(message) => {
                    let failure_stage = materialization_attempt
                        .active_stage
                        .unwrap_or(OperationStage::WorktreeProvisioning);
                    settle_failure(
                        &mut materialization_attempt,
                        BuildExecutionFailure {
                            stage: failure_stage,
                            category: OperationFailureCategory::ProvisioningFailed,
                            message,
                        },
                    );
                    self.database
                        .attempts()
                        .save(&materialization_attempt)
                        .map_err(|error| error.to_string())?;
                    return self.view(&build.id);
                }
            };
        materialization_attempt.execution = OperationExecutionState::Completed;
        materialization_attempt.verdict = OperationVerdict::Passed;
        materialization_attempt.completed_at = Some(Utc::now());
        self.database
            .transaction(|transaction| {
                transaction.workspaces().save(materialized.workspace())?;
                if let Some(association) = materialized.association() {
                    transaction.associations().save(association)?;
                }
                transaction.attempts().save(&materialization_attempt)
            })
            .map_err(|error| error.to_string())?;

        let mut attempt = ReviewOperationAttempt {
            id: OperationAttemptId::random(),
            build_id: build_id.clone(),
            kind: ReviewOperationKind::Build,
            execution: OperationExecutionState::Pending,
            verdict: OperationVerdict::Unknown,
            active_stage: None,
            failure: None,
            requested_at: Utc::now(),
            started_at: None,
            completed_at: None,
        };
        self.database
            .attempts()
            .save(&attempt)
            .map_err(|error| error.to_string())?;
        attempt.execution = OperationExecutionState::Running;
        attempt.active_stage = Some(OperationStage::DependencyProvisioning);
        attempt.started_at = Some(Utc::now());
        self.database
            .attempts()
            .save(&attempt)
            .map_err(|error| error.to_string())?;

        let executor = ReviewBuildExecutor::open(self.application.review_root(), &repository);
        let execution = executor.and_then(|executor| {
            executor.provision_dependencies(materialized.workspace())?;
            attempt.active_stage = Some(OperationStage::Compilation);
            self.database
                .attempts()
                .save(&attempt)
                .map_err(|error| BuildExecutionFailure {
                    stage: OperationStage::Compilation,
                    category: OperationFailureCategory::Internal,
                    message: error.to_string(),
                })?;
            executor.execute(&build.id, &attempt.id, materialized.workspace())
        });
        match execution {
            Ok(artifacts) => {
                attempt.execution = OperationExecutionState::Completed;
                attempt.verdict = OperationVerdict::Passed;
                attempt.active_stage = Some(OperationStage::ArtifactPromotion);
                attempt.completed_at = Some(Utc::now());
                self.database
                    .transaction(|transaction| {
                        transaction.artifacts().save_verified(&artifacts)?;
                        transaction.builds().set_current_artifact(
                            &build.id,
                            &artifacts.id,
                            Utc::now(),
                        )?;
                        transaction.attempts().save(&attempt)
                    })
                    .map_err(|error| error.to_string())?;
                if let Err(error) = self.cleanup_superseded_appdata(&build) {
                    self.database
                        .attentions()
                        .append(&BuildAttention {
                            id: BuildAttentionId::random(),
                            build_id: build.id.clone(),
                            category: BuildAttentionCategory::CleanupCoordination,
                            summary: format!(
                                "The build passed, but retention cleanup needs attention: {error}"
                            ),
                            recorded_at: Utc::now(),
                            resolved_at: None,
                        })
                        .map_err(|attention_error| {
                            format!(
                                "The build passed, but its cleanup attention could not be retained: {attention_error}"
                            )
                        })?;
                }
            }
            Err(failure) => {
                settle_failure(&mut attempt, failure);
                self.database
                    .attempts()
                    .save(&attempt)
                    .map_err(|error| error.to_string())?;
            }
        }
        self.view(&build.id)
    }

    pub(crate) fn list_for_branch(
        &self,
        repository_id: &str,
        branch_ref: &str,
    ) -> Result<Vec<ReviewBuildView>, String> {
        let repository_id = RepositoryId::new(repository_id).map_err(|error| error.to_string())?;
        let branch_ref = BranchRef::new(branch_ref).map_err(|error| error.to_string())?;
        self.database
            .builds()
            .list_for_branch(&repository_id, &branch_ref)
            .map_err(|error| error.to_string())?
            .into_iter()
            .map(|build| self.view_build(build))
            .collect()
    }

    pub(crate) fn view(&self, build_id: &ReviewBuildId) -> Result<ReviewBuildView, String> {
        let build = self
            .database
            .builds()
            .find(build_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The review build is unavailable.".to_string())?;
        self.view_build(build)
    }

    /// Best-effort focus-or-launch. Opening is intentionally not a persisted lifecycle operation.
    pub(crate) fn open_build(&self, build_id: &str) -> Result<(), String> {
        let build_id = ReviewBuildId::new(build_id).map_err(|error| error.to_string())?;
        let build = self
            .database
            .builds()
            .find(&build_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The review build is unavailable.".to_string())?;
        let workspace = self
            .database
            .workspaces()
            .find(&build.workspace_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The review build worktree record is unavailable.".to_string())?;
        let artifact_id = build
            .current_artifact_set_id
            .as_ref()
            .ok_or_else(|| "This build has no retained application output.".to_string())?;
        let artifacts = self
            .database
            .artifacts()
            .find(artifact_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The retained application output is unavailable.".to_string())?;
        #[cfg(windows)]
        let executable_relative = "cargo-target/debug/codex-orchestrator.exe";
        #[cfg(not(windows))]
        let executable_relative = "cargo-target/debug/codex-orchestrator";
        let executable = artifacts
            .files
            .iter()
            .find(|file| file.relative_path.as_str() == executable_relative)
            .ok_or_else(|| "The retained application executable is unavailable.".to_string())?;
        let artifact_store = ArtifactStore::open(self.application.review_root().to_path_buf())?;
        let (output_root, executable_path) =
            artifact_store.resolve_file(&artifacts.storage_key, &executable.relative_path)?;
        let launch_context = WorktreeApplicationLaunchContext::new([
            (
                OsString::from(WORKTREE_REVIEW_DATA_DIR_ENV),
                self.application.review_root().as_os_str().to_os_string(),
            ),
            (
                OsString::from(ACTIVE_REVIEW_BUILD_ID_ENV),
                OsString::from(build.id.as_str()),
            ),
            (
                OsString::from(ACTIVE_REVIEW_WORKTREE_ID_ENV),
                OsString::from(workspace.worktree_id.as_str()),
            ),
        ])
        .map_err(|error| error.message)?;
        PhysicalWorktreeApplication
            .open(
                &PhysicalWorktreeBuildResult {
                    worktree_root: workspace.location.as_str().into(),
                    output_root,
                    executable: executable_path,
                },
                &launch_context,
            )
            .map_err(|error| error.message)?;
        Ok(())
    }

    fn view_build(&self, build: ReviewBuild) -> Result<ReviewBuildView, String> {
        let workspace = self
            .database
            .workspaces()
            .find(&build.workspace_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The review build workspace is unavailable.".to_string())?;
        let latest_attempt = self
            .database
            .attempts()
            .list_for_build(&build.id)
            .map_err(|error| error.to_string())?
            .into_iter()
            .next();
        let cleanup_job = self
            .database
            .cleanup()
            .list_for_build(&build.id)
            .map_err(|error| error.to_string())?
            .into_iter()
            .next();
        let artifacts = match build.current_artifact_set_id.as_ref() {
            Some(artifact_id) => Some(
                self.database
                    .artifacts()
                    .find(artifact_id)
                    .map_err(|error| error.to_string())?
                    .ok_or_else(|| {
                        "The build references an unavailable artifact manifest.".to_string()
                    })?,
            ),
            None => None,
        };
        let cleanup_effects = match cleanup_job.as_ref() {
            Some(job) => self
                .database
                .cleanup()
                .effects(&job.id)
                .map_err(|error| error.to_string())?,
            None => Vec::new(),
        };
        let artifact = artifact_view(
            &build,
            latest_attempt.as_ref(),
            artifacts.as_ref(),
            cleanup_job.as_ref(),
            &cleanup_effects,
        )?;
        let cleanup = match cleanup_job.as_ref() {
            Some(job) => cleanup_presentation_view(
                self.cleanup
                    .presentation(&job.id)
                    .map_err(|error| error.message)?,
            ),
            None => cleanup_view(&build, latest_attempt.as_ref()),
        };
        let attention = self
            .database
            .attentions()
            .find_active_for_build(&build.id)
            .map_err(|error| error.to_string())?
            .into_iter()
            .next();
        Ok(review_build_view(
            &build,
            &workspace,
            latest_attempt.as_ref(),
            artifact,
            cleanup,
            attention,
        ))
    }

    fn reconcile_unsettled(&self) -> Result<(), String> {
        let now = Utc::now();
        for mut attempt in self
            .database
            .attempts()
            .list_unsettled()
            .map_err(|error| error.to_string())?
        {
            attempt.execution = OperationExecutionState::Interrupted;
            attempt.verdict = OperationVerdict::Unknown;
            attempt.active_stage = Some(OperationStage::InterruptionReconciliation);
            attempt.failure = None;
            attempt.completed_at = Some(now);
            self.database
                .attempts()
                .save(&attempt)
                .map_err(|error| error.to_string())?;
        }
        Ok(())
    }

    fn cleanup_superseded_appdata(&self, successful: &ReviewBuild) -> Result<(), String> {
        let builds = self
            .database
            .builds()
            .list_for_branch(
                &successful.source.repository_id,
                &successful.source.branch_ref,
            )
            .map_err(|error| error.to_string())?;
        for candidate in builds {
            if candidate.id == successful.id || candidate.retention_key != successful.retention_key
            {
                continue;
            }
            if self
                .database
                .cleanup()
                .list_for_build(&candidate.id)
                .map_err(|error| error.to_string())?
                .into_iter()
                .any(|job| job.state == CleanupJobState::Completed)
            {
                continue;
            }
            let mut resources = Vec::new();
            if let Some(artifact_id) = &candidate.current_artifact_set_id {
                if let Some(artifacts) = self
                    .database
                    .artifacts()
                    .find(artifact_id)
                    .map_err(|error| error.to_string())?
                {
                    resources.push(CleanupResource::ArtifactSet {
                        id: CleanupResourceId::random(),
                        artifact_set_id: artifacts.id,
                        storage_key: artifacts.storage_key,
                        containment_root: self.cleanup.containment_root().clone(),
                    });
                }
            }
            resources.push(CleanupResource::BuildScratch {
                id: CleanupResourceId::random(),
                build_id: candidate.id.clone(),
                storage_key: ArtifactStorageKey::new(format!(
                    "repositories/{}/runtime/dependency-logs/{}.log",
                    candidate.source.repository_id.as_str(),
                    candidate.workspace_id.as_str(),
                ))
                .map_err(|error| error.to_string())?,
                containment_root: self.cleanup.containment_root().clone(),
            });
            self.cleanup
                .cleanup_superseded_appdata_resources(CleanupRequest {
                    build_id: candidate.id,
                    resources,
                })
                .map_err(|error| error.message)?;
        }
        Ok(())
    }
}

fn retention_key(
    repository: &RepositoryIdentity,
    branch: &ObservedBranch,
    source: &BuildSourceInput,
) -> Result<RetentionKey, String> {
    let semantic_source = match source {
        BuildSourceInput::ExistingWorktree { association_id, .. } => {
            format!("existing:{association_id}")
        }
        BuildSourceInput::WorktreeSnapshot { association_id, .. } => {
            format!("snapshot:{association_id}")
        }
        BuildSourceInput::BranchCommit { .. } => "branch-commit".into(),
    };
    let mut digest = Sha256::new();
    for value in [
        repository.id.as_str(),
        branch.full_name.as_str(),
        semantic_source.as_str(),
    ] {
        digest.update((value.len() as u64).to_be_bytes());
        digest.update(value.as_bytes());
    }
    RetentionKey::new(format!("retention-{:x}", digest.finalize()))
        .map_err(|error| error.to_string())
}

fn settle_failure(attempt: &mut ReviewOperationAttempt, failure: BuildExecutionFailure) {
    attempt.execution = OperationExecutionState::Completed;
    attempt.verdict = OperationVerdict::Failed;
    attempt.active_stage = Some(failure.stage);
    attempt.failure = Some(OperationFailure {
        stage: failure.stage,
        category: failure.category,
        message: failure.message,
    });
    attempt.completed_at = Some(Utc::now());
}

fn validate_build_name(name: &str) -> Result<(), String> {
    if name.trim().is_empty() || name != name.trim() || name.chars().count() > 120 {
        Err("Build name must be a non-blank label of at most 120 characters.".into())
    } else {
        Ok(())
    }
}
