use super::{
    BranchRef, GitObjectId, RepositoryId, ReviewBuildId, WorkspaceId, WorktreeAssociationId,
    WorktreeId, WorktreeLocation,
};
use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewRepository {
    pub(crate) id: RepositoryId,
    pub(crate) label: String,
    pub(crate) first_seen_at: DateTime<Utc>,
    pub(crate) last_seen_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewBranch {
    pub(crate) repository_id: RepositoryId,
    pub(crate) branch_ref: BranchRef,
    pub(crate) observed_tip: GitObjectId,
    pub(crate) observed_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorktreeAssociationProvenance {
    GitAttachedBranch,
    UserAssociatedDetached,
    ProductCreated,
    LegacyUnverified,
}

impl WorktreeAssociationProvenance {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::GitAttachedBranch => "git_attached_branch",
            Self::UserAssociatedDetached => "user_associated_detached",
            Self::ProductCreated => "product_created",
            Self::LegacyUnverified => "legacy_unverified",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "git_attached_branch" => Some(Self::GitAttachedBranch),
            "user_associated_detached" => Some(Self::UserAssociatedDetached),
            "product_created" => Some(Self::ProductCreated),
            "legacy_unverified" => Some(Self::LegacyUnverified),
            _ => None,
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum AssociationBaselineKind {
    CreatedAtObject,
    ObservedAtAssociation,
    UserSupplied,
    LegacyUnknown,
}

impl AssociationBaselineKind {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::CreatedAtObject => "created_at_object",
            Self::ObservedAtAssociation => "observed_at_association",
            Self::UserSupplied => "user_supplied",
            Self::LegacyUnknown => "legacy_unknown",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "created_at_object" => Some(Self::CreatedAtObject),
            "observed_at_association" => Some(Self::ObservedAtAssociation),
            "user_supplied" => Some(Self::UserSupplied),
            "legacy_unknown" => Some(Self::LegacyUnknown),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct AssociationBaseline {
    pub(crate) object_id: Option<GitObjectId>,
    pub(crate) kind: AssociationBaselineKind,
}

#[derive(Clone, Debug, Default, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeChangeSummary {
    pub(crate) staged_paths: u32,
    pub(crate) unstaged_paths: u32,
    pub(crate) untracked_paths: u32,
}

impl WorktreeChangeSummary {
    pub(crate) fn has_local_work(&self) -> bool {
        self.staged_paths > 0 || self.unstaged_paths > 0 || self.untracked_paths > 0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeObservedState {
    pub(crate) head: GitObjectId,
    pub(crate) commits_ahead_of_baseline: u32,
    pub(crate) commits_behind_baseline: u32,
    pub(crate) changes: WorktreeChangeSummary,
    pub(crate) detached_head: bool,
    pub(crate) reachable_from_associated_branch: bool,
    pub(crate) observed_at: DateTime<Utc>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorktreeAssociationLifecycle {
    Active,
    Missing,
    Moved,
    ForeignRepository,
    BranchMismatch,
    Unverified,
    Disassociated,
}

impl WorktreeAssociationLifecycle {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Missing => "missing",
            Self::Moved => "moved",
            Self::ForeignRepository => "foreign_repository",
            Self::BranchMismatch => "branch_mismatch",
            Self::Unverified => "unverified",
            Self::Disassociated => "disassociated",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "active" => Some(Self::Active),
            "missing" => Some(Self::Missing),
            "moved" => Some(Self::Moved),
            "foreign_repository" => Some(Self::ForeignRepository),
            "branch_mismatch" => Some(Self::BranchMismatch),
            "unverified" => Some(Self::Unverified),
            "disassociated" => Some(Self::Disassociated),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct WorktreeAssociation {
    pub(crate) id: WorktreeAssociationId,
    pub(crate) repository_id: RepositoryId,
    pub(crate) branch_ref: BranchRef,
    pub(crate) worktree_id: WorktreeId,
    /// An observed location, never a durable identity or cleanup authorization by itself.
    pub(crate) location: WorktreeLocation,
    pub(crate) provenance: WorktreeAssociationProvenance,
    pub(crate) baseline: AssociationBaseline,
    pub(crate) observed_state: WorktreeObservedState,
    pub(crate) lifecycle: WorktreeAssociationLifecycle,
    pub(crate) associated_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum ReviewSourceSelection {
    ExistingWorktree {
        association_id: WorktreeAssociationId,
        head_object_id: GitObjectId,
        /// Immutable trigger identity only; this is not retained-object or reachability evidence.
        virtual_commit_id: Option<GitObjectId>,
    },
    WorktreeSnapshot {
        association_id: WorktreeAssociationId,
        head_object_id: GitObjectId,
        captured_object_id: GitObjectId,
        /// Immutable trigger identity only; this is not retained-object or reachability evidence.
        virtual_commit_id: Option<GitObjectId>,
    },
    BranchCommit {
        selected_object: GitObjectId,
    },
}

impl ReviewSourceSelection {
    /// Exact object accepted by the server as this build's immutable source receipt.
    pub(crate) fn accepted_object(&self) -> &GitObjectId {
        match self {
            Self::ExistingWorktree {
                head_object_id,
                virtual_commit_id,
                ..
            } => virtual_commit_id.as_ref().unwrap_or(head_object_id),
            Self::WorktreeSnapshot {
                captured_object_id, ..
            } => captured_object_id,
            Self::BranchCommit {
                selected_object, ..
            } => selected_object,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct SourceBinding {
    pub(crate) repository_id: RepositoryId,
    pub(crate) branch_ref: BranchRef,
    pub(crate) selection: ReviewSourceSelection,
    pub(crate) workspace_id: WorkspaceId,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum WorkspaceOwnership {
    BorrowedExternal {
        association_id: WorktreeAssociationId,
    },
    ManagedBranchWorktree {
        association_id: WorktreeAssociationId,
    },
    OwnedBuildWorktree {
        build_id: ReviewBuildId,
    },
}

impl WorkspaceOwnership {
    pub(crate) fn as_str(&self) -> &'static str {
        match self {
            Self::BorrowedExternal { .. } => "borrowed_external",
            Self::ManagedBranchWorktree { .. } => "managed_branch_worktree",
            Self::OwnedBuildWorktree { .. } => "owned_build_worktree",
        }
    }

    pub(crate) fn is_removable_with_build(&self, build_id: &ReviewBuildId) -> bool {
        matches!(self, Self::OwnedBuildWorktree { build_id: owner } if owner == build_id)
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum WorkspaceLifecycle {
    Ready,
    Missing,
    RemovalPending,
    Removed,
    Unverified,
}

impl WorkspaceLifecycle {
    pub(crate) fn as_str(self) -> &'static str {
        match self {
            Self::Ready => "ready",
            Self::Missing => "missing",
            Self::RemovalPending => "removal_pending",
            Self::Removed => "removed",
            Self::Unverified => "unverified",
        }
    }

    pub(crate) fn parse(value: &str) -> Option<Self> {
        match value {
            "ready" => Some(Self::Ready),
            "missing" => Some(Self::Missing),
            "removal_pending" => Some(Self::RemovalPending),
            "removed" => Some(Self::Removed),
            "unverified" => Some(Self::Unverified),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub(crate) struct ReviewWorkspace {
    pub(crate) id: WorkspaceId,
    pub(crate) repository_id: RepositoryId,
    pub(crate) worktree_id: WorktreeId,
    /// An observed location. Deletion still requires ownership and containment evidence.
    pub(crate) location: WorktreeLocation,
    pub(crate) ownership: WorkspaceOwnership,
    pub(crate) lifecycle: WorkspaceLifecycle,
    pub(crate) created_at: DateTime<Utc>,
    pub(crate) updated_at: DateTime<Utc>,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn build_id(value: &str) -> ReviewBuildId {
        ReviewBuildId::new(value).unwrap()
    }

    #[test]
    fn only_the_owning_build_can_remove_an_owned_workspace() {
        let owner = build_id("owner");
        let ownership = WorkspaceOwnership::OwnedBuildWorktree {
            build_id: owner.clone(),
        };
        assert!(ownership.is_removable_with_build(&owner));
        assert!(!ownership.is_removable_with_build(&build_id("other")));
    }

    #[test]
    fn local_work_is_not_conflated_with_committed_divergence() {
        let summary = WorktreeChangeSummary {
            staged_paths: 0,
            unstaged_paths: 1,
            untracked_paths: 0,
        };
        assert!(summary.has_local_work());
    }

    #[test]
    fn accepted_source_object_prefers_a_server_captured_virtual_commit() {
        let head = GitObjectId::new("a".repeat(40)).unwrap();
        let captured = GitObjectId::new("b".repeat(40)).unwrap();
        let association = WorktreeAssociationId::new("association-one").unwrap();

        let direct = ReviewSourceSelection::ExistingWorktree {
            association_id: association.clone(),
            head_object_id: head.clone(),
            virtual_commit_id: Some(captured.clone()),
        };
        let snapshot = ReviewSourceSelection::WorktreeSnapshot {
            association_id: association,
            head_object_id: head,
            captured_object_id: captured.clone(),
            virtual_commit_id: Some(captured.clone()),
        };

        assert_eq!(direct.accepted_object(), &captured);
        assert_eq!(snapshot.accepted_object(), &captured);
    }
}
