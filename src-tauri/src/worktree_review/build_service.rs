use super::{
    build_executor::{resolve_retained_output, BuildExecutionFailure, ReviewBuildExecutor},
    build_presentation::{
        build_output_view, cleanup_presentation_view, cleanup_view, review_build_view,
        CreateBuildSourceInput,
    },
    build_storage::attempt_storage_key,
    cleanup_service::{CleanupRequest, WorktreeReviewCleanupService},
    domain::{
        BuildAttention, BuildAttentionCategory, BuildAttentionId, BuildLifecycle, CleanupJobState,
        CleanupResource, CleanupResourceId, OperationAttemptId, OperationExecutionState,
        OperationFailure, OperationFailureCategory, OperationStage, OperationVerdict, RepositoryId,
        RetentionKey, ReviewBuild, ReviewBuildId, ReviewBuildName, ReviewOperationAttempt,
        ReviewOperationKind, WorkspaceId,
    },
    source_materialization::SourceMaterializationService,
    state::{
        WorktreeReviewApplication, ACTIVE_REVIEW_BUILD_ID_ENV, ACTIVE_REVIEW_WORKTREE_ID_ENV,
        WORKTREE_REVIEW_DATA_DIR_ENV,
    },
    storage::{
        BuildAttentionRepository, BuildOutputRepository, CleanupRepository,
        OperationAttemptRepository, ReviewBuildRepository, WorkspaceRepository,
        WorktreeAssociationRepository, WorktreeReviewDatabase,
    },
};
use crate::{
    repository_context::{BranchRef as ObservedBranch, RepositoryIdentity},
    worktree_application::{PhysicalWorktreeApplication, WorktreeApplicationLaunchContext},
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
        let branch = input
            .branch_ref
            .as_ref()
            .map(|reference| {
                self.materialization
                    .selected_branch(&context, &repository, reference)
            })
            .transpose()?;
        let build_id = ReviewBuildId::random();
        let workspace_id = WorkspaceId::random();
        let prepared = self.materialization.prepare(
            &context,
            &repository,
            branch.as_ref(),
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
            retention_key: retention_key(&repository, branch.as_ref(), &input.source)?,
            current_output_id: None,
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
                .materialize(&context, &repository, branch.as_ref(), prepared)
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
        attempt.active_stage = Some(OperationStage::Compilation);
        attempt.started_at = Some(Utc::now());
        self.database
            .attempts()
            .save(&attempt)
            .map_err(|error| error.to_string())?;

        let executor = ReviewBuildExecutor::open(self.application.review_root(), &repository);
        let execution = executor.and_then(|executor| {
            executor.execute(&build.id, &attempt.id, materialized.workspace())
        });
        match execution {
            Ok(output) => {
                attempt.execution = OperationExecutionState::Completed;
                attempt.verdict = OperationVerdict::Passed;
                attempt.active_stage = Some(OperationStage::OutputPublication);
                attempt.completed_at = Some(Utc::now());
                let retained = self.database.transaction(|transaction| {
                    transaction.attempts().save(&attempt)?;
                    transaction.outputs().save(&output)?;
                    transaction
                        .builds()
                        .set_current_output(&build.id, &output.id, Utc::now())
                });
                if let Err(error) = retained {
                    settle_failure(
                        &mut attempt,
                        BuildExecutionFailure {
                            stage: OperationStage::OutputPublication,
                            category: OperationFailureCategory::OutputPublicationFailed,
                            message: format!(
                                "The compiled output could not be recorded durably: {error}"
                            ),
                        },
                    );
                    self.database
                        .attempts()
                        .save(&attempt)
                        .map_err(|save_error| save_error.to_string())?;
                } else if let Err(error) = self.cleanup_superseded_appdata(&build) {
                    self.database
                        .attentions()
                        .append(&BuildAttention {
                            id: BuildAttentionId::random(),
                            build_id: build.id.clone(),
                            category: BuildAttentionCategory::CleanupCoordination,
                            summary: format!(
                                "The build succeeded, but retention cleanup needs attention: {error}"
                            ),
                            recorded_at: Utc::now(),
                            resolved_at: None,
                        })
                        .map_err(|attention_error| {
                            format!(
                                "The build succeeded, but its cleanup attention could not be retained: {attention_error}"
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
        self.list_for_target(&super::domain::ReviewTarget::Branch {
            repository_id: repository_id.into(),
            branch_ref: branch_ref.into(),
        })
    }

    pub(crate) fn list_for_target(
        &self,
        target: &super::domain::ReviewTarget,
    ) -> Result<Vec<ReviewBuildView>, String> {
        use super::domain::{CommitSourceContext, ReviewSourceSelection, ReviewTarget};
        let id = RepositoryId::new(target.repository_id()).map_err(|error| error.to_string())?;
        let associations = self
            .database
            .associations()
            .list_for_repository(&id)
            .map_err(|error| error.to_string())?;
        let mut result = Vec::new();
        for build in self
            .database
            .builds()
            .list_for_repository(&id)
            .map_err(|error| error.to_string())?
        {
            let origin_worktree = match &build.source.selection {
                ReviewSourceSelection::PhysicalWorktree { worktree_id, .. } => {
                    Some(worktree_id.as_str())
                }
                ReviewSourceSelection::LiveWorktree { association_id, .. }
                | ReviewSourceSelection::WorktreeSnapshot { association_id, .. } => associations
                    .iter()
                    .find(|item| &item.id == association_id)
                    .map(|item| item.worktree_id.as_str()),
                _ => None,
            };
            let matches = match target {
                ReviewTarget::Branch { branch_ref, .. } => {
                    build
                        .source
                        .branch_ref
                        .as_ref()
                        .is_some_and(|reference| reference.as_str() == branch_ref)
                        || matches!(&build.source.selection, ReviewSourceSelection::ExactCommit { context: CommitSourceContext::Branch { branch_ref: source, .. }, .. } if source == branch_ref)
                        || origin_worktree.is_some_and(|worktree| {
                            associations.iter().any(|item| {
                                item.worktree_id.as_str() == worktree
                                    && item.branch_ref.as_str() == branch_ref
                            })
                        })
                }
                ReviewTarget::Worktree { worktree_id, .. } => {
                    origin_worktree == Some(worktree_id.as_str())
                        || self
                            .database
                            .workspaces()
                            .find(&build.workspace_id)
                            .map_err(|error| error.to_string())?
                            .is_some_and(|workspace| workspace.worktree_id.as_str() == worktree_id)
                }
                ReviewTarget::Commit { object_id, .. } => match &build.source.selection {
                    ReviewSourceSelection::BranchCommit { selected_object }
                    | ReviewSourceSelection::ExactCommit {
                        selected_object, ..
                    } => selected_object.as_str() == object_id,
                    _ => false,
                },
            };
            if matches {
                result.push(self.view_build(build)?);
            }
        }
        Ok(result)
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
        let output_id = build
            .current_output_id
            .as_ref()
            .ok_or_else(|| "This build has no retained application output.".to_string())?;
        let output = self
            .database
            .outputs()
            .find(output_id)
            .map_err(|error| error.to_string())?
            .ok_or_else(|| "The retained application output is unavailable.".to_string())?;
        let physical_build =
            resolve_retained_output(self.application.review_root(), &workspace, &output)?;
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
            .open(&physical_build, &launch_context)
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
        let output = match build.current_output_id.as_ref() {
            Some(output_id) => self
                .database
                .outputs()
                .find(output_id)
                .map_err(|error| error.to_string())?,
            None => None,
        };
        let output_exists = output.as_ref().is_some_and(|output| {
            resolve_retained_output(self.application.review_root(), &workspace, output).is_ok()
        });
        let cleanup_effects = match cleanup_job.as_ref() {
            Some(job) => self
                .database
                .cleanup()
                .effects(&job.id)
                .map_err(|error| error.to_string())?,
            None => Vec::new(),
        };
        let output_view = build_output_view(
            &build,
            output.as_ref(),
            output_exists,
            cleanup_job.as_ref(),
            &cleanup_effects,
        );
        let cleanup = match cleanup_job.as_ref() {
            Some(job) => cleanup_presentation_view(
                self.cleanup
                    .presentation(&job.id)
                    .map_err(|error| error.message)?,
            )?,
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
            output_view,
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
            if let Some(output_id) = &candidate.current_output_id {
                if let Some(output) = self
                    .database
                    .outputs()
                    .find(output_id)
                    .map_err(|error| error.to_string())?
                {
                    resources.push(CleanupResource::BuildOutput {
                        id: CleanupResourceId::random(),
                        output_id: output.id,
                        storage_key: output.storage_key,
                        containment_root: self.cleanup.containment_root().clone(),
                    });
                }
            }
            for attempt in self
                .database
                .attempts()
                .list_for_build(&candidate.id)
                .map_err(|error| error.to_string())?
            {
                if attempt.kind != ReviewOperationKind::Build || !attempt.is_terminal() {
                    continue;
                }
                resources.push(CleanupResource::BuildAttemptStorage {
                    id: CleanupResourceId::random(),
                    build_id: candidate.id.clone(),
                    attempt_id: Some(attempt.id.clone()),
                    storage_key: attempt_storage_key(
                        &candidate.source.repository_id,
                        &candidate.id,
                        &attempt.id,
                    )
                    .map_err(|error| error.to_string())?,
                    containment_root: self.cleanup.containment_root().clone(),
                });
            }
            if resources.is_empty() {
                continue;
            }
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
    branch: Option<&ObservedBranch>,
    source: &CreateBuildSourceInput,
) -> Result<RetentionKey, String> {
    let semantic_source = match source {
        CreateBuildSourceInput::PhysicalWorktree {
            worktree_id,
            snapshot,
            ..
        } => format!("physical:{worktree_id}:{snapshot}"),
        CreateBuildSourceInput::ExactCommit { context, .. } => format!(
            "exact:{}",
            serde_json::to_string(context).map_err(|error| error.to_string())?
        ),
        CreateBuildSourceInput::LiveWorktree { association_id, .. } => {
            format!("existing:{association_id}")
        }
        CreateBuildSourceInput::WorktreeSnapshot { association_id, .. } => {
            format!("snapshot:{association_id}")
        }
        CreateBuildSourceInput::BranchCommit { .. } => "branch-commit".into(),
    };
    let mut digest = Sha256::new();
    for value in [
        repository.id.as_str(),
        branch.map(|branch| branch.full_name.as_str()).unwrap_or(""),
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
