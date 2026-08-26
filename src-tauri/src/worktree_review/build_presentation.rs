//! Transport representation of durable build facts.
//!
//! This module has no storage or process authority. It maps already-loaded domain facts into the
//! Worktree Review product contract; orchestration and source materialization stay in the build
//! coordinator.

use super::{
    cleanup_service::CleanupPresentation,
    domain::{
        BranchRef, BuildAttention, BuildLifecycle, CleanupDisposition, CleanupEffect,
        CleanupEligibility, CleanupJob, CleanupJobState, CleanupResource, OperationExecutionState,
        OperationFailureCategory, OperationStage, OperationVerdict, ReviewBuild,
        ReviewOperationAttempt, ReviewSourceSelection, ReviewWorkspace, VerifiedArtifactSet,
    },
};
use chrono::Utc;
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CreateBuildInput {
    pub(crate) repository_id: String,
    pub(crate) branch_ref: String,
    pub(crate) name: String,
    pub(crate) source: BuildSourceInput,
    pub(crate) workspace_plan: BuildWorkspacePlanInput,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum BuildSourceInput {
    ExistingWorktree {
        association_id: String,
        expected_head: String,
        state_fingerprint: String,
    },
    WorktreeSnapshot {
        association_id: String,
        base_object_id: String,
        state_fingerprint: String,
    },
    BranchCommit {
        branch_ref: String,
        object_id: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum BuildWorkspacePlanInput {
    BorrowSelectedWorktree {
        association_id: String,
    },
    CreateManagedBranchWorktree {
        branch_ref: String,
    },
    CreateOwnedBuildWorktree {
        originating_association_id: Option<String>,
        object_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewBuildView {
    pub(crate) build_id: String,
    pub(crate) name: String,
    pub(crate) branch_ref: String,
    pub(crate) source: BuildSourceInput,
    pub(crate) workspace: BuildWorkspaceView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) latest_attempt: Option<ReviewOperationAttemptView>,
    pub(crate) artifact: ArtifactStateView,
    pub(crate) cleanup: CleanupStateView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) attention: Option<BuildAttentionView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildAttentionView {
    pub(crate) category: String,
    pub(crate) summary: String,
    pub(crate) recorded_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildWorkspaceView {
    pub(crate) worktree_id: String,
    pub(crate) ownership: String,
    pub(crate) location_label: String,
    pub(crate) lifecycle: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewOperationAttemptView {
    pub(crate) attempt_id: String,
    pub(crate) execution_state: String,
    pub(crate) verdict: String,
    pub(crate) stage: String,
    pub(crate) started_at: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) completed_at: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) failure: Option<BuildAttemptFailureView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BuildAttemptFailureView {
    pub(crate) category: String,
    pub(crate) stage: String,
    pub(crate) summary: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum ArtifactStateView {
    NotProduced,
    PromotionFailed {
        summary: String,
    },
    Available {
        artifact_set_id: String,
        file_count: usize,
        manifest_hash: String,
        storage_label: String,
    },
    Removed {
        artifact_set_id: String,
        removed_at: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum CleanupStateView {
    Retained {
        policy: String,
    },
    NotEligible {
        reason: String,
    },
    Eligible {
        reason: String,
    },
    Running {
        cleanup_job_id: String,
        completed_effects: usize,
        total_effects: usize,
    },
    Failed {
        cleanup_job_id: String,
        summary: String,
    },
    Complete {
        cleanup_receipt_id: String,
        completed_at: String,
        summary: String,
    },
}

pub(super) fn review_build_view(
    build: &ReviewBuild,
    workspace: &ReviewWorkspace,
    latest_attempt: Option<&ReviewOperationAttempt>,
    artifact: ArtifactStateView,
    cleanup: CleanupStateView,
    attention: Option<BuildAttention>,
) -> ReviewBuildView {
    ReviewBuildView {
        build_id: build.id.as_str().to_owned(),
        name: build.name.as_str().to_owned(),
        branch_ref: build.source.branch_ref.as_str().to_owned(),
        source: source_view(&build.source.branch_ref, &build.source.selection),
        workspace: BuildWorkspaceView {
            worktree_id: workspace.worktree_id.as_str().to_owned(),
            ownership: workspace.ownership.as_str().to_owned(),
            location_label: workspace.location.as_str().to_owned(),
            lifecycle: workspace.lifecycle.as_str().to_owned(),
        },
        latest_attempt: latest_attempt.map(attempt_view),
        artifact,
        cleanup,
        attention: attention.map(|attention| BuildAttentionView {
            category: attention.category.as_str().to_owned(),
            summary: attention.summary,
            recorded_at: attention.recorded_at.to_rfc3339(),
        }),
    }
}

pub(super) fn artifact_view(
    build: &ReviewBuild,
    latest_attempt: Option<&ReviewOperationAttempt>,
    artifacts: Option<&VerifiedArtifactSet>,
    cleanup_job: Option<&CleanupJob>,
    cleanup_effects: &[CleanupEffect],
) -> Result<ArtifactStateView, String> {
    let Some(artifact_id) = &build.current_artifact_set_id else {
        return Ok(
            match latest_attempt.and_then(|attempt| attempt.failure.as_ref()) {
                Some(failure)
                    if matches!(
                        failure.stage,
                        OperationStage::ArtifactVerification | OperationStage::ArtifactPromotion
                    ) =>
                {
                    ArtifactStateView::PromotionFailed {
                        summary: failure.message.clone(),
                    }
                }
                _ => ArtifactStateView::NotProduced,
            },
        );
    };
    let artifacts = artifacts
        .filter(|artifacts| &artifacts.id == artifact_id)
        .ok_or_else(|| "The build references an unavailable artifact manifest.".to_string())?;
    if let Some(job) = cleanup_job {
        let removed_resource = job.resources.iter().find_map(|resource| match resource {
            CleanupResource::ArtifactSet {
                id,
                artifact_set_id,
                ..
            } if artifact_set_id == &artifacts.id => Some(id),
            _ => None,
        });
        if let Some(resource_id) = removed_resource {
            let removed = cleanup_effects.iter().any(|effect| {
                effect.resource_id == *resource_id
                    && matches!(
                        effect.disposition,
                        CleanupDisposition::Removed | CleanupDisposition::AlreadyAbsent
                    )
            });
            if removed {
                return Ok(ArtifactStateView::Removed {
                    artifact_set_id: artifacts.id.as_str().to_owned(),
                    removed_at: job.settled_at.unwrap_or(job.created_at).to_rfc3339(),
                });
            }
        }
    }
    Ok(ArtifactStateView::Available {
        artifact_set_id: artifacts.id.as_str().to_owned(),
        file_count: artifacts.files.len(),
        manifest_hash: artifacts.manifest_hash.as_str().to_owned(),
        storage_label: format!("Worktree Review AppData/{}", artifacts.storage_key.as_str()),
    })
}

pub(super) fn cleanup_presentation_view(presentation: CleanupPresentation) -> CleanupStateView {
    match presentation.state {
        CleanupJobState::Planned => CleanupStateView::Eligible {
            reason: presentation.summary,
        },
        CleanupJobState::Running => CleanupStateView::Running {
            cleanup_job_id: presentation.job_id.as_str().to_owned(),
            completed_effects: presentation
                .resources
                .iter()
                .filter(|resource| {
                    resource
                        .disposition
                        .is_some_and(CleanupDisposition::is_terminal)
                })
                .count(),
            total_effects: presentation.resources.len(),
        },
        CleanupJobState::AttentionRequired => CleanupStateView::Failed {
            cleanup_job_id: presentation.job_id.as_str().to_owned(),
            summary: presentation.summary,
        },
        CleanupJobState::Completed => CleanupStateView::Complete {
            cleanup_receipt_id: presentation.job_id.as_str().to_owned(),
            completed_at: presentation
                .completed_at
                .unwrap_or_else(Utc::now)
                .to_rfc3339(),
            summary: presentation.summary,
        },
        CleanupJobState::NotEligible => CleanupStateView::NotEligible {
            reason: match presentation.eligibility {
                CleanupEligibility::BuildRunning => "The running build was left unchanged.".into(),
                CleanupEligibility::RetentionProtected => presentation.summary,
                CleanupEligibility::BorrowedResource => {
                    "Borrowed worktree resources are never removed by build cleanup.".into()
                }
                CleanupEligibility::AlreadyCleaned => presentation.summary,
                CleanupEligibility::SourceUnverified => {
                    "Cleanup authority could not be verified; no effect was applied.".into()
                }
                CleanupEligibility::Eligible => presentation.summary,
            },
        },
    }
}

pub(super) fn cleanup_view(
    build: &ReviewBuild,
    latest_attempt: Option<&ReviewOperationAttempt>,
) -> CleanupStateView {
    match latest_attempt {
        Some(attempt)
            if matches!(
                attempt.execution,
                OperationExecutionState::Pending | OperationExecutionState::Running
            ) =>
        {
            CleanupStateView::NotEligible {
                reason: "An active build is never a cleanup candidate.".into(),
            }
        }
        Some(attempt)
            if attempt.execution == OperationExecutionState::Completed
                && attempt.verdict == OperationVerdict::Passed =>
        {
            CleanupStateView::Retained {
                policy: "Newest successful build for this logical source is retained.".into(),
            }
        }
        Some(attempt) if attempt.execution == OperationExecutionState::Completed => {
            CleanupStateView::Eligible {
                reason: "This terminal build can be evaluated by the retention policy.".into(),
            }
        }
        _ => CleanupStateView::NotEligible {
            reason: match build.lifecycle {
                BuildLifecycle::UnverifiedLegacy => {
                    "Legacy build evidence is unverified and cannot authorize cleanup."
                }
                _ => "No terminal build verdict authorizes cleanup.",
            }
            .into(),
        },
    }
}

fn source_view(branch_ref: &BranchRef, selection: &ReviewSourceSelection) -> BuildSourceInput {
    match selection {
        ReviewSourceSelection::ExistingWorktree {
            association_id,
            expected_head,
            captured_state_fingerprint,
        } => BuildSourceInput::ExistingWorktree {
            association_id: association_id.as_str().to_owned(),
            expected_head: expected_head.as_str().to_owned(),
            state_fingerprint: captured_state_fingerprint.as_str().to_owned(),
        },
        ReviewSourceSelection::WorktreeSnapshot {
            association_id,
            baseline_object,
            captured_state_fingerprint,
        } => BuildSourceInput::WorktreeSnapshot {
            association_id: association_id.as_str().to_owned(),
            base_object_id: baseline_object.as_str().to_owned(),
            state_fingerprint: captured_state_fingerprint.as_str().to_owned(),
        },
        ReviewSourceSelection::BranchCommit { selected_object } => BuildSourceInput::BranchCommit {
            branch_ref: branch_ref.as_str().to_owned(),
            object_id: selected_object.as_str().to_owned(),
        },
    }
}

fn attempt_view(attempt: &ReviewOperationAttempt) -> ReviewOperationAttemptView {
    let stage = attempt
        .failure
        .as_ref()
        .map(|failure| failure.stage)
        .or(attempt.active_stage)
        .map(OperationStage::as_str)
        .unwrap_or("requested")
        .to_owned();
    ReviewOperationAttemptView {
        attempt_id: attempt.id.as_str().to_owned(),
        execution_state: attempt.execution.as_str().to_owned(),
        verdict: match (attempt.execution, attempt.verdict) {
            (OperationExecutionState::Pending | OperationExecutionState::Running, _) => {
                "not_evaluated"
            }
            (_, OperationVerdict::Passed) => "passed",
            (_, OperationVerdict::Failed) => "failed",
            _ => "unknown",
        }
        .into(),
        stage: stage.clone(),
        started_at: attempt
            .started_at
            .unwrap_or(attempt.requested_at)
            .to_rfc3339(),
        completed_at: attempt.completed_at.map(|time| time.to_rfc3339()),
        failure: attempt
            .failure
            .as_ref()
            .map(|failure| BuildAttemptFailureView {
                category: failure_category(failure.category).into(),
                stage,
                summary: failure.message.clone(),
            }),
    }
}

fn failure_category(category: OperationFailureCategory) -> &'static str {
    match category {
        OperationFailureCategory::SourceChanged => "source_changed",
        OperationFailureCategory::SourceUnavailable
        | OperationFailureCategory::ProvisioningFailed => "provisioning",
        OperationFailureCategory::ToolchainUnavailable => "toolchain",
        OperationFailureCategory::CommandFailed | OperationFailureCategory::Internal => "build",
        OperationFailureCategory::ArtifactMissing | OperationFailureCategory::ArtifactInvalid => {
            "artifact"
        }
        OperationFailureCategory::Interrupted => "interrupted",
    }
}
