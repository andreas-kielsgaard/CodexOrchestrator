//! Transport representation of durable build facts.
//!
//! This module has no storage or process authority. It maps already-loaded domain facts into the
//! Worktree Review product contract; orchestration and source materialization stay in the build
//! coordinator.

use super::{
    cleanup_service::CleanupPresentation,
    domain::{
        BranchRef, BuildAttention, BuildLifecycle, CleanupDisposition, CleanupEffect,
        CleanupEligibility, CleanupJob, CleanupJobState, CleanupResource, CommitSourceContext,
        OperationExecutionState, OperationFailureCategory, OperationStage, OperationVerdict,
        RetainedBuildOutput, ReviewBuild, ReviewOperationAttempt, ReviewSourceSelection,
        ReviewWorkspace,
    },
};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub(crate) struct CreateBuildInput {
    #[serde(default)]
    pub(crate) profile: crate::worktree_application::ApplicationBuildProfile,
    pub(crate) repository_id: String,
    pub(crate) branch_ref: Option<String>,
    pub(crate) name: String,
    pub(crate) source: CreateBuildSourceInput,
    pub(crate) workspace_plan: BuildWorkspacePlanInput,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum CreateBuildSourceInput {
    PhysicalWorktree {
        worktree_id: String,
        head_object_id: String,
        snapshot: bool,
    },
    ExactCommit {
        object_id: String,
        context: CommitSourceContext,
    },
    #[serde(rename = "existing_worktree")]
    LiveWorktree {
        association_id: String,
    },
    WorktreeSnapshot {
        association_id: String,
    },
    BranchCommit {
        branch_ref: String,
        object_id: String,
    },
}

#[derive(Clone, Debug, Deserialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase",
    deny_unknown_fields
)]
pub(crate) enum BuildWorkspacePlanInput {
    BorrowPhysicalWorktree {
        worktree_id: String,
    },
    BorrowSelectedWorktree {
        association_id: String,
    },
    CreateManagedBranchWorktree {
        branch_ref: String,
    },
    CreateOwnedBuildWorktree {
        originating_association_id: Option<String>,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "kind",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum ReviewBuildSourceView {
    PhysicalWorktree {
        worktree_id: String,
        head_object_id: String,
        captured_object_id: String,
        snapshot: bool,
    },
    ExactCommit {
        object_id: String,
    },
    #[serde(rename = "existing_worktree")]
    LiveWorktree {
        association_id: String,
        trigger_head_object_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        trigger_virtual_commit_id: Option<String>,
    },
    WorktreeSnapshot {
        association_id: String,
        head_object_id: String,
        captured_object_id: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        virtual_commit_id: Option<String>,
    },
    BranchCommit {
        branch_ref: String,
        object_id: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewBuildView {
    pub(crate) profile: Option<crate::worktree_application::ApplicationBuildProfile>,
    pub(crate) build_id: String,
    pub(crate) name: String,
    pub(crate) branch_ref: Option<String>,
    pub(crate) source: ReviewBuildSourceView,
    pub(crate) workspace: BuildWorkspaceView,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) latest_attempt: Option<ReviewOperationAttemptView>,
    pub(crate) output: BuildOutputStateView,
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
    pub(crate) outcome: String,
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
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
pub(crate) enum BuildOutputStateView {
    NotProduced,
    Unavailable {
        summary: String,
        #[serde(skip_serializing_if = "Option::is_none")]
        build_output_id: Option<String>,
    },
    Available {
        build_output_id: String,
        storage_label: String,
    },
    Removed {
        build_output_id: String,
        removed_at: String,
    },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(
    tag = "state",
    rename_all = "snake_case",
    rename_all_fields = "camelCase"
)]
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
    output: BuildOutputStateView,
    cleanup: CleanupStateView,
    attention: Option<BuildAttention>,
) -> ReviewBuildView {
    ReviewBuildView {
        profile: build.profile,
        build_id: build.id.as_str().to_owned(),
        name: build.name.as_str().to_owned(),
        branch_ref: build
            .source
            .branch_ref
            .as_ref()
            .map(|reference| reference.as_str().to_owned()),
        source: source_view(&build.source.branch_ref, &build.source.selection),
        workspace: BuildWorkspaceView {
            worktree_id: workspace.worktree_id.as_str().to_owned(),
            ownership: workspace.ownership.as_str().to_owned(),
            location_label: workspace.location.as_str().to_owned(),
            lifecycle: workspace.lifecycle.as_str().to_owned(),
        },
        latest_attempt: latest_attempt.map(attempt_view),
        output,
        cleanup,
        attention: attention.map(|attention| BuildAttentionView {
            category: attention.category.as_str().to_owned(),
            summary: attention.summary,
            recorded_at: attention.recorded_at.to_rfc3339(),
        }),
    }
}

pub(super) fn build_output_view(
    build: &ReviewBuild,
    output: Option<&RetainedBuildOutput>,
    output_exists: bool,
    cleanup_job: Option<&CleanupJob>,
    cleanup_effects: &[CleanupEffect],
) -> BuildOutputStateView {
    let Some(output_id) = &build.current_output_id else {
        return BuildOutputStateView::NotProduced;
    };
    let Some(output) = output.filter(|output| &output.id == output_id) else {
        return BuildOutputStateView::Unavailable {
            summary: "The retained build output record is unavailable.".into(),
            build_output_id: Some(output_id.as_str().to_owned()),
        };
    };
    if let Some(job) = cleanup_job {
        let removed_resource = job.resources.iter().find_map(|resource| match resource {
            CleanupResource::BuildOutput { id, output_id, .. } if output_id == &output.id => {
                Some(id)
            }
            _ => None,
        });
        if let Some(resource_id) = removed_resource {
            let removed = cleanup_effects.iter().find(|effect| {
                effect.resource_id == *resource_id
                    && matches!(
                        effect.disposition,
                        CleanupDisposition::Removed | CleanupDisposition::AlreadyAbsent
                    )
            });
            if let Some(removed) = removed {
                return BuildOutputStateView::Removed {
                    build_output_id: output.id.as_str().to_owned(),
                    removed_at: removed.recorded_at.to_rfc3339(),
                };
            }
        }
    }
    if !output_exists {
        return BuildOutputStateView::Unavailable {
            summary: "The retained build output or executable is no longer available.".into(),
            build_output_id: Some(output.id.as_str().to_owned()),
        };
    }
    BuildOutputStateView::Available {
        build_output_id: output.id.as_str().to_owned(),
        storage_label: format!("Worktree Review AppData/{}", output.storage_key.as_str()),
    }
}

pub(super) fn cleanup_presentation_view(
    presentation: CleanupPresentation,
) -> Result<CleanupStateView, String> {
    Ok(match presentation.state {
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
        CleanupJobState::Completed => {
            let completed_at = presentation.completed_at.ok_or_else(|| {
                "A completed cleanup job has no durable completion timestamp.".to_string()
            })?;
            CleanupStateView::Complete {
                cleanup_receipt_id: presentation.job_id.as_str().to_owned(),
                completed_at: completed_at.to_rfc3339(),
                summary: presentation.summary,
            }
        }
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
    })
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

fn source_view(
    branch_ref: &Option<BranchRef>,
    selection: &ReviewSourceSelection,
) -> ReviewBuildSourceView {
    match selection {
        ReviewSourceSelection::PhysicalWorktree {
            worktree_id,
            head_object_id,
            captured_object_id,
            snapshot,
        } => ReviewBuildSourceView::PhysicalWorktree {
            worktree_id: worktree_id.as_str().into(),
            head_object_id: head_object_id.as_str().into(),
            captured_object_id: captured_object_id.as_str().into(),
            snapshot: *snapshot,
        },
        ReviewSourceSelection::ExactCommit {
            selected_object, ..
        } => ReviewBuildSourceView::ExactCommit {
            object_id: selected_object.as_str().into(),
        },
        ReviewSourceSelection::LiveWorktree {
            association_id,
            trigger_head_object_id,
            trigger_virtual_commit_id,
        } => ReviewBuildSourceView::LiveWorktree {
            association_id: association_id.as_str().to_owned(),
            trigger_head_object_id: trigger_head_object_id.as_str().to_owned(),
            trigger_virtual_commit_id: trigger_virtual_commit_id
                .as_ref()
                .map(|object| object.as_str().to_owned()),
        },
        ReviewSourceSelection::WorktreeSnapshot {
            association_id,
            head_object_id,
            captured_object_id,
            virtual_commit_id,
        } => ReviewBuildSourceView::WorktreeSnapshot {
            association_id: association_id.as_str().to_owned(),
            head_object_id: head_object_id.as_str().to_owned(),
            captured_object_id: captured_object_id.as_str().to_owned(),
            virtual_commit_id: virtual_commit_id
                .as_ref()
                .map(|object| object.as_str().to_owned()),
        },
        ReviewSourceSelection::BranchCommit { selected_object } => {
            ReviewBuildSourceView::BranchCommit {
                branch_ref: branch_ref
                    .as_ref()
                    .expect("legacy branch commit has provenance")
                    .as_str()
                    .to_owned(),
                object_id: selected_object.as_str().to_owned(),
            }
        }
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
        outcome: match (attempt.execution, attempt.verdict) {
            (OperationExecutionState::Pending | OperationExecutionState::Running, _) => {
                "not_completed"
            }
            (_, OperationVerdict::Passed) => "succeeded",
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
        OperationFailureCategory::OutputMissing
        | OperationFailureCategory::OutputPublicationFailed => "output",
        OperationFailureCategory::Interrupted => "interrupted",
    }
}

#[cfg(test)]
mod source_contract_tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn accepts_only_source_selection_facts_owned_by_the_caller() {
        let input: CreateBuildInput = serde_json::from_value(json!({
            "repositoryId": "repository-one",
            "branchRef": "refs/heads/feature",
            "name": "Feature build",
            "source": {
                "kind": "worktree_snapshot",
                "associationId": "association-one"
            },
            "workspacePlan": {
                "kind": "create_owned_build_worktree",
                "originatingAssociationId": "association-one"
            }
        }))
        .expect("minimal source request should deserialize");

        assert_eq!(
            input.source,
            CreateBuildSourceInput::WorktreeSnapshot {
                association_id: "association-one".into(),
            }
        );
        assert!(matches!(
            input.workspace_plan,
            BuildWorkspacePlanInput::CreateOwnedBuildWorktree {
                originating_association_id: Some(ref association_id)
            } if association_id == "association-one"
        ));
    }

    #[test]
    fn rejects_client_claims_about_trigger_observation() {
        let stale_source_claim = json!({
            "repositoryId": "repository-one",
            "branchRef": "refs/heads/feature",
            "name": "Feature build",
            "source": {
                "kind": "existing_worktree",
                "associationId": "association-one",
                "expectedHead": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "stateFingerprint": "caller-owned-fingerprint"
            },
            "workspacePlan": {
                "kind": "borrow_selected_worktree",
                "associationId": "association-one"
            }
        });
        assert!(serde_json::from_value::<CreateBuildInput>(stale_source_claim).is_err());

        let stale_workspace_claim = json!({
            "repositoryId": "repository-one",
            "branchRef": "refs/heads/feature",
            "name": "Feature build",
            "source": {
                "kind": "branch_commit",
                "branchRef": "refs/heads/feature",
                "objectId": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            },
            "workspacePlan": {
                "kind": "create_owned_build_worktree",
                "objectId": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa"
            }
        });
        assert!(serde_json::from_value::<CreateBuildInput>(stale_workspace_claim).is_err());
    }

    #[test]
    fn source_receipt_discloses_server_observed_trigger_identity() {
        let live = ReviewBuildSourceView::LiveWorktree {
            association_id: "association-one".into(),
            trigger_head_object_id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            trigger_virtual_commit_id: Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into()),
        };
        assert_eq!(
            serde_json::to_value(live).expect("live source receipt should serialize"),
            json!({
                "kind": "existing_worktree",
                "associationId": "association-one",
                "triggerHeadObjectId": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "triggerVirtualCommitId": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            })
        );

        let view = ReviewBuildSourceView::WorktreeSnapshot {
            association_id: "association-one".into(),
            head_object_id: "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa".into(),
            captured_object_id: "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into(),
            virtual_commit_id: Some("bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb".into()),
        };

        assert_eq!(
            serde_json::to_value(view).expect("source receipt should serialize"),
            json!({
                "kind": "worktree_snapshot",
                "associationId": "association-one",
                "headObjectId": "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                "capturedObjectId": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb",
                "virtualCommitId": "bbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbbb"
            })
        );
    }

    #[test]
    fn output_and_cleanup_receipts_use_the_product_camel_case_contract() {
        let output = BuildOutputStateView::Available {
            build_output_id: "output-one".into(),
            storage_label: "Worktree Review AppData/output".into(),
        };
        assert_eq!(
            serde_json::to_value(output).expect("output state should serialize"),
            json!({
                "state": "available",
                "buildOutputId": "output-one",
                "storageLabel": "Worktree Review AppData/output"
            })
        );

        let cleanup = CleanupStateView::Running {
            cleanup_job_id: "cleanup-one".into(),
            completed_effects: 1,
            total_effects: 2,
        };
        assert_eq!(
            serde_json::to_value(cleanup).expect("cleanup state should serialize"),
            json!({
                "state": "running",
                "cleanupJobId": "cleanup-one",
                "completedEffects": 1,
                "totalEffects": 2
            })
        );
    }
}
