//! Pure transport projection for repository, branch, and physical worktree facts.

use super::{
    build_service::ReviewBuildView,
    domain::{
        AssociationBaselineKind, ReviewRepository, WorktreeAssociation,
        WorktreeAssociationProvenance,
    },
    state::{
        ActiveBuildContextView, CapabilityReadinessStatus, CapabilityReadinessView,
        WorktreeReviewCapabilitiesView,
    },
};
use crate::repository_context::{
    BranchRef as ObservedBranch, CommitFacts, RepositoryIdentity, WorktreeObservation,
};
use serde::{Deserialize, Serialize};
use std::path::Path;

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ProductOverviewView {
    pub(crate) repositories: Vec<RepositoryView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) selected_repository_id: Option<String>,
    pub(crate) branches: Vec<BranchView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) active_build_context: Option<ActiveBuildContextView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryView {
    pub(crate) repository_id: String,
    pub(crate) name: String,
    pub(crate) location_label: String,
    pub(crate) readiness: RepositoryReadinessView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct RepositoryReadinessView {
    pub(crate) state: String,
    pub(crate) browse: CapabilityAvailabilityView,
    pub(crate) create_worktree: CapabilityAvailabilityView,
    pub(crate) build: CapabilityAvailabilityView,
    pub(crate) build_output_storage: CapabilityAvailabilityView,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum CapabilityAvailabilityView {
    Available,
    Unavailable { reason: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchView {
    pub(crate) repository_id: String,
    pub(crate) branch_ref: String,
    pub(crate) display_name: String,
    pub(crate) tip: CommitView,
    pub(crate) ahead_of_default: usize,
    pub(crate) behind_default: usize,
    pub(crate) associated_worktree_count: usize,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct CommitView {
    pub(crate) object_id: String,
    pub(crate) abbreviated_object_id: String,
    pub(crate) subject: String,
    pub(crate) author: String,
    pub(crate) committed_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchDetailView {
    pub(crate) branch: BranchView,
    pub(crate) worktrees: Vec<AssociatedWorktreeView>,
    pub(crate) association_candidates: Vec<AssociationCandidateView>,
    pub(crate) builds: Vec<ReviewBuildView>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct BranchHistoryPageView {
    pub(crate) commits: Vec<CommitView>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub(crate) next_cursor: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssociatedWorktreeView {
    pub(crate) association_id: String,
    pub(crate) worktree_id: String,
    pub(crate) branch_ref: String,
    pub(crate) name: String,
    pub(crate) location_label: String,
    pub(crate) provenance: String,
    pub(crate) ownership: WorkspaceOwnershipView,
    pub(crate) baseline: BaselineView,
    pub(crate) current_head: CommitView,
    pub(crate) changes: WorktreeChangesView,
    pub(crate) detached_head: bool,
    pub(crate) branch_reachability: String,
    pub(crate) availability: WorktreeAvailabilityView,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkspaceOwnershipView {
    BorrowedExternal,
    ManagedBranchWorktree,
    OwnedBuildWorktree,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum BaselineView {
    CreatedAt { commit: CommitView },
    ObservedAtAssociation { commit: CommitView },
    UserSelected { commit: CommitView },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeChangesView {
    pub(crate) commits_ahead_of_baseline: u32,
    pub(crate) commits_behind_baseline: u32,
    pub(crate) staged_files: u32,
    pub(crate) unstaged_files: u32,
    pub(crate) untracked_files: u32,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(tag = "state", rename_all = "snake_case")]
pub(crate) enum WorktreeAvailabilityView {
    Available,
    Missing { detail: String },
    BranchMismatch { detail: String },
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssociationCandidateView {
    pub(crate) worktree_id: String,
    pub(crate) name: String,
    pub(crate) location_label: String,
    pub(crate) current_head: CommitView,
    pub(crate) detached_head: bool,
    pub(crate) association_reason: String,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssociateWorktreeInput {
    pub(crate) repository_id: String,
    pub(crate) branch_ref: String,
    pub(crate) worktree_id: String,
    pub(crate) baseline: AssociateBaselineInput,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum AssociateBaselineInput {
    ObservedCurrentHead,
    SelectedCommit { object_id: String },
}

pub(super) fn repository_view_with_readiness(
    repository: &RepositoryIdentity,
    readiness: RepositoryReadinessView,
) -> RepositoryView {
    RepositoryView {
        repository_id: repository.id.as_str().to_owned(),
        name: repository_name(repository.top_level.path()),
        location_label: repository.top_level.path().to_string_lossy().into_owned(),
        readiness,
    }
}

pub(super) fn registered_repository_view(
    repository: &ReviewRepository,
    readiness: RepositoryReadinessView,
) -> RepositoryView {
    RepositoryView {
        repository_id: repository.id.as_str().to_owned(),
        name: repository.label.clone(),
        location_label: repository.anchor_root.to_string_lossy().into_owned(),
        readiness,
    }
}

pub(super) fn available_repository_readiness() -> RepositoryReadinessView {
    RepositoryReadinessView {
        state: "ready".into(),
        browse: CapabilityAvailabilityView::Available,
        create_worktree: CapabilityAvailabilityView::Available,
        build: CapabilityAvailabilityView::Available,
        build_output_storage: CapabilityAvailabilityView::Available,
    }
}

pub(super) fn repository_readiness(
    capabilities: &WorktreeReviewCapabilitiesView,
) -> RepositoryReadinessView {
    let browse = capability_availability(&capabilities.repository_browsing);
    let create_worktree = browse.clone();
    let build = if matches!(&browse, CapabilityAvailabilityView::Available) {
        capability_availability(&capabilities.build_runtime)
    } else {
        browse.clone()
    };
    let build_output_storage = capability_availability(&capabilities.build_output_storage);
    let state = if !matches!(&browse, CapabilityAvailabilityView::Available) {
        "unavailable"
    } else if !matches!(&build, CapabilityAvailabilityView::Available)
        || !matches!(&build_output_storage, CapabilityAvailabilityView::Available)
    {
        "degraded"
    } else {
        "ready"
    };
    RepositoryReadinessView {
        state: state.into(),
        browse,
        create_worktree,
        build,
        build_output_storage,
    }
}

pub(super) fn commit_view(facts: CommitFacts) -> CommitView {
    CommitView {
        object_id: facts.object_id.as_str().to_owned(),
        abbreviated_object_id: facts.abbreviated_id,
        subject: facts.subject,
        author: facts.author,
        committed_at: facts.committed_at,
    }
}

pub(super) fn branch_view(
    repository: &RepositoryIdentity,
    branch: &ObservedBranch,
    tip: CommitView,
    ahead_of_default: usize,
    behind_default: usize,
    associated_worktree_count: usize,
) -> BranchView {
    BranchView {
        repository_id: repository.id.as_str().to_owned(),
        branch_ref: branch.full_name.as_str().to_owned(),
        display_name: branch.full_name.display_name().to_owned(),
        tip,
        ahead_of_default,
        behind_default,
        associated_worktree_count,
    }
}

pub(super) fn baseline_view(
    kind: AssociationBaselineKind,
    commit: CommitView,
) -> Result<BaselineView, String> {
    match kind {
        AssociationBaselineKind::CreatedAtObject => Ok(BaselineView::CreatedAt { commit }),
        AssociationBaselineKind::ObservedAtAssociation => {
            Ok(BaselineView::ObservedAtAssociation { commit })
        }
        AssociationBaselineKind::UserSupplied => Ok(BaselineView::UserSelected { commit }),
        AssociationBaselineKind::LegacyUnknown => {
            Err("A legacy association requires explicit verification.".into())
        }
    }
}

pub(super) fn associated_worktree_view(
    association: &WorktreeAssociation,
    observation: Option<&WorktreeObservation>,
    name: String,
    location_label: String,
    ownership: WorkspaceOwnershipView,
    availability: WorktreeAvailabilityView,
    baseline: BaselineView,
    current_head: CommitView,
) -> AssociatedWorktreeView {
    AssociatedWorktreeView {
        association_id: association.id.as_str().to_owned(),
        worktree_id: association.worktree_id.as_str().to_owned(),
        branch_ref: association.branch_ref.as_str().to_owned(),
        name,
        location_label,
        provenance: match association.provenance {
            WorktreeAssociationProvenance::GitAttachedBranch => "git_branch_checkout",
            WorktreeAssociationProvenance::UserAssociatedDetached => {
                "user_associated_detached_checkout"
            }
            WorktreeAssociationProvenance::ProductCreated => "worktree_review_created",
            WorktreeAssociationProvenance::LegacyUnverified => "user_associated_detached_checkout",
        }
        .into(),
        ownership,
        baseline,
        current_head,
        changes: WorktreeChangesView {
            commits_ahead_of_baseline: association.observed_state.commits_ahead_of_baseline,
            commits_behind_baseline: association.observed_state.commits_behind_baseline,
            staged_files: association.observed_state.changes.staged_paths,
            unstaged_files: association.observed_state.changes.unstaged_paths,
            untracked_files: association.observed_state.changes.untracked_paths,
        },
        detached_head: observation
            .map(|worktree| worktree.head_ref.is_none())
            .unwrap_or(association.observed_state.detached_head),
        branch_reachability: if association.observed_state.reachable_from_associated_branch {
            "reachable"
        } else {
            "not_reachable"
        }
        .into(),
        availability,
    }
}

pub(super) fn association_candidate_view(
    worktree: &WorktreeObservation,
    path: &Path,
    current_head: CommitView,
) -> AssociationCandidateView {
    AssociationCandidateView {
        worktree_id: worktree.id.as_str().to_owned(),
        name: repository_name(path),
        location_label: path.to_string_lossy().into_owned(),
        current_head,
        detached_head: worktree.head_ref.is_none(),
        association_reason: "This detached checkout is reachable from the selected branch but has no explicit Worktree Review association.".into(),
    }
}

pub(super) fn repository_name(path: &Path) -> String {
    path.file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("Repository")
        .to_owned()
}

fn capability_availability(readiness: &CapabilityReadinessView) -> CapabilityAvailabilityView {
    match readiness.status {
        CapabilityReadinessStatus::Ready | CapabilityReadinessStatus::NotEvaluated => {
            CapabilityAvailabilityView::Available
        }
        _ => CapabilityAvailabilityView::Unavailable {
            reason: readiness.message.clone(),
        },
    }
}
